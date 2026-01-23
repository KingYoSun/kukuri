use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize)]
pub struct ReindexRequest {
    pub topic_id: Option<String>,
}

#[derive(Serialize)]
pub struct ReindexResponse {
    pub job_id: String,
    pub status: String,
}

pub async fn enqueue_reindex(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(payload): Json<ReindexRequest>,
) -> ApiResult<Json<ReindexResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let topic_id = if let Some(topic_id) = payload.topic_id.as_ref() {
        Some(
            cn_core::topic::normalize_topic_id(topic_id).map_err(|err| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string())
            })?,
        )
    } else {
        None
    };

    let job_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_index.reindex_jobs          (job_id, topic_id, status, requested_by)          VALUES ($1, $2, 'pending', $3)",
    )
    .bind(&job_id)
    .bind(&topic_id)
    .bind(&admin.admin_user_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let diff = json!({ "topic_id": topic_id });
    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "index.reindex.request",
        "index:reindex",
        Some(diff),
        None,
    )
    .await
    .ok();

    sqlx::query("NOTIFY cn_index_reindex, $1")
        .bind(&job_id)
        .execute(&state.pool)
        .await
        .ok();

    Ok(Json(ReindexResponse {
        job_id,
        status: "pending".to_string(),
    }))
}
