use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};
use utoipa::ToSchema;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize, ToSchema)]
pub struct RotateRequest {
    pub topic_id: String,
    pub scope: String,
}

#[derive(Serialize, ToSchema)]
pub struct RotateResponse {
    pub topic_id: String,
    pub scope: String,
    pub previous_epoch: i64,
    pub new_epoch: i64,
    pub recipients: usize,
}

#[derive(Deserialize, ToSchema)]
pub struct RevokeRequest {
    pub topic_id: String,
    pub scope: String,
    pub pubkey: String,
    pub reason: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RevokeResponse {
    pub topic_id: String,
    pub scope: String,
    pub revoked_pubkey: String,
    pub previous_epoch: i64,
    pub new_epoch: i64,
    pub recipients: usize,
}

#[derive(Deserialize, ToSchema)]
pub struct MembershipQuery {
    pub topic_id: Option<String>,
    pub scope: Option<String>,
    pub pubkey: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct MembershipRow {
    pub topic_id: String,
    pub scope: String,
    pub pubkey: String,
    pub status: String,
    pub joined_at: i64,
    pub revoked_at: Option<i64>,
    pub revoked_reason: Option<String>,
}

pub async fn list_memberships(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<MembershipQuery>,
) -> ApiResult<Json<Vec<MembershipRow>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT topic_id, scope, pubkey, status, joined_at, revoked_at, revoked_reason FROM cn_user.topic_memberships WHERE 1=1",
    );

    if let Some(topic_id) = query
        .topic_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND topic_id ILIKE ");
        builder.push_bind(format!("%{topic_id}%"));
    }

    if let Some(scope_raw) = query
        .scope
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let scope = cn_core::access_control::normalize_scope(scope_raw)
            .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string()))?;
        builder.push(" AND scope = ");
        builder.push_bind(scope);
    }

    if let Some(pubkey_raw) = query
        .pubkey
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if pubkey_raw.len() == 64 && pubkey_raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
            let pubkey = cn_core::access_control::normalize_pubkey(pubkey_raw).map_err(|err| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_PUBKEY", err.to_string())
            })?;
            builder.push(" AND pubkey = ");
            builder.push_bind(pubkey);
        } else {
            builder.push(" AND pubkey ILIKE ");
            builder.push_bind(format!("%{pubkey_raw}%"));
        }
    }

    if let Some(status) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND status = ");
        builder.push_bind(status.to_lowercase());
    }

    builder.push(" ORDER BY joined_at DESC");
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

    let mut memberships = Vec::new();
    for row in rows {
        let joined_at: chrono::DateTime<chrono::Utc> = row.try_get("joined_at")?;
        let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at")?;
        memberships.push(MembershipRow {
            topic_id: row.try_get("topic_id")?,
            scope: row.try_get("scope")?,
            pubkey: row.try_get("pubkey")?,
            status: row.try_get("status")?,
            joined_at: joined_at.timestamp(),
            revoked_at: revoked_at.map(|value| value.timestamp()),
            revoked_reason: row.try_get("revoked_reason")?,
        });
    }

    Ok(Json(memberships))
}

pub async fn rotate_epoch(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RotateRequest>,
) -> ApiResult<Json<RotateResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let topic_id = cn_core::topic::normalize_topic_id(&payload.topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    let scope = cn_core::access_control::normalize_scope(&payload.scope)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string()))?;

    let summary =
        cn_core::access_control::rotate_epoch(&state.pool, &state.node_keys, &topic_id, &scope)
            .await
            .map_err(|err| {
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ROTATE_ERROR",
                    err.to_string(),
                )
            })?;

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "access_control.rotate",
        &format!("access_control:{topic_id}:{scope}"),
        Some(json!({
            "previous_epoch": summary.previous_epoch,
            "new_epoch": summary.new_epoch,
            "recipients": summary.recipients
        })),
        None,
    )
    .await
    .ok();

    Ok(Json(RotateResponse {
        topic_id: summary.topic_id,
        scope: summary.scope,
        previous_epoch: summary.previous_epoch,
        new_epoch: summary.new_epoch,
        recipients: summary.recipients,
    }))
}

pub async fn revoke_member(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RevokeRequest>,
) -> ApiResult<Json<RevokeResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let topic_id = cn_core::topic::normalize_topic_id(&payload.topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    let scope = cn_core::access_control::normalize_scope(&payload.scope)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string()))?;
    let pubkey = cn_core::access_control::normalize_pubkey(&payload.pubkey)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_PUBKEY", err.to_string()))?;

    let result = cn_core::access_control::revoke_member_and_rotate(
        &state.pool,
        &state.node_keys,
        &topic_id,
        &scope,
        &pubkey,
        payload.reason.as_deref(),
    )
    .await
    .map_err(|err| {
        let message = err.to_string();
        if message.contains("membership not found") {
            ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", message)
        } else if message.contains("membership is not active") {
            ApiError::new(StatusCode::BAD_REQUEST, "INVALID_MEMBERSHIP", message)
        } else {
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "REVOKE_ERROR", message)
        }
    })?;

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "access_control.revoke",
        &format!("access_control:{topic_id}:{scope}:{pubkey}"),
        Some(json!({
            "reason": payload.reason,
            "previous_epoch": result.rotation.previous_epoch,
            "new_epoch": result.rotation.new_epoch,
            "recipients": result.rotation.recipients
        })),
        None,
    )
    .await
    .ok();

    Ok(Json(RevokeResponse {
        topic_id: result.topic_id,
        scope: result.scope,
        revoked_pubkey: result.revoked_pubkey,
        previous_epoch: result.rotation.previous_epoch,
        new_epoch: result.rotation.new_epoch,
        recipients: result.rotation.recipients,
    }))
}
