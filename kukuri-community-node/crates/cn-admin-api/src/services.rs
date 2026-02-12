use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::HashMap;
use std::time::Duration;
use utoipa::ToSchema;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Serialize, ToSchema)]
pub struct ServiceInfo {
    pub service: String,
    pub version: i64,
    pub config_json: Value,
    pub updated_at: i64,
    pub updated_by: String,
    pub health: Option<ServiceHealth>,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceHealth {
    pub status: String,
    pub checked_at: i64,
    pub details: Option<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceConfigResponse {
    pub service: String,
    pub version: i64,
    pub config_json: Value,
    pub updated_at: i64,
    pub updated_by: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateServiceConfigRequest {
    pub config_json: Value,
    pub expected_version: Option<i64>,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub target: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLog {
    pub audit_id: i64,
    pub actor_admin_user_id: String,
    pub action: String,
    pub target: String,
    pub diff_json: Option<Value>,
    pub request_id: Option<String>,
    pub created_at: i64,
}

pub async fn list_services(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<ServiceInfo>>> {
    require_admin(&state, &jar).await?;

    let rows = sqlx::query(
        "SELECT service, version, config_json, updated_at, updated_by FROM cn_admin.service_configs",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let health_rows = sqlx::query(
        "SELECT service, status, checked_at, details_json FROM cn_admin.service_health",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    let mut health_map = HashMap::new();
    for row in health_rows {
        let checked_at: chrono::DateTime<chrono::Utc> = row.try_get("checked_at")?;
        health_map.insert(
            row.try_get::<String, _>("service")?,
            ServiceHealth {
                status: row.try_get("status")?,
                checked_at: checked_at.timestamp(),
                details: row.try_get("details_json").ok(),
            },
        );
    }

    let mut services = Vec::new();
    for row in rows {
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        let service: String = row.try_get("service")?;
        services.push(ServiceInfo {
            service: service.clone(),
            version: row.try_get("version")?,
            config_json: row.try_get("config_json")?,
            updated_at: updated_at.timestamp(),
            updated_by: row.try_get("updated_by")?,
            health: health_map.remove(&service),
        });
    }

    Ok(Json(services))
}

pub async fn get_service_config(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(service): Path<String>,
) -> ApiResult<Json<ServiceConfigResponse>> {
    require_admin(&state, &jar).await?;

    let row = sqlx::query(
        "SELECT service, version, config_json, updated_at, updated_by FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "service not found",
        ));
    };

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(Json(ServiceConfigResponse {
        service: row.try_get("service")?,
        version: row.try_get("version")?,
        config_json: row.try_get("config_json")?,
        updated_at: updated_at.timestamp(),
        updated_by: row.try_get("updated_by")?,
    }))
}

pub async fn update_service_config(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(service): Path<String>,
    Json(payload): Json<UpdateServiceConfigRequest>,
) -> ApiResult<Json<ServiceConfigResponse>> {
    let admin = require_admin(&state, &jar).await?;

    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let row = sqlx::query("SELECT version FROM cn_admin.service_configs WHERE service = $1")
        .bind(&service)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut next_version = 1_i64;
    if let Some(row) = row {
        let current_version: i64 = row.try_get("version")?;
        if let Some(expected) = payload.expected_version {
            if expected != current_version {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "VERSION_MISMATCH",
                    "service config version mismatch",
                ));
            }
        }
        next_version = current_version + 1;
        sqlx::query(
            "UPDATE cn_admin.service_configs              SET config_json = $1, version = $2, updated_at = NOW(), updated_by = $3              WHERE service = $4",
        )
        .bind(&payload.config_json)
        .bind(next_version)
        .bind(&admin.admin_user_id)
        .bind(&service)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    } else {
        sqlx::query(
            "INSERT INTO cn_admin.service_configs              (service, version, config_json, updated_by)              VALUES ($1, 1, $2, $3)",
        )
        .bind(&service)
        .bind(&payload.config_json)
        .bind(&admin.admin_user_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "service_config.update",
        &format!("service:{service}"),
        Some(payload.config_json.clone()),
        None,
    )
    .await?;

    sqlx::query("SELECT pg_notify('cn_admin_config', $1)")
        .bind(format!("{service}:{next_version}"))
        .execute(&mut *tx)
        .await
        .ok();

    let row = sqlx::query(
        "SELECT service, version, config_json, updated_at, updated_by FROM cn_admin.service_configs WHERE service = $1",
    )
    .bind(&service)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    tx.commit().await.ok();

    Ok(Json(ServiceConfigResponse {
        service: row.try_get("service")?,
        version: row.try_get("version")?,
        config_json: row.try_get("config_json")?,
        updated_at: updated_at.timestamp(),
        updated_by: row.try_get("updated_by")?,
    }))
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditLog>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT audit_id, actor_admin_user_id, action, target, diff_json, request_id, created_at FROM cn_admin.audit_logs WHERE 1=1",
    );
    if let Some(action) = query.action {
        builder.push(" AND action = ");
        builder.push_bind(action);
    }
    if let Some(target) = query.target {
        builder.push(" AND target = ");
        builder.push_bind(target);
    }
    if let Some(since) = query.since {
        builder.push(" AND created_at >= to_timestamp(");
        builder.push_bind(since);
        builder.push(")");
    }
    builder.push(" ORDER BY created_at DESC");
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    builder.push(" LIMIT ");
    builder.push(limit.to_string());

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut logs = Vec::new();
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        logs.push(AuditLog {
            audit_id: row.try_get("audit_id")?,
            actor_admin_user_id: row.try_get("actor_admin_user_id")?,
            action: row.try_get("action")?,
            target: row.try_get("target")?,
            diff_json: row.try_get("diff_json").ok(),
            request_id: row.try_get("request_id").ok(),
            created_at: created_at.timestamp(),
        });
    }

    Ok(Json(logs))
}

pub fn spawn_health_poll(state: AppState, interval: Duration) {
    tokio::spawn(async move {
        loop {
            poll_health_once(&state).await;
            tokio::time::sleep(interval).await;
        }
    });
}

pub(crate) async fn poll_health_once(state: &AppState) {
    for (service, url) in state.health_targets.iter() {
        let result = state.health_client.get(url).send().await;
        let (status, details) = match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    (
                        "healthy".to_string(),
                        Some(serde_json::json!({ "status": resp.status().as_u16() })),
                    )
                } else {
                    (
                        "degraded".to_string(),
                        Some(serde_json::json!({ "status": resp.status().as_u16() })),
                    )
                }
            }
            Err(err) => (
                "unreachable".to_string(),
                Some(serde_json::json!({ "error": err.to_string() })),
            ),
        };

        let _ = sqlx::query(
            "INSERT INTO cn_admin.service_health              (service, status, checked_at, details_json)              VALUES ($1, $2, NOW(), $3)              ON CONFLICT (service) DO UPDATE SET status = EXCLUDED.status, checked_at = EXCLUDED.checked_at, details_json = EXCLUDED.details_json",
        )
        .bind(service)
        .bind(status)
        .bind(details)
        .execute(&state.pool)
        .await;
    }
}
