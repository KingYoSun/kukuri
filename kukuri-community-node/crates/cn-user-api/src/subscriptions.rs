use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use cn_core::nostr;
use cn_core::topic::normalize_topic_id;
use cn_kip_types::{validate_kip_event, ValidationOptions};
use nostr_sdk::prelude::PublicKey;
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

    let subject_pubkey = parse_trust_subject(&query.subject)?;
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;

    let row = sqlx::query(
        "SELECT score, report_count, label_count, window_start, window_end, attestation_id, attestation_exp, updated_at          FROM cn_trust.report_scores          WHERE subject_pubkey = $1",
    )
    .bind(&subject_pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let subject = format!("pubkey:{subject_pubkey}");
    let Some(row) = row else {
        return Ok(Json(json!({
            "subject": subject,
            "method": "report-based",
            "score": 0.0,
            "report_count": 0,
            "label_count": 0,
            "window_start": null,
            "window_end": null,
            "attestation": null,
            "updated_at": null
        })));
    };

    let attestation_id: Option<String> = row.try_get("attestation_id")?;
    let attestation_exp: Option<i64> = row.try_get("attestation_exp")?;
    let attestation = if let (Some(attestation_id), Some(attestation_exp)) =
        (attestation_id.as_ref(), attestation_exp)
    {
        if attestation_exp > now {
            let event_json = sqlx::query_scalar::<_, serde_json::Value>(
                "SELECT event_json FROM cn_trust.attestations WHERE attestation_id = $1",
            )
            .bind(attestation_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|err| {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string())
            })?;
            Some(json!({
                "attestation_id": attestation_id,
                "exp": attestation_exp,
                "event_json": event_json
            }))
        } else {
            None
        }
    } else {
        None
    };

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;

    Ok(Json(json!({
        "subject": subject,
        "method": "report-based",
        "score": row.try_get::<f64, _>("score")?,
        "report_count": row.try_get::<i64, _>("report_count")?,
        "label_count": row.try_get::<i64, _>("label_count")?,
        "window_start": row.try_get::<i64, _>("window_start")?,
        "window_end": row.try_get::<i64, _>("window_end")?,
        "attestation": attestation,
        "updated_at": updated_at.timestamp()
    })))
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

    let subject_pubkey = parse_trust_subject(&query.subject)?;
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;

    let row = sqlx::query(
        "SELECT score, interaction_count, peer_count, window_start, window_end, attestation_id, attestation_exp, updated_at          FROM cn_trust.communication_scores          WHERE subject_pubkey = $1",
    )
    .bind(&subject_pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let subject = format!("pubkey:{subject_pubkey}");
    let Some(row) = row else {
        return Ok(Json(json!({
            "subject": subject,
            "method": "communication-density",
            "score": 0.0,
            "interaction_count": 0,
            "peer_count": 0,
            "window_start": null,
            "window_end": null,
            "attestation": null,
            "updated_at": null
        })));
    };

    let attestation_id: Option<String> = row.try_get("attestation_id")?;
    let attestation_exp: Option<i64> = row.try_get("attestation_exp")?;
    let attestation = if let (Some(attestation_id), Some(attestation_exp)) =
        (attestation_id.as_ref(), attestation_exp)
    {
        if attestation_exp > now {
            let event_json = sqlx::query_scalar::<_, serde_json::Value>(
                "SELECT event_json FROM cn_trust.attestations WHERE attestation_id = $1",
            )
            .bind(attestation_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|err| {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string())
            })?;
            Some(json!({
                "attestation_id": attestation_id,
                "exp": attestation_exp,
                "event_json": event_json
            }))
        } else {
            None
        }
    } else {
        None
    };

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;

    Ok(Json(json!({
        "subject": subject,
        "method": "communication-density",
        "score": row.try_get::<f64, _>("score")?,
        "interaction_count": row.try_get::<i64, _>("interaction_count")?,
        "peer_count": row.try_get::<i64, _>("peer_count")?,
        "window_start": row.try_get::<i64, _>("window_start")?,
        "window_end": row.try_get::<i64, _>("window_end")?,
        "attestation": attestation,
        "updated_at": updated_at.timestamp()
    })))
}

fn parse_trust_subject(subject: &str) -> ApiResult<String> {
    let subject = subject.trim();
    let pubkey = subject.strip_prefix("pubkey:").ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SUBJECT",
            "subject must start with pubkey:",
        )
    })?;
    if PublicKey::from_hex(pubkey).is_err() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SUBJECT",
            "invalid pubkey",
        ));
    }
    Ok(pubkey.to_string())
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
            let options = ValidationOptions {
                now: cn_core::auth::unix_seconds().unwrap_or(0) as i64,
                verify_signature: false,
                ..ValidationOptions::default()
            };
            if validate_kip_event(&raw, options).is_err() {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_EVENT",
                    "invalid report event",
                ));
            }
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

#[cfg(test)]
mod trust_subject_tests {
    use super::parse_trust_subject;
    use nostr_sdk::prelude::Keys;

    #[test]
    fn parse_trust_subject_accepts_pubkey_prefix() {
        let pubkey = Keys::generate().public_key().to_hex();
        let subject = format!("pubkey:{pubkey}");
        let parsed = parse_trust_subject(&subject).unwrap_or_else(|_| String::new());
        assert_eq!(parsed, pubkey);
    }

    #[test]
    fn parse_trust_subject_rejects_invalid_prefix() {
        assert!(parse_trust_subject("npub1example").is_err());
    }
}

#[cfg(test)]
mod api_contract_tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use cn_core::service_config;
    use nostr_sdk::prelude::Keys;
    use sqlx::postgres::PgPoolOptions;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state() -> crate::AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool");
        let jwt_config = cn_core::auth::JwtConfig {
            issuer: "http://localhost".to_string(),
            audience: crate::TOKEN_AUDIENCE.to_string(),
            secret: "test-secret".to_string(),
            ttl_seconds: 3600,
        };
        let user_config = service_config::static_handle(serde_json::json!({
            "rate_limit": { "enabled": false }
        }));
        let bootstrap_config = service_config::static_handle(serde_json::json!({
            "auth": { "mode": "off" }
        }));
        let meili = cn_core::meili::MeiliClient::new("http://localhost:7700".to_string(), None)
            .expect("meili");

        crate::AppState {
            pool,
            jwt_config,
            public_base_url: "http://localhost".to_string(),
            user_config,
            bootstrap_config,
            rate_limiter: Arc::new(cn_core::rate_limit::RateLimiter::new()),
            node_keys: Keys::generate(),
            export_dir: PathBuf::from("tmp/test_exports"),
            hmac_secret: b"test-secret".to_vec(),
            meili,
        }
    }

    async fn request_status(app: Router, uri: &str) -> StatusCode {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        response.status()
    }

    #[tokio::test]
    async fn list_labels_requires_auth() {
        let app = Router::new()
            .route("/v1/labels", get(list_labels))
            .with_state(test_state());
        let status = request_status(app, "/v1/labels?target=event:abc").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn trust_report_based_requires_auth() {
        let app = Router::new()
            .route("/v1/trust/report-based", get(trust_report_based))
            .with_state(test_state());
        let status = request_status(
            app,
            "/v1/trust/report-based?subject=pubkey:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn trust_communication_density_requires_auth() {
        let app = Router::new()
            .route("/v1/trust/communication-density", get(trust_communication_density))
            .with_state(test_state());
        let status = request_status(
            app,
            "/v1/trust/communication-density?subject=pubkey:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
