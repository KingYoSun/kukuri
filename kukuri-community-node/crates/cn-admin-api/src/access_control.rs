use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, QueryBuilder, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use cn_kip_types::{KIND_INVITE_CAPABILITY, KIP_NAMESPACE, KIP_VERSION, SCHEMA_INVITE_CAPABILITY};

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
    pub distribution_results: Vec<DistributionResult>,
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
    pub distribution_results: Vec<DistributionResult>,
}

#[derive(Serialize, ToSchema)]
pub struct DistributionResult {
    pub recipient_pubkey: String,
    pub status: String,
    pub reason: Option<String>,
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

#[derive(Deserialize)]
pub struct InviteCapabilityQuery {
    pub topic_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct DistributionResultQuery {
    pub topic_id: Option<String>,
    pub scope: Option<String>,
    pub pubkey: Option<String>,
    pub epoch: Option<i64>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct DistributionResultRow {
    pub topic_id: String,
    pub scope: String,
    pub epoch: i64,
    pub recipient_pubkey: String,
    pub status: String,
    pub reason: Option<String>,
    pub updated_at: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct IssueInviteCapabilityRequest {
    pub topic_id: String,
    pub scope: String,
    pub expires_in_seconds: Option<i64>,
    pub max_uses: Option<i32>,
    pub nonce: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct InviteCapabilityRow {
    pub topic_id: String,
    pub scope: String,
    pub issuer_pubkey: String,
    pub nonce: String,
    pub event_id: String,
    pub expires_at: i64,
    pub max_uses: i32,
    pub used_count: i32,
    pub status: String,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
    pub capability_event_json: Value,
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
        let scope = cn_core::access_control::normalize_scope(scope_raw).map_err(|err| {
            ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string())
        })?;
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

pub async fn list_distribution_results(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<DistributionResultQuery>,
) -> ApiResult<Json<Vec<DistributionResultRow>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT topic_id, scope, epoch, recipient_pubkey, status, reason, updated_at FROM cn_user.key_envelope_distribution_results WHERE 1=1",
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
        let scope = cn_core::access_control::normalize_scope(scope_raw).map_err(|err| {
            ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string())
        })?;
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
            builder.push(" AND recipient_pubkey = ");
            builder.push_bind(pubkey);
        } else {
            builder.push(" AND recipient_pubkey ILIKE ");
            builder.push_bind(format!("%{pubkey_raw}%"));
        }
    }

    if let Some(epoch) = query.epoch {
        if epoch <= 0 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_EPOCH",
                "epoch must be positive",
            ));
        }
        builder.push(" AND epoch = ");
        builder.push_bind(epoch as i32);
    }

    if let Some(status_raw) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let status =
            cn_core::access_control::normalize_distribution_status(status_raw).map_err(|err| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_STATUS", err.to_string())
            })?;
        builder.push(" AND status = ");
        builder.push_bind(status);
    }

    builder.push(" ORDER BY epoch DESC, updated_at DESC, recipient_pubkey ASC");
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

    let mut distribution_results = Vec::new();
    for row in rows {
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        distribution_results.push(DistributionResultRow {
            topic_id: row.try_get("topic_id")?,
            scope: row.try_get("scope")?,
            epoch: i64::from(row.try_get::<i32, _>("epoch")?),
            recipient_pubkey: row.try_get("recipient_pubkey")?,
            status: row.try_get("status")?,
            reason: row.try_get("reason")?,
            updated_at: updated_at.timestamp(),
        });
    }

    Ok(Json(distribution_results))
}

pub async fn list_invites(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<InviteCapabilityQuery>,
) -> ApiResult<Json<Vec<InviteCapabilityRow>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT topic_id, issuer_pubkey, nonce, expires_at, max_uses, used_count, revoked_at, capability_event_json, created_at FROM cn_user.invite_capabilities WHERE 1=1",
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

    if let Some(status) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let normalized = status.to_lowercase();
        match normalized.as_str() {
            "active" => {
                builder.push(
                    " AND revoked_at IS NULL AND expires_at > NOW() AND used_count < max_uses",
                );
            }
            "revoked" => {
                builder.push(" AND revoked_at IS NOT NULL");
            }
            "expired" => {
                builder.push(" AND revoked_at IS NULL AND expires_at <= NOW()");
            }
            "exhausted" => {
                builder.push(" AND revoked_at IS NULL AND used_count >= max_uses");
            }
            _ => {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_STATUS",
                    "status must be one of active|revoked|expired|exhausted",
                ));
            }
        }
    }

    builder.push(" ORDER BY created_at DESC");
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
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

    let mut invites = Vec::new();
    for row in rows {
        invites.push(map_invite_row(row)?);
    }
    Ok(Json(invites))
}

pub async fn issue_invite(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<IssueInviteCapabilityRequest>,
) -> ApiResult<Json<InviteCapabilityRow>> {
    let admin = require_admin(&state, &jar).await?;

    let topic_id = cn_core::topic::normalize_topic_id(&payload.topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    let scope = cn_core::access_control::normalize_scope(&payload.scope)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_SCOPE", err.to_string()))?;
    if scope != "invite" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SCOPE",
            "invite capability scope must be invite",
        ));
    }

    let expires_in_seconds = payload.expires_in_seconds.unwrap_or(86400);
    if expires_in_seconds < 60 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EXPIRES",
            "expires_in_seconds must be 60 or greater",
        ));
    }
    let max_uses = payload.max_uses.unwrap_or(1);
    if max_uses <= 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MAX_USES",
            "max_uses must be positive",
        ));
    }

    let nonce = payload
        .nonce
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    if nonce.len() > 128 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_NONCE",
            "nonce must be 128 characters or fewer",
        ));
    }

    let nonce_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM cn_user.invite_capabilities WHERE nonce = $1 LIMIT 1",
    )
    .bind(&nonce)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    if nonce_exists.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "NONCE_CONFLICT",
            "invite nonce already exists",
        ));
    }

    let now = chrono::Utc::now().timestamp();
    let expires_at = now.saturating_add(expires_in_seconds);
    let d_tag = format!("invite:{nonce}");
    let content = json!({
        "schema": SCHEMA_INVITE_CAPABILITY,
        "topic": topic_id,
        "scope": scope,
        "expires": expires_at,
        "max_uses": max_uses,
        "nonce": nonce,
        "issuer": state.node_keys.public_key().to_hex()
    })
    .to_string();
    let tags = vec![
        vec!["t".to_string(), topic_id.clone()],
        vec!["scope".to_string(), scope.clone()],
        vec!["d".to_string(), d_tag],
        vec!["k".to_string(), KIP_NAMESPACE.to_string()],
        vec!["ver".to_string(), KIP_VERSION.to_string()],
        vec!["exp".to_string(), expires_at.to_string()],
    ];
    let event = cn_core::nostr::build_signed_event(
        &state.node_keys,
        KIND_INVITE_CAPABILITY as u16,
        tags,
        content,
    )
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INVITE_BUILD_ERROR",
            err.to_string(),
        )
    })?;
    let capability_event_json = serde_json::to_value(event).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INVITE_BUILD_ERROR",
            err.to_string(),
        )
    })?;

    let row = sqlx::query(
        "INSERT INTO cn_user.invite_capabilities          (topic_id, issuer_pubkey, nonce, expires_at, max_uses, used_count, revoked_at, capability_event_json)          VALUES ($1, $2, $3, to_timestamp($4), $5, 0, NULL, $6)          RETURNING topic_id, issuer_pubkey, nonce, expires_at, max_uses, used_count, revoked_at, capability_event_json, created_at",
    )
    .bind(&topic_id)
    .bind(state.node_keys.public_key().to_hex())
    .bind(&nonce)
    .bind(expires_at)
    .bind(max_uses)
    .bind(capability_event_json)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "access_control.invite.issue",
        &format!("invite:{nonce}"),
        Some(json!({
            "topic_id": topic_id,
            "scope": scope,
            "expires_at": expires_at,
            "max_uses": max_uses
        })),
        None,
    )
    .await
    .ok();

    Ok(Json(map_invite_row(row)?))
}

pub async fn revoke_invite(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(nonce): Path<String>,
) -> ApiResult<Json<InviteCapabilityRow>> {
    let admin = require_admin(&state, &jar).await?;
    let nonce = nonce.trim();
    if nonce.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_NONCE",
            "nonce is required",
        ));
    }

    let row = sqlx::query(
        "UPDATE cn_user.invite_capabilities          SET revoked_at = COALESCE(revoked_at, NOW())          WHERE nonce = $1          RETURNING topic_id, issuer_pubkey, nonce, expires_at, max_uses, used_count, revoked_at, capability_event_json, created_at",
    )
    .bind(nonce)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "invite capability not found",
        ));
    };

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "access_control.invite.revoke",
        &format!("invite:{nonce}"),
        None,
        None,
    )
    .await
    .ok();

    Ok(Json(map_invite_row(row)?))
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
        distribution_results: map_distribution_results(summary.distribution_results),
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
        distribution_results: map_distribution_results(result.rotation.distribution_results),
    }))
}

fn map_invite_row(row: sqlx::postgres::PgRow) -> Result<InviteCapabilityRow, ApiError> {
    let capability_event_json: Value = row.try_get("capability_event_json")?;
    let expires_at: chrono::DateTime<chrono::Utc> = row.try_get("expires_at")?;
    let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at")?;
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let max_uses: i32 = row.try_get("max_uses")?;
    let used_count: i32 = row.try_get("used_count")?;

    Ok(InviteCapabilityRow {
        topic_id: row.try_get("topic_id")?,
        scope: invite_scope(&capability_event_json),
        issuer_pubkey: row.try_get("issuer_pubkey")?,
        nonce: row.try_get("nonce")?,
        event_id: invite_event_id(&capability_event_json),
        expires_at: expires_at.timestamp(),
        max_uses,
        used_count,
        status: invite_status(expires_at, revoked_at, used_count, max_uses),
        revoked_at: revoked_at.map(|value| value.timestamp()),
        created_at: created_at.timestamp(),
        capability_event_json,
    })
}

fn invite_status(
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    used_count: i32,
    max_uses: i32,
) -> String {
    if revoked_at.is_some() {
        "revoked".to_string()
    } else if used_count >= max_uses {
        "exhausted".to_string()
    } else if expires_at <= chrono::Utc::now() {
        "expired".to_string()
    } else {
        "active".to_string()
    }
}

fn invite_scope(event_json: &Value) -> String {
    event_json
        .get("tags")
        .and_then(Value::as_array)
        .and_then(|tags| {
            tags.iter().find_map(|tag| {
                let tag = tag.as_array()?;
                if tag.first().and_then(Value::as_str) == Some("scope") {
                    tag.get(1).and_then(Value::as_str).map(ToString::to_string)
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "invite".to_string())
}

fn invite_event_id(event_json: &Value) -> String {
    event_json
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn map_distribution_results(
    rows: Vec<cn_core::access_control::DistributionResult>,
) -> Vec<DistributionResult> {
    rows.into_iter()
        .map(|row| DistributionResult {
            recipient_pubkey: row.recipient_pubkey,
            status: row.status,
            reason: row.reason,
        })
        .collect()
}
