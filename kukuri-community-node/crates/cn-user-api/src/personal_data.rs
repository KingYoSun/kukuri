use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::fs;
use std::path::PathBuf;

use crate::auth::require_auth;
use crate::policies::require_consents;
use crate::{ApiError, ApiResult, AppState};

#[derive(Serialize)]
pub struct ExportRequestResponse {
    pub export_request_id: String,
    pub status: String,
    pub download_token: Option<String>,
    pub download_expires_at: Option<i64>,
}

#[derive(Serialize)]
pub struct DeletionRequestResponse {
    pub deletion_request_id: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    pub token: String,
}

pub async fn create_export_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<ExportRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    cleanup_expired_exports(&state).await?;

    let export_request_id = uuid::Uuid::new_v4().to_string();
    let download_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let path = build_export_path(&state.export_dir, &export_request_id);

    let payload = build_export_payload(&state, &auth.pubkey).await?;
    let contents = serde_json::to_vec_pretty(&payload)
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "EXPORT_ERROR", err.to_string()))?;
    fs::write(&path, contents)
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "EXPORT_ERROR", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_user.personal_data_export_requests          (export_request_id, requester_pubkey, status, completed_at, download_token, download_expires_at, file_path)          VALUES ($1, $2, 'completed', NOW(), $3, $4, $5)",
    )
    .bind(&export_request_id)
    .bind(&auth.pubkey)
    .bind(&download_token)
    .bind(expires_at)
    .bind(path.to_string_lossy().to_string())
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(ExportRequestResponse {
        export_request_id,
        status: "completed".to_string(),
        download_token: Some(download_token),
        download_expires_at: Some(expires_at.timestamp()),
    }))
}

pub async fn get_export_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(export_request_id): Path<String>,
) -> ApiResult<Json<ExportRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT status, download_token, download_expires_at FROM cn_user.personal_data_export_requests          WHERE export_request_id = $1 AND requester_pubkey = $2",
    )
    .bind(&export_request_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "export not found"));
    };

    let status: String = row.try_get("status")?;
    let token: Option<String> = row.try_get("download_token")?;
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("download_expires_at")?;

    Ok(Json(ExportRequestResponse {
        export_request_id,
        status,
        download_token: token,
        download_expires_at: expires_at.map(|value| value.timestamp()),
    }))
}

pub async fn download_export(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(export_request_id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> ApiResult<Response> {
    let auth = require_auth(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT download_token, download_expires_at, file_path FROM cn_user.personal_data_export_requests          WHERE export_request_id = $1 AND requester_pubkey = $2",
    )
    .bind(&export_request_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "export not found"));
    };

    let token: Option<String> = row.try_get("download_token")?;
    if token.as_deref() != Some(query.token.as_str()) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_TOKEN",
            "invalid download token",
        ));
    }

    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("download_expires_at")?;
    if let Some(expires_at) = expires_at {
        if chrono::Utc::now() > expires_at {
            return Err(ApiError::new(
                StatusCode::GONE,
                "EXPIRED",
                "download expired",
            ));
        }
    }

    let file_path: Option<String> = row.try_get("file_path")?;
    let Some(file_path) = file_path else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "file missing"));
    };
    let data = fs::read(&file_path)
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "EXPORT_ERROR", err.to_string()))?;

    let mut response = axum::response::Response::new(axum::body::Body::from(data));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    response.headers_mut().insert(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}.json\"", export_request_id)
            .parse()
            .unwrap(),
    );
    Ok(response)
}

pub async fn create_deletion_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<DeletionRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    let deletion_request_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_user.personal_data_deletion_requests          (deletion_request_id, requester_pubkey, status)          VALUES ($1, $2, 'queued')",
    )
    .bind(&deletion_request_id)
    .bind(&auth.pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.subscriber_accounts SET status = 'deleting', updated_at = NOW() WHERE subscriber_pubkey = $1",
    )
    .bind(&auth.pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    perform_deletion(&state, &auth.pubkey, &deletion_request_id).await?;

    Ok(Json(DeletionRequestResponse {
        deletion_request_id,
        status: "completed".to_string(),
    }))
}

pub async fn get_deletion_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(deletion_request_id): Path<String>,
) -> ApiResult<Json<DeletionRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT status FROM cn_user.personal_data_deletion_requests          WHERE deletion_request_id = $1 AND requester_pubkey = $2",
    )
    .bind(&deletion_request_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "deletion not found"));
    };
    let status: String = row.try_get("status")?;
    Ok(Json(DeletionRequestResponse {
        deletion_request_id,
        status,
    }))
}

fn build_export_path(base: &PathBuf, export_request_id: &str) -> PathBuf {
    let mut path = base.clone();
    path.push(format!("{export_request_id}.json"));
    path
}

async fn build_export_payload(state: &AppState, pubkey: &str) -> ApiResult<serde_json::Value> {
    let consents = sqlx::query(
        "SELECT policy_id, accepted_at FROM cn_user.policy_consents WHERE accepter_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let accepted_at: chrono::DateTime<chrono::Utc> = row.try_get("accepted_at").ok()?;
        Some(json!({
            "policy_id": row.try_get::<String, _>("policy_id").ok()?,
            "accepted_at": accepted_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let subscriptions = sqlx::query(
        "SELECT topic_id, status, started_at, ended_at FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let started_at: chrono::DateTime<chrono::Utc> = row.try_get("started_at").ok()?;
        let ended_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("ended_at").ok();
        Some(json!({
            "topic_id": row.try_get::<String, _>("topic_id").ok()?,
            "status": row.try_get::<String, _>("status").ok()?,
            "started_at": started_at.timestamp(),
            "ended_at": ended_at.map(|value| value.timestamp())
        }))
    })
    .collect::<Vec<_>>();

    let usage_events = sqlx::query(
        "SELECT metric, day, units, outcome, created_at FROM cn_user.usage_events WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").ok()?;
        Some(json!({
            "metric": row.try_get::<String, _>("metric").ok()?,
            "day": row.try_get::<chrono::NaiveDate, _>("day").ok()?.to_string(),
            "units": row.try_get::<i64, _>("units").ok()?,
            "outcome": row.try_get::<String, _>("outcome").ok()?,
            "created_at": created_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let reports = sqlx::query(
        "SELECT target, reason, created_at FROM cn_user.reports WHERE reporter_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").ok()?;
        Some(json!({
            "target": row.try_get::<String, _>("target").ok()?,
            "reason": row.try_get::<String, _>("reason").ok()?,
            "created_at": created_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let memberships = sqlx::query(
        "SELECT topic_id, scope, status, joined_at FROM cn_user.topic_memberships WHERE pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let joined_at: chrono::DateTime<chrono::Utc> = row.try_get("joined_at").ok()?;
        Some(json!({
            "topic_id": row.try_get::<String, _>("topic_id").ok()?,
            "scope": row.try_get::<String, _>("scope").ok()?,
            "status": row.try_get::<String, _>("status").ok()?,
            "joined_at": joined_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let events = sqlx::query(
        "SELECT raw_json FROM cn_relay.events WHERE pubkey = $1 AND is_deleted = FALSE LIMIT 1000",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| row.try_get::<serde_json::Value, _>("raw_json").ok())
    .collect::<Vec<_>>();

    Ok(json!({
        "pubkey": pubkey,
        "generated_at": chrono::Utc::now().timestamp(),
        "consents": consents,
        "subscriptions": subscriptions,
        "usage_events": usage_events,
        "reports": reports,
        "memberships": memberships,
        "events": events
    }))
}

async fn cleanup_expired_exports(state: &AppState) -> ApiResult<()> {
    let rows = sqlx::query(
        "SELECT export_request_id, file_path FROM cn_user.personal_data_export_requests          WHERE download_expires_at IS NOT NULL AND download_expires_at < NOW()",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    for row in rows {
        let export_request_id: String = row.try_get("export_request_id")?;
        let file_path: Option<String> = row.try_get("file_path")?;
        if let Some(file_path) = file_path {
            let _ = fs::remove_file(&file_path);
        }
        sqlx::query(
            "UPDATE cn_user.personal_data_export_requests              SET download_token = NULL, download_expires_at = NULL, file_path = NULL              WHERE export_request_id = $1",
        )
        .bind(&export_request_id)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    Ok(())
}

async fn perform_deletion(
    state: &AppState,
    pubkey: &str,
    deletion_request_id: &str,
) -> ApiResult<()> {
    let anon = anonymize_pubkey(&state.hmac_secret, pubkey);

    sqlx::query(
        "UPDATE cn_user.policy_consents          SET accepter_pubkey = $1, accepter_hmac = $1, ip = NULL, user_agent = NULL          WHERE accepter_pubkey = $2",
    )
    .bind(&anon)
    .bind(pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.usage_events SET subscriber_pubkey = $1 WHERE subscriber_pubkey = $2",
    )
    .bind(&anon)
    .bind(pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("UPDATE cn_user.reports SET reporter_pubkey = $1 WHERE reporter_pubkey = $2")
        .bind(&anon)
        .bind(pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1")
        .bind(pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = $1")
        .bind(pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "DELETE FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1",
    )
    .bind(pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_user.topic_memberships WHERE pubkey = $1")
        .bind(pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_user.key_envelopes WHERE recipient_pubkey = $1")
        .bind(pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_user.subscriptions WHERE subscriber_pubkey = $1")
        .bind(pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.subscriber_accounts SET status = 'deleted', updated_at = NOW() WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.personal_data_deletion_requests SET status = 'completed', completed_at = NOW() WHERE deletion_request_id = $1",
    )
    .bind(deletion_request_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(())
}

fn anonymize_pubkey(secret: &[u8], pubkey: &str) -> String {
    let mut key = [0u8; 32];
    if secret.len() >= 32 {
        key.copy_from_slice(&secret[..32]);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let hash = blake3::keyed_hash(&key, pubkey.as_bytes());
    format!("hmac:{}", hash.to_hex())
}
