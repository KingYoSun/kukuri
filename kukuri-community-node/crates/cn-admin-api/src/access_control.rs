use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize)]
pub struct RotateRequest {
    pub topic_id: String,
    pub scope: String,
}

#[derive(Serialize)]
pub struct RotateResponse {
    pub topic_id: String,
    pub scope: String,
    pub previous_epoch: i64,
    pub new_epoch: i64,
    pub recipients: usize,
}

#[derive(Deserialize)]
pub struct RevokeRequest {
    pub topic_id: String,
    pub scope: String,
    pub pubkey: String,
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct RevokeResponse {
    pub topic_id: String,
    pub scope: String,
    pub revoked_pubkey: String,
    pub previous_epoch: i64,
    pub new_epoch: i64,
    pub recipients: usize,
}

pub async fn rotate_epoch(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RotateRequest>,
) -> ApiResult<Json<RotateResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let topic_id = cn_core::topic::normalize_topic_id(&payload.topic_id).map_err(|err| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string())
    })?;
    let scope = cn_core::access_control::normalize_scope(&payload.scope).map_err(|err| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string())
    })?;

    let summary = cn_core::access_control::rotate_epoch(
        &state.pool,
        &state.node_keys,
        &topic_id,
        &scope,
    )
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "ROTATE_ERROR", err.to_string()))?;

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
    let topic_id = cn_core::topic::normalize_topic_id(&payload.topic_id).map_err(|err| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string())
    })?;
    let scope = cn_core::access_control::normalize_scope(&payload.scope).map_err(|err| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string())
    })?;
    let pubkey = cn_core::access_control::normalize_pubkey(&payload.pubkey).map_err(|err| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_PUBKEY", err.to_string())
    })?;

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
