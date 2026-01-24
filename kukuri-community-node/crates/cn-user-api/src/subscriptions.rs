use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use cn_core::topic::normalize_topic_id;
use cn_core::nostr;
use chrono::TimeZone;
use nostr_sdk::prelude::{nip44, Keys, PublicKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};

use crate::auth::{current_rate_limit, enforce_rate_limit, require_auth, AuthContext};
use crate::billing::{check_topic_limit, consume_quota};
use crate::policies::require_consents;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize)]
pub struct SubscriptionRequestPayload {
    pub topic_id: String,
    pub requested_services: Vec<String>,
}

#[derive(Serialize)]
pub struct SubscriptionRequestResponse {
    pub request_id: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct TopicSubscription {
    pub topic_id: String,
    pub status: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Deserialize)]
pub struct InviteRedeemRequest {
    pub capability_event_json: serde_json::Value,
}

#[derive(Serialize)]
pub struct InviteRedeemResponse {
    pub topic_id: String,
    pub scope: String,
    pub epoch: i64,
    pub key_envelope_event: serde_json::Value,
}

#[derive(Deserialize)]
pub struct KeyEnvelopeQuery {
    pub topic_id: String,
    pub scope: Option<String>,
    pub after_epoch: Option<i64>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub topic: String,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Deserialize)]
pub struct TrendingQuery {
    pub topic: String,
}

#[derive(Deserialize)]
pub struct TrustQuery {
    pub subject: String,
}

#[derive(Deserialize)]
pub struct LabelsQuery {
    pub target: Option<String>,
    pub topic: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Deserialize)]
pub struct ReportRequest {
    pub report_event_json: Option<serde_json::Value>,
    pub target: Option<String>,
    pub reason: Option<String>,
}

pub async fn create_subscription_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(payload): Json<SubscriptionRequestPayload>,
) -> ApiResult<Json<SubscriptionRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    let rate = current_rate_limit(&state).await;
    if rate.enabled {
        let key = rate_key(addr, &auth.pubkey);
        enforce_rate_limit(&state, &key, rate.protected_per_minute).await?;
    }

    let topic_id = normalize_topic_id(&payload.topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    check_topic_limit(&state.pool, &auth.pubkey).await?;

    let existing = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if let Some(status) = existing {
        return Ok(Json(SubscriptionRequestResponse {
            request_id: "existing".to_string(),
            status,
        }));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_user.topic_subscription_requests          (request_id, requester_pubkey, topic_id, requested_services, status)          VALUES ($1, $2, $3, $4, 'pending')",
    )
    .bind(&request_id)
    .bind(&auth.pubkey)
    .bind(&topic_id)
    .bind(serde_json::to_value(&payload.requested_services).unwrap_or(json!([])))
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(SubscriptionRequestResponse {
        request_id,
        status: "pending".to_string(),
    }))
}

pub async fn list_topic_subscriptions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<Vec<TopicSubscription>>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    let rows = sqlx::query(
        "SELECT topic_id, status, started_at, ended_at FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = $1",
    )
    .bind(&auth.pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut subscriptions = Vec::new();
    for row in rows {
        let started_at: chrono::DateTime<chrono::Utc> = row.try_get("started_at")?;
        let ended_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("ended_at")?;
        subscriptions.push(TopicSubscription {
            topic_id: row.try_get("topic_id")?,
            status: row.try_get("status")?,
            started_at: started_at.timestamp(),
            ended_at: ended_at.map(|value| value.timestamp()),
        });
    }

    Ok(Json(subscriptions))
}

pub async fn delete_topic_subscription(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    let topic_id = normalize_topic_id(&topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&auth.pubkey)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if status.as_deref() != Some("active") {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "subscription not found"));
    }

    sqlx::query(
        "UPDATE cn_user.topic_subscriptions          SET status = 'ended', ended_at = NOW()          WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&auth.pubkey)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_admin.node_subscriptions          SET ref_count = GREATEST(ref_count - 1, 0),              enabled = CASE WHEN ref_count - 1 > 0 THEN TRUE ELSE FALSE END,              updated_at = NOW()          WHERE topic_id = $1",
    )
    .bind(&topic_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    tx.commit().await.ok();

    Ok(Json(json!({ "status": "ended" })))
}

pub async fn redeem_invite(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(payload): Json<InviteRedeemRequest>,
) -> ApiResult<Json<InviteRedeemResponse>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    let rate = current_rate_limit(&state).await;
    if rate.enabled {
        let key = rate_key(addr, &auth.pubkey);
        enforce_rate_limit(&state, &key, rate.protected_per_minute).await?;
    }

    consume_quota(
        &state.pool,
        &auth.pubkey,
        "invite.redeem_attempts",
        1,
        request_id(&headers),
    )
    .await?;

    let raw = nostr::parse_event(&payload.capability_event_json)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string()))?;
    nostr::verify_event(&raw)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string()))?;
    if raw.kind != 39021 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVENT",
            "invalid capability kind",
        ));
    }

    let topic_id = raw
        .first_tag_value("t")
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", "missing topic"))?;
    let topic_id = normalize_topic_id(&topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    let scope = raw
        .first_tag_value("scope")
        .unwrap_or_else(|| "invite".to_string());
    let d_tag = raw
        .d_tag()
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", "missing d tag"))?;
    let nonce = d_tag
        .strip_prefix("invite:")
        .unwrap_or(&d_tag)
        .to_string();

    let content: serde_json::Value = serde_json::from_str(&raw.content).unwrap_or(json!({}));
    let expires = content
        .get("expires")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let max_uses = content
        .get("max_uses")
        .and_then(|value| value.as_i64())
        .unwrap_or(1)
        .max(1);

    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    let expires_at_ts = if expires > 0 {
        expires
    } else {
        now + 86400
    };
    if now > expires_at_ts {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVITE_EXPIRED",
            "invite expired",
        ));
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let row = sqlx::query(
        "SELECT used_count, max_uses, expires_at, revoked_at FROM cn_user.invite_capabilities WHERE nonce = $1 FOR UPDATE",
    )
    .bind(&nonce)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if let Some(row) = row {
        let used_count: i32 = row.try_get("used_count")?;
        let max_uses_db: i32 = row.try_get("max_uses")?;
        let expires_at: chrono::DateTime<chrono::Utc> = row.try_get("expires_at")?;
        let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at")?;
        if revoked_at.is_some() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVITE_REVOKED",
                "invite revoked",
            ));
        }
        if chrono::Utc::now() > expires_at {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVITE_EXPIRED",
                "invite expired",
            ));
        }
        if used_count >= max_uses_db {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVITE_MAXED",
                "invite exhausted",
            ));
        }

        sqlx::query(
            "UPDATE cn_user.invite_capabilities SET used_count = used_count + 1 WHERE nonce = $1",
        )
        .bind(&nonce)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    } else {
        let expires_at = chrono::Utc
            .timestamp_opt(expires_at_ts, 0)
            .single()
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(24));
        sqlx::query(
            "INSERT INTO cn_user.invite_capabilities                  (topic_id, issuer_pubkey, nonce, expires_at, max_uses, used_count, capability_event_json)                  VALUES ($1, $2, $3, $4, $5, 1, $6)",
        )
        .bind(&topic_id)
        .bind(&raw.pubkey)
        .bind(&nonce)
        .bind(expires_at)
        .bind(max_uses)
        .bind(&payload.capability_event_json)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    sqlx::query(
        "INSERT INTO cn_user.topic_memberships          (topic_id, scope, pubkey, status)          VALUES ($1, $2, $3, 'active')          ON CONFLICT (topic_id, scope, pubkey) DO UPDATE SET status = 'active', revoked_at = NULL, revoked_reason = NULL",
    )
    .bind(&topic_id)
    .bind(&scope)
    .bind(&auth.pubkey)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let existing_sub = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&auth.pubkey)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if existing_sub.as_deref() != Some("active") {
        sqlx::query(
            "INSERT INTO cn_user.topic_subscriptions              (topic_id, subscriber_pubkey, status)              VALUES ($1, $2, 'active')              ON CONFLICT (topic_id, subscriber_pubkey) DO UPDATE SET status = 'active', ended_at = NULL",
        )
        .bind(&topic_id)
        .bind(&auth.pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

        sqlx::query(
            "INSERT INTO cn_admin.node_subscriptions                  (topic_id, enabled, ref_count)                  VALUES ($1, TRUE, 1)                  ON CONFLICT (topic_id) DO UPDATE SET ref_count = cn_admin.node_subscriptions.ref_count + 1, enabled = TRUE, updated_at = NOW()",
        )
        .bind(&topic_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    let epoch = ensure_topic_epoch(&mut tx, &topic_id, &scope).await?;
    let key_b64 = load_or_create_group_key(&mut tx, &state.node_keys, &topic_id, &scope, epoch).await?;
    let envelope = build_key_envelope(&state.node_keys, &auth.pubkey, &topic_id, &scope, epoch, &key_b64)?;

    sqlx::query(
        "INSERT INTO cn_user.key_envelopes          (topic_id, scope, epoch, recipient_pubkey, key_envelope_event_json)          VALUES ($1, $2, $3, $4, $5)          ON CONFLICT (topic_id, scope, epoch, recipient_pubkey) DO UPDATE SET key_envelope_event_json = EXCLUDED.key_envelope_event_json",
    )
    .bind(&topic_id)
    .bind(&scope)
    .bind(epoch as i32)
    .bind(&auth.pubkey)
    .bind(&envelope)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    tx.commit().await.ok();

    Ok(Json(InviteRedeemResponse {
        topic_id,
        scope,
        epoch,
        key_envelope_event: envelope,
    }))
}

pub async fn list_key_envelopes(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<KeyEnvelopeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    let topic_id = normalize_topic_id(&query.topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    let scope = query.scope.unwrap_or_else(|| "invite".to_string());
    ensure_membership(&state.pool, &auth.pubkey, &topic_id, &scope).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT key_envelope_event_json FROM cn_user.key_envelopes WHERE recipient_pubkey = ",
    );
    builder.push_bind(&auth.pubkey);
    builder.push(" AND topic_id = ");
    builder.push_bind(&topic_id);
    builder.push(" AND scope = ");
    builder.push_bind(&scope);
    if let Some(after_epoch) = query.after_epoch {
        builder.push(" AND epoch > ");
        builder.push_bind(after_epoch as i32);
    }

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|row| row.try_get("key_envelope_event_json").ok())
        .collect();

    Ok(Json(json!({ "items": items })))
}

pub async fn search(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;
    let topic = normalize_topic_id(&query.topic)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    ensure_subscription(&state.pool, &auth.pubkey, &topic).await?;
    consume_quota(
        &state.pool,
        &auth.pubkey,
        "index.search_requests",
        1,
        request_id(&headers),
    )
    .await?;

    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let offset = query
        .cursor
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let uid = cn_core::meili::topic_index_uid(&topic);
    let search_result = match state
        .meili
        .search(&uid, query.q.as_deref().unwrap_or(""), limit, offset)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            let message = err.to_string();
            if message.contains("404") {
                return Ok(Json(json!({
                    "topic": topic,
                    "query": query.q,
                    "items": [],
                    "next_cursor": null,
                    "total": 0
                })));
            }
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "SEARCH_UNAVAILABLE",
                message,
            ));
        }
    };

    let hits = search_result
        .get("hits")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let total = search_result
        .get("estimatedTotalHits")
        .and_then(|value| value.as_u64())
        .unwrap_or(hits.len() as u64);
    let next_offset = offset + hits.len();
    let next_cursor = if (next_offset as u64) < total {
        Some(next_offset.to_string())
    } else {
        None
    };

    Ok(Json(json!({
        "topic": topic,
        "query": query.q,
        "items": hits,
        "next_cursor": next_cursor,
        "total": total
    })))
}

pub async fn trending(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<TrendingQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;
    let topic = normalize_topic_id(&query.topic)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC", err.to_string()))?;
    ensure_subscription(&state.pool, &auth.pubkey, &topic).await?;
    consume_quota(
        &state.pool,
        &auth.pubkey,
        "index.trending_requests",
        1,
        request_id(&headers),
    )
    .await?;

    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    let window_hours = 24;
    let since = now.saturating_sub(window_hours * 3600);

    let row = sqlx::query(
        "SELECT              COUNT(*) FILTER (WHERE kind = 1) AS post_count,              COUNT(*) FILTER (WHERE kind IN (6, 7)) AS reaction_count          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON e.event_id = t.event_id          WHERE t.topic_id = $1            AND e.is_deleted = FALSE            AND e.is_ephemeral = FALSE            AND e.is_current = TRUE            AND (e.expires_at IS NULL OR e.expires_at > $2)            AND e.created_at >= $3",
    )
    .bind(&topic)
    .bind(now)
    .bind(since)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let post_count: i64 = row.try_get("post_count")?;
    let reaction_count: i64 = row.try_get("reaction_count")?;
    let score = post_count.saturating_add(reaction_count);

    Ok(Json(json!({
        "topic": topic,
        "window_hours": window_hours,
        "metrics": {
            "posts": post_count,
            "reactions": reaction_count,
            "score": score
        },
        "items": []
    })))
}

pub async fn trust_report_based(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<TrustQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;
    consume_quota(
        &state.pool,
        &auth.pubkey,
        "trust.requests",
        1,
        request_id(&headers),
    )
    .await?;

    Ok(Json(json!({ "subject": query.subject, "score": 0.0 })))
}

pub async fn trust_communication_density(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<TrustQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;
    consume_quota(
        &state.pool,
        &auth.pubkey,
        "trust.requests",
        1,
        request_id(&headers),
    )
    .await?;

    Ok(Json(json!({ "subject": query.subject, "score": 0.0 })))
}

pub async fn list_labels(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<LabelsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;

    let target = query.target.ok_or_else(|| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "target is required")
    })?;

    let limit = query.limit.unwrap_or(50).clamp(1, 200) as i64;
    let offset = query
        .cursor
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT label_event_json FROM cn_moderation.labels WHERE target = ",
    );
    builder.push_bind(&target);
    builder.push(" AND exp > ");
    builder.push_bind(now);
    if let Some(topic) = query.topic {
        builder.push(" AND topic_id = ");
        builder.push_bind(topic);
    }
    builder.push(" ORDER BY issued_at DESC");
    if offset > 0 {
        builder.push(" OFFSET ");
        builder.push(offset.to_string());
    }
    builder.push(" LIMIT ");
    builder.push(limit.to_string());

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|row| row.try_get("label_event_json").ok())
        .collect();
    let next_cursor = if items.len() as i64 >= limit {
        Some((offset + items.len() as i64).to_string())
    } else {
        None
    };

    Ok(Json(json!({
        "target": target,
        "items": items,
        "next_cursor": next_cursor
    })))
}

pub async fn submit_report(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(payload): Json<ReportRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;
    consume_quota(
        &state.pool,
        &auth.pubkey,
        "moderation.report_submits",
        1,
        request_id(&headers),
    )
    .await?;

    let (report_id, target, reason, report_event_json) =
        if let Some(event_json) = payload.report_event_json {
            let raw = nostr::parse_event(&event_json)
                .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string()))?;
            if raw.kind != 39005 {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_EVENT",
                    "invalid report kind",
                ));
            }
            nostr::verify_event(&raw)
                .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string()))?;
            if raw.pubkey != auth.pubkey {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_EVENT",
                    "reporter pubkey mismatch",
                ));
            }
            let target = raw
                .first_tag_value("target")
                .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", "missing target"))?;
            let reason = raw
                .first_tag_value("reason")
                .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", "missing reason"))?;
            let report_id = raw.id.clone();
            let normalized = serde_json::to_value(&raw).unwrap_or(json!({}));
            (report_id, target, reason, normalized)
        } else {
            let target = payload
                .target
                .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "target is required"))?;
            let reason = payload
                .reason
                .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "reason is required"))?;
            let tags = vec![
                vec!["k".to_string(), "kukuri".to_string()],
                vec!["ver".to_string(), "1".to_string()],
                vec!["target".to_string(), target.clone()],
                vec!["reason".to_string(), reason.clone()],
            ];
            let event = nostr::build_signed_event(&state.node_keys, 39005, tags, String::new())
                .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "REPORT_ERROR", err.to_string()))?;
            let event_json = serde_json::to_value(&event).unwrap_or(json!({}));
            (event.id, target, reason, event_json)
        };

    sqlx::query(
        "INSERT INTO cn_user.reports          (report_id, reporter_pubkey, target, reason, report_event_json)          VALUES ($1, $2, $3, $4, $5)          ON CONFLICT (report_id) DO NOTHING",
    )
    .bind(&report_id)
    .bind(&auth.pubkey)
    .bind(&target)
    .bind(&reason)
    .bind(report_event_json)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(json!({ "status": "accepted", "report_id": report_id })))
}

fn request_id(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
}

fn rate_key(addr: std::net::SocketAddr, pubkey: &str) -> String {
    format!("pubkey:{}:{}", pubkey, addr.ip())
}

async fn apply_protected_rate_limit(
    state: &AppState,
    auth: &AuthContext,
    addr: std::net::SocketAddr,
) -> ApiResult<()> {
    let rate = current_rate_limit(state).await;
    if rate.enabled {
        let key = rate_key(addr, &auth.pubkey);
        enforce_rate_limit(state, &key, rate.protected_per_minute).await?;
    }
    Ok(())
}

async fn ensure_membership(
    pool: &sqlx::Pool<Postgres>,
    pubkey: &str,
    topic_id: &str,
    scope: &str,
) -> ApiResult<()> {
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_memberships WHERE topic_id = $1 AND scope = $2 AND pubkey = $3",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(pubkey)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if status.as_deref() != Some("active") {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "MEMBERSHIP_REQUIRED",
            "membership required",
        ));
    }
    Ok(())
}

async fn ensure_subscription(
    pool: &sqlx::Pool<Postgres>,
    pubkey: &str,
    topic_id: &str,
) -> ApiResult<()> {
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(topic_id)
    .bind(pubkey)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if status.as_deref() != Some("active") {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "SUBSCRIPTION_REQUIRED",
            "subscription required",
        ));
    }
    Ok(())
}

async fn ensure_topic_epoch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    topic_id: &str,
    scope: &str,
) -> ApiResult<i64> {
    let row = sqlx::query(
        "SELECT current_epoch FROM cn_admin.topic_scope_state WHERE topic_id = $1 AND scope = $2 FOR UPDATE",
    )
    .bind(topic_id)
    .bind(scope)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if let Some(row) = row {
        let current_epoch: i32 = row.try_get("current_epoch")?;
        return Ok(current_epoch as i64);
    }

    sqlx::query(
        "INSERT INTO cn_admin.topic_scope_state          (topic_id, scope, current_epoch)          VALUES ($1, $2, 1)",
    )
    .bind(topic_id)
    .bind(scope)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(1)
}

async fn load_or_create_group_key(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    node_keys: &Keys,
    topic_id: &str,
    scope: &str,
    epoch: i64,
) -> ApiResult<String> {
    let row = sqlx::query(
        "SELECT key_ciphertext FROM cn_admin.topic_scope_keys WHERE topic_id = $1 AND scope = $2 AND epoch = $3",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if let Some(row) = row {
        let ciphertext: String = row.try_get("key_ciphertext")?;
        let plain = nip44::decrypt(
            node_keys.secret_key(),
            &node_keys.public_key(),
            ciphertext,
        )
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "KEY_ERROR", err.to_string()))?;
        return Ok(plain);
    }

    let mut bytes = [0u8; 32];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    let key_b64 = STANDARD.encode(bytes);
    let ciphertext = nip44::encrypt(
        node_keys.secret_key(),
        &node_keys.public_key(),
        &key_b64,
        nip44::Version::V2,
    )
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "KEY_ERROR", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_admin.topic_scope_keys          (topic_id, scope, epoch, key_ciphertext)          VALUES ($1, $2, $3, $4)",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .bind(ciphertext)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(key_b64)
}

fn build_key_envelope(
    node_keys: &Keys,
    recipient_pubkey: &str,
    topic_id: &str,
    scope: &str,
    epoch: i64,
    key_b64: &str,
) -> ApiResult<serde_json::Value> {
    let recipient = PublicKey::from_hex(recipient_pubkey).map_err(|_| {
        ApiError::new(StatusCode::BAD_REQUEST, "INVALID_PUBKEY", "invalid pubkey")
    })?;

    let payload = json!({
        "schema": "kukuri-key-envelope-v1",
        "topic": topic_id,
        "scope": scope,
        "epoch": epoch,
        "key_b64": key_b64,
        "issued_at": cn_core::auth::unix_seconds().unwrap_or(0) as i64
    });
    let encrypted = nip44::encrypt(
        node_keys.secret_key(),
        &recipient,
        payload.to_string(),
        nip44::Version::V2,
    )
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "KEY_ERROR", err.to_string()))?;

    let d_tag = format!(
        "keyenv:{topic_id}:{scope}:{epoch}:{recipient_pubkey}"
    );
    let tags = vec![
        vec!["p".to_string(), recipient_pubkey.to_string()],
        vec!["t".to_string(), topic_id.to_string()],
        vec!["scope".to_string(), scope.to_string()],
        vec!["epoch".to_string(), epoch.to_string()],
        vec!["ver".to_string(), "1".to_string()],
        vec!["d".to_string(), d_tag],
    ];

    let raw = nostr::build_signed_event(node_keys, 39020, tags, encrypted)
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "KEY_ERROR", err.to_string()))?;
    Ok(serde_json::to_value(raw).unwrap_or(json!({})))
}
