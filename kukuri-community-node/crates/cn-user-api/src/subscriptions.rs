use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use cn_core::community_search_terms;
use cn_core::nostr;
use cn_core::search_normalizer;
use cn_core::search_runtime_flags;
use cn_core::topic::normalize_topic_id;
use cn_kip_types::{validate_kip_event, ValidationOptions};
use nostr_sdk::prelude::PublicKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, QueryBuilder, Row};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::auth::{current_rate_limit, enforce_rate_limit, require_auth, AuthContext};
use crate::billing::{check_topic_limit, consume_quota};
use crate::policies::require_consents;
use crate::{ApiError, ApiResult, AppState};

const DEFAULT_MAX_PENDING_SUBSCRIPTION_REQUESTS_PER_PUBKEY: i64 = 5;
const TOPIC_SUBSCRIPTION_PENDING_LOCK_CONTEXT: &[u8] =
    b"cn-user-api.topic-subscription-request.pending-limit";
const SUGGEST_STAGE_A_LIMIT_MULTIPLIER: usize = 3;
const SUGGEST_STAGE_A_MAX_LIMIT: usize = 100;

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
pub struct CommunitySuggestQuery {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchReadBackend {
    Meili,
    Pg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuggestReadBackend {
    Legacy,
    Pg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuggestRerankMode {
    Shadow,
    Enabled,
}

impl SuggestRerankMode {
    fn as_str(self) -> &'static str {
        match self {
            SuggestRerankMode::Shadow => "shadow",
            SuggestRerankMode::Enabled => "enabled",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SuggestRelationWeights {
    is_member: f64,
    is_following_community: f64,
    friends_member_count: f64,
    two_hop_follow_count: f64,
    last_view_decay: f64,
    muted_or_blocked: f64,
}

impl Default for SuggestRelationWeights {
    fn default() -> Self {
        Self {
            is_member: 1.20,
            is_following_community: 0.80,
            friends_member_count: 0.35,
            two_hop_follow_count: 0.25,
            last_view_decay: 0.15,
            muted_or_blocked: -1.0,
        }
    }
}

#[derive(Debug, Clone)]
struct SuggestRuntimeConfig {
    backend: SuggestReadBackend,
    rerank_mode: SuggestRerankMode,
    relation_weights: SuggestRelationWeights,
    shadow_sample_rate: u8,
}

#[derive(Debug, Clone)]
struct CommunityCandidate {
    community_id: String,
    exact_hit: bool,
    prefix_hit: bool,
    trgm_score: f64,
    name_match_score: f64,
}

#[derive(Debug, Clone)]
struct RerankedCommunityCandidate {
    community_id: String,
    exact_hit: bool,
    prefix_hit: bool,
    trgm_score: f64,
    name_match_score: f64,
    relation_score: f64,
    global_popularity: f64,
    recency_boost: f64,
    final_suggest_score: f64,
    stage_a_rank: i64,
    stage_b_rank: i64,
}

#[derive(Debug, Clone)]
struct SuggestRerankResult {
    candidates: Vec<RerankedCommunityCandidate>,
    blocked_or_muted_drop_count: usize,
    visibility_drop_count: usize,
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

    let max_pending_requests = max_pending_subscription_requests_per_pubkey(&state).await;
    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let (lock_key_high, lock_key_low) = advisory_lock_keys_for_pubkey(&auth.pubkey);
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(lock_key_high)
        .bind(lock_key_low)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let existing = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(&topic_id)
    .bind(&auth.pubkey)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if let Some(status) = existing {
        return Ok(Json(SubscriptionRequestResponse {
            request_id: "existing".to_string(),
            status,
        }));
    }

    let pending_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1 AND status = 'pending'",
    )
    .bind(&auth.pubkey)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if pending_count >= max_pending_requests {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED",
            "pending subscription request limit reached",
        )
        .with_details(json!({
            "metric": "topic_subscription_requests.pending",
            "scope": "pubkey",
            "current": pending_count,
            "limit": max_pending_requests
        })));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_user.topic_subscription_requests          (request_id, requester_pubkey, topic_id, requested_services, status)          VALUES ($1, $2, $3, $4, 'pending')",
    )
    .bind(&request_id)
    .bind(&auth.pubkey)
    .bind(&topic_id)
    .bind(serde_json::to_value(&payload.requested_services).unwrap_or(json!([])))
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(Json(SubscriptionRequestResponse {
        request_id,
        status: "pending".to_string(),
    }))
}

fn advisory_lock_keys_for_pubkey(pubkey: &str) -> (i32, i32) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TOPIC_SUBSCRIPTION_PENDING_LOCK_CONTEXT);
    hasher.update(pubkey.as_bytes());
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();
    (
        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
    )
}

async fn max_pending_subscription_requests_per_pubkey(state: &AppState) -> i64 {
    let snapshot = state.user_config.get().await;
    snapshot
        .config_json
        .get("subscription_request")
        .and_then(|value| value.get("max_pending_per_pubkey"))
        .and_then(|value| value.as_i64())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_PENDING_SUBSCRIPTION_REQUESTS_PER_PUBKEY)
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

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

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
    let backend = load_search_read_backend(&state.pool).await;

    if backend == SearchReadBackend::Pg {
        return search_with_pg_backend(&state, &topic, query.q, limit, offset).await;
    }

    search_with_meili_backend(&state, &topic, query.q, limit, offset).await
}

async fn load_search_read_backend(pool: &sqlx::Pool<Postgres>) -> SearchReadBackend {
    let flags = match search_runtime_flags::load_search_runtime_flags(pool).await {
        Ok(flags) => flags,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to load search runtime flags; fallback to meili read backend"
            );
            return SearchReadBackend::Meili;
        }
    };

    if flags
        .search_read_backend
        .trim()
        .eq_ignore_ascii_case(search_runtime_flags::SEARCH_READ_BACKEND_PG)
    {
        SearchReadBackend::Pg
    } else {
        SearchReadBackend::Meili
    }
}

async fn search_with_meili_backend(
    state: &AppState,
    topic: &str,
    query: Option<String>,
    limit: usize,
    offset: usize,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = cn_core::meili::topic_index_uid(topic);
    let search_result = match state
        .meili
        .search(&uid, query.as_deref().unwrap_or(""), limit, offset)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            let message = err.to_string();
            if message.contains("404") {
                return Ok(Json(json!({
                    "topic": topic,
                    "query": query,
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
        "query": query,
        "items": hits,
        "next_cursor": next_cursor,
        "total": total
    })))
}

async fn search_with_pg_backend(
    state: &AppState,
    topic: &str,
    query: Option<String>,
    limit: usize,
    offset: usize,
) -> ApiResult<Json<serde_json::Value>> {
    let query_raw = query.as_deref().unwrap_or("");
    let query_norm = search_normalizer::normalize_search_text(query_raw);
    let normalizer_version = search_normalizer::SEARCH_NORMALIZER_VERSION;

    let total = if query_norm.is_empty() {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) \
             FROM cn_search.post_search_documents d \
             WHERE d.topic_id = $1 \
               AND d.visibility = 'public' \
               AND d.is_deleted = FALSE \
               AND d.normalizer_version = $2",
        )
        .bind(topic)
        .bind(normalizer_version)
        .fetch_one(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) \
             FROM cn_search.post_search_documents d \
             WHERE d.topic_id = $1 \
               AND d.visibility = 'public' \
               AND d.is_deleted = FALSE \
               AND d.normalizer_version = $2 \
               AND d.search_text &@~ $3",
        )
        .bind(topic)
        .bind(normalizer_version)
        .bind(&query_norm)
        .fetch_one(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?
    };

    let rows = if query_norm.is_empty() {
        sqlx::query(
            "SELECT d.post_id, d.topic_id, d.author_id, d.body_raw, d.hashtags_norm, d.created_at \
             FROM cn_search.post_search_documents d \
             WHERE d.topic_id = $1 \
               AND d.visibility = 'public' \
               AND d.is_deleted = FALSE \
               AND d.normalizer_version = $2 \
             ORDER BY d.created_at DESC \
             LIMIT $3 OFFSET $4",
        )
        .bind(topic)
        .bind(normalizer_version)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?
    } else {
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as f64;
        sqlx::query(
            "SELECT d.post_id, d.topic_id, d.author_id, d.body_raw, d.hashtags_norm, d.created_at, \
                    ( \
                      0.55 * pgroonga_score(tableoid, ctid) + \
                      0.25 * exp(-((($1 - d.created_at::double precision) / 3600.0) / 72.0)) + \
                      0.20 * LEAST(1.0, LN(1 + d.popularity_score) / LN(101.0)) \
                    ) AS final_score \
             FROM cn_search.post_search_documents d \
             WHERE d.topic_id = $2 \
               AND d.visibility = 'public' \
               AND d.is_deleted = FALSE \
               AND d.normalizer_version = $3 \
               AND d.search_text &@~ $4 \
             ORDER BY final_score DESC, d.created_at DESC \
             LIMIT $5 OFFSET $6",
        )
        .bind(now)
        .bind(topic)
        .bind(normalizer_version)
        .bind(&query_norm)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?
    };

    let mut items = Vec::new();
    for row in rows {
        let content: String = row.try_get("body_raw")?;
        let tags: Vec<String> = row.try_get("hashtags_norm")?;
        let created_at: i64 = row.try_get("created_at")?;
        items.push(json!({
            "event_id": row.try_get::<String, _>("post_id")?,
            "topic_id": row.try_get::<String, _>("topic_id")?,
            "kind": 1,
            "author": row.try_get::<String, _>("author_id")?,
            "created_at": created_at,
            "title": search_result_title(&content),
            "summary": search_result_summary(&content),
            "content": content,
            "tags": tags
        }));
    }

    let total = total.max(0) as u64;
    let next_offset = offset + items.len();
    let next_cursor = if (next_offset as u64) < total {
        Some(next_offset.to_string())
    } else {
        None
    };

    Ok(Json(json!({
        "topic": topic,
        "query": query,
        "items": items,
        "next_cursor": next_cursor,
        "total": total
    })))
}

pub async fn community_suggest(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<CommunitySuggestQuery>,
) -> ApiResult<Json<Value>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;
    apply_protected_rate_limit(&state, &auth, addr).await?;

    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let stage_a_limit = suggest_stage_a_limit(limit);
    let query_norm = search_normalizer::normalize_search_text(&query.q);
    let runtime = load_suggest_runtime_config(&state.pool).await;

    if query_norm.is_empty() {
        let backend_label = match runtime.backend {
            SuggestReadBackend::Pg => "pg",
            SuggestReadBackend::Legacy => "legacy",
        };
        return Ok(Json(json!({
            "query": query.q,
            "query_norm": query_norm,
            "backend": backend_label,
            "rerank_mode": runtime.rerank_mode.as_str(),
            "items": []
        })));
    }

    let stage_a_started = Instant::now();
    let (backend_label, stage_a_candidates) = match runtime.backend {
        SuggestReadBackend::Pg => {
            let pg_candidates =
                fetch_pg_community_candidates(&state.pool, &query_norm, stage_a_limit).await?;
            if pg_candidates.is_empty() {
                let fallback =
                    fetch_legacy_community_candidates(&state.pool, &query_norm, stage_a_limit)
                        .await?;
                ("legacy_fallback", fallback)
            } else {
                ("pg", pg_candidates)
            }
        }
        SuggestReadBackend::Legacy => (
            "legacy",
            fetch_legacy_community_candidates(&state.pool, &query_norm, stage_a_limit).await?,
        ),
    };
    let stage_a_elapsed = stage_a_started.elapsed();
    cn_core::metrics::observe_suggest_stage_a_latency_ms(
        crate::SERVICE_NAME,
        backend_label,
        stage_a_elapsed,
    );

    let mut items: Vec<Value> = stage_a_candidates
        .iter()
        .take(limit)
        .map(|candidate| {
            json!({
                "community_id": candidate.community_id,
                "name_match_score": candidate.name_match_score,
                "prefix_hit": candidate.prefix_hit,
                "exact_hit": candidate.exact_hit,
                "trgm_score": candidate.trgm_score,
            })
        })
        .collect();

    let mut blocked_or_muted_drop_count = 0usize;
    let mut visibility_drop_count = 0usize;
    let mut stage_b_latency_ms = 0.0_f64;
    let mut shadow_sampled = false;
    let mut shadow_topk_overlap = None;
    let mut shadow_rank_drift_count = None;

    if backend_label == "pg" && !stage_a_candidates.is_empty() {
        let muted_or_blocked_ids =
            fetch_muted_or_blocked_community_ids(&state.pool, &auth.pubkey).await?;
        let stage_b_started = Instant::now();
        let rerank_result = rerank_pg_community_candidates(
            &state.pool,
            &auth.pubkey,
            &stage_a_candidates,
            &muted_or_blocked_ids,
            limit,
            runtime.rerank_mode,
            runtime.relation_weights,
        )
        .await?;
        let stage_b_elapsed = stage_b_started.elapsed();
        stage_b_latency_ms = stage_b_elapsed.as_secs_f64() * 1000.0;
        cn_core::metrics::observe_suggest_stage_b_latency_ms(
            crate::SERVICE_NAME,
            runtime.rerank_mode.as_str(),
            stage_b_elapsed,
        );

        blocked_or_muted_drop_count = rerank_result.blocked_or_muted_drop_count;
        visibility_drop_count = rerank_result.visibility_drop_count;
        cn_core::metrics::inc_suggest_block_filter_drop_count(
            crate::SERVICE_NAME,
            backend_label,
            "block_or_mute",
            blocked_or_muted_drop_count as u64,
        );

        shadow_sampled = runtime.rerank_mode == SuggestRerankMode::Shadow
            && should_sample_shadow(runtime.shadow_sample_rate, &auth.pubkey, &query_norm);
        if shadow_sampled {
            let top_k = limit.min(10);
            shadow_topk_overlap = Some(shadow_top_k_overlap(
                &stage_a_candidates,
                &rerank_result.candidates,
                top_k,
            ));
            shadow_rank_drift_count = Some(
                rerank_result
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.stage_a_rank != candidate.stage_b_rank)
                    .count() as u64,
            );
        }

        items = rerank_result
            .candidates
            .into_iter()
            .map(|candidate| {
                json!({
                    "community_id": candidate.community_id,
                    "name_match_score": candidate.name_match_score,
                    "prefix_hit": candidate.prefix_hit,
                    "exact_hit": candidate.exact_hit,
                    "trgm_score": candidate.trgm_score,
                    "relation_score": candidate.relation_score,
                    "global_popularity": candidate.global_popularity,
                    "recency_boost": candidate.recency_boost,
                    "final_suggest_score": candidate.final_suggest_score,
                    "stage_a_rank": candidate.stage_a_rank,
                    "stage_b_rank": candidate.stage_b_rank,
                })
            })
            .collect();
    }

    tracing::info!(
        backend = backend_label,
        rerank_mode = runtime.rerank_mode.as_str(),
        stage_a_candidate_count = stage_a_candidates.len(),
        result_count = items.len(),
        blocked_or_muted_drop_count = blocked_or_muted_drop_count,
        visibility_drop_count = visibility_drop_count,
        shadow_sampled = shadow_sampled,
        suggest_stage_a_latency_ms = stage_a_elapsed.as_secs_f64() * 1000.0,
        suggest_stage_b_latency_ms = stage_b_latency_ms,
        suggest_block_filter_drop_count = blocked_or_muted_drop_count,
        "community suggest query processed"
    );

    let mut response = json!({
        "query": query.q,
        "query_norm": query_norm,
        "backend": backend_label,
        "rerank_mode": runtime.rerank_mode.as_str(),
        "stage_a_candidate_count": stage_a_candidates.len(),
        "blocked_or_muted_drop_count": blocked_or_muted_drop_count,
        "visibility_drop_count": visibility_drop_count,
        "items": items
    });

    if shadow_sampled {
        if let Some(object) = response.as_object_mut() {
            object.insert(
                "shadow_topk_overlap".to_string(),
                json!(shadow_topk_overlap.unwrap_or(0.0)),
            );
            object.insert(
                "shadow_rank_drift_count".to_string(),
                json!(shadow_rank_drift_count.unwrap_or(0)),
            );
        }
    }

    Ok(Json(response))
}

fn suggest_stage_a_limit(limit: usize) -> usize {
    limit
        .saturating_mul(SUGGEST_STAGE_A_LIMIT_MULTIPLIER)
        .min(SUGGEST_STAGE_A_MAX_LIMIT)
}

async fn load_suggest_runtime_config(pool: &sqlx::Pool<Postgres>) -> SuggestRuntimeConfig {
    let flags = match search_runtime_flags::load_search_runtime_flags(pool).await {
        Ok(flags) => flags,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to load search runtime flags; fallback to legacy suggest backend"
            );
            return SuggestRuntimeConfig {
                backend: SuggestReadBackend::Legacy,
                rerank_mode: SuggestRerankMode::Shadow,
                relation_weights: SuggestRelationWeights::default(),
                shadow_sample_rate: 0,
            };
        }
    };

    let backend = if flags
        .suggest_read_backend
        .trim()
        .eq_ignore_ascii_case(search_runtime_flags::SUGGEST_READ_BACKEND_PG)
    {
        SuggestReadBackend::Pg
    } else {
        SuggestReadBackend::Legacy
    };

    SuggestRuntimeConfig {
        backend,
        rerank_mode: parse_suggest_rerank_mode(&flags.suggest_rerank_mode),
        relation_weights: parse_suggest_relation_weights(&flags.suggest_relation_weights),
        shadow_sample_rate: parse_shadow_sample_rate(&flags.shadow_sample_rate),
    }
}

fn parse_suggest_rerank_mode(value: &str) -> SuggestRerankMode {
    if value
        .trim()
        .eq_ignore_ascii_case(search_runtime_flags::SUGGEST_RERANK_MODE_ENABLED)
    {
        SuggestRerankMode::Enabled
    } else {
        SuggestRerankMode::Shadow
    }
}

fn parse_suggest_relation_weights(raw: &str) -> SuggestRelationWeights {
    let default = SuggestRelationWeights::default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return default;
    }

    let value: Value = match serde_json::from_str(trimmed) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(
                error = %err,
                suggest_relation_weights = trimmed,
                "invalid suggest_relation_weights; fallback to default"
            );
            return default;
        }
    };

    let Some(object) = value.as_object() else {
        tracing::warn!(
            suggest_relation_weights = trimmed,
            "suggest_relation_weights must be a JSON object; fallback to default"
        );
        return default;
    };

    SuggestRelationWeights {
        is_member: parse_relation_weight(object, "is_member", default.is_member),
        is_following_community: parse_relation_weight(
            object,
            "is_following_community",
            default.is_following_community,
        ),
        friends_member_count: parse_relation_weight(
            object,
            "friends_member_count",
            default.friends_member_count,
        ),
        two_hop_follow_count: parse_relation_weight(
            object,
            "two_hop_follow_count",
            default.two_hop_follow_count,
        ),
        last_view_decay: parse_relation_weight(object, "last_view_decay", default.last_view_decay),
        muted_or_blocked: parse_relation_weight(
            object,
            "muted_or_blocked",
            default.muted_or_blocked,
        ),
    }
}

fn parse_relation_weight(object: &serde_json::Map<String, Value>, key: &str, default: f64) -> f64 {
    object.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn parse_shadow_sample_rate(raw: &str) -> u8 {
    raw.trim()
        .parse::<u8>()
        .map(|rate| rate.min(100))
        .unwrap_or(0)
}

fn should_sample_shadow(rate: u8, viewer_id: &str, query_norm: &str) -> bool {
    if rate >= 100 {
        return true;
    }
    if rate == 0 {
        return false;
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(viewer_id.as_bytes());
    hasher.update(b":");
    hasher.update(query_norm.as_bytes());
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();
    let bucket = u16::from_be_bytes([bytes[0], bytes[1]]) % 100;
    bucket < rate as u16
}

fn shadow_top_k_overlap(
    stage_a_candidates: &[CommunityCandidate],
    reranked_candidates: &[RerankedCommunityCandidate],
    top_k: usize,
) -> f64 {
    if top_k == 0 {
        return 1.0;
    }

    let stage_a_top: HashSet<&str> = stage_a_candidates
        .iter()
        .take(top_k)
        .map(|candidate| candidate.community_id.as_str())
        .collect();

    let mut stage_b_sorted: Vec<&RerankedCommunityCandidate> = reranked_candidates.iter().collect();
    stage_b_sorted.sort_by_key(|candidate| candidate.stage_b_rank);
    let stage_b_top: HashSet<&str> = stage_b_sorted
        .into_iter()
        .take(top_k)
        .map(|candidate| candidate.community_id.as_str())
        .collect();

    let overlap = stage_a_top.intersection(&stage_b_top).count();
    overlap as f64 / top_k as f64
}

async fn fetch_muted_or_blocked_community_ids(
    pool: &sqlx::Pool<Postgres>,
    viewer_id: &str,
) -> ApiResult<Vec<String>> {
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    let rows = sqlx::query(
        "SELECT raw_json \
         FROM cn_relay.events \
         WHERE pubkey = $1 \
           AND kind = 10000 \
           AND is_deleted = FALSE \
           AND is_current = TRUE \
           AND is_ephemeral = FALSE \
           AND (expires_at IS NULL OR expires_at > $2)",
    )
    .bind(viewer_id)
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let mut blocked_or_muted = HashSet::new();
    for row in rows {
        let raw_json: Value = row.try_get("raw_json")?;
        let Ok(raw_event) = nostr::parse_event(&raw_json) else {
            continue;
        };

        for topic_id in raw_event.topic_ids() {
            if let Some(community_id) =
                community_search_terms::community_id_from_topic_id(topic_id.trim())
            {
                blocked_or_muted.insert(community_id);
            }
        }

        for address in raw_event.tag_values("a") {
            if let Some(community_id) = extract_community_id_from_address_tag(address.trim()) {
                blocked_or_muted.insert(community_id);
            }
        }
    }

    let mut community_ids: Vec<String> = blocked_or_muted.into_iter().collect();
    community_ids.sort();
    Ok(community_ids)
}

fn extract_community_id_from_address_tag(address: &str) -> Option<String> {
    if address.starts_with("kukuri:") {
        return community_search_terms::community_id_from_topic_id(address);
    }

    let mut parts = address.splitn(3, ':');
    let _kind = parts.next()?;
    let _author = parts.next()?;
    let topic_like_id = parts.next()?.trim();
    community_search_terms::community_id_from_topic_id(topic_like_id)
}

async fn rerank_pg_community_candidates(
    pool: &sqlx::Pool<Postgres>,
    viewer_id: &str,
    stage_a_candidates: &[CommunityCandidate],
    muted_or_blocked_ids: &[String],
    limit: usize,
    rerank_mode: SuggestRerankMode,
    relation_weights: SuggestRelationWeights,
) -> ApiResult<SuggestRerankResult> {
    if stage_a_candidates.is_empty() {
        return Ok(SuggestRerankResult {
            candidates: Vec::new(),
            blocked_or_muted_drop_count: 0,
            visibility_drop_count: 0,
        });
    }

    let stage_a_ids: Vec<String> = stage_a_candidates
        .iter()
        .map(|candidate| candidate.community_id.clone())
        .collect();
    let stage_a_name_scores: Vec<f64> = stage_a_candidates
        .iter()
        .map(|candidate| candidate.name_match_score)
        .collect();
    let stage_a_prefix_hits: Vec<bool> = stage_a_candidates
        .iter()
        .map(|candidate| candidate.prefix_hit)
        .collect();
    let stage_a_exact_hits: Vec<bool> = stage_a_candidates
        .iter()
        .map(|candidate| candidate.exact_hit)
        .collect();
    let stage_a_trgm_scores: Vec<f64> = stage_a_candidates
        .iter()
        .map(|candidate| candidate.trgm_score)
        .collect();
    let stage_a_ranks: Vec<i32> = (1..=stage_a_candidates.len() as i32).collect();
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as f64;

    let blocked_or_muted_set: HashSet<&str> = muted_or_blocked_ids
        .iter()
        .map(|value| value.as_str())
        .collect();
    let blocked_or_muted_drop_count = stage_a_candidates
        .iter()
        .filter(|candidate| blocked_or_muted_set.contains(candidate.community_id.as_str()))
        .count();

    let rows = sqlx::query(
        "WITH candidate AS ( \
             SELECT * \
             FROM unnest( \
               $1::text[], \
               $2::double precision[], \
               $3::boolean[], \
               $4::boolean[], \
               $5::double precision[], \
               $6::integer[] \
             ) AS c(community_id, name_match_score, prefix_hit, exact_hit, trgm_score, stage_a_rank) \
         ), \
         viewer_following AS ( \
             SELECT topic_id \
             FROM cn_user.topic_subscriptions \
             WHERE subscriber_pubkey = $7 \
               AND status = 'active' \
         ), \
         viewer_membership AS ( \
             SELECT topic_id \
             FROM cn_user.topic_memberships \
             WHERE pubkey = $7 \
               AND status = 'active' \
         ), \
         joined AS ( \
             SELECT \
                 c.community_id, \
                 c.name_match_score, \
                 c.prefix_hit, \
                 c.exact_hit, \
                 c.trgm_score, \
                 c.stage_a_rank, \
                 (c.community_id = ANY($8::text[])) AS is_hidden, \
                 COALESCE(ns.enabled, FALSE) AS is_public, \
                 COALESCE(ns.ref_count, 0) AS ref_count, \
                 (vf.topic_id IS NOT NULL) AS is_following_live, \
                 (vm.topic_id IS NOT NULL) AS is_member_live, \
                 COALESCE((a.signals_json ->> 'is_member')::boolean, FALSE) AS is_member_signal, \
                 COALESCE((a.signals_json ->> 'is_following_community')::boolean, FALSE) AS is_following_signal, \
                 COALESCE((a.signals_json ->> 'friends_member_count')::double precision, 0.0) AS friends_member_count, \
                 COALESCE((a.signals_json ->> 'two_hop_follow_count')::double precision, 0.0) AS two_hop_follow_count, \
                 COALESCE((a.signals_json ->> 'last_seen_at')::double precision, 0.0) AS last_seen_at \
             FROM candidate c \
             LEFT JOIN cn_search.user_community_affinity a \
               ON a.user_id = $7 \
              AND a.community_id = c.community_id \
             LEFT JOIN cn_admin.node_subscriptions ns \
               ON ns.topic_id = c.community_id \
             LEFT JOIN viewer_following vf \
               ON vf.topic_id = c.community_id \
             LEFT JOIN viewer_membership vm \
               ON vm.topic_id = c.community_id \
         ), \
         scored AS ( \
             SELECT \
                 community_id, \
                 name_match_score, \
                 prefix_hit, \
                 exact_hit, \
                 trgm_score, \
                 stage_a_rank, \
                 is_hidden, \
                 is_public, \
                 is_following_live, \
                 is_member_live, \
                 ( \
                     $10 * CASE WHEN (is_member_live OR is_member_signal) THEN 1.0 ELSE 0.0 END + \
                     $11 * CASE WHEN (is_following_live OR is_following_signal) THEN 1.0 ELSE 0.0 END + \
                     $12 * LEAST(1.0, GREATEST(0.0, friends_member_count) / 5.0) + \
                     $13 * LEAST(1.0, GREATEST(0.0, two_hop_follow_count) / 10.0) + \
                     $14 * CASE \
                           WHEN last_seen_at > 0.0 \
                           THEN EXP(-(GREATEST(0.0, ($9 - last_seen_at) / 3600.0) / 168.0)) \
                           ELSE 0.0 \
                         END + \
                     $15 * CASE WHEN is_hidden THEN 1.0 ELSE 0.0 END \
                 ) AS relation_score, \
                 LEAST(1.0, LN(1.0 + GREATEST(ref_count, 0)::double precision) / LN(101.0)) AS global_popularity, \
                 CASE \
                     WHEN last_seen_at > 0.0 \
                     THEN EXP(-(GREATEST(0.0, ($9 - last_seen_at) / 3600.0) / 168.0)) \
                     ELSE 0.0 \
                 END AS recency_boost \
             FROM joined \
         ), \
         filtered AS ( \
             SELECT * \
             FROM scored \
             WHERE is_hidden = FALSE \
               AND (is_public = TRUE OR is_following_live = TRUE OR is_member_live = TRUE) \
         ), \
         ranked AS ( \
             SELECT \
                 community_id, \
                 name_match_score, \
                 prefix_hit, \
                 exact_hit, \
                 trgm_score, \
                 relation_score, \
                 global_popularity, \
                 recency_boost, \
                 ( \
                     0.40 * name_match_score + \
                     0.45 * relation_score + \
                     0.10 * global_popularity + \
                     0.05 * recency_boost \
                 ) AS final_suggest_score, \
                stage_a_rank::bigint AS stage_a_rank, \
                ROW_NUMBER() OVER ( \
                    ORDER BY \
                        ( \
                             0.40 * name_match_score + \
                             0.45 * relation_score + \
                             0.10 * global_popularity + \
                             0.05 * recency_boost \
                         ) DESC, \
                         stage_a_rank ASC, \
                         community_id ASC \
                 ) AS stage_b_rank, \
                 COUNT(*) OVER () AS filtered_total_count \
             FROM filtered \
         ) \
         SELECT \
             community_id, \
             name_match_score, \
             prefix_hit, \
             exact_hit, \
             trgm_score, \
             relation_score, \
             global_popularity, \
             recency_boost, \
             final_suggest_score, \
             stage_a_rank, \
             stage_b_rank, \
             filtered_total_count \
         FROM ranked \
         ORDER BY \
             CASE WHEN $16 = 'enabled' THEN final_suggest_score ELSE NULL END DESC NULLS LAST, \
             stage_a_rank ASC, \
             community_id ASC \
         LIMIT $17",
    )
    .bind(&stage_a_ids)
    .bind(&stage_a_name_scores)
    .bind(&stage_a_prefix_hits)
    .bind(&stage_a_exact_hits)
    .bind(&stage_a_trgm_scores)
    .bind(&stage_a_ranks)
    .bind(viewer_id)
    .bind(muted_or_blocked_ids)
    .bind(now)
    .bind(relation_weights.is_member)
    .bind(relation_weights.is_following_community)
    .bind(relation_weights.friends_member_count)
    .bind(relation_weights.two_hop_follow_count)
    .bind(relation_weights.last_view_decay)
    .bind(relation_weights.muted_or_blocked)
    .bind(rerank_mode.as_str())
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut filtered_total_count = 0usize;
    let mut reranked_candidates = Vec::with_capacity(rows.len());
    for row in rows {
        let filtered_total: i64 = row.try_get("filtered_total_count")?;
        filtered_total_count = filtered_total.max(0) as usize;
        reranked_candidates.push(RerankedCommunityCandidate {
            community_id: row.try_get("community_id")?,
            exact_hit: row.try_get("exact_hit")?,
            prefix_hit: row.try_get("prefix_hit")?,
            trgm_score: row.try_get("trgm_score")?,
            name_match_score: row.try_get("name_match_score")?,
            relation_score: row.try_get("relation_score")?,
            global_popularity: row.try_get("global_popularity")?,
            recency_boost: row.try_get("recency_boost")?,
            final_suggest_score: row.try_get("final_suggest_score")?,
            stage_a_rank: row.try_get("stage_a_rank")?,
            stage_b_rank: row.try_get("stage_b_rank")?,
        });
    }

    let visibility_drop_count = stage_a_candidates
        .len()
        .saturating_sub(blocked_or_muted_drop_count)
        .saturating_sub(filtered_total_count);

    Ok(SuggestRerankResult {
        candidates: reranked_candidates,
        blocked_or_muted_drop_count,
        visibility_drop_count,
    })
}

async fn fetch_pg_community_candidates(
    pool: &sqlx::Pool<Postgres>,
    query_norm: &str,
    limit: usize,
) -> ApiResult<Vec<CommunityCandidate>> {
    if query_norm.is_empty() {
        return Ok(Vec::new());
    }

    let prefix_pattern = format!("{query_norm}%");
    let query_len = query_norm.chars().count();
    let rows = if query_len <= 2 {
        match sqlx::query(
            "SELECT community_id, \
                    MAX((term_norm = $1)::int) AS exact_hit, \
                    MAX((term_norm LIKE $2)::int) AS prefix_hit, \
                    COALESCE(MAX(similarity(term_norm, $1)), 0.0) AS trgm_score \
             FROM cn_search.community_search_terms \
             WHERE term_norm LIKE $2 \
                OR similarity(term_norm, $1) >= 0.7 \
             GROUP BY community_id \
             ORDER BY exact_hit DESC, prefix_hit DESC, trgm_score DESC, community_id ASC \
             LIMIT $3",
        )
        .bind(query_norm)
        .bind(&prefix_pattern)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        {
            Ok(rows) => rows,
            Err(err) if is_missing_community_search_terms_table(&err) => return Ok(Vec::new()),
            Err(err) => {
                return Err(ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    err.to_string(),
                ));
            }
        }
    } else {
        match sqlx::query(
            "SELECT community_id, \
                    MAX((term_norm = $1)::int) AS exact_hit, \
                    MAX((term_norm LIKE $2)::int) AS prefix_hit, \
                    COALESCE(MAX(similarity(term_norm, $1)), 0.0) AS trgm_score \
             FROM cn_search.community_search_terms \
             WHERE term_norm LIKE $2 \
                OR term_norm % $1 \
             GROUP BY community_id \
             ORDER BY exact_hit DESC, prefix_hit DESC, trgm_score DESC, community_id ASC \
             LIMIT $3",
        )
        .bind(query_norm)
        .bind(&prefix_pattern)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        {
            Ok(rows) => rows,
            Err(err) if is_missing_community_search_terms_table(&err) => return Ok(Vec::new()),
            Err(err) => {
                return Err(ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    err.to_string(),
                ));
            }
        }
    };

    let mut candidates = Vec::new();
    for row in rows {
        let community_id: String = row.try_get("community_id")?;
        let exact_hit = row.try_get::<i32, _>("exact_hit").unwrap_or(0) > 0;
        let prefix_hit = row.try_get::<i32, _>("prefix_hit").unwrap_or(0) > 0;
        let trgm_score = row
            .try_get::<f64, _>("trgm_score")
            .or_else(|_| row.try_get::<f32, _>("trgm_score").map(f64::from))
            .unwrap_or(0.0);
        let name_match_score = candidate_name_match_score(exact_hit, prefix_hit, trgm_score);
        candidates.push(CommunityCandidate {
            community_id,
            exact_hit,
            prefix_hit,
            trgm_score,
            name_match_score,
        });
    }
    Ok(candidates)
}

async fn fetch_legacy_community_candidates(
    pool: &sqlx::Pool<Postgres>,
    query_norm: &str,
    limit: usize,
) -> ApiResult<Vec<CommunityCandidate>> {
    if query_norm.is_empty() {
        return Ok(Vec::new());
    }

    let community_ids = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT topic_id \
         FROM ( \
           SELECT topic_id FROM cn_admin.node_subscriptions \
           UNION \
           SELECT topic_id FROM cn_user.topic_subscriptions WHERE status = 'active' \
         ) AS source_topics",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let mut by_community = HashMap::<String, CommunityCandidate>::new();
    for topic_id in community_ids {
        let Some(community_id) = community_search_terms::community_id_from_topic_id(&topic_id)
        else {
            continue;
        };

        let terms = community_search_terms::build_terms_from_topic_id(&community_id);
        if terms.is_empty() {
            continue;
        }

        let entry =
            by_community
                .entry(community_id.clone())
                .or_insert_with(|| CommunityCandidate {
                    community_id: community_id.clone(),
                    exact_hit: false,
                    prefix_hit: false,
                    trgm_score: 0.0,
                    name_match_score: 0.0,
                });

        for term in terms {
            let exact_hit = term.term_norm == query_norm;
            let prefix_hit = term.term_norm.starts_with(query_norm);
            if !(exact_hit || prefix_hit) {
                continue;
            }

            let term_score = if exact_hit { 1.0 } else { 0.8 };
            entry.exact_hit |= exact_hit;
            entry.prefix_hit |= prefix_hit;
            if term_score > entry.trgm_score {
                entry.trgm_score = term_score;
            }
            entry.name_match_score =
                candidate_name_match_score(entry.exact_hit, entry.prefix_hit, entry.trgm_score);
        }
    }

    let mut candidates: Vec<CommunityCandidate> = by_community
        .into_values()
        .filter(|candidate| candidate.exact_hit || candidate.prefix_hit)
        .collect();
    candidates.sort_by(compare_community_candidates);
    candidates.truncate(limit);
    Ok(candidates)
}

fn compare_community_candidates(left: &CommunityCandidate, right: &CommunityCandidate) -> Ordering {
    right
        .exact_hit
        .cmp(&left.exact_hit)
        .then_with(|| right.prefix_hit.cmp(&left.prefix_hit))
        .then_with(|| {
            right
                .name_match_score
                .partial_cmp(&left.name_match_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            right
                .trgm_score
                .partial_cmp(&left.trgm_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.community_id.cmp(&right.community_id))
}

fn candidate_name_match_score(exact_hit: bool, prefix_hit: bool, trgm_score: f64) -> f64 {
    if exact_hit {
        return 1.0;
    }

    let trgm_score = trgm_score.clamp(0.0, 1.0);
    if prefix_hit {
        trgm_score.max(0.75).min(0.99)
    } else {
        trgm_score
    }
}

fn is_missing_community_search_terms_table(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            matches!(db_err.code().as_deref(), Some("42P01") | Some("3F000"))
        }
        _ => false,
    }
}

fn search_result_title(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();
    truncate_chars(first_line, 80)
}

fn search_result_summary(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    truncate_chars(trimmed, 200)
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value.chars().take(max).collect()
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

#[allow(clippy::result_large_err)]
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
    builder.push(" AND review_status = ");
    builder.push_bind("active");
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
    use super::{advisory_lock_keys_for_pubkey, parse_trust_subject};
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

    #[test]
    fn advisory_lock_keys_are_stable_for_same_pubkey() {
        let pubkey = Keys::generate().public_key().to_hex();
        let first = advisory_lock_keys_for_pubkey(&pubkey);
        let second = advisory_lock_keys_for_pubkey(&pubkey);
        assert_eq!(first, second);
    }

    #[test]
    fn advisory_lock_keys_differ_for_different_pubkeys() {
        let pubkey_a = Keys::generate().public_key().to_hex();
        let mut pubkey_b = Keys::generate().public_key().to_hex();
        while pubkey_b == pubkey_a {
            pubkey_b = Keys::generate().public_key().to_hex();
        }
        let first = advisory_lock_keys_for_pubkey(&pubkey_a);
        let second = advisory_lock_keys_for_pubkey(&pubkey_b);
        assert_ne!(first, second);
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
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;
    use tokio::sync::{Mutex, OnceCell};
    use tower::ServiceExt;
    use uuid::Uuid;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
    static SEARCH_BACKEND_TEST_LOCK: OnceCell<Mutex<()>> = OnceCell::const_new();

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

    async fn lock_search_backend_contract_tests() -> tokio::sync::MutexGuard<'static, ()> {
        SEARCH_BACKEND_TEST_LOCK
            .get_or_init(|| async { Mutex::new(()) })
            .await
            .lock()
            .await
    }

    async fn test_state_with_meili_url_bootstrap_auth_mode_and_user_config(
        meili_url: &str,
        bootstrap_auth_mode: &str,
        user_config_json: Value,
    ) -> crate::AppState {
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
        let user_config = service_config::static_handle(user_config_json);
        let bootstrap_config = service_config::static_handle(serde_json::json!({
            "auth": { "mode": bootstrap_auth_mode }
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
            bootstrap_hints: Arc::new(crate::BootstrapHintStore::default()),
        }
    }

    async fn test_state_with_meili_url_and_bootstrap_auth_mode(
        meili_url: &str,
        bootstrap_auth_mode: &str,
    ) -> crate::AppState {
        test_state_with_meili_url_bootstrap_auth_mode_and_user_config(
            meili_url,
            bootstrap_auth_mode,
            serde_json::json!({
                "rate_limit": { "enabled": false }
            }),
        )
        .await
    }

    async fn test_state_with_meili_url(meili_url: &str) -> crate::AppState {
        test_state_with_meili_url_and_bootstrap_auth_mode(meili_url, "off").await
    }

    async fn test_state() -> crate::AppState {
        test_state_with_meili_url("http://localhost:7700").await
    }

    async fn test_state_with_bootstrap_auth_required() -> crate::AppState {
        test_state_with_meili_url_and_bootstrap_auth_mode("http://localhost:7700", "required").await
    }

    async fn test_state_with_rate_limits(
        auth_per_minute: u64,
        public_per_minute: u64,
        protected_per_minute: u64,
    ) -> crate::AppState {
        test_state_with_meili_url_bootstrap_auth_mode_and_user_config(
            "http://localhost:7700",
            "off",
            serde_json::json!({
                "rate_limit": {
                    "enabled": true,
                    "auth_per_minute": auth_per_minute,
                    "public_per_minute": public_per_minute,
                    "protected_per_minute": protected_per_minute
                }
            }),
        )
        .await
    }

    fn issue_token(config: &cn_core::auth::JwtConfig, pubkey: &str) -> String {
        let (token, _) = cn_core::auth::issue_token(pubkey, config).expect("issue token");
        token
    }

    async fn ensure_consents(pool: &Pool<Postgres>, pubkey: &str) {
        // 別テストが同時に current policy を追加しても取りこぼさないように短い再試行を行う。
        for _ in 0..5 {
            let missing_policies = sqlx::query_scalar::<_, String>(
                "SELECT p.policy_id \
                 FROM cn_admin.policies p \
                 LEFT JOIN cn_user.policy_consents c \
                   ON c.policy_id = p.policy_id AND c.accepter_pubkey = $1 \
                 WHERE p.is_current = TRUE \
                   AND p.type IN ('terms', 'privacy') \
                   AND c.policy_id IS NULL",
            )
            .bind(pubkey)
            .fetch_all(pool)
            .await
            .expect("fetch missing policies");

            if missing_policies.is_empty() {
                return;
            }

            for policy_id in missing_policies {
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

            tokio::task::yield_now().await;
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

    async fn set_search_runtime_flags(pool: &Pool<Postgres>, read_backend: &str, write_mode: &str) {
        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) \
             VALUES ($1, $2, 'contract-test') \
             ON CONFLICT (flag_name) DO UPDATE \
             SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(cn_core::search_runtime_flags::FLAG_SEARCH_READ_BACKEND)
        .bind(read_backend)
        .execute(pool)
        .await
        .expect("upsert search_read_backend flag");

        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) \
             VALUES ($1, $2, 'contract-test') \
             ON CONFLICT (flag_name) DO UPDATE \
             SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(cn_core::search_runtime_flags::FLAG_SEARCH_WRITE_MODE)
        .bind(write_mode)
        .execute(pool)
        .await
        .expect("upsert search_write_mode flag");
    }

    async fn set_suggest_runtime_flag(pool: &Pool<Postgres>, backend: &str) {
        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) \
             VALUES ($1, $2, 'contract-test') \
             ON CONFLICT (flag_name) DO UPDATE \
             SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(cn_core::search_runtime_flags::FLAG_SUGGEST_READ_BACKEND)
        .bind(backend)
        .execute(pool)
        .await
        .expect("upsert suggest_read_backend flag");
    }

    async fn set_suggest_rerank_mode(pool: &Pool<Postgres>, mode: &str) {
        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) \
             VALUES ($1, $2, 'contract-test') \
             ON CONFLICT (flag_name) DO UPDATE \
             SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(cn_core::search_runtime_flags::FLAG_SUGGEST_RERANK_MODE)
        .bind(mode)
        .execute(pool)
        .await
        .expect("upsert suggest_rerank_mode flag");
    }

    async fn set_suggest_relation_weights(pool: &Pool<Postgres>, weights: &str) {
        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) \
             VALUES ($1, $2, 'contract-test') \
             ON CONFLICT (flag_name) DO UPDATE \
             SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(cn_core::search_runtime_flags::FLAG_SUGGEST_RELATION_WEIGHTS)
        .bind(weights)
        .execute(pool)
        .await
        .expect("upsert suggest_relation_weights flag");
    }

    async fn set_shadow_sample_rate(pool: &Pool<Postgres>, sample_rate: &str) {
        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) \
             VALUES ($1, $2, 'contract-test') \
             ON CONFLICT (flag_name) DO UPDATE \
             SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(cn_core::search_runtime_flags::FLAG_SHADOW_SAMPLE_RATE)
        .bind(sample_rate)
        .execute(pool)
        .await
        .expect("upsert shadow_sample_rate flag");
    }

    async fn insert_node_subscription(pool: &Pool<Postgres>, topic_id: &str) {
        sqlx::query(
            "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) \
             VALUES ($1, TRUE, 5) \
             ON CONFLICT (topic_id) DO UPDATE \
             SET enabled = TRUE, ref_count = GREATEST(cn_admin.node_subscriptions.ref_count, 5), updated_at = NOW()",
        )
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("insert node subscription");
    }

    async fn insert_community_search_terms(pool: &Pool<Postgres>, community_id: &str) {
        let terms = cn_core::community_search_terms::build_terms_from_topic_id(community_id);
        for term in terms {
            sqlx::query(
                "INSERT INTO cn_search.community_search_terms \
                 (community_id, term_type, term_raw, term_norm, is_primary, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, NOW()) \
                 ON CONFLICT (community_id, term_type, term_norm) DO UPDATE \
                 SET term_raw = EXCLUDED.term_raw, is_primary = EXCLUDED.is_primary, updated_at = NOW()",
            )
            .bind(community_id)
            .bind(term.term_type)
            .bind(term.term_raw)
            .bind(term.term_norm)
            .bind(term.is_primary)
            .execute(pool)
            .await
                .expect("insert community search term");
        }
    }

    async fn insert_user_community_affinity(
        pool: &Pool<Postgres>,
        user_id: &str,
        community_id: &str,
        signals_json: Value,
    ) {
        sqlx::query(
            "INSERT INTO cn_search.user_community_affinity \
             (user_id, community_id, relation_score, signals_json, computed_at) \
             VALUES ($1, $2, 0.0, $3, NOW()) \
             ON CONFLICT (user_id, community_id) DO UPDATE \
             SET relation_score = EXCLUDED.relation_score, \
                 signals_json = EXCLUDED.signals_json, \
                 computed_at = NOW()",
        )
        .bind(user_id)
        .bind(community_id)
        .bind(signals_json)
        .execute(pool)
        .await
        .expect("insert user community affinity");
    }

    async fn insert_mute_event_for_community(
        pool: &Pool<Postgres>,
        user_id: &str,
        community_id: &str,
    ) {
        let event_id = format!("mute-{}", Uuid::new_v4());
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let tags = json!([["t", community_id]]);
        let raw_json = json!({
            "id": event_id,
            "pubkey": user_id,
            "kind": 10000,
            "created_at": now,
            "tags": tags,
            "content": "",
            "sig": "sig"
        });
        sqlx::query(
            "INSERT INTO cn_relay.events (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, is_deleted, is_ephemeral, is_current, expires_at) \
             VALUES ($1, $2, 10000, $3, $4, '', 'sig', $5, FALSE, FALSE, TRUE, NULL) \
             ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(now)
        .bind(tags)
        .bind(raw_json)
        .execute(pool)
        .await
        .expect("insert mute list event");
    }

    async fn run_alias_backfill_for_topic(pool: &Pool<Postgres>, topic_id: &str) {
        sqlx::query(
            "WITH source_topics AS ( \
                 SELECT topic_id \
                 FROM cn_admin.node_subscriptions \
                 WHERE topic_id = $1 \
                 UNION \
                 SELECT topic_id \
                 FROM cn_user.topic_subscriptions \
                 WHERE status = 'active' \
                   AND topic_id = $1 \
             ), \
             normalized_terms AS ( \
                 SELECT \
                     topic_id, \
                     TRIM(REGEXP_REPLACE(LOWER(topic_id), '[^[:alnum:]#@]+', ' ', 'g')) AS name_norm, \
                     TRIM( \
                         REGEXP_REPLACE( \
                             LOWER(REGEXP_REPLACE(topic_id, '^kukuri:(tauri:)?', '')), \
                             '[^[:alnum:]#@]+', \
                             ' ', \
                             'g' \
                         ) \
                     ) AS alias_norm \
                 FROM source_topics \
             ) \
             INSERT INTO cn_search.community_search_terms \
                 (community_id, term_type, term_raw, term_norm, is_primary) \
             SELECT \
                 topic_id, \
                 'alias', \
                 topic_id, \
                 alias_norm, \
                 TRUE \
             FROM normalized_terms \
             WHERE alias_norm <> '' \
               AND alias_norm <> name_norm \
               AND LOWER(TRIM(topic_id)) !~ '^kukuri:[0-9a-f]{64}$' \
             ON CONFLICT (community_id, term_type, term_norm) DO NOTHING",
        )
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("run community search alias backfill");
    }

    async fn insert_pending_subscription_request(
        pool: &Pool<Postgres>,
        pubkey: &str,
        topic_id: &str,
    ) -> String {
        let request_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO cn_user.topic_subscription_requests              (request_id, requester_pubkey, topic_id, requested_services, status)              VALUES ($1, $2, $3, $4, 'pending')",
        )
        .bind(&request_id)
        .bind(pubkey)
        .bind(topic_id)
        .bind(json!(["relay", "index"]))
        .execute(pool)
        .await
        .expect("insert pending subscription request");
        request_id
    }

    async fn update_subscription_request_status(
        pool: &Pool<Postgres>,
        request_id: &str,
        status: &str,
    ) {
        let result = sqlx::query(
            "UPDATE cn_user.topic_subscription_requests SET status = $1, reviewed_at = NOW() WHERE request_id = $2",
        )
        .bind(status)
        .bind(request_id)
        .execute(pool)
        .await
        .expect("update subscription request status");
        assert_eq!(result.rows_affected(), 1);
    }

    async fn pending_subscription_request_count_for_pubkey(
        pool: &Pool<Postgres>,
        pubkey: &str,
    ) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1 AND status = 'pending'",
        )
        .bind(pubkey)
        .fetch_one(pool)
        .await
        .expect("count pending subscription requests")
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

    async fn assign_active_plan_limit(
        pool: &Pool<Postgres>,
        pubkey: &str,
        metric: &str,
        window: &str,
        limit: i64,
    ) {
        let plan_id = format!("contract-plan-{}", Uuid::new_v4());
        let subscription_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO cn_user.plans (plan_id, name, is_active) VALUES ($1, $2, TRUE) ON CONFLICT (plan_id) DO NOTHING",
        )
        .bind(&plan_id)
        .bind("Contract Test Plan")
        .execute(pool)
        .await
        .expect("insert contract plan");
        sqlx::query(
            "INSERT INTO cn_user.plan_limits (plan_id, metric, \"window\", \"limit\") VALUES ($1, $2, $3, $4)",
        )
        .bind(&plan_id)
        .bind(metric)
        .bind(window)
        .bind(limit)
        .execute(pool)
        .await
        .expect("insert contract plan limit");
        sqlx::query(
            "UPDATE cn_user.subscriptions SET status = 'ended', ended_at = NOW() WHERE subscriber_pubkey = $1 AND status = 'active'",
        )
        .bind(pubkey)
        .execute(pool)
        .await
        .expect("deactivate existing subscriptions");
        sqlx::query(
            "INSERT INTO cn_user.subscriptions (subscription_id, subscriber_pubkey, plan_id, status) VALUES ($1, $2, $3, 'active')",
        )
        .bind(subscription_id)
        .bind(pubkey)
        .bind(plan_id)
        .execute(pool)
        .await
        .expect("insert active subscription");
    }

    async fn usage_event_count_for_request_id(
        pool: &Pool<Postgres>,
        pubkey: &str,
        metric: &str,
        request_id: &str,
    ) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_user.usage_events WHERE subscriber_pubkey = $1 AND metric = $2 AND request_id = $3",
        )
        .bind(pubkey)
        .bind(metric)
        .bind(request_id)
        .fetch_one(pool)
        .await
        .expect("count usage_events")
    }

    async fn usage_counter_daily_count(pool: &Pool<Postgres>, pubkey: &str, metric: &str) -> i64 {
        let day = chrono::Utc::now().date_naive();
        sqlx::query_scalar::<_, i64>(
            "SELECT count FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1 AND metric = $2 AND day = $3",
        )
        .bind(pubkey)
        .bind(metric)
        .bind(day)
        .fetch_optional(pool)
        .await
        .expect("fetch usage counter daily")
        .unwrap_or(0)
    }

    async fn install_usage_events_commit_failure_trigger(
        pool: &Pool<Postgres>,
        request_id: &str,
    ) -> (String, String) {
        let suffix = Uuid::new_v4().simple().to_string();
        let function_name = format!("force_usage_events_commit_fail_{suffix}");
        let trigger_name = format!("force_usage_events_commit_fail_trigger_{suffix}");
        let escaped_request_id = request_id.replace('\'', "''");

        let create_function_sql = format!(
            "CREATE OR REPLACE FUNCTION cn_user.{function_name}() \
             RETURNS trigger LANGUAGE plpgsql AS $$ \
             BEGIN \
                 IF NEW.request_id = '{escaped_request_id}' THEN \
                     RAISE EXCEPTION 'forced usage_events commit failure for request_id={escaped_request_id}'; \
                 END IF; \
                 RETURN NULL; \
             END; \
             $$"
        );
        sqlx::query(&create_function_sql)
            .execute(pool)
            .await
            .expect("create usage_events commit failure function");

        let create_trigger_sql = format!(
            "CREATE CONSTRAINT TRIGGER {trigger_name} \
             AFTER INSERT ON cn_user.usage_events \
             DEFERRABLE INITIALLY DEFERRED \
             FOR EACH ROW EXECUTE FUNCTION cn_user.{function_name}()"
        );
        sqlx::query(&create_trigger_sql)
            .execute(pool)
            .await
            .expect("create usage_events commit failure trigger");

        (trigger_name, function_name)
    }

    async fn remove_usage_events_commit_failure_trigger(
        pool: &Pool<Postgres>,
        trigger_name: &str,
        function_name: &str,
    ) {
        let drop_trigger_sql =
            format!("DROP TRIGGER IF EXISTS {trigger_name} ON cn_user.usage_events");
        sqlx::query(&drop_trigger_sql)
            .execute(pool)
            .await
            .expect("drop usage_events commit failure trigger");

        let drop_function_sql = format!("DROP FUNCTION IF EXISTS cn_user.{function_name}()");
        sqlx::query(&drop_function_sql)
            .execute(pool)
            .await
            .expect("drop usage_events commit failure function");
    }

    async fn install_topic_subscription_commit_failure_trigger(
        pool: &Pool<Postgres>,
        topic_id: &str,
    ) -> (String, String) {
        let suffix = Uuid::new_v4().simple().to_string();
        let function_name = format!("force_topic_subscriptions_commit_fail_{suffix}");
        let trigger_name = format!("force_topic_subscriptions_commit_fail_trigger_{suffix}");
        let escaped_topic_id = topic_id.replace('\'', "''");

        let create_function_sql = format!(
            "CREATE OR REPLACE FUNCTION cn_user.{function_name}() \
             RETURNS trigger LANGUAGE plpgsql AS $$ \
             BEGIN \
                 IF NEW.topic_id = '{escaped_topic_id}' AND NEW.status = 'ended' THEN \
                     RAISE EXCEPTION 'forced topic_subscriptions commit failure for topic_id={escaped_topic_id}'; \
                 END IF; \
                 RETURN NULL; \
             END; \
             $$"
        );
        sqlx::query(&create_function_sql)
            .execute(pool)
            .await
            .expect("create topic_subscriptions commit failure function");

        let create_trigger_sql = format!(
            "CREATE CONSTRAINT TRIGGER {trigger_name} \
             AFTER UPDATE ON cn_user.topic_subscriptions \
             DEFERRABLE INITIALLY DEFERRED \
             FOR EACH ROW EXECUTE FUNCTION cn_user.{function_name}()"
        );
        sqlx::query(&create_trigger_sql)
            .execute(pool)
            .await
            .expect("create topic_subscriptions commit failure trigger");

        (trigger_name, function_name)
    }

    async fn remove_topic_subscription_commit_failure_trigger(
        pool: &Pool<Postgres>,
        trigger_name: &str,
        function_name: &str,
    ) {
        let drop_trigger_sql =
            format!("DROP TRIGGER IF EXISTS {trigger_name} ON cn_user.topic_subscriptions");
        sqlx::query(&drop_trigger_sql)
            .execute(pool)
            .await
            .expect("drop topic_subscriptions commit failure trigger");

        let drop_function_sql = format!("DROP FUNCTION IF EXISTS cn_user.{function_name}()");
        sqlx::query(&drop_function_sql)
            .execute(pool)
            .await
            .expect("drop topic_subscriptions commit failure function");
    }

    fn assert_quota_exceeded_response(
        payload: &Value,
        metric: &str,
        expected_current: i64,
        expected_limit: i64,
        expect_reset_at: bool,
    ) {
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("QUOTA_EXCEEDED")
        );
        let details = payload
            .get("details")
            .and_then(Value::as_object)
            .expect("quota details");
        assert_eq!(details.get("metric").and_then(Value::as_str), Some(metric));
        assert_eq!(
            details.get("current").and_then(Value::as_i64),
            Some(expected_current)
        );
        assert_eq!(
            details.get("limit").and_then(Value::as_i64),
            Some(expected_limit)
        );
        if expect_reset_at {
            assert!(details.get("reset_at").and_then(Value::as_i64).is_some());
        } else {
            assert!(details.get("reset_at").is_none());
        }
    }

    fn assert_rate_limited_contract(status: StatusCode, headers: &HeaderMap, payload: &Value) {
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("RATE_LIMITED")
        );
        let retry_after = headers
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        assert!(retry_after >= 1, "Retry-After must be >= 1: {retry_after}");
    }

    fn assert_pending_subscription_request_limit_response(
        payload: &Value,
        expected_current: i64,
        expected_limit: i64,
    ) {
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED")
        );
        let details = payload
            .get("details")
            .and_then(Value::as_object)
            .expect("pending request limit details");
        assert_eq!(
            details.get("metric").and_then(Value::as_str),
            Some("topic_subscription_requests.pending")
        );
        assert_eq!(details.get("scope").and_then(Value::as_str), Some("pubkey"));
        assert_eq!(
            details.get("current").and_then(Value::as_i64),
            Some(expected_current)
        );
        assert_eq!(
            details.get("limit").and_then(Value::as_i64),
            Some(expected_limit)
        );
    }

    fn prometheus_counter_value(body: &str, metric: &str, required_labels: &[(&str, &str)]) -> f64 {
        for line in body.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(sample) = parts.next() else {
                continue;
            };
            let Some(value_text) = parts.next() else {
                continue;
            };

            let (name, labels) = if let Some((name, rest)) = sample.split_once('{') {
                (name, rest.strip_suffix('}').unwrap_or(rest))
            } else {
                (sample, "")
            };
            if name != metric {
                continue;
            }

            let matched = required_labels
                .iter()
                .all(|(label, value)| labels.contains(&format!(r#"{label}="{value}""#)));
            if matched {
                return value_text.parse::<f64>().unwrap_or(0.0);
            }
        }
        0.0
    }

    fn assert_metric_line(body: &str, metric_name: &str, labels: &[(&str, &str)]) {
        let found = body.lines().any(|line| {
            if !line.starts_with(metric_name) {
                return false;
            }
            labels.iter().all(|(key, value)| {
                let token = format!("{key}=\"{value}\"");
                line.contains(&token)
            })
        });

        assert!(
            found,
            "metrics body did not contain {metric_name} with labels {labels:?}: {body}"
        );
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

    async fn set_label_review_status(pool: &Pool<Postgres>, label_id: &str, review_status: &str) {
        sqlx::query(
            "UPDATE cn_moderation.labels \
             SET review_status = $1, reviewed_by = 'contract-test', reviewed_at = NOW() \
             WHERE label_id = $2",
        )
        .bind(review_status)
        .bind(label_id)
        .execute(pool)
        .await
        .expect("update label review status");
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
        (
            status,
            content_type,
            String::from_utf8_lossy(&body).to_string(),
        )
    }

    async fn get_json(app: Router, uri: &str, token: &str) -> (StatusCode, Value) {
        get_json_with_headers(app, uri, token, &[]).await
    }

    async fn get_json_with_headers(
        app: Router,
        uri: &str,
        token: &str,
        extra_headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let (status, _, payload) =
            get_json_with_headers_and_response_headers(app, uri, token, extra_headers).await;
        (status, payload)
    }

    async fn get_json_with_headers_and_response_headers(
        app: Router,
        uri: &str,
        token: &str,
        extra_headers: &[(&str, &str)],
    ) -> (StatusCode, HeaderMap, Value) {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
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
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, headers, payload)
    }

    async fn post_json(app: Router, uri: &str, token: &str, payload: Value) -> (StatusCode, Value) {
        post_json_with_headers(app, uri, token, payload, &[]).await
    }

    async fn post_json_with_headers(
        app: Router,
        uri: &str,
        token: &str,
        payload: Value,
        extra_headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
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
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, payload)
    }

    async fn get_json_with_consent_retry(
        app: Router,
        uri: &str,
        token: &str,
        pool: &Pool<Postgres>,
        pubkey: &str,
    ) -> (StatusCode, Value) {
        let (status, payload) = get_json(app.clone(), uri, token).await;
        if status == StatusCode::PRECONDITION_REQUIRED {
            ensure_consents(pool, pubkey).await;
            return get_json(app, uri, token).await;
        }
        (status, payload)
    }

    async fn get_json_with_headers_and_consent_retry(
        app: Router,
        uri: &str,
        token: &str,
        pool: &Pool<Postgres>,
        pubkey: &str,
    ) -> (StatusCode, HeaderMap, Value) {
        for _ in 0..5 {
            let (status, headers, payload) =
                get_json_with_headers_and_response_headers(app.clone(), uri, token, &[]).await;
            if status != StatusCode::PRECONDITION_REQUIRED {
                return (status, headers, payload);
            }
            ensure_consents(pool, pubkey).await;
        }

        get_json_with_headers_and_response_headers(app, uri, token, &[]).await
    }

    async fn post_json_with_consent_retry(
        app: Router,
        uri: &str,
        token: &str,
        payload: Value,
        pool: &Pool<Postgres>,
        pubkey: &str,
    ) -> (StatusCode, Value) {
        let (status, body) = post_json(app.clone(), uri, token, payload.clone()).await;
        if status == StatusCode::PRECONDITION_REQUIRED {
            ensure_consents(pool, pubkey).await;
            return post_json(app, uri, token, payload).await;
        }
        (status, body)
    }

    async fn delete_json_with_consent_retry(
        app: Router,
        uri: &str,
        token: &str,
        pool: &Pool<Postgres>,
        pubkey: &str,
    ) -> (StatusCode, Value) {
        let (status, body) = delete_json(app.clone(), uri, token).await;
        if status == StatusCode::PRECONDITION_REQUIRED {
            ensure_consents(pool, pubkey).await;
            return delete_json(app, uri, token).await;
        }
        (status, body)
    }

    async fn post_json_public(app: Router, uri: &str, payload: Value) -> (StatusCode, Value) {
        let (status, _, body) = post_json_public_with_headers(app, uri, payload).await;
        (status, body)
    }

    async fn post_json_public_with_headers(
        app: Router,
        uri: &str,
        payload: Value,
    ) -> (StatusCode, HeaderMap, Value) {
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
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        (status, headers, payload)
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

    async fn spawn_mock_meili_health(
        status_code: Arc<AtomicU16>,
    ) -> (String, tokio::task::JoinHandle<()>) {
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
        let route = "/metrics-contract";
        cn_core::metrics::record_http_request(
            crate::SERVICE_NAME,
            "GET",
            route,
            200,
            std::time::Duration::from_millis(5),
        );

        let app = Router::new().route("/metrics", get(crate::metrics_endpoint));
        let (status, content_type, body) = get_text_public(app, "/metrics").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/plain; version=0.0.4"));
        assert!(
            body.contains("cn_up{service=\"cn-user-api\"} 1"),
            "metrics body did not contain cn_up for cn-user-api: {body}"
        );
        assert_metric_line(
            &body,
            "http_requests_total",
            &[
                ("service", crate::SERVICE_NAME),
                ("route", route),
                ("method", "GET"),
                ("status", "200"),
            ],
        );
        assert_metric_line(
            &body,
            "http_request_duration_seconds_bucket",
            &[
                ("service", crate::SERVICE_NAME),
                ("route", route),
                ("method", "GET"),
                ("status", "200"),
            ],
        );
    }

    #[tokio::test]
    async fn auth_consent_quota_metrics_regression_counters_increment() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let auth_keys = Keys::generate();
        let auth_pubkey = auth_keys.public_key().to_hex();
        let subscriber_pubkey = Keys::generate().public_key().to_hex();
        let token = issue_token(&state.jwt_config, &subscriber_pubkey);
        let consent_topic_id = format!("kukuri:consent-required-{}", Uuid::new_v4().simple());
        let existing_topic_id = format!("kukuri:quota-existing-{}", Uuid::new_v4().simple());
        let quota_topic_id = format!("kukuri:quota-exceeded-{}", Uuid::new_v4().simple());
        insert_current_policy(&pool, "terms", "v1.0.0", "ja-JP", "Terms").await;
        insert_current_policy(&pool, "privacy", "v1.0.0", "ja-JP", "Privacy").await;

        let app = Router::new()
            .route("/metrics", get(crate::metrics_endpoint))
            .route("/v1/auth/challenge", post(crate::auth::auth_challenge))
            .route("/v1/auth/verify", post(crate::auth::auth_verify))
            .route(
                "/v1/topic-subscription-requests",
                post(create_subscription_request),
            )
            .with_state(state);

        let (status, content_type, before_body) = get_text_public(app.clone(), "/metrics").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/plain; version=0.0.4"));
        let auth_success_before = prometheus_counter_value(
            &before_body,
            "auth_success_total",
            &[("service", "cn-user-api")],
        );
        let auth_failure_before = prometheus_counter_value(
            &before_body,
            "auth_failure_total",
            &[("service", "cn-user-api")],
        );
        let consent_required_before = prometheus_counter_value(
            &before_body,
            "consent_required_total",
            &[("service", "cn-user-api")],
        );
        let quota_exceeded_before = prometheus_counter_value(
            &before_body,
            "quota_exceeded_total",
            &[("service", "cn-user-api"), ("metric", "max_topics")],
        );

        let (status, challenge_payload) = post_json_public(
            app.clone(),
            "/v1/auth/challenge",
            json!({ "pubkey": auth_pubkey }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let challenge = challenge_payload
            .get("challenge")
            .and_then(Value::as_str)
            .expect("challenge");

        let auth_event = nostr::build_signed_event(
            &auth_keys,
            22242,
            vec![
                vec!["relay".to_string(), "http://localhost".to_string()],
                vec!["challenge".to_string(), challenge.to_string()],
            ],
            String::new(),
        )
        .expect("build auth event");
        let (status, _) = post_json_public(
            app.clone(),
            "/v1/auth/verify",
            json!({ "auth_event_json": auth_event }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let invalid_auth_event =
            nostr::build_signed_event(&auth_keys, 1, Vec::new(), String::new())
                .expect("build invalid auth event");
        let (status, payload) = post_json_public(
            app.clone(),
            "/v1/auth/verify",
            json!({ "auth_event_json": invalid_auth_event }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("INVALID_EVENT")
        );

        let (status, payload) = post_json(
            app.clone(),
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": consent_topic_id,
                "requested_services": ["relay", "index"]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("CONSENT_REQUIRED")
        );

        ensure_consents(&pool, &subscriber_pubkey).await;
        insert_topic_subscription(&pool, &existing_topic_id, &subscriber_pubkey).await;
        assign_active_plan_limit(&pool, &subscriber_pubkey, "max_topics", "limit", 1).await;

        let (status, payload) = post_json(
            app.clone(),
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": quota_topic_id,
                "requested_services": ["relay", "index"]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "max_topics", 1, 1, false);

        let (status, content_type, after_body) = get_text_public(app, "/metrics").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/plain; version=0.0.4"));
        let auth_success_after = prometheus_counter_value(
            &after_body,
            "auth_success_total",
            &[("service", "cn-user-api")],
        );
        let auth_failure_after = prometheus_counter_value(
            &after_body,
            "auth_failure_total",
            &[("service", "cn-user-api")],
        );
        let consent_required_after = prometheus_counter_value(
            &after_body,
            "consent_required_total",
            &[("service", "cn-user-api")],
        );
        let quota_exceeded_after = prometheus_counter_value(
            &after_body,
            "quota_exceeded_total",
            &[("service", "cn-user-api"), ("metric", "max_topics")],
        );

        assert!(
            auth_success_after >= auth_success_before + 1.0,
            "auth_success_total did not increase: before={auth_success_before}, after={auth_success_after}"
        );
        assert!(
            auth_failure_after >= auth_failure_before + 1.0,
            "auth_failure_total did not increase: before={auth_failure_before}, after={auth_failure_after}"
        );
        assert!(
            consent_required_after >= consent_required_before + 1.0,
            "consent_required_total did not increase: before={consent_required_before}, after={consent_required_after}"
        );
        assert!(
            quota_exceeded_after >= quota_exceeded_before + 1.0,
            "quota_exceeded_total{{metric=\"max_topics\"}} did not increase: before={quota_exceeded_before}, after={quota_exceeded_after}"
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
    async fn search_quota_commit_failure_on_success_path_returns_db_error_and_rolls_back() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let topic_id = format!("kukuri:search-commit-success-{}", Uuid::new_v4().simple());
        let request_id = format!("search-commit-success-{}", Uuid::new_v4());
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "index.search_requests", "day", 10).await;

        let (trigger_name, function_name) =
            install_usage_events_commit_failure_trigger(&pool, &request_id).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);
        let path = format!("/v1/search?topic={topic_id}&q=commit-failure");
        let (status, payload) =
            get_json_with_headers(app, &path, &token, &[("x-request-id", request_id.as_str())])
                .await;

        remove_usage_events_commit_failure_trigger(&pool, &trigger_name, &function_name).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("DB_ERROR")
        );
        assert_eq!(
            usage_event_count_for_request_id(&pool, &pubkey, "index.search_requests", &request_id)
                .await,
            0
        );
        assert_eq!(
            usage_counter_daily_count(&pool, &pubkey, "index.search_requests").await,
            0
        );
    }

    #[tokio::test]
    async fn search_quota_commit_failure_on_quota_exceeded_path_returns_db_error_and_rolls_back() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let topic_id = format!("kukuri:search-commit-exceeded-{}", Uuid::new_v4().simple());
        let request_id = format!("search-commit-exceeded-{}", Uuid::new_v4());
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "index.search_requests", "day", 0).await;

        let (trigger_name, function_name) =
            install_usage_events_commit_failure_trigger(&pool, &request_id).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);
        let path = format!("/v1/search?topic={topic_id}&q=quota-exceeded");
        let (status, payload) =
            get_json_with_headers(app, &path, &token, &[("x-request-id", request_id.as_str())])
                .await;

        remove_usage_events_commit_failure_trigger(&pool, &trigger_name, &function_name).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("DB_ERROR")
        );
        assert_eq!(
            usage_event_count_for_request_id(&pool, &pubkey, "index.search_requests", &request_id)
                .await,
            0
        );
        assert_eq!(
            usage_counter_daily_count(&pool, &pubkey, "index.search_requests").await,
            0
        );
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
    async fn search_quota_contract_payment_required_with_request_id_idempotent() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let topic_id = format!("kukuri:search-quota-{}", Uuid::new_v4().simple());
        let request_id = format!("search-quota-{}", Uuid::new_v4());
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "index.search_requests", "day", 0).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);
        let path = format!("/v1/search?topic={topic_id}&q=quota");

        let (status, payload) = get_json_with_headers(
            app.clone(),
            &path,
            &token,
            &[("x-request-id", request_id.as_str())],
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "index.search_requests", 0, 0, true);
        let first_reset_at = payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("first reset_at");

        let (retry_status, retry_payload) =
            get_json_with_headers(app, &path, &token, &[("x-request-id", request_id.as_str())])
                .await;
        assert_eq!(retry_status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&retry_payload, "index.search_requests", 0, 0, true);
        let retry_reset_at = retry_payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("retry reset_at");
        assert_eq!(retry_reset_at, first_reset_at);
        assert_eq!(
            usage_event_count_for_request_id(&pool, &pubkey, "index.search_requests", &request_id)
                .await,
            1
        );
    }

    #[tokio::test]
    async fn trending_quota_contract_payment_required_with_request_id_idempotent() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let topic_id = format!("kukuri:trending-quota-{}", Uuid::new_v4().simple());
        let request_id = format!("trending-quota-{}", Uuid::new_v4());
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "index.trending_requests", "day", 0).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/trending", get(trending))
            .with_state(state);
        let path = format!("/v1/trending?topic={topic_id}");

        let (status, payload) = get_json_with_headers(
            app.clone(),
            &path,
            &token,
            &[("x-request-id", request_id.as_str())],
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "index.trending_requests", 0, 0, true);
        let first_reset_at = payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("first reset_at");

        let (retry_status, retry_payload) =
            get_json_with_headers(app, &path, &token, &[("x-request-id", request_id.as_str())])
                .await;
        assert_eq!(retry_status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&retry_payload, "index.trending_requests", 0, 0, true);
        let retry_reset_at = retry_payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("retry reset_at");
        assert_eq!(retry_reset_at, first_reset_at);
        assert_eq!(
            usage_event_count_for_request_id(
                &pool,
                &pubkey,
                "index.trending_requests",
                &request_id
            )
            .await,
            1
        );
    }

    #[tokio::test]
    async fn submit_report_quota_contract_payment_required_with_request_id_idempotent() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let request_id = format!("report-quota-{}", Uuid::new_v4());
        let target = format!("event:report-quota-{}", Uuid::new_v4());
        ensure_consents(&pool, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "moderation.report_submits", "day", 0).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/reports", post(submit_report))
            .with_state(state);
        let body = json!({
            "target": target,
            "reason": "spam"
        });

        let (status, payload) = post_json_with_headers(
            app.clone(),
            "/v1/reports",
            &token,
            body.clone(),
            &[("x-request-id", request_id.as_str())],
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "moderation.report_submits", 0, 0, true);
        let first_reset_at = payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("first reset_at");

        let (retry_status, retry_payload) = post_json_with_headers(
            app,
            "/v1/reports",
            &token,
            body,
            &[("x-request-id", request_id.as_str())],
        )
        .await;
        assert_eq!(retry_status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&retry_payload, "moderation.report_submits", 0, 0, true);
        let retry_reset_at = retry_payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("retry reset_at");
        assert_eq!(retry_reset_at, first_reset_at);
        assert_eq!(
            usage_event_count_for_request_id(
                &pool,
                &pubkey,
                "moderation.report_submits",
                &request_id
            )
            .await,
            1
        );

        let report_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_user.reports WHERE reporter_pubkey = $1 AND target = $2",
        )
        .bind(&pubkey)
        .bind(&target)
        .fetch_one(&pool)
        .await
        .expect("count reports");
        assert_eq!(report_count, 0);
    }

    #[tokio::test]
    async fn trust_report_based_quota_contract_payment_required_with_request_id_idempotent() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let request_id = format!("trust-report-based-quota-{}", Uuid::new_v4());
        let subject = format!("pubkey:{}", Keys::generate().public_key().to_hex());
        ensure_consents(&pool, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "trust.requests", "day", 0).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/trust/report-based", get(trust_report_based))
            .with_state(state);
        let path = format!("/v1/trust/report-based?subject={subject}");

        let (status, payload) = get_json_with_headers(
            app.clone(),
            &path,
            &token,
            &[("x-request-id", request_id.as_str())],
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "trust.requests", 0, 0, true);
        let first_reset_at = payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("first reset_at");

        let (retry_status, retry_payload) =
            get_json_with_headers(app, &path, &token, &[("x-request-id", request_id.as_str())])
                .await;
        assert_eq!(retry_status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&retry_payload, "trust.requests", 0, 0, true);
        let retry_reset_at = retry_payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("retry reset_at");
        assert_eq!(retry_reset_at, first_reset_at);
        assert_eq!(
            usage_event_count_for_request_id(&pool, &pubkey, "trust.requests", &request_id).await,
            1
        );
    }

    #[tokio::test]
    async fn trust_communication_density_quota_contract_payment_required_with_request_id_idempotent(
    ) {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let request_id = format!("trust-communication-density-quota-{}", Uuid::new_v4());
        let subject = format!("pubkey:{}", Keys::generate().public_key().to_hex());
        ensure_consents(&pool, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "trust.requests", "day", 0).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/trust/communication-density",
                get(trust_communication_density),
            )
            .with_state(state);
        let path = format!("/v1/trust/communication-density?subject={subject}");

        let (status, payload) = get_json_with_headers(
            app.clone(),
            &path,
            &token,
            &[("x-request-id", request_id.as_str())],
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "trust.requests", 0, 0, true);
        let first_reset_at = payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("first reset_at");

        let (retry_status, retry_payload) =
            get_json_with_headers(app, &path, &token, &[("x-request-id", request_id.as_str())])
                .await;
        assert_eq!(retry_status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&retry_payload, "trust.requests", 0, 0, true);
        let retry_reset_at = retry_payload
            .pointer("/details/reset_at")
            .and_then(Value::as_i64)
            .expect("retry reset_at");
        assert_eq!(retry_reset_at, first_reset_at);
        assert_eq!(
            usage_event_count_for_request_id(&pool, &pubkey, "trust.requests", &request_id).await,
            1
        );
    }

    #[tokio::test]
    async fn topic_subscription_quota_contract_payment_required() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let existing_topic_id = format!("kukuri:max-topics-{}", Uuid::new_v4().simple());
        let request_topic_id = format!("kukuri:quota-request-{}", Uuid::new_v4().simple());
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &existing_topic_id, &pubkey).await;
        assign_active_plan_limit(&pool, &pubkey, "max_topics", "limit", 1).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/topic-subscription-requests",
                post(create_subscription_request),
            )
            .with_state(state);

        let (status, payload) = post_json(
            app,
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": request_topic_id,
                "requested_services": ["relay", "index"]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_quota_exceeded_response(&payload, "max_topics", 1, 1, false);

        let request_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1 AND topic_id = $2",
        )
        .bind(&pubkey)
        .bind(&request_topic_id)
        .fetch_one(&pool)
        .await
        .expect("count topic subscription requests");
        assert_eq!(request_count, 0);
    }

    #[tokio::test]
    async fn topic_subscription_pending_request_limit_contract_accepts_under_limit_requests() {
        let state = test_state_with_meili_url_bootstrap_auth_mode_and_user_config(
            "http://localhost:7700",
            "off",
            json!({
                "rate_limit": { "enabled": false },
                "subscription_request": { "max_pending_per_pubkey": 2 }
            }),
        )
        .await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let existing_pending_topic_id =
            format!("kukuri:pending-existing-{}", Uuid::new_v4().simple());
        let request_topic_id = format!("kukuri:pending-limit-{}", Uuid::new_v4().simple());

        ensure_consents(&pool, &pubkey).await;
        insert_pending_subscription_request(&pool, &pubkey, &existing_pending_topic_id).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/topic-subscription-requests",
                post(create_subscription_request),
            )
            .with_state(state);

        let (status, payload) = post_json(
            app,
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": request_topic_id,
                "requested_services": ["relay", "index"]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(payload
            .get("request_id")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty() && value != "existing"));
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some("pending")
        );

        let request_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1 AND topic_id = $2",
        )
        .bind(&pubkey)
        .bind(&request_topic_id)
        .fetch_one(&pool)
        .await
        .expect("count accepted topic subscription request");
        assert_eq!(request_count, 1);
        assert_eq!(
            pending_subscription_request_count_for_pubkey(&pool, &pubkey).await,
            2
        );
    }

    #[tokio::test]
    async fn topic_subscription_pending_request_limit_contract_rejects_at_limit() {
        let state = test_state_with_meili_url_bootstrap_auth_mode_and_user_config(
            "http://localhost:7700",
            "off",
            json!({
                "rate_limit": { "enabled": false },
                "subscription_request": { "max_pending_per_pubkey": 2 }
            }),
        )
        .await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let existing_pending_topic_id_1 =
            format!("kukuri:pending-existing-a-{}", Uuid::new_v4().simple());
        let existing_pending_topic_id_2 =
            format!("kukuri:pending-existing-b-{}", Uuid::new_v4().simple());
        let request_topic_id = format!("kukuri:pending-limit-{}", Uuid::new_v4().simple());

        ensure_consents(&pool, &pubkey).await;
        insert_pending_subscription_request(&pool, &pubkey, &existing_pending_topic_id_1).await;
        insert_pending_subscription_request(&pool, &pubkey, &existing_pending_topic_id_2).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/topic-subscription-requests",
                post(create_subscription_request),
            )
            .with_state(state);

        let (status, payload) = post_json(
            app,
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": request_topic_id,
                "requested_services": ["relay", "index"]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_pending_subscription_request_limit_response(&payload, 2, 2);

        let request_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1 AND topic_id = $2",
        )
        .bind(&pubkey)
        .bind(&request_topic_id)
        .fetch_one(&pool)
        .await
        .expect("count denied topic subscription request");
        assert_eq!(request_count, 0);
        assert_eq!(
            pending_subscription_request_count_for_pubkey(&pool, &pubkey).await,
            2
        );
    }

    #[tokio::test]
    async fn topic_subscription_pending_request_limit_contract_allows_rerequest_after_approve_or_reject(
    ) {
        for reviewed_status in ["approved", "rejected"] {
            let state = test_state_with_meili_url_bootstrap_auth_mode_and_user_config(
                "http://localhost:7700",
                "off",
                json!({
                    "rate_limit": { "enabled": false },
                    "subscription_request": { "max_pending_per_pubkey": 1 }
                }),
            )
            .await;
            let pool = state.pool.clone();
            let pubkey = Keys::generate().public_key().to_hex();
            let initial_pending_topic_id = format!(
                "kukuri:pending-rerequest-{}-initial-{}",
                reviewed_status,
                Uuid::new_v4().simple()
            );
            let blocked_topic_id = format!(
                "kukuri:pending-rerequest-{}-blocked-{}",
                reviewed_status,
                Uuid::new_v4().simple()
            );
            let reopened_topic_id = format!(
                "kukuri:pending-rerequest-{}-reopened-{}",
                reviewed_status,
                Uuid::new_v4().simple()
            );

            ensure_consents(&pool, &pubkey).await;
            let request_id =
                insert_pending_subscription_request(&pool, &pubkey, &initial_pending_topic_id)
                    .await;

            let token = issue_token(&state.jwt_config, &pubkey);
            let app = Router::new()
                .route(
                    "/v1/topic-subscription-requests",
                    post(create_subscription_request),
                )
                .with_state(state);

            let (blocked_status, blocked_payload) = post_json(
                app.clone(),
                "/v1/topic-subscription-requests",
                &token,
                json!({
                    "topic_id": blocked_topic_id,
                    "requested_services": ["relay", "index"]
                }),
            )
            .await;
            assert_eq!(blocked_status, StatusCode::TOO_MANY_REQUESTS);
            assert_pending_subscription_request_limit_response(&blocked_payload, 1, 1);

            update_subscription_request_status(&pool, &request_id, reviewed_status).await;
            assert_eq!(
                pending_subscription_request_count_for_pubkey(&pool, &pubkey).await,
                0
            );

            let (reopen_status, reopen_payload) = post_json(
                app,
                "/v1/topic-subscription-requests",
                &token,
                json!({
                    "topic_id": reopened_topic_id,
                    "requested_services": ["relay", "index"]
                }),
            )
            .await;
            assert_eq!(
                reopen_status,
                StatusCode::OK,
                "re-request should be accepted after status={reviewed_status}"
            );
            assert_eq!(
                reopen_payload.get("status").and_then(Value::as_str),
                Some("pending")
            );
            assert_eq!(
                pending_subscription_request_count_for_pubkey(&pool, &pubkey).await,
                1
            );
        }
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
    async fn auth_challenge_and_verify_rate_limit_boundary_contract() {
        let state = test_state_with_rate_limits(1, 120, 120).await;
        let app = Router::new()
            .route("/v1/auth/challenge", post(crate::auth::auth_challenge))
            .route("/v1/auth/verify", post(crate::auth::auth_verify))
            .with_state(state);

        let (challenge_status, _, _) = post_json_public_with_headers(
            app.clone(),
            "/v1/auth/challenge",
            json!({ "pubkey": "invalid-pubkey" }),
        )
        .await;
        assert_ne!(challenge_status, StatusCode::TOO_MANY_REQUESTS);

        let (verify_status, verify_headers, verify_payload) = post_json_public_with_headers(
            app,
            "/v1/auth/verify",
            json!({ "auth_event_json": { "kind": 1 } }),
        )
        .await;
        assert_rate_limited_contract(verify_status, &verify_headers, &verify_payload);
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
            .route(
                "/v1/policies/current",
                get(crate::policies::get_current_policies),
            )
            .route(
                "/v1/policies/{policy_type}/{version}",
                get(crate::policies::get_policy_by_version),
            )
            .route(
                "/v1/consents/status",
                get(crate::policies::get_consent_status),
            )
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

        let (status, consent_before_payload) =
            get_json(app.clone(), "/v1/consents/status", &token).await;
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

        let (status, create_payload) = post_json_with_consent_retry(
            app.clone(),
            "/v1/topic-subscription-requests",
            &token,
            json!({
                "topic_id": request_topic_id,
                "requested_services": ["relay", "index"]
            }),
            &pool,
            &pubkey,
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
        let (status, list_payload) = get_json_with_consent_retry(
            app.clone(),
            "/v1/topic-subscriptions",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let list = list_payload.as_array().cloned().unwrap_or_default();
        let active_row = list
            .iter()
            .find(|row| {
                row.get("topic_id").and_then(Value::as_str) == Some(active_topic_id.as_str())
            })
            .expect("active subscription row");
        assert_eq!(
            active_row.get("status").and_then(Value::as_str),
            Some("active")
        );
        assert!(active_row
            .get("started_at")
            .and_then(Value::as_i64)
            .is_some());
        assert!(active_row
            .get("ended_at")
            .is_some_and(serde_json::Value::is_null));

        let (status, delete_payload) = delete_json_with_consent_retry(
            app.clone(),
            &format!("/v1/topic-subscriptions/{active_topic_id}"),
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            delete_payload.get("status").and_then(Value::as_str),
            Some("ended")
        );

        let (status, list_after_payload) =
            get_json_with_consent_retry(app, "/v1/topic-subscriptions", &token, &pool, &pubkey)
                .await;
        assert_eq!(status, StatusCode::OK);
        let list_after = list_after_payload.as_array().cloned().unwrap_or_default();
        let ended_row = list_after
            .iter()
            .find(|row| {
                row.get("topic_id").and_then(Value::as_str) == Some(active_topic_id.as_str())
            })
            .expect("ended subscription row");
        assert_eq!(
            ended_row.get("status").and_then(Value::as_str),
            Some("ended")
        );
        assert!(ended_row.get("ended_at").and_then(Value::as_i64).is_some());
    }

    #[tokio::test]
    async fn topic_subscription_delete_commit_failure_returns_db_error_and_rolls_back() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let topic_id = format!("kukuri:delete-commit-failure-{}", Uuid::new_v4().simple());
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;
        sqlx::query(
            "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) \
             VALUES ($1, TRUE, 1) \
             ON CONFLICT (topic_id) DO UPDATE \
             SET enabled = TRUE, ref_count = 1, updated_at = NOW()",
        )
        .bind(&topic_id)
        .execute(&pool)
        .await
        .expect("insert node subscription");
        let (trigger_name, function_name) =
            install_topic_subscription_commit_failure_trigger(&pool, &topic_id).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/topic-subscriptions/{topic_id}",
                delete(delete_topic_subscription),
            )
            .with_state(state);
        let (status, payload) = delete_json_with_consent_retry(
            app,
            &format!("/v1/topic-subscriptions/{topic_id}"),
            &token,
            &pool,
            &pubkey,
        )
        .await;

        remove_topic_subscription_commit_failure_trigger(&pool, &trigger_name, &function_name)
            .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("DB_ERROR")
        );
        assert!(payload.get("status").is_none());

        let subscription_row = sqlx::query(
            "SELECT status, ended_at \
             FROM cn_user.topic_subscriptions \
             WHERE topic_id = $1 AND subscriber_pubkey = $2",
        )
        .bind(&topic_id)
        .bind(&pubkey)
        .fetch_one(&pool)
        .await
        .expect("fetch topic subscription");
        let status_after: String = subscription_row.get("status");
        let ended_at_after: Option<chrono::DateTime<chrono::Utc>> =
            subscription_row.get("ended_at");
        assert_eq!(status_after, "active");
        assert!(ended_at_after.is_none());

        let node_row = sqlx::query(
            "SELECT ref_count, enabled FROM cn_admin.node_subscriptions WHERE topic_id = $1",
        )
        .bind(&topic_id)
        .fetch_one(&pool)
        .await
        .expect("fetch node subscription");
        let ref_count_after: i64 = node_row.get("ref_count");
        let enabled_after: bool = node_row.get("enabled");
        assert_eq!(ref_count_after, 1);
        assert!(enabled_after);
    }

    #[tokio::test]
    async fn personal_data_export_contract_success_shape_compatible() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_active_subscriber(&state.pool, &pubkey).await;
        ensure_consents(&state.pool, &pubkey).await;
        let token = issue_token(&state.jwt_config, &pubkey);
        let pool = state.pool.clone();

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

        let (status, create_payload) = post_json_with_consent_retry(
            app.clone(),
            "/v1/personal-data-export-requests",
            &token,
            json!({}),
            &pool,
            &pubkey,
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

        let (status, create_payload) = post_json_with_consent_retry(
            app.clone(),
            "/v1/personal-data-deletion-requests",
            &token,
            json!({}),
            &pool,
            &pubkey,
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
    async fn bootstrap_nodes_contract_requires_www_authenticate_header() {
        let state = test_state_with_bootstrap_auth_required().await;

        let app = Router::new()
            .route(
                "/v1/bootstrap/nodes",
                get(crate::bootstrap::get_bootstrap_nodes),
            )
            .with_state(state);
        let (status, headers, payload) =
            get_json_public_with_headers(app, "/v1/bootstrap/nodes", &[]).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("AUTH_REQUIRED")
        );
        assert_eq!(
            headers
                .get(header::WWW_AUTHENTICATE)
                .and_then(|value| value.to_str().ok()),
            Some(r#"Bearer realm="cn-user-api""#)
        );
    }

    #[tokio::test]
    async fn bootstrap_services_contract_requires_www_authenticate_header() {
        let state = test_state_with_bootstrap_auth_required().await;
        let topic_id = format!("kukuri:bootstrap-auth-{}", Uuid::new_v4());

        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(crate::bootstrap::get_bootstrap_services),
            )
            .with_state(state);
        let path = format!("/v1/bootstrap/topics/{topic_id}/services");
        let (status, headers, payload) = get_json_public_with_headers(app, &path, &[]).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("AUTH_REQUIRED")
        );
        assert_eq!(
            headers
                .get(header::WWW_AUTHENTICATE)
                .and_then(|value| value.to_str().ok()),
            Some(r#"Bearer realm="cn-user-api""#)
        );
    }

    #[tokio::test]
    async fn bootstrap_nodes_contract_requires_consent_when_authenticated() {
        let state = test_state_with_bootstrap_auth_required().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let token = issue_token(&state.jwt_config, &pubkey);
        insert_current_policy(&pool, "terms", "v1.0.0", "ja-JP", "Terms").await;
        insert_current_policy(&pool, "privacy", "v1.0.0", "ja-JP", "Privacy").await;

        let app = Router::new()
            .route(
                "/v1/bootstrap/nodes",
                get(crate::bootstrap::get_bootstrap_nodes),
            )
            .with_state(state);
        let (status, headers, payload) =
            get_json_with_headers_and_response_headers(app, "/v1/bootstrap/nodes", &token, &[])
                .await;

        assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("CONSENT_REQUIRED")
        );
        assert!(payload
            .get("details")
            .and_then(|details| details.get("required"))
            .and_then(Value::as_array)
            .map(|required| !required.is_empty())
            .unwrap_or(false));
        assert!(
            headers.get(header::WWW_AUTHENTICATE).is_none(),
            "consent-required response must not include WWW-Authenticate"
        );
    }

    #[tokio::test]
    async fn bootstrap_services_contract_requires_consent_when_authenticated() {
        let state = test_state_with_bootstrap_auth_required().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let token = issue_token(&state.jwt_config, &pubkey);
        let topic_id = format!("kukuri:bootstrap-consent-{}", Uuid::new_v4());
        insert_current_policy(&pool, "terms", "v1.0.0", "ja-JP", "Terms").await;
        insert_current_policy(&pool, "privacy", "v1.0.0", "ja-JP", "Privacy").await;

        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(crate::bootstrap::get_bootstrap_services),
            )
            .with_state(state);
        let path = format!("/v1/bootstrap/topics/{topic_id}/services");
        let (status, headers, payload) =
            get_json_with_headers_and_response_headers(app, &path, &token, &[]).await;

        assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
        assert_eq!(
            payload.get("code").and_then(Value::as_str),
            Some("CONSENT_REQUIRED")
        );
        assert!(payload
            .get("details")
            .and_then(|details| details.get("required"))
            .and_then(Value::as_array)
            .map(|required| !required.is_empty())
            .unwrap_or(false));
        assert!(
            headers.get(header::WWW_AUTHENTICATE).is_none(),
            "consent-required response must not include WWW-Authenticate"
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

        let (status, headers, payload) =
            get_json_public_with_headers(app.clone(), &path, &[]).await;
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

        let (modified_status, modified_headers) =
            get_status_public_with_headers(app, &path, &[("if-modified-since", &last_modified)])
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
    async fn bootstrap_services_etag_changes_when_body_changes_with_same_count_and_second() {
        let state = test_state().await;
        let topic_id = format!("kukuri:bootstrap-etag-same-second-{}", Uuid::new_v4());
        let event_id = Uuid::new_v4().to_string();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let expires_at = now + 600;

        let event_before = json!({
            "id": event_id,
            "kind": 39001,
            "pubkey": Keys::generate().public_key().to_hex(),
            "tags": [["k", "kukuri"], ["ver", "1"], ["topic", topic_id.clone()], ["service", "relay"]],
            "content": "before",
            "sig": "signature"
        });
        let event_after = json!({
            "id": event_id,
            "kind": 39001,
            "pubkey": Keys::generate().public_key().to_hex(),
            "tags": [["k", "kukuri"], ["ver", "1"], ["topic", topic_id.clone()], ["service", "relay"]],
            "content": "after",
            "sig": "signature"
        });

        insert_bootstrap_event(
            &state.pool,
            &event_id,
            39001,
            Some(topic_id.as_str()),
            expires_at,
            event_before,
        )
        .await;

        let updated_at_before: chrono::DateTime<chrono::Utc> =
            sqlx::query("SELECT updated_at FROM cn_bootstrap.events WHERE event_id = $1")
                .bind(&event_id)
                .fetch_one(&state.pool)
                .await
                .expect("fetch bootstrap event before update")
                .get("updated_at");
        let pool = state.pool.clone();

        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(crate::bootstrap::get_bootstrap_services),
            )
            .with_state(state);
        let path = format!("/v1/bootstrap/topics/{topic_id}/services");

        let (status_before, headers_before, _) =
            get_json_public_with_headers(app.clone(), &path, &[]).await;
        assert_eq!(status_before, StatusCode::OK);

        let etag_before = headers_before
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(!etag_before.is_empty());

        sqlx::query(
            "UPDATE cn_bootstrap.events \
             SET event_json = $2, updated_at = date_trunc('second', updated_at) \
             WHERE event_id = $1",
        )
        .bind(&event_id)
        .bind(event_after.clone())
        .execute(&pool)
        .await
        .expect("update bootstrap event json");

        let updated_at_after: chrono::DateTime<chrono::Utc> =
            sqlx::query("SELECT updated_at FROM cn_bootstrap.events WHERE event_id = $1")
                .bind(&event_id)
                .fetch_one(&pool)
                .await
                .expect("fetch bootstrap event after update")
                .get("updated_at");
        assert_eq!(updated_at_before.timestamp(), updated_at_after.timestamp());

        let (status_after, headers_after, payload_after) =
            get_json_public_with_headers(app.clone(), &path, &[]).await;
        assert_eq!(status_after, StatusCode::OK);

        let items_after = payload_after
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items_after.len(), 1);
        assert_eq!(items_after.first(), Some(&event_after));

        let etag_after = headers_after
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(!etag_after.is_empty());
        assert_ne!(etag_before, etag_after);

        let (stale_status, _) =
            get_status_public_with_headers(app.clone(), &path, &[("if-none-match", &etag_before)])
                .await;
        assert_eq!(stale_status, StatusCode::OK);

        let (fresh_status, fresh_headers) =
            get_status_public_with_headers(app, &path, &[("if-none-match", &etag_after)]).await;
        assert_eq!(fresh_status, StatusCode::NOT_MODIFIED);
        assert_eq!(
            fresh_headers
                .get(header::ETAG)
                .and_then(|value| value.to_str().ok()),
            Some(etag_after.as_str())
        );
    }

    #[tokio::test]
    async fn bootstrap_nodes_and_services_rate_limit_boundary_contract() {
        let state = test_state_with_rate_limits(120, 1, 120).await;
        let topic_id = format!("kukuri:bootstrap-rate-limit-{}", Uuid::new_v4());

        let app = Router::new()
            .route(
                "/v1/bootstrap/nodes",
                get(crate::bootstrap::get_bootstrap_nodes),
            )
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(crate::bootstrap::get_bootstrap_services),
            )
            .with_state(state);

        let (nodes_status, _, _) =
            get_json_public_with_headers(app.clone(), "/v1/bootstrap/nodes", &[]).await;
        assert_ne!(nodes_status, StatusCode::TOO_MANY_REQUESTS);

        let services_path = format!("/v1/bootstrap/topics/{topic_id}/services");
        let (services_status, services_headers, services_payload) =
            get_json_public_with_headers(app, &services_path, &[]).await;
        assert_rate_limited_contract(services_status, &services_headers, &services_payload);
    }

    #[tokio::test]
    async fn protected_search_and_trending_rate_limit_boundary_contract() {
        let state = test_state_with_rate_limits(120, 120, 1).await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let topic_id = format!("kukuri:rate-limit-protected-{}", Uuid::new_v4().simple());
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;
        ensure_consents(&pool, &pubkey).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .route("/v1/trending", get(trending))
            .with_state(state);

        let search_path = format!("/v1/search?topic={topic_id}&q=rate-limit");
        let (search_status, _, _) = get_json_with_headers_and_consent_retry(
            app.clone(),
            &search_path,
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_ne!(search_status, StatusCode::TOO_MANY_REQUESTS);

        let trending_path = format!("/v1/trending?topic={topic_id}");
        let (trending_status, trending_headers, trending_payload) =
            get_json_with_headers_and_consent_retry(app, &trending_path, &token, &pool, &pubkey)
                .await;
        assert_rate_limited_contract(trending_status, &trending_headers, &trending_payload);
    }

    #[tokio::test]
    async fn search_contract_success_shape_compatible() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
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
        let pool = state.pool.clone();
        set_search_runtime_flags(
            &pool,
            cn_core::search_runtime_flags::SEARCH_READ_BACKEND_MEILI,
            cn_core::search_runtime_flags::SEARCH_WRITE_MODE_MEILI_ONLY,
        )
        .await;
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            &format!("/v1/search?topic={topic_id}&q=hello&limit=1"),
            &token,
            &pool,
            &pubkey,
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
    async fn search_contract_pg_backend_switch_normalization_and_version_filter() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let topic_id = format!("kukuri:search-pg-{}", Uuid::new_v4());
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;

        set_search_runtime_flags(
            &pool,
            cn_core::search_runtime_flags::SEARCH_READ_BACKEND_PG,
            cn_core::search_runtime_flags::SEARCH_WRITE_MODE_MEILI_ONLY,
        )
        .await;

        let current_post_id = Uuid::new_v4().to_string();
        let stale_post_id = Uuid::new_v4().to_string();
        let body_raw = "Ｈｅｌｌｏ PG Search #Rust";
        let body_norm = search_normalizer::normalize_search_text(body_raw);
        let hashtags_norm = vec!["rust".to_string()];
        let mentions_norm: Vec<String> = Vec::new();
        let community_terms_norm = search_normalizer::normalize_search_terms([topic_id.as_str()]);
        let search_text = search_normalizer::build_search_text(
            &body_norm,
            &hashtags_norm,
            &mentions_norm,
            &community_terms_norm,
        );
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;

        sqlx::query(
            "INSERT INTO cn_search.post_search_documents \
             (post_id, topic_id, author_id, visibility, body_raw, body_norm, hashtags_norm, mentions_norm, community_terms_norm, search_text, language_hint, popularity_score, created_at, is_deleted, normalizer_version, updated_at) \
             VALUES ($1, $2, $3, 'public', $4, $5, $6, $7, $8, $9, NULL, 0, $10, FALSE, $11, NOW())",
        )
        .bind(&current_post_id)
        .bind(&topic_id)
        .bind(&pubkey)
        .bind(body_raw)
        .bind(&body_norm)
        .bind(&hashtags_norm)
        .bind(&mentions_norm)
        .bind(&community_terms_norm)
        .bind(&search_text)
        .bind(now)
        .bind(search_normalizer::SEARCH_NORMALIZER_VERSION)
        .execute(&pool)
        .await
        .expect("insert current normalized document");

        sqlx::query(
            "INSERT INTO cn_search.post_search_documents \
             (post_id, topic_id, author_id, visibility, body_raw, body_norm, hashtags_norm, mentions_norm, community_terms_norm, search_text, language_hint, popularity_score, created_at, is_deleted, normalizer_version, updated_at) \
             VALUES ($1, $2, $3, 'public', $4, $5, $6, $7, $8, $9, NULL, 0, $10, FALSE, $11, NOW())",
        )
        .bind(&stale_post_id)
        .bind(&topic_id)
        .bind(&pubkey)
        .bind("hello old normalizer")
        .bind("hello old normalizer")
        .bind(Vec::<String>::new())
        .bind(Vec::<String>::new())
        .bind(Vec::<String>::new())
        .bind("hello old normalizer")
        .bind(now - 1)
        .bind(search_normalizer::SEARCH_NORMALIZER_VERSION - 1)
        .execute(&pool)
        .await
        .expect("insert stale normalizer document");

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            &format!("/v1/search?topic={topic_id}&q=ｈｅｌｌｏ"),
            &token,
            &pool,
            &pubkey,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("total").and_then(Value::as_u64),
            Some(1),
            "expected only current normalizer version document to match"
        );
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items
                .first()
                .and_then(|item| item.get("event_id"))
                .and_then(Value::as_str),
            Some(current_post_id.as_str())
        );

        sqlx::query("DELETE FROM cn_search.post_search_documents WHERE post_id = ANY($1)")
            .bind(vec![current_post_id, stale_post_id])
            .execute(&pool)
            .await
            .expect("cleanup pg search documents");

        set_search_runtime_flags(
            &pool,
            cn_core::search_runtime_flags::SEARCH_READ_BACKEND_MEILI,
            cn_core::search_runtime_flags::SEARCH_WRITE_MODE_MEILI_ONLY,
        )
        .await;
    }

    #[tokio::test]
    async fn search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let topic_a = format!("kukuri:search-pg-topic-a-{}", Uuid::new_v4().simple());
        let topic_b = format!("kukuri:search-pg-topic-b-{}", Uuid::new_v4().simple());
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_a, &pubkey).await;
        insert_topic_subscription(&pool, &topic_b, &pubkey).await;

        set_search_runtime_flags(
            &pool,
            cn_core::search_runtime_flags::SEARCH_READ_BACKEND_PG,
            cn_core::search_runtime_flags::SEARCH_WRITE_MODE_MEILI_ONLY,
        )
        .await;

        let shared_post_id = Uuid::new_v4().to_string();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let body_a = "Shared multi topic body for topic A";
        let body_a_norm = search_normalizer::normalize_search_text(body_a);
        let hashtags_a = vec!["shared".to_string(), "topica".to_string()];
        let mentions_a: Vec<String> = Vec::new();
        let community_terms_a = search_normalizer::normalize_search_terms([topic_a.as_str()]);
        let search_text_a = search_normalizer::build_search_text(
            &body_a_norm,
            &hashtags_a,
            &mentions_a,
            &community_terms_a,
        );

        sqlx::query(
            "INSERT INTO cn_search.post_search_documents \
             (post_id, topic_id, author_id, visibility, body_raw, body_norm, hashtags_norm, mentions_norm, community_terms_norm, search_text, language_hint, popularity_score, created_at, is_deleted, normalizer_version, updated_at) \
             VALUES ($1, $2, $3, 'public', $4, $5, $6, $7, $8, $9, NULL, 0, $10, FALSE, $11, NOW())",
        )
        .bind(&shared_post_id)
        .bind(&topic_a)
        .bind(&pubkey)
        .bind(body_a)
        .bind(&body_a_norm)
        .bind(&hashtags_a)
        .bind(&mentions_a)
        .bind(&community_terms_a)
        .bind(&search_text_a)
        .bind(now)
        .bind(search_normalizer::SEARCH_NORMALIZER_VERSION)
        .execute(&pool)
        .await
        .expect("insert topic A search document");

        let body_b = "Shared multi topic body for topic B";
        let body_b_norm = search_normalizer::normalize_search_text(body_b);
        let hashtags_b = vec!["shared".to_string(), "topicb".to_string()];
        let mentions_b: Vec<String> = Vec::new();
        let community_terms_b = search_normalizer::normalize_search_terms([topic_b.as_str()]);
        let search_text_b = search_normalizer::build_search_text(
            &body_b_norm,
            &hashtags_b,
            &mentions_b,
            &community_terms_b,
        );

        sqlx::query(
            "INSERT INTO cn_search.post_search_documents \
             (post_id, topic_id, author_id, visibility, body_raw, body_norm, hashtags_norm, mentions_norm, community_terms_norm, search_text, language_hint, popularity_score, created_at, is_deleted, normalizer_version, updated_at) \
             VALUES ($1, $2, $3, 'public', $4, $5, $6, $7, $8, $9, NULL, 0, $10, FALSE, $11, NOW())",
        )
        .bind(&shared_post_id)
        .bind(&topic_b)
        .bind(&pubkey)
        .bind(body_b)
        .bind(&body_b_norm)
        .bind(&hashtags_b)
        .bind(&mentions_b)
        .bind(&community_terms_b)
        .bind(&search_text_b)
        .bind(now - 1)
        .bind(search_normalizer::SEARCH_NORMALIZER_VERSION)
        .execute(&pool)
        .await
        .expect("insert topic B search document");

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(state);

        let (status_a, payload_a) = get_json_with_consent_retry(
            app.clone(),
            &format!("/v1/search?topic={topic_a}&q=shared"),
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status_a, StatusCode::OK);
        assert_eq!(payload_a.get("total").and_then(Value::as_u64), Some(1));
        let items_a = payload_a
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items_a.len(), 1);
        let item_a = items_a.first().expect("topic A item");
        assert_eq!(
            item_a.get("event_id").and_then(Value::as_str),
            Some(shared_post_id.as_str())
        );
        assert_eq!(
            item_a.get("topic_id").and_then(Value::as_str),
            Some(topic_a.as_str())
        );
        assert_eq!(item_a.get("content").and_then(Value::as_str), Some(body_a));

        let (status_b, payload_b) = get_json_with_consent_retry(
            app,
            &format!("/v1/search?topic={topic_b}&q=shared"),
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status_b, StatusCode::OK);
        assert_eq!(payload_b.get("total").and_then(Value::as_u64), Some(1));
        let items_b = payload_b
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items_b.len(), 1);
        let item_b = items_b.first().expect("topic B item");
        assert_eq!(
            item_b.get("event_id").and_then(Value::as_str),
            Some(shared_post_id.as_str())
        );
        assert_eq!(
            item_b.get("topic_id").and_then(Value::as_str),
            Some(topic_b.as_str())
        );
        assert_eq!(item_b.get("content").and_then(Value::as_str), Some(body_b));

        sqlx::query(
            "DELETE FROM cn_search.post_search_documents \
             WHERE post_id = $1 AND topic_id = ANY($2)",
        )
        .bind(&shared_post_id)
        .bind(vec![topic_a, topic_b])
        .execute(&pool)
        .await
        .expect("cleanup multi topic post search documents");

        set_search_runtime_flags(
            &pool,
            cn_core::search_runtime_flags::SEARCH_READ_BACKEND_MEILI,
            cn_core::search_runtime_flags::SEARCH_WRITE_MODE_MEILI_ONLY,
        )
        .await;
    }

    #[tokio::test]
    async fn community_suggest_pg_backend_supports_exact_prefix_and_trgm() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_PG,
        )
        .await;
        set_suggest_rerank_mode(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RERANK_MODE_SHADOW,
        )
        .await;
        set_shadow_sample_rate(&pool, "0").await;

        let community_exact = "kukuri:tauri:pr03rustalpha";
        let community_prefix = "kukuri:tauri:pr03rubybeta";
        let community_other = "kukuri:tauri:pr03ravengamma";
        insert_community_search_terms(&pool, community_exact).await;
        insert_community_search_terms(&pool, community_prefix).await;
        insert_community_search_terms(&pool, community_other).await;
        insert_node_subscription(&pool, community_exact).await;
        insert_node_subscription(&pool, community_prefix).await;
        insert_node_subscription(&pool, community_other).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/communities/suggest", get(community_suggest))
            .with_state(state);

        let (exact_status, exact_payload) = get_json_with_consent_retry(
            app.clone(),
            "/v1/communities/suggest?q=pr03rustalpha&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(
            exact_status,
            StatusCode::OK,
            "unexpected exact payload: {exact_payload}"
        );
        assert_eq!(
            exact_payload.get("backend").and_then(Value::as_str),
            Some("pg")
        );
        assert_eq!(
            exact_payload.get("rerank_mode").and_then(Value::as_str),
            Some("shadow")
        );
        let exact_items = exact_payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!exact_items.is_empty());
        assert_eq!(
            exact_items[0].get("community_id").and_then(Value::as_str),
            Some(community_exact)
        );
        assert_eq!(
            exact_items[0].get("exact_hit").and_then(Value::as_bool),
            Some(true)
        );

        let (prefix_status, prefix_payload) = get_json_with_consent_retry(
            app.clone(),
            "/v1/communities/suggest?q=pr03ru&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(
            prefix_status,
            StatusCode::OK,
            "unexpected prefix payload: {prefix_payload}"
        );
        let prefix_items = prefix_payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!prefix_items.is_empty());
        assert_eq!(
            prefix_items[0].get("prefix_hit").and_then(Value::as_bool),
            Some(true)
        );

        let (trgm_status, trgm_payload) = get_json_with_consent_retry(
            app,
            "/v1/communities/suggest?q=pr03rutsalpha&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(
            trgm_status,
            StatusCode::OK,
            "unexpected trgm payload: {trgm_payload}"
        );
        let trgm_items = trgm_payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!trgm_items.is_empty());
        let typo_match = trgm_items
            .iter()
            .find(|item| item.get("community_id").and_then(Value::as_str) == Some(community_exact))
            .cloned()
            .expect("expected typo query to include trgm community candidate");
        assert_eq!(
            typo_match.get("exact_hit").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            typo_match.get("prefix_hit").and_then(Value::as_bool),
            Some(false)
        );
        assert!(
            typo_match
                .get("trgm_score")
                .and_then(Value::as_f64)
                .unwrap_or_default()
                > 0.0
        );

        sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = ANY($1)")
            .bind(vec![
                community_exact.to_string(),
                community_prefix.to_string(),
                community_other.to_string(),
            ])
            .execute(&pool)
            .await
            .expect("cleanup community suggest pg terms");
        sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = ANY($1)")
            .bind(vec![
                community_exact.to_string(),
                community_prefix.to_string(),
                community_other.to_string(),
            ])
            .execute(&pool)
            .await
            .expect("cleanup node subscriptions for suggest test");
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_LEGACY,
        )
        .await;
    }

    #[tokio::test]
    async fn community_suggest_legacy_backend_uses_topic_sources() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_LEGACY,
        )
        .await;

        let topic_id = format!("kukuri:tauri:pr03legacy-{}", Uuid::new_v4().simple());
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/communities/suggest", get(community_suggest))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            "/v1/communities/suggest?q=pr03legacy&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("backend").and_then(Value::as_str),
            Some("legacy")
        );
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(items.iter().any(|item| {
            item.get("community_id").and_then(Value::as_str) == Some(topic_id.as_str())
        }));

        sqlx::query(
            "DELETE FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
        )
        .bind(&topic_id)
        .bind(&pubkey)
        .execute(&pool)
        .await
        .expect("cleanup legacy suggest topic subscription");
    }

    #[tokio::test]
    async fn community_suggest_pg_rerank_enabled_prioritizes_affinity_and_visibility() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_PG,
        )
        .await;
        set_suggest_rerank_mode(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RERANK_MODE_ENABLED,
        )
        .await;
        set_suggest_relation_weights(
            &pool,
            r#"{"is_member":20.0,"is_following_community":0.0,"friends_member_count":0.0,"two_hop_follow_count":0.0,"last_view_decay":0.0,"muted_or_blocked":-1.0}"#,
        )
        .await;

        let community_a = "kukuri:tauri:pr05rank-a";
        let community_b = "kukuri:tauri:pr05rank-b";
        let community_hidden = "kukuri:tauri:pr05rank-hidden";
        insert_community_search_terms(&pool, community_a).await;
        insert_community_search_terms(&pool, community_b).await;
        insert_community_search_terms(&pool, community_hidden).await;
        insert_node_subscription(&pool, community_a).await;
        insert_node_subscription(&pool, community_b).await;
        insert_user_community_affinity(
            &pool,
            &pubkey,
            community_b,
            json!({
                "is_member": true,
                "is_following_community": false,
                "friends_member_count": 0,
                "two_hop_follow_count": 0,
                "last_seen_at": null
            }),
        )
        .await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/communities/suggest", get(community_suggest))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            "/v1/communities/suggest?q=pr05rank&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload.get("backend").and_then(Value::as_str), Some("pg"));
        assert_eq!(
            payload.get("rerank_mode").and_then(Value::as_str),
            Some("enabled")
        );
        assert_eq!(
            payload
                .get("blocked_or_muted_drop_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload.get("visibility_drop_count").and_then(Value::as_u64),
            Some(1)
        );

        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(items.len() >= 2);
        assert_eq!(
            items[0].get("community_id").and_then(Value::as_str),
            Some(community_b)
        );
        assert!(
            !items
                .iter()
                .any(|item| item.get("community_id").and_then(Value::as_str)
                    == Some(community_hidden))
        );

        sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = ANY($1)")
            .bind(vec![
                community_a.to_string(),
                community_b.to_string(),
                community_hidden.to_string(),
            ])
            .execute(&pool)
            .await
            .expect("cleanup suggest rerank terms");
        sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = ANY($1)")
            .bind(vec![community_a.to_string(), community_b.to_string()])
            .execute(&pool)
            .await
            .expect("cleanup suggest rerank node subscriptions");
        sqlx::query(
            "DELETE FROM cn_search.user_community_affinity WHERE user_id = $1 AND community_id = ANY($2)",
        )
        .bind(&pubkey)
        .bind(vec![community_a.to_string(), community_b.to_string()])
        .execute(&pool)
        .await
        .expect("cleanup suggest rerank affinity");
        set_suggest_relation_weights(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RELATION_WEIGHTS_DEFAULT,
        )
        .await;
        set_suggest_rerank_mode(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RERANK_MODE_SHADOW,
        )
        .await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_LEGACY,
        )
        .await;
    }

    #[tokio::test]
    async fn community_suggest_pg_rerank_filters_muted_communities() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_PG,
        )
        .await;
        set_suggest_rerank_mode(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RERANK_MODE_ENABLED,
        )
        .await;

        let community_muted = "kukuri:tauri:pr05mute-a";
        let community_allowed = "kukuri:tauri:pr05mute-b";
        insert_community_search_terms(&pool, community_muted).await;
        insert_community_search_terms(&pool, community_allowed).await;
        insert_node_subscription(&pool, community_muted).await;
        insert_node_subscription(&pool, community_allowed).await;
        insert_mute_event_for_community(&pool, &pubkey, community_muted).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/communities/suggest", get(community_suggest))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            "/v1/communities/suggest?q=pr05mute&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(items.iter().any(
            |item| item.get("community_id").and_then(Value::as_str) == Some(community_allowed)
        ));
        assert!(!items
            .iter()
            .any(|item| item.get("community_id").and_then(Value::as_str) == Some(community_muted)));
        assert_eq!(
            payload
                .get("blocked_or_muted_drop_count")
                .and_then(Value::as_u64),
            Some(1)
        );

        sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = ANY($1)")
            .bind(vec![
                community_muted.to_string(),
                community_allowed.to_string(),
            ])
            .execute(&pool)
            .await
            .expect("cleanup suggest mute terms");
        sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = ANY($1)")
            .bind(vec![
                community_muted.to_string(),
                community_allowed.to_string(),
            ])
            .execute(&pool)
            .await
            .expect("cleanup suggest mute node subscriptions");
        sqlx::query("DELETE FROM cn_relay.events WHERE pubkey = $1 AND kind = 10000")
            .bind(&pubkey)
            .execute(&pool)
            .await
            .expect("cleanup mute list events");
        set_suggest_rerank_mode(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RERANK_MODE_SHADOW,
        )
        .await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_LEGACY,
        )
        .await;
    }

    #[tokio::test]
    async fn community_suggest_pg_shadow_mode_preserves_stage_a_order_and_reports_shadow() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_PG,
        )
        .await;
        set_suggest_rerank_mode(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RERANK_MODE_SHADOW,
        )
        .await;
        set_shadow_sample_rate(&pool, "100").await;
        set_suggest_relation_weights(
            &pool,
            r#"{"is_member":20.0,"is_following_community":0.0,"friends_member_count":0.0,"two_hop_follow_count":0.0,"last_view_decay":0.0,"muted_or_blocked":-1.0}"#,
        )
        .await;

        let community_a = "kukuri:tauri:pr05shadow-a";
        let community_b = "kukuri:tauri:pr05shadow-b";
        insert_community_search_terms(&pool, community_a).await;
        insert_community_search_terms(&pool, community_b).await;
        insert_node_subscription(&pool, community_a).await;
        insert_node_subscription(&pool, community_b).await;
        insert_user_community_affinity(
            &pool,
            &pubkey,
            community_b,
            json!({
                "is_member": true,
                "is_following_community": false,
                "friends_member_count": 0,
                "two_hop_follow_count": 0,
                "last_seen_at": null
            }),
        )
        .await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/communities/suggest", get(community_suggest))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            "/v1/communities/suggest?q=pr05shadow&limit=5",
            &token,
            &pool,
            &pubkey,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("rerank_mode").and_then(Value::as_str),
            Some("shadow")
        );
        assert!(payload.get("shadow_topk_overlap").is_some());
        assert!(payload.get("shadow_rank_drift_count").is_some());
        let items = payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(items.len() >= 2);
        assert_eq!(
            items[0].get("community_id").and_then(Value::as_str),
            Some(community_a)
        );
        let stage_b_top = items
            .iter()
            .find(|item| item.get("stage_b_rank").and_then(Value::as_i64) == Some(1))
            .and_then(|item| item.get("community_id").and_then(Value::as_str));
        assert_eq!(stage_b_top, Some(community_b));

        sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = ANY($1)")
            .bind(vec![community_a.to_string(), community_b.to_string()])
            .execute(&pool)
            .await
            .expect("cleanup suggest shadow terms");
        sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = ANY($1)")
            .bind(vec![community_a.to_string(), community_b.to_string()])
            .execute(&pool)
            .await
            .expect("cleanup suggest shadow node subscriptions");
        sqlx::query(
            "DELETE FROM cn_search.user_community_affinity WHERE user_id = $1 AND community_id = ANY($2)",
        )
        .bind(&pubkey)
        .bind(vec![community_a.to_string(), community_b.to_string()])
        .execute(&pool)
        .await
        .expect("cleanup suggest shadow affinity");
        set_shadow_sample_rate(&pool, "0").await;
        set_suggest_relation_weights(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_RELATION_WEIGHTS_DEFAULT,
        )
        .await;
        set_suggest_runtime_flag(
            &pool,
            cn_core::search_runtime_flags::SUGGEST_READ_BACKEND_LEGACY,
        )
        .await;
    }

    #[tokio::test]
    async fn community_search_alias_backfill_skips_kukuri_hashed_tail_topics() {
        let _search_backend_guard = lock_search_backend_contract_tests().await;
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        let hashed_tail = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let hashed_topic_id = format!("kukuri:{hashed_tail}");
        let normal_topic_id = format!("kukuri:tauri:pr30-backfill-{}", Uuid::new_v4().simple());

        sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = ANY($1)")
            .bind(vec![hashed_topic_id.clone(), normal_topic_id.clone()])
            .execute(&pool)
            .await
            .expect("cleanup existing community search terms");

        insert_topic_subscription(&pool, &hashed_topic_id, &pubkey).await;
        insert_topic_subscription(&pool, &normal_topic_id, &pubkey).await;

        run_alias_backfill_for_topic(&pool, &hashed_topic_id).await;
        run_alias_backfill_for_topic(&pool, &normal_topic_id).await;

        let hashed_alias_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) \
             FROM cn_search.community_search_terms \
             WHERE community_id = $1 \
               AND term_type = 'alias'",
        )
        .bind(&hashed_topic_id)
        .fetch_one(&pool)
        .await
        .expect("count hashed alias terms");
        assert_eq!(hashed_alias_count, 0);

        let normal_alias_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) \
             FROM cn_search.community_search_terms \
             WHERE community_id = $1 \
               AND term_type = 'alias'",
        )
        .bind(&normal_topic_id)
        .fetch_one(&pool)
        .await
        .expect("count normal alias terms");
        assert!(normal_alias_count > 0);

        sqlx::query("DELETE FROM cn_search.community_search_terms WHERE community_id = ANY($1)")
            .bind(vec![hashed_topic_id.clone(), normal_topic_id.clone()])
            .execute(&pool)
            .await
            .expect("cleanup community search terms");

        sqlx::query(
            "DELETE FROM cn_user.topic_subscriptions \
             WHERE topic_id = ANY($1) \
               AND subscriber_pubkey = $2",
        )
        .bind(vec![hashed_topic_id, normal_topic_id])
        .bind(&pubkey)
        .execute(&pool)
        .await
        .expect("cleanup topic subscriptions");
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
        let (status, payload) = post_json_with_consent_retry(
            app,
            "/v1/reports",
            &token,
            json!({
                "target": "event:report-contract-target",
                "reason": "spam"
            }),
            &pool,
            &pubkey,
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
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        let target = "event:contract-label";
        let issuer_pubkey = Keys::generate().public_key().to_hex();
        let label_id_a = Uuid::new_v4().to_string();
        let label_id_b = Uuid::new_v4().to_string();
        insert_label(&pool, target, None, &issuer_pubkey, &label_id_a).await;
        insert_label(&pool, target, None, &issuer_pubkey, &label_id_b).await;
        set_label_review_status(&pool, &label_id_b, "disabled").await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/labels", get(list_labels))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            "/v1/labels?target=event:contract-label&limit=1",
            &token,
            &pool,
            &pubkey,
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
        assert_eq!(returned_id, label_id_a);
        assert!(payload.get("next_cursor").and_then(Value::as_str).is_some());
    }

    #[tokio::test]
    async fn trust_report_based_contract_success() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        let subject = format!("pubkey:{pubkey}");
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let attestation_id = Uuid::new_v4().to_string();
        let attestation_exp = now + 3600;
        let event_json = insert_attestation(
            &pool,
            &subject,
            "report-based",
            attestation_exp,
            &attestation_id,
        )
        .await;
        insert_report_score(&pool, &pubkey, &attestation_id, attestation_exp).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route("/v1/trust/report-based", get(trust_report_based))
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            &format!("/v1/trust/report-based?subject={subject}"),
            &token,
            &pool,
            &pubkey,
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
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        let subject = format!("pubkey:{pubkey}");
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let attestation_id = Uuid::new_v4().to_string();
        let attestation_exp = now + 3600;
        let event_json = insert_attestation(
            &pool,
            &subject,
            "communication-density",
            attestation_exp,
            &attestation_id,
        )
        .await;
        insert_communication_score(&pool, &pubkey, &attestation_id, attestation_exp).await;

        let token = issue_token(&state.jwt_config, &pubkey);
        let app = Router::new()
            .route(
                "/v1/trust/communication-density",
                get(trust_communication_density),
            )
            .with_state(state);
        let (status, payload) = get_json_with_consent_retry(
            app,
            &format!("/v1/trust/communication-density?subject={subject}"),
            &token,
            &pool,
            &pubkey,
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
        let pool = state.pool.clone();
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        let topic_id = format!("kukuri:contract-{}", Uuid::new_v4());
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;

        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        insert_relay_event(
            &pool,
            &Uuid::new_v4().to_string(),
            &pubkey,
            1,
            now,
            &topic_id,
        )
        .await;
        insert_relay_event(
            &pool,
            &Uuid::new_v4().to_string(),
            &pubkey,
            7,
            now,
            &topic_id,
        )
        .await;
        insert_relay_event(
            &pool,
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
        let (status, payload) = get_json_with_consent_retry(
            app,
            &format!("/v1/trending?topic={topic_id}"),
            &token,
            &pool,
            &pubkey,
        )
        .await;

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
