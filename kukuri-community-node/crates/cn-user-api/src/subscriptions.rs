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

    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&auth.pubkey)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if status.as_deref() != Some("active") {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "subscription not found",
        ));
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
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    err.to_string(),
                )
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
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    err.to_string(),
                )
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
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "target is required",
        )
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
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

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
            let raw = nostr::parse_event(&event_json).map_err(|err| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string())
            })?;
            if raw.kind != 39005 {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_EVENT",
                    "invalid report kind",
                ));
            }
            nostr::verify_event(&raw).map_err(|err| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string())
            })?;
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
            let target = raw.first_tag_value("target").ok_or_else(|| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", "missing target")
            })?;
            let reason = raw.first_tag_value("reason").ok_or_else(|| {
                ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", "missing reason")
            })?;
            let report_id = raw.id.clone();
            let normalized = serde_json::to_value(&raw).unwrap_or(json!({}));
            (report_id, target, reason, normalized)
        } else {
            let target = payload.target.ok_or_else(|| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_REQUEST",
                    "target is required",
                )
            })?;
            let reason = payload.reason.ok_or_else(|| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_REQUEST",
                    "reason is required",
                )
            })?;
            let tags = vec![
                vec!["k".to_string(), "kukuri".to_string()],
                vec!["ver".to_string(), "1".to_string()],
                vec!["target".to_string(), target.clone()],
                vec!["reason".to_string(), reason.clone()],
            ];
            let event = nostr::build_signed_event(&state.node_keys, 39005, tags, String::new())
                .map_err(|err| {
                    ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "REPORT_ERROR",
                        err.to_string(),
                    )
                })?;
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

    Ok(Json(
        json!({ "status": "accepted", "report_id": report_id }),
    ))
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
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{header, HeaderMap, Request, StatusCode};
    use axum::routing::{delete, get, post};
    use axum::Router;
    use cn_core::service_config;
    use nostr_sdk::prelude::Keys;
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{Pool, Postgres};
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::OnceCell;
    use tower::ServiceExt;
    use uuid::Uuid;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:5432/cn".to_string())
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                cn_core::migrations::run(pool)
                    .await
                    .expect("run migrations");
            })
            .await;
    }

    async fn test_state_with_meili_url(meili_url: &str) -> crate::AppState {
        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        crate::billing::ensure_default_plan(&pool)
            .await
            .expect("seed plans");

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
        let meili = cn_core::meili::MeiliClient::new(meili_url.to_string(), None).expect("meili");
        let export_dir = PathBuf::from(format!("tmp/test_exports/{}", Uuid::new_v4()));
        std::fs::create_dir_all(&export_dir).expect("create test export dir");

        crate::AppState {
            pool,
            jwt_config,
            public_base_url: "http://localhost".to_string(),
            user_config,
            bootstrap_config,
            rate_limiter: Arc::new(cn_core::rate_limit::RateLimiter::new()),
            node_keys: Keys::generate(),
            export_dir,
            hmac_secret: b"test-secret".to_vec(),
            meili,
        }
    }

    async fn test_state() -> crate::AppState {
        test_state_with_meili_url("http://localhost:7700").await
    }

    fn issue_token(config: &cn_core::auth::JwtConfig, pubkey: &str) -> String {
        let (token, _) = cn_core::auth::issue_token(pubkey, config).expect("issue token");
        token
    }

    async fn ensure_consents(pool: &Pool<Postgres>, pubkey: &str) {
        let policies = sqlx::query_scalar::<_, String>(
            "SELECT policy_id FROM cn_admin.policies WHERE is_current = TRUE AND type IN ('terms','privacy')",
        )
        .fetch_all(pool)
        .await
        .expect("fetch policies");
        for policy_id in policies {
            let consent_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO cn_user.policy_consents (consent_id, policy_id, accepter_pubkey) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(consent_id)
            .bind(policy_id)
            .bind(pubkey)
            .execute(pool)
            .await
            .expect("insert consent");
        }
    }

    async fn insert_topic_subscription(pool: &Pool<Postgres>, topic_id: &str, pubkey: &str) {
        sqlx::query(
            "INSERT INTO cn_user.topic_subscriptions (topic_id, subscriber_pubkey, status) VALUES ($1, $2, 'active') ON CONFLICT DO NOTHING",
        )
        .bind(topic_id)
        .bind(pubkey)
        .execute(pool)
        .await
        .expect("insert subscription");
    }

    async fn ensure_active_subscriber(pool: &Pool<Postgres>, pubkey: &str) {
        sqlx::query(
            "INSERT INTO cn_user.subscriber_accounts \
                (subscriber_pubkey, status) \
             VALUES ($1, 'active') \
             ON CONFLICT (subscriber_pubkey) DO UPDATE \
             SET status = 'active', updated_at = NOW()",
        )
        .bind(pubkey)
        .execute(pool)
        .await
        .expect("upsert active subscriber");
    }

    async fn insert_current_policy(
        pool: &Pool<Postgres>,
        policy_type: &str,
        version: &str,
        locale: &str,
        title: &str,
    ) -> String {
        let policy_id = format!("{policy_type}-{}", Uuid::new_v4());
        let now = chrono::Utc::now();
        let content_md = format!("# {title}\n\ncontract test policy.");
        let content_hash = format!("sha256:{policy_id}");

        sqlx::query(
            "INSERT INTO cn_admin.policies \
                (policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)",
        )
        .bind(&policy_id)
        .bind(policy_type)
        .bind(version)
        .bind(locale)
        .bind(title)
        .bind(content_md)
        .bind(content_hash)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert policy");

        policy_id
    }

    async fn insert_bootstrap_event(
        pool: &Pool<Postgres>,
        event_id: &str,
        kind: i32,
        topic_id: Option<&str>,
        expires_at: i64,
        event_json: Value,
    ) {
        let created_at = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        sqlx::query(
            "INSERT INTO cn_bootstrap.events \
                (event_id, kind, d_tag, topic_id, role, scope, event_json, created_at, expires_at, is_active) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)",
        )
        .bind(event_id)
        .bind(kind)
        .bind(event_id)
        .bind(topic_id)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(event_json)
        .bind(created_at)
        .bind(expires_at)
        .execute(pool)
        .await
        .expect("insert bootstrap event");
    }

    async fn insert_label(
        pool: &Pool<Postgres>,
        target: &str,
        topic_id: Option<&str>,
        issuer_pubkey: &str,
        label_id: &str,
    ) {
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let exp = now + 3600;
        let label_event_json = json!({
            "id": label_id,
            "target": target,
            "label": "spam"
        });
        sqlx::query(
            "INSERT INTO cn_moderation.labels \
                (label_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, source, label_event_json) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(label_id)
        .bind(target)
        .bind(topic_id)
        .bind("spam")
        .bind(0.9_f64)
        .bind("https://example.com/policy")
        .bind("contract-test")
        .bind(exp)
        .bind(issuer_pubkey)
        .bind("contract-test")
        .bind(label_event_json)
        .execute(pool)
        .await
        .expect("insert label");
    }

    async fn insert_attestation(
        pool: &Pool<Postgres>,
        subject: &str,
        claim: &str,
        exp: i64,
        attestation_id: &str,
    ) -> Value {
        let issuer_pubkey = Keys::generate().public_key().to_hex();
        let event_json = json!({
            "id": attestation_id,
            "subject": subject,
            "claim": claim,
            "exp": exp
        });
        sqlx::query(
            "INSERT INTO cn_trust.attestations \
                (attestation_id, subject, claim, score, exp, issuer_pubkey, event_json) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(attestation_id)
        .bind(subject)
        .bind(claim)
        .bind(0.82_f64)
        .bind(exp)
        .bind(issuer_pubkey)
        .bind(event_json.clone())
        .execute(pool)
        .await
        .expect("insert attestation");
        event_json
    }

    async fn insert_report_score(
        pool: &Pool<Postgres>,
        subject_pubkey: &str,
        attestation_id: &str,
        attestation_exp: i64,
    ) {
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        sqlx::query(
            "INSERT INTO cn_trust.report_scores \
                (subject_pubkey, score, report_count, label_count, window_start, window_end, attestation_id, attestation_exp) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(subject_pubkey)
        .bind(0.75_f64)
        .bind(3_i64)
        .bind(2_i64)
        .bind(now - 3600)
        .bind(now)
        .bind(attestation_id)
        .bind(attestation_exp)
        .execute(pool)
        .await
        .expect("insert report score");
    }

    async fn insert_communication_score(
        pool: &Pool<Postgres>,
        subject_pubkey: &str,
        attestation_id: &str,
        attestation_exp: i64,
    ) {
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        sqlx::query(
            "INSERT INTO cn_trust.communication_scores \
                (subject_pubkey, score, interaction_count, peer_count, window_start, window_end, attestation_id, attestation_exp) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(subject_pubkey)
        .bind(1.25_f64)
        .bind(5_i64)
        .bind(3_i64)
        .bind(now - 7200)
        .bind(now)
        .bind(attestation_id)
        .bind(attestation_exp)
        .execute(pool)
        .await
        .expect("insert communication score");
    }

    async fn insert_relay_event(
        pool: &Pool<Postgres>,
        event_id: &str,
        pubkey: &str,
        kind: i32,
        created_at: i64,
        topic_id: &str,
    ) {
        let tags = json!([]);
        let raw_json = json!({
            "id": event_id,
            "pubkey": pubkey,
            "kind": kind,
            "created_at": created_at,
            "tags": tags,
            "content": "",
            "sig": "sig"
        });
        sqlx::query(
            "INSERT INTO cn_relay.events (event_id, pubkey, kind, created_at, tags, content, sig, raw_json) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(event_id)
        .bind(pubkey)
        .bind(kind)
        .bind(created_at)
        .bind(tags)
        .bind("")
        .bind("sig")
        .bind(raw_json)
        .execute(pool)
        .await
        .expect("insert relay event");

        sqlx::query(
            "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(event_id)
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("insert event topic");
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

    async fn request_status_with_body(
        app: Router,
        method: &str,
        uri: &str,
        body: &'static str,
    ) -> StatusCode {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        response.status()
    }

    async fn get_json_public(app: Router, uri: &str) -> (StatusCode, Value) {
        let (status, _, payload) = get_json_public_with_headers(app, uri, &[]).await;
        (status, payload)
    }

    async fn get_json_public_with_headers(
        app: Router,
        uri: &str,
        extra_headers: &[(&str, &str)],
    ) -> (StatusCode, HeaderMap, Value) {
        let response = get_response_public_with_headers(app, uri, extra_headers).await;
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, headers, payload)
    }

    async fn get_status_public_with_headers(
        app: Router,
        uri: &str,
        extra_headers: &[(&str, &str)],
    ) -> (StatusCode, HeaderMap) {
        let response = get_response_public_with_headers(app, uri, extra_headers).await;
        (response.status(), response.headers().clone())
    }

    async fn get_response_public_with_headers(
        app: Router,
        uri: &str,
        extra_headers: &[(&str, &str)],
    ) -> axum::response::Response {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        for (name, value) in extra_headers {
            request.headers_mut().insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
                axum::http::HeaderValue::from_str(value).expect("header value"),
            );
        }
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        app.oneshot(request).await.expect("response")
    }

    async fn get_text_public(app: Router, uri: &str) -> (StatusCode, Option<String>, String) {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(std::string::ToString::to_string);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, content_type, String::from_utf8_lossy(&body).to_string())
    }

    async fn get_json(app: Router, uri: &str, token: &str) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, payload)
    }

    async fn post_json(app: Router, uri: &str, token: &str, payload: Value) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, payload)
    }

    async fn post_json_public(app: Router, uri: &str, payload: Value) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, payload)
    }

    async fn delete_json(app: Router, uri: &str, token: &str) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("DELETE")
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, payload)
    }

    async fn spawn_mock_meili(search_response: Value) -> (String, tokio::task::JoinHandle<()>) {
        let response = Arc::new(search_response);
        let app = Router::new().route(
            "/indexes/{uid}/search",
            post({
                let response = Arc::clone(&response);
                move |_path: axum::extract::Path<String>, _payload: axum::Json<Value>| {
                    let response = Arc::clone(&response);
                    async move { (StatusCode::OK, axum::Json((*response).clone())) }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock meili");
        let addr = listener.local_addr().expect("mock meili addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock meili");
        });
        (format!("http://{addr}"), handle)
    }

    async fn spawn_mock_meili_health(status_code: Arc<AtomicU16>) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/health",
            get({
                let status_code = Arc::clone(&status_code);
                move || {
                    let status_code = Arc::clone(&status_code);
                    async move {
                        let status = StatusCode::from_u16(status_code.load(Ordering::Relaxed))
                            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                        (status, axum::Json(json!({ "status": "mock" })))
                    }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock meili health");
        let addr = listener.local_addr().expect("mock meili health addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve mock meili health");
        });

        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn healthz_contract_success_shape_compatible() {
        let meili_status = Arc::new(AtomicU16::new(200));
        let (meili_url, meili_server) = spawn_mock_meili_health(Arc::clone(&meili_status)).await;
        let state = test_state_with_meili_url(&meili_url).await;

        let app = Router::new()
            .route("/healthz", get(crate::healthz))
            .with_state(state);
        let (status, payload) = get_json_public(app, "/healthz").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"));

        meili_server.abort();
        let _ = meili_server.await;
    }

    #[tokio::test]
    async fn healthz_contract_unavailable_shape_compatible() {
        let meili_status = Arc::new(AtomicU16::new(503));
        let (meili_url, meili_server) = spawn_mock_meili_health(Arc::clone(&meili_status)).await;
        let state = test_state_with_meili_url(&meili_url).await;

        let app = Router::new()
            .route("/healthz", get(crate::healthz))
            .with_state(state);
        let (status, payload) = get_json_public(app, "/healthz").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some("unavailable")
        );

        meili_server.abort();
        let _ = meili_server.await;
    }

    #[tokio::test]
    async fn metrics_contract_prometheus_content_type_shape_compatible() {
        let app = Router::new().route("/metrics", get(crate::metrics_endpoint));
        let (status, content_type, body) = get_text_public(app, "/metrics").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/plain; version=0.0.4"));
        assert!(
            body.contains("cn_up{service=\"cn-user-api\"} 1"),
            "metrics body did not contain cn_up for cn-user-api: {body}"
        );
    }

    #[tokio::test]
    async fn list_labels_requires_auth() {
        let app = Router::new()
            .route("/v1/labels", get(list_labels))
            .with_state(test_state().await);
        let status = request_status(app, "/v1/labels?target=event:abc").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn search_requires_auth() {
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(test_state().await);
        let status = request_status(app, "/v1/search?topic=kukuri:topic1").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn submit_report_requires_auth() {
        let app = Router::new()
            .route("/v1/reports", post(submit_report))
            .with_state(test_state().await);
        let status = request_status_with_body(app, "POST", "/v1/reports", "{}").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn trust_report_based_requires_auth() {
        let app = Router::new()
            .route("/v1/trust/report-based", get(trust_report_based))
            .with_state(test_state().await);
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
            .route(
                "/v1/trust/communication-density",
                get(trust_communication_density),
            )
            .with_state(test_state().await);
        let status = request_status(
            app,
            "/v1/trust/communication-density?subject=pubkey:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_contract_challenge_verify_success_shape_compatible() {
        let state = test_state().await;
        let keys = Keys::generate();
        let pubkey = keys.public_key().to_hex();
        let app = Router::new()
            .route("/v1/auth/challenge", post(crate::auth::auth_challenge))
            .route("/v1/auth/verify", post(crate::auth::auth_verify))
            .with_state(state);

        let (status, challenge_payload) = post_json_public(
            app.clone(),
            "/v1/auth/challenge",
            json!({ "pubkey": pubkey }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let challenge = challenge_payload
            .get("challenge")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(!challenge.is_empty());
        assert!(challenge_payload
            .get("expires_at")
            .and_then(Value::as_i64)
            .is_some());

        let auth_event = cn_core::nostr::build_signed_event(
            &keys,
            22242,
            vec![
                vec!["relay".to_string(), "http://localhost".to_string()],
                vec!["challenge".to_string(), challenge.to_string()],
            ],
            String::new(),
        )
        .expect("build auth event");

        let (status, verify_payload) = post_json_public(
            app,
            "/v1/auth/verify",
            json!({ "auth_event_json": auth_event }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let access_token = verify_payload
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(!access_token.is_empty());
        assert_eq!(
            verify_payload.get("token_type").and_then(Value::as_str),
            Some("Bearer")
        );
        assert!(verify_payload
            .get("expires_at")
            .and_then(Value::as_i64)
            .is_some());
        assert_eq!(
            verify_payload.get("pubkey").and_then(Value::as_str),
            Some(pubkey.as_str())
        );
    }

    #[tokio::test]
    async fn policies_consents_contract_success_shape_compatible() {
        let state = test_state().await;
        let locale = "ja-JP";
        let terms_version = format!("contract-terms-{}", Uuid::new_v4().simple());
        let privacy_version = format!("contract-privacy-{}", Uuid::new_v4().simple());
        let terms_policy_id = insert_current_policy(
            &state.pool,
            "terms",
            &terms_version,
            locale,
            "Contract Terms",
        )
        .await;
        let privacy_policy_id = insert_current_policy(
            &state.pool,
            "privacy",
            &privacy_version,
            locale,
            "Contract Privacy",
        )
        .await;

        let pubkey = Keys::generate().public_key().to_hex();
        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/policies/current", get(crate::policies::get_current_policies))
            .route(
                "/v1/policies/{policy_type}/{version}",
                get(crate::policies::get_policy_by_version),
            )
            .route("/v1/consents/status", get(crate::policies::get_consent_status))
            .route("/v1/consents", post(crate::policies::accept_consents))
            .with_state(state);

        let (status, current_payload) = get_json_public(app.clone(), "/v1/policies/current").await;
        assert_eq!(status, StatusCode::OK);
        let policies = current_payload.as_array().expect("policies array");
        assert!(policies.iter().any(|policy| {
            policy.get("policy_id").and_then(Value::as_str) == Some(terms_policy_id.as_str())
        }));
        assert!(policies.iter().any(|policy| {
            policy.get("policy_id").and_then(Value::as_str) == Some(privacy_policy_id.as_str())
        }));
        assert!(policies.iter().all(|policy| {
            policy.get("policy_type").and_then(Value::as_str).is_some()
                && policy.get("version").and_then(Value::as_str).is_some()
                && policy.get("locale").and_then(Value::as_str).is_some()
                && policy.get("title").and_then(Value::as_str).is_some()
                && policy.get("content_hash").and_then(Value::as_str).is_some()
                && policy.get("url").and_then(Value::as_str).is_some()
        }));

        let (status, detail_payload) = get_json_public(
            app.clone(),
            &format!("/v1/policies/terms/{terms_version}?locale={locale}"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            detail_payload.get("policy_id").and_then(Value::as_str),
            Some(terms_policy_id.as_str())
        );
        assert_eq!(
            detail_payload.get("policy_type").and_then(Value::as_str),
            Some("terms")
        );
        assert_eq!(
            detail_payload.get("version").and_then(Value::as_str),
            Some(terms_version.as_str())
        );
        assert_eq!(
            detail_payload.get("locale").and_then(Value::as_str),
            Some(locale)
        );
        assert!(detail_payload
            .get("content_md")
            .and_then(Value::as_str)
            .is_some());
        assert!(detail_payload
            .get("content_hash")
            .and_then(Value::as_str)
            .is_some());

        let (status, consent_before_payload) = get_json(app.clone(), "/v1/consents/status", &token).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            consent_before_payload.get("pubkey").and_then(Value::as_str),
            Some(pubkey.as_str())
        );
        assert!(consent_before_payload
            .get("consents")
            .and_then(Value::as_array)
            .is_some());
        assert!(consent_before_payload
            .get("missing")
            .and_then(Value::as_array)
            .is_some());

        let (status, accept_payload) = post_json(
            app.clone(),
            "/v1/consents",
            &token,
            json!({ "accept_all_current": true }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            accept_payload.get("status").and_then(Value::as_str),
            Some("ok")
        );

        let (status, consent_after_payload) = get_json(app, "/v1/consents/status", &token).await;
        assert_eq!(status, StatusCode::OK);
        let consents = consent_after_payload
            .get("consents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(consents.iter().any(|consent| {
            consent.get("policy_id").and_then(Value::as_str) == Some(terms_policy_id.as_str())
        }));
        assert!(consents.iter().any(|consent| {
            consent.get("policy_id").and_then(Value::as_str) == Some(privacy_policy_id.as_str())
        }));
        let missing = consent_after_payload
            .get("missing")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn topic_subscription_contract_success_shape_compatible() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&state.pool, &pubkey).await;
        let token = issue_token(&state.jwt_config, &pubkey);
        let pool = state.pool.clone();

        let request_topic_id = format!("kukuri:req-{}", Uuid::new_v4().simple());
        let active_topic_id = format!("kukuri:active-{}", Uuid::new_v4().simple());

        let app = Router::new()
            .route(
                "/v1/topic-subscription-requests",
                post(create_subscription_request),
            )
            .route("/v1/topic-subscriptions", get(list_topic_subscriptions))
            .route(
                "/v1/topic-subscriptions/{topic_id}",
                delete(delete_topic_subscription),
            )
            .with_state(state);

        let (status, create_payload) = post_json(
            app.clone(),
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": request_topic_id,
                "requested_services": ["relay", "index"]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(create_payload
            .get("request_id")
            .and_then(Value::as_str)
            .is_some());
        assert_eq!(
            create_payload.get("status").and_then(Value::as_str),
            Some("pending")
        );

        insert_topic_subscription(&pool, &active_topic_id, &pubkey).await;
        let (status, list_payload) = get_json(app.clone(), "/v1/topic-subscriptions", &token).await;
        assert_eq!(status, StatusCode::OK);
        let list = list_payload.as_array().cloned().unwrap_or_default();
        let active_row = list
            .iter()
            .find(|row| {
                row.get("topic_id").and_then(Value::as_str) == Some(active_topic_id.as_str())
            })
            .expect("active subscription row");
        assert_eq!(active_row.get("status").and_then(Value::as_str), Some("active"));
        assert!(active_row.get("started_at").and_then(Value::as_i64).is_some());
        assert!(active_row
            .get("ended_at")
            .is_some_and(serde_json::Value::is_null));

        let (status, delete_payload) = delete_json(
            app.clone(),
            &format!("/v1/topic-subscriptions/{active_topic_id}"),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            delete_payload.get("status").and_then(Value::as_str),
            Some("ended")
        );

        let (status, list_after_payload) = get_json(app, "/v1/topic-subscriptions", &token).await;
        assert_eq!(status, StatusCode::OK);
        let list_after = list_after_payload.as_array().cloned().unwrap_or_default();
        let ended_row = list_after
            .iter()
            .find(|row| {
                row.get("topic_id").and_then(Value::as_str) == Some(active_topic_id.as_str())
            })
            .expect("ended subscription row");
        assert_eq!(ended_row.get("status").and_then(Value::as_str), Some("ended"));
        assert!(ended_row.get("ended_at").and_then(Value::as_i64).is_some());
    }

    #[tokio::test]
    async fn personal_data_export_contract_success_shape_compatible() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_active_subscriber(&state.pool, &pubkey).await;
        ensure_consents(&state.pool, &pubkey).await;
        let token = issue_token(&state.jwt_config, &pubkey);

        let app = Router::new()
            .route(
                "/v1/personal-data-export-requests",
                post(crate::personal_data::create_export_request),
            )
            .route(
                "/v1/personal-data-export-requests/{export_request_id}",
                get(crate::personal_data::get_export_request),
            )
            .route(
                "/v1/personal-data-export-requests/{export_request_id}/download",
                get(crate::personal_data::download_export),
            )
            .with_state(state);

        let (status, create_payload) = post_json(
            app.clone(),
            "/v1/personal-data-export-requests",
            &token,
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let export_request_id = create_payload
            .get("export_request_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!export_request_id.is_empty());
        assert_eq!(
            create_payload.get("status").and_then(Value::as_str),
            Some("completed")
        );
        let download_token = create_payload
            .get("download_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!download_token.is_empty());
        assert!(create_payload
            .get("download_expires_at")
            .and_then(Value::as_i64)
            .is_some());

        let (status, get_payload) = get_json(
            app.clone(),
            &format!("/v1/personal-data-export-requests/{export_request_id}"),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            get_payload.get("export_request_id").and_then(Value::as_str),
            Some(export_request_id.as_str())
        );
        assert_eq!(
            get_payload.get("status").and_then(Value::as_str),
            Some("completed")
        );
        assert!(get_payload
            .get("download_token")
            .and_then(Value::as_str)
            .is_some());
        assert!(get_payload
            .get("download_expires_at")
            .and_then(Value::as_i64)
            .is_some());

        let mut request = Request::builder()
            .method("GET")
            .uri(format!(
                "/v1/personal-data-export-requests/{export_request_id}/download?token={download_token}"
            ))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(
            headers
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        assert!(headers
            .get("content-disposition")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains(export_request_id.as_str())));

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let download_payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            download_payload.get("pubkey").and_then(Value::as_str),
            Some(pubkey.as_str())
        );
        assert!(download_payload
            .get("generated_at")
            .and_then(Value::as_i64)
            .is_some());
        assert!(download_payload
            .get("consents")
            .and_then(Value::as_array)
            .is_some());
        assert!(download_payload
            .get("subscriptions")
            .and_then(Value::as_array)
            .is_some());
        assert!(download_payload
            .get("usage_events")
            .and_then(Value::as_array)
            .is_some());
        assert!(download_payload
            .get("reports")
            .and_then(Value::as_array)
            .is_some());
        assert!(download_payload
            .get("memberships")
            .and_then(Value::as_array)
            .is_some());
        assert!(download_payload
            .get("events")
            .and_then(Value::as_array)
            .is_some());
    }

    #[tokio::test]
    async fn personal_data_deletion_contract_success_shape_compatible() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_active_subscriber(&state.pool, &pubkey).await;
        ensure_consents(&state.pool, &pubkey).await;
        let jwt_config = state.jwt_config.clone();
        let token = issue_token(&jwt_config, &pubkey);
        let pool = state.pool.clone();

        let app = Router::new()
            .route(
                "/v1/personal-data-deletion-requests",
                post(crate::personal_data::create_deletion_request),
            )
            .route(
                "/v1/personal-data-deletion-requests/{deletion_request_id}",
                get(crate::personal_data::get_deletion_request),
            )
            .with_state(state);

        let (status, create_payload) = post_json(
            app.clone(),
            "/v1/personal-data-deletion-requests",
            &token,
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let deletion_request_id = create_payload
            .get("deletion_request_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!deletion_request_id.is_empty());
        assert_eq!(
            create_payload.get("status").and_then(Value::as_str),
            Some("completed")
        );

        let lookup_pubkey = Keys::generate().public_key().to_hex();
        ensure_active_subscriber(&pool, &lookup_pubkey).await;
        let lookup_token = issue_token(&jwt_config, &lookup_pubkey);
        let lookup_request_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO cn_user.personal_data_deletion_requests \
                (deletion_request_id, requester_pubkey, status) \
             VALUES ($1, $2, 'queued')",
        )
        .bind(&lookup_request_id)
        .bind(&lookup_pubkey)
        .execute(&pool)
        .await
        .expect("insert lookup deletion request");

        let (status, get_payload) = get_json(
            app,
            &format!("/v1/personal-data-deletion-requests/{lookup_request_id}"),
            &lookup_token,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            get_payload
                .get("deletion_request_id")
                .and_then(Value::as_str),
            Some(lookup_request_id.as_str())
        );
        assert_eq!(
            get_payload.get("status").and_then(Value::as_str),
            Some("queued")
        );

        let account_status = sqlx::query_scalar::<_, String>(
            "SELECT status FROM cn_user.subscriber_accounts WHERE subscriber_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_optional(&pool)
        .await
        .expect("select subscriber status");
        assert_eq!(account_status.as_deref(), Some("deleted"));
    }

    #[tokio::test]
    async fn bootstrap_nodes_contract_success_shape_compatible() {
        let state = test_state().await;
        let event_id = Uuid::new_v4().to_string();
        let expires_at = cn_core::auth::unix_seconds().unwrap_or(0) as i64 + 1800;
        let event_json = json!({
            "id": event_id,
            "kind": 39000,
            "pubkey": Keys::generate().public_key().to_hex(),
            "tags": [["k", "kukuri"], ["ver", "1"]],
            "content": "",
            "sig": "signature"
        });
        insert_bootstrap_event(
            &state.pool,
            &event_id,
            39000,
            None,
            expires_at,
            event_json.clone(),
        )
        .await;

        let app = Router::new()
            .route(
                "/v1/bootstrap/nodes",
                get(crate::bootstrap::get_bootstrap_nodes),
            )
            .with_state(state);
        let (status, payload) = get_json_public(app, "/v1/bootstrap/nodes").await;

        assert_eq!(status, StatusCode::OK);
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());
        assert!(items.iter().any(|value| value == &event_json));
        assert!(payload
            .get("next_refresh_at")
            .and_then(Value::as_i64)
            .is_some());
    }

    #[tokio::test]
    async fn bootstrap_services_contract_success_shape_compatible() {
        let state = test_state().await;
        let topic_id = format!("kukuri:bootstrap-{}", Uuid::new_v4());
        let event_id = Uuid::new_v4().to_string();
        let expires_at = cn_core::auth::unix_seconds().unwrap_or(0) as i64 + 3600;
        let event_json = json!({
            "id": event_id,
            "kind": 39001,
            "pubkey": Keys::generate().public_key().to_hex(),
            "tags": [["k", "kukuri"], ["ver", "1"], ["topic", topic_id.clone()]],
            "content": "",
            "sig": "signature"
        });
        insert_bootstrap_event(
            &state.pool,
            &event_id,
            39001,
            Some(topic_id.as_str()),
            expires_at,
            event_json.clone(),
        )
        .await;

        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(crate::bootstrap::get_bootstrap_services),
            )
            .with_state(state);
        let (status, payload) =
            get_json_public(app, &format!("/v1/bootstrap/topics/{topic_id}/services")).await;

        assert_eq!(status, StatusCode::OK);
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 1);
        assert_eq!(items.first(), Some(&event_json));
        assert_eq!(
            payload.get("next_refresh_at").and_then(Value::as_i64),
            Some(expires_at)
        );
    }

    #[tokio::test]
    async fn bootstrap_services_conditional_get_and_cache_headers_contract_compatible() {
        let state = test_state().await;
        let topic_id = format!("kukuri:bootstrap-cache-{}", Uuid::new_v4());
        let event_id_one = Uuid::new_v4().to_string();
        let event_id_two = Uuid::new_v4().to_string();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let first_expires_at = now + 600;
        let second_expires_at = now + 1200;

        let event_one = json!({
            "id": event_id_one,
            "kind": 39001,
            "pubkey": Keys::generate().public_key().to_hex(),
            "tags": [["k", "kukuri"], ["ver", "1"], ["topic", topic_id.clone()], ["service", "relay"]],
            "content": "",
            "sig": "signature"
        });
        let event_two = json!({
            "id": event_id_two,
            "kind": 39001,
            "pubkey": Keys::generate().public_key().to_hex(),
            "tags": [["k", "kukuri"], ["ver", "1"], ["topic", topic_id.clone()], ["service", "bootstrap"]],
            "content": "",
            "sig": "signature"
        });

        insert_bootstrap_event(
            &state.pool,
            &event_id_one,
            39001,
            Some(topic_id.as_str()),
            first_expires_at,
            event_one.clone(),
        )
        .await;
        insert_bootstrap_event(
            &state.pool,
            &event_id_two,
            39001,
            Some(topic_id.as_str()),
            second_expires_at,
            event_two.clone(),
        )
        .await;

        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(crate::bootstrap::get_bootstrap_services),
            )
            .with_state(state);
        let path = format!("/v1/bootstrap/topics/{topic_id}/services");

        let (status, headers, payload) = get_json_public_with_headers(app.clone(), &path, &[]).await;
        assert_eq!(status, StatusCode::OK);

        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|value| value == &event_one));
        assert!(items.iter().any(|value| value == &event_two));
        assert_eq!(
            payload.get("next_refresh_at").and_then(Value::as_i64),
            Some(first_expires_at)
        );

        let cache_control = headers
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert_eq!(cache_control, "public, max-age=300");

        let etag = headers
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(!etag.is_empty());

        let last_modified = headers
            .get(header::LAST_MODIFIED)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(!last_modified.is_empty());
        assert!(httpdate::parse_http_date(&last_modified).is_ok());

        let (etag_status, etag_headers) =
            get_status_public_with_headers(app.clone(), &path, &[("if-none-match", &etag)]).await;
        assert_eq!(etag_status, StatusCode::NOT_MODIFIED);
        assert_eq!(
            etag_headers
                .get(header::ETAG)
                .and_then(|value| value.to_str().ok()),
            Some(etag.as_str())
        );

        let (modified_status, modified_headers) = get_status_public_with_headers(
            app,
            &path,
            &[("if-modified-since", &last_modified)],
        )
        .await;
        assert_eq!(modified_status, StatusCode::NOT_MODIFIED);
        assert_eq!(
            modified_headers
                .get(header::ETAG)
                .and_then(|value| value.to_str().ok()),
            Some(etag.as_str())
        );
    }

    #[tokio::test]
    async fn search_contract_success_shape_compatible() {
        let topic_id = format!("kukuri:search-{}", Uuid::new_v4());
        let (meili_url, meili_handle) = spawn_mock_meili(json!({
            "hits": [
                {
                    "event_id": Uuid::new_v4().to_string(),
                    "topic_id": topic_id.clone(),
                    "content": "hello contract"
                }
            ],
            "estimatedTotalHits": 2
        }))
        .await;

        let state = test_state_with_meili_url(&meili_url).await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&state.pool, &pubkey).await;
        insert_topic_subscription(&state.pool, &topic_id, &pubkey).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);
        let (status, payload) = get_json(
            app,
            &format!("/v1/search?topic={topic_id}&q=hello&limit=1"),
            &token,
        )
        .await;
        meili_handle.abort();

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("topic").and_then(Value::as_str),
            Some(topic_id.as_str())
        );
        assert_eq!(payload.get("query").and_then(Value::as_str), Some("hello"));
        assert_eq!(payload.get("total").and_then(Value::as_u64), Some(2));
        assert_eq!(
            payload.get("next_cursor").and_then(Value::as_str),
            Some("1")
        );
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 1);
        assert!(items
            .first()
            .and_then(|item| item.get("event_id"))
            .and_then(Value::as_str)
            .is_some());
    }

    #[tokio::test]
    async fn submit_report_contract_success_shape_compatible() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/reports", post(submit_report))
            .with_state(state);
        let (status, payload) = post_json(
            app,
            "/v1/reports",
            &token,
            json!({
                "target": "event:report-contract-target",
                "reason": "spam"
            }),
        )
        .await;

        assert!(
            status == StatusCode::OK || status == StatusCode::CREATED,
            "unexpected status: {status}"
        );
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some("accepted")
        );
        let report_id = payload
            .get("report_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(!report_id.is_empty());

        let row = sqlx::query("SELECT target, reason FROM cn_user.reports WHERE report_id = $1")
            .bind(report_id)
            .fetch_optional(&pool)
            .await
            .expect("select report row");
        let row = row.expect("report row exists");
        assert_eq!(
            row.try_get::<String, _>("target").unwrap_or_default(),
            "event:report-contract-target"
        );
        assert_eq!(
            row.try_get::<String, _>("reason").unwrap_or_default(),
            "spam"
        );
    }

    #[tokio::test]
    async fn list_labels_contract_success() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&state.pool, &pubkey).await;
        let target = "event:contract-label";
        let issuer_pubkey = Keys::generate().public_key().to_hex();
        let label_id_a = Uuid::new_v4().to_string();
        let label_id_b = Uuid::new_v4().to_string();
        insert_label(&state.pool, target, None, &issuer_pubkey, &label_id_a).await;
        insert_label(&state.pool, target, None, &issuer_pubkey, &label_id_b).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/labels", get(list_labels))
            .with_state(state);
        let (status, payload) = get_json(
            app,
            "/v1/labels?target=event:contract-label&limit=1",
            &token,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload.get("target").and_then(Value::as_str), Some(target));
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 1);
        let returned_id = items
            .first()
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(returned_id == label_id_a || returned_id == label_id_b);
        assert!(payload.get("next_cursor").and_then(Value::as_str).is_some());
    }

    #[tokio::test]
    async fn trust_report_based_contract_success() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&state.pool, &pubkey).await;
        let subject = format!("pubkey:{pubkey}");
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let attestation_id = Uuid::new_v4().to_string();
        let attestation_exp = now + 3600;
        let event_json = insert_attestation(
            &state.pool,
            &subject,
            "report-based",
            attestation_exp,
            &attestation_id,
        )
        .await;
        insert_report_score(&state.pool, &pubkey, &attestation_id, attestation_exp).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/trust/report-based", get(trust_report_based))
            .with_state(state);
        let (status, payload) = get_json(
            app,
            &format!("/v1/trust/report-based?subject={subject}"),
            &token,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("subject").and_then(Value::as_str),
            Some(subject.as_str())
        );
        assert_eq!(
            payload.get("method").and_then(Value::as_str),
            Some("report-based")
        );
        assert_eq!(payload.get("score").and_then(Value::as_f64), Some(0.75));
        assert_eq!(payload.get("report_count").and_then(Value::as_i64), Some(3));
        assert_eq!(payload.get("label_count").and_then(Value::as_i64), Some(2));
        let attestation = payload
            .get("attestation")
            .and_then(Value::as_object)
            .expect("attestation");
        assert_eq!(
            attestation.get("attestation_id").and_then(Value::as_str),
            Some(attestation_id.as_str())
        );
        assert_eq!(
            attestation.get("exp").and_then(Value::as_i64),
            Some(attestation_exp)
        );
        assert_eq!(attestation.get("event_json"), Some(&event_json));
    }

    #[tokio::test]
    async fn trust_communication_density_contract_success() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&state.pool, &pubkey).await;
        let subject = format!("pubkey:{pubkey}");
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let attestation_id = Uuid::new_v4().to_string();
        let attestation_exp = now + 3600;
        let event_json = insert_attestation(
            &state.pool,
            &subject,
            "communication-density",
            attestation_exp,
            &attestation_id,
        )
        .await;
        insert_communication_score(&state.pool, &pubkey, &attestation_id, attestation_exp).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/trust/communication-density",
                get(trust_communication_density),
            )
            .with_state(state);
        let (status, payload) = get_json(
            app,
            &format!("/v1/trust/communication-density?subject={subject}"),
            &token,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("subject").and_then(Value::as_str),
            Some(subject.as_str())
        );
        assert_eq!(
            payload.get("method").and_then(Value::as_str),
            Some("communication-density")
        );
        assert_eq!(payload.get("score").and_then(Value::as_f64), Some(1.25));
        assert_eq!(
            payload.get("interaction_count").and_then(Value::as_i64),
            Some(5)
        );
        assert_eq!(payload.get("peer_count").and_then(Value::as_i64), Some(3));
        let attestation = payload
            .get("attestation")
            .and_then(Value::as_object)
            .expect("attestation");
        assert_eq!(
            attestation.get("attestation_id").and_then(Value::as_str),
            Some(attestation_id.as_str())
        );
        assert_eq!(
            attestation.get("exp").and_then(Value::as_i64),
            Some(attestation_exp)
        );
        assert_eq!(attestation.get("event_json"), Some(&event_json));
    }

    #[tokio::test]
    async fn trending_contract_success() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&state.pool, &pubkey).await;
        let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());
        insert_topic_subscription(&state.pool, &topic_id, &pubkey).await;

        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        insert_relay_event(
            &state.pool,
            &Uuid::new_v4().to_string(),
            &pubkey,
            1,
            now,
            &topic_id,
        )
        .await;
        insert_relay_event(
            &state.pool,
            &Uuid::new_v4().to_string(),
            &pubkey,
            7,
            now,
            &topic_id,
        )
        .await;
        insert_relay_event(
            &state.pool,
            &Uuid::new_v4().to_string(),
            &pubkey,
            6,
            now,
            &topic_id,
        )
        .await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/trending", get(trending))
            .with_state(state);
        let (status, payload) =
            get_json(app, &format!("/v1/trending?topic={topic_id}"), &token).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("topic").and_then(Value::as_str),
            Some(topic_id.as_str())
        );
        assert_eq!(
            payload.get("window_hours").and_then(Value::as_i64),
            Some(24)
        );
        let metrics = payload
            .get("metrics")
            .and_then(Value::as_object)
            .expect("metrics");
        assert_eq!(metrics.get("posts").and_then(Value::as_i64), Some(1));
        assert_eq!(metrics.get("reactions").and_then(Value::as_i64), Some(2));
        assert_eq!(metrics.get("score").and_then(Value::as_i64), Some(3));
    }
}
