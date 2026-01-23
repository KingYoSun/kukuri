use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use chrono::TimeZone;
use sqlx::Row;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize)]
pub struct PolicyQuery {
    pub policy_type: Option<String>,
    pub locale: Option<String>,
}

#[derive(Deserialize)]
pub struct PolicyRequest {
    pub policy_type: String,
    pub version: String,
    pub locale: String,
    pub title: String,
    pub content_md: String,
}

#[derive(Deserialize)]
pub struct PolicyUpdateRequest {
    pub title: String,
    pub content_md: String,
}

#[derive(Deserialize)]
pub struct PublishRequest {
    pub effective_at: Option<i64>,
}

#[derive(Serialize)]
pub struct PolicyResponse {
    pub policy_id: String,
    pub policy_type: String,
    pub version: String,
    pub locale: String,
    pub title: String,
    pub content_md: String,
    pub content_hash: String,
    pub published_at: Option<i64>,
    pub effective_at: Option<i64>,
    pub is_current: bool,
}

pub async fn list_policies(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<PolicyQuery>,
) -> ApiResult<Json<Vec<PolicyResponse>>> {
    require_admin(&state, &jar).await?;

    let mut builder = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        "SELECT policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current FROM cn_admin.policies WHERE 1=1",
    );
    if let Some(policy_type) = query.policy_type {
        builder.push(" AND type = ");
        builder.push_bind(policy_type);
    }
    if let Some(locale) = query.locale {
        builder.push(" AND locale = ");
        builder.push_bind(locale);
    }
    builder.push(" ORDER BY created_at DESC");

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut policies = Vec::new();
    for row in rows {
        policies.push(row_to_policy(row)?);
    }

    Ok(Json(policies))
}

pub async fn create_policy(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(payload): Json<PolicyRequest>,
) -> ApiResult<Json<PolicyResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let policy_id = policy_id(&payload.policy_type, &payload.version, &payload.locale);
    let content_hash = blake3::hash(payload.content_md.as_bytes()).to_hex().to_string();

    sqlx::query(
        "INSERT INTO cn_admin.policies          (policy_id, type, version, locale, title, content_md, content_hash)          VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&policy_id)
    .bind(&payload.policy_type)
    .bind(&payload.version)
    .bind(&payload.locale)
    .bind(&payload.title)
    .bind(&payload.content_md)
    .bind(&content_hash)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "policy.create",
        &format!("policy:{policy_id}"),
        Some(serde_json::json!({ "title": payload.title })),
        None,
    )
    .await
    .ok();

    let row = sqlx::query(
        "SELECT policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current          FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&policy_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(row_to_policy(row)?))
}

pub async fn update_policy(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(policy_id): Path<String>,
    Json(payload): Json<PolicyUpdateRequest>,
) -> ApiResult<Json<PolicyResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let content_hash = blake3::hash(payload.content_md.as_bytes()).to_hex().to_string();

    let result = sqlx::query(
        "UPDATE cn_admin.policies          SET title = $1, content_md = $2, content_hash = $3, updated_at = NOW()          WHERE policy_id = $4",
    )
    .bind(&payload.title)
    .bind(&payload.content_md)
    .bind(&content_hash)
    .bind(&policy_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "policy not found"));
    }

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "policy.update",
        &format!("policy:{policy_id}"),
        Some(serde_json::json!({ "title": payload.title })),
        None,
    )
    .await
    .ok();

    let row = sqlx::query(
        "SELECT policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current          FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&policy_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(row_to_policy(row)?))
}

pub async fn publish_policy(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(policy_id): Path<String>,
    Json(payload): Json<PublishRequest>,
) -> ApiResult<Json<PolicyResponse>> {
    let admin = require_admin(&state, &jar).await?;
    let effective_at = payload
        .effective_at
        .map(|ts| chrono::Utc.timestamp_opt(ts, 0).single())
        .flatten()
        .unwrap_or_else(chrono::Utc::now);

    let result = sqlx::query(
        "UPDATE cn_admin.policies          SET published_at = COALESCE(published_at, NOW()), effective_at = $1, updated_at = NOW()          WHERE policy_id = $2",
    )
    .bind(effective_at)
    .bind(&policy_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "policy not found"));
    }

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "policy.publish",
        &format!("policy:{policy_id}"),
        None,
        None,
    )
    .await
    .ok();

    let row = sqlx::query(
        "SELECT policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current          FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&policy_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(row_to_policy(row)?))
}

pub async fn make_current_policy(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(policy_id): Path<String>,
) -> ApiResult<Json<PolicyResponse>> {
    let admin = require_admin(&state, &jar).await?;

    let row = sqlx::query(
        "SELECT type, locale FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&policy_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "policy not found"));
    };
    let policy_type: String = row.try_get("type")?;
    let locale: String = row.try_get("locale")?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_admin.policies SET is_current = FALSE WHERE type = $1 AND locale = $2",
    )
    .bind(&policy_type)
    .bind(&locale)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_admin.policies          SET is_current = TRUE, published_at = COALESCE(published_at, NOW()), effective_at = COALESCE(effective_at, NOW())          WHERE policy_id = $1",
    )
    .bind(&policy_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    tx.commit().await.ok();

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "policy.make_current",
        &format!("policy:{policy_id}"),
        None,
        None,
    )
    .await
    .ok();

    let row = sqlx::query(
        "SELECT policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current          FROM cn_admin.policies WHERE policy_id = $1",
    )
    .bind(&policy_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(row_to_policy(row)?))
}

fn row_to_policy(row: sqlx::postgres::PgRow) -> ApiResult<PolicyResponse> {
    let published_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("published_at")?;
    let effective_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("effective_at")?;
    Ok(PolicyResponse {
        policy_id: row.try_get("policy_id")?,
        policy_type: row.try_get("type")?,
        version: row.try_get("version")?,
        locale: row.try_get("locale")?,
        title: row.try_get("title")?,
        content_md: row.try_get("content_md")?,
        content_hash: row.try_get("content_hash")?,
        published_at: published_at.map(|value| value.timestamp()),
        effective_at: effective_at.map(|value| value.timestamp()),
        is_current: row.try_get("is_current")?,
    })
}

fn policy_id(policy_type: &str, version: &str, locale: &str) -> String {
    format!("{policy_type}:{version}:{locale}")
}
