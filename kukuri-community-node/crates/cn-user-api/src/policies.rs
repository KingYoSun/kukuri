use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use cn_core::metrics;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::{require_auth, AuthContext};
use crate::{ApiError, ApiResult, AppState};

#[derive(Serialize)]
pub(crate) struct PolicySummary {
    policy_id: String,
    policy_type: String,
    version: String,
    locale: String,
    title: String,
    content_hash: String,
    url: String,
    published_at: Option<i64>,
    effective_at: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct PolicyDetail {
    policy_id: String,
    policy_type: String,
    version: String,
    locale: String,
    title: String,
    content_md: String,
    content_hash: String,
    published_at: Option<i64>,
    effective_at: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct ConsentStatusResponse {
    pubkey: String,
    consents: Vec<ConsentRecord>,
    missing: Vec<PolicySummary>,
}

#[derive(Serialize)]
struct ConsentRecord {
    policy_id: String,
    accepted_at: i64,
}

#[derive(Deserialize)]
pub struct PolicyQuery {
    locale: Option<String>,
}

#[derive(Deserialize)]
pub struct ConsentRequest {
    policy_ids: Option<Vec<String>>,
    accept_all_current: Option<bool>,
}

pub async fn get_current_policies(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<PolicySummary>>> {
    let rows = sqlx::query(
        "SELECT policy_id, type, version, locale, title, content_hash, published_at, effective_at          FROM cn_admin.policies          WHERE is_current = TRUE",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut policies = Vec::new();
    for row in rows {
        let policy_type: String = row.try_get("type")?;
        let version: String = row.try_get("version")?;
        let locale: String = row.try_get("locale")?;
        policies.push(PolicySummary {
            policy_id: row.try_get("policy_id")?,
            policy_type: policy_type.clone(),
            version: version.clone(),
            locale: locale.clone(),
            title: row.try_get("title")?,
            content_hash: row.try_get("content_hash")?,
            url: policy_url(&state.public_base_url, &policy_type, &version, &locale),
            published_at: row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("published_at")?
                .map(|value| value.timestamp()),
            effective_at: row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("effective_at")?
                .map(|value| value.timestamp()),
        });
    }

    Ok(Json(policies))
}

pub async fn get_policy_by_version(
    State(state): State<AppState>,
    Path((policy_type, version)): Path<(String, String)>,
    Query(query): Query<PolicyQuery>,
) -> ApiResult<Json<PolicyDetail>> {
    let locale = query.locale.unwrap_or_else(|| "ja-JP".to_string());
    let row = sqlx::query(
        "SELECT policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at          FROM cn_admin.policies          WHERE type = $1 AND version = $2 AND locale = $3",
    )
    .bind(&policy_type)
    .bind(&version)
    .bind(&locale)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "policy not found"));
    };

    Ok(Json(PolicyDetail {
        policy_id: row.try_get("policy_id")?,
        policy_type: row.try_get("type")?,
        version: row.try_get("version")?,
        locale: row.try_get("locale")?,
        title: row.try_get("title")?,
        content_md: row.try_get("content_md")?,
        content_hash: row.try_get("content_hash")?,
        published_at: row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("published_at")?
            .map(|value| value.timestamp()),
        effective_at: row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("effective_at")?
            .map(|value| value.timestamp()),
    }))
}

pub async fn get_consent_status(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<ConsentStatusResponse>> {
    let auth = require_auth(&state, &headers).await?;
    let missing = missing_consents(&state, &auth).await?;
    let rows = sqlx::query(
        "SELECT consent_id, policy_id, accepted_at FROM cn_user.policy_consents WHERE accepter_pubkey = $1",
    )
    .bind(&auth.pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let consents = rows
        .into_iter()
        .filter_map(|row| {
            let accepted_at: chrono::DateTime<chrono::Utc> = row.try_get("accepted_at").ok()?;
            Some(ConsentRecord {
                policy_id: row.try_get("policy_id").ok()?,
                accepted_at: accepted_at.timestamp(),
            })
        })
        .collect();

    Ok(Json(ConsentStatusResponse {
        pubkey: auth.pubkey,
        consents,
        missing,
    }))
}

pub async fn accept_consents(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ConsentRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    let policy_ids = if payload.accept_all_current.unwrap_or(false)
        || payload.policy_ids.as_ref().map(|list| list.is_empty()).unwrap_or(true)
    {
        let rows = sqlx::query("SELECT policy_id FROM cn_admin.policies WHERE is_current = TRUE")
            .fetch_all(&state.pool)
            .await
            .map_err(|err| {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string())
            })?;
        rows.into_iter()
            .filter_map(|row| row.try_get::<String, _>("policy_id").ok())
            .collect::<Vec<_>>()
    } else {
        payload.policy_ids.unwrap_or_default()
    };

    for policy_id in policy_ids.iter() {
        let consent_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO cn_user.policy_consents              (consent_id, policy_id, accepter_pubkey)              VALUES ($1, $2, $3)              ON CONFLICT DO NOTHING",
        )
        .bind(&consent_id)
        .bind(policy_id)
        .bind(&auth.pubkey)
        .execute(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string())
        })?;
    }

    let missing = missing_consents(&state, &auth).await?;
    if !missing.is_empty() {
        metrics::inc_consent_required(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::PRECONDITION_REQUIRED,
            "CONSENT_REQUIRED",
            "consent required",
        )
        .with_details(json!({ "required": missing })));
    }

    Ok(Json(json!({ "status": "ok" })))
}

pub(crate) async fn require_consents(state: &AppState, auth: &AuthContext) -> ApiResult<()> {
    let missing = missing_consents(state, auth).await?;
    if !missing.is_empty() {
        metrics::inc_consent_required(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::PRECONDITION_REQUIRED,
            "CONSENT_REQUIRED",
            "consent required",
        )
        .with_details(json!({ "required": missing })));
    }
    Ok(())
}

async fn missing_consents(state: &AppState, auth: &AuthContext) -> ApiResult<Vec<PolicySummary>> {
    let rows = sqlx::query(
        "SELECT p.policy_id, p.type, p.version, p.locale, p.title, p.content_hash, p.published_at, p.effective_at          FROM cn_admin.policies p          LEFT JOIN cn_user.policy_consents c            ON c.policy_id = p.policy_id AND c.accepter_pubkey = $1          WHERE p.is_current = TRUE AND p.type IN ('terms','privacy') AND c.policy_id IS NULL",
    )
    .bind(&auth.pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut missing = Vec::new();
    for row in rows {
        let policy_type: String = row.try_get("type")?;
        let version: String = row.try_get("version")?;
        let locale: String = row.try_get("locale")?;
        missing.push(PolicySummary {
            policy_id: row.try_get("policy_id")?,
            policy_type: policy_type.clone(),
            version: version.clone(),
            locale: locale.clone(),
            title: row.try_get("title")?,
            content_hash: row.try_get("content_hash")?,
            url: policy_url(&state.public_base_url, &policy_type, &version, &locale),
            published_at: row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("published_at")?
                .map(|value| value.timestamp()),
            effective_at: row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("effective_at")?
                .map(|value| value.timestamp()),
        });
    }
    Ok(missing)
}

fn policy_url(base: &str, policy_type: &str, version: &str, locale: &str) -> String {
    format!("{base}/policies/{policy_type}/{version}?locale={locale}")
}
