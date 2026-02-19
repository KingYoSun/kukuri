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

#[derive(Debug, Clone)]
struct SearchBackendPayload {
    items: Vec<Value>,
    total: u64,
    next_cursor: Option<String>,
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
    let search_query = query.q.clone();
    let primary =
        search_with_pg_backend(&state, &topic, search_query.clone(), limit, offset).await?;

    Ok(Json(json!({
        "topic": topic,
        "query": search_query,
        "items": primary.items,
        "next_cursor": primary.next_cursor,
        "total": primary.total
    })))
}

async fn search_with_pg_backend(
    state: &AppState,
    topic: &str,
    query: Option<String>,
    limit: usize,
    offset: usize,
) -> ApiResult<SearchBackendPayload> {
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
        let post_id: String = row.try_get("post_id")?;
        items.push(json!({
            "event_id": post_id,
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

    Ok(SearchBackendPayload {
        items,
        total,
        next_cursor,
    })
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
        trgm_score.clamp(0.75, 0.99)
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
    use axum::http::{header, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use cn_core::{search_normalizer, service_config};
    use nostr_sdk::prelude::Keys;
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{Pool, Postgres};
    use std::net::SocketAddr;
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

    async fn test_state() -> crate::AppState {
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
        let user_config = service_config::static_handle(json!({
            "rate_limit": { "enabled": false },
            "subscription_request": { "max_pending_per_pubkey": 5 }
        }));
        let bootstrap_config = service_config::static_handle(json!({
            "auth": { "mode": "off" }
        }));
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
            bootstrap_hints: Arc::new(crate::BootstrapHintStore::default()),
        }
    }

    fn issue_token(config: &cn_core::auth::JwtConfig, pubkey: &str) -> String {
        let (token, _) = cn_core::auth::issue_token(pubkey, config).expect("issue token");
        token
    }

    async fn ensure_consents(pool: &Pool<Postgres>, pubkey: &str) {
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
                    "INSERT INTO cn_user.policy_consents (consent_id, policy_id, accepter_pubkey) \
                     VALUES ($1, $2, $3) \
                     ON CONFLICT DO NOTHING",
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
            "INSERT INTO cn_user.topic_subscriptions (topic_id, subscriber_pubkey, status) \
             VALUES ($1, $2, 'active') \
             ON CONFLICT DO NOTHING",
        )
        .bind(topic_id)
        .bind(pubkey)
        .execute(pool)
        .await
        .expect("insert subscription");
    }

    async fn insert_post_search_document(
        pool: &Pool<Postgres>,
        post_id: &str,
        topic_id: &str,
        author_id: &str,
        body_raw: &str,
        hashtags_norm: &[String],
        created_at: i64,
        normalizer_version: i16,
    ) {
        let body_norm = search_normalizer::normalize_search_text(body_raw);
        let mentions_norm: Vec<String> = Vec::new();
        let community_terms_norm = search_normalizer::normalize_search_terms([topic_id]);
        let search_text = search_normalizer::build_search_text(
            &body_norm,
            hashtags_norm,
            &mentions_norm,
            &community_terms_norm,
        );

        sqlx::query(
            "INSERT INTO cn_search.post_search_documents \
             (post_id, topic_id, author_id, visibility, body_raw, body_norm, hashtags_norm, mentions_norm, community_terms_norm, search_text, language_hint, popularity_score, created_at, is_deleted, normalizer_version, updated_at) \
             VALUES ($1, $2, $3, 'public', $4, $5, $6, $7, $8, $9, NULL, 0, $10, FALSE, $11, NOW())",
        )
        .bind(post_id)
        .bind(topic_id)
        .bind(author_id)
        .bind(body_raw)
        .bind(&body_norm)
        .bind(hashtags_norm)
        .bind(&mentions_norm)
        .bind(&community_terms_norm)
        .bind(&search_text)
        .bind(created_at)
        .bind(normalizer_version)
        .execute(pool)
        .await
        .expect("insert post_search_documents row");
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

    #[tokio::test]
    async fn search_requires_auth() {
        let app = Router::new()
            .route("/v1/search", get(search))
            .with_state(test_state().await);
        let status = request_status(app, "/v1/search?topic=kukuri:topic1").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn search_contract_success_shape_compatible() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let topic_id = format!("kukuri:search-contract-{}", Uuid::new_v4().simple());
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;

        let post_id = Uuid::new_v4().to_string();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        insert_post_search_document(
            &pool,
            &post_id,
            &topic_id,
            &pubkey,
            "hello contract",
            &["contract".to_string()],
            now,
            search_normalizer::SEARCH_NORMALIZER_VERSION,
        )
        .await;

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

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("topic").and_then(Value::as_str),
            Some(topic_id.as_str())
        );
        assert_eq!(payload.get("query").and_then(Value::as_str), Some("hello"));
        assert_eq!(payload.get("total").and_then(Value::as_u64), Some(1));
        assert_eq!(payload.get("next_cursor"), Some(&Value::Null));
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
            Some(post_id.as_str())
        );

        sqlx::query("DELETE FROM cn_search.post_search_documents WHERE post_id = $1")
            .bind(post_id)
            .execute(&pool)
            .await
            .expect("cleanup post search document");
    }

    #[tokio::test]
    async fn search_contract_pg_backend_switch_normalization_and_version_filter() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let topic_id = format!("kukuri:search-pg-{}", Uuid::new_v4());
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_id, &pubkey).await;

        let current_post_id = Uuid::new_v4().to_string();
        let stale_post_id = Uuid::new_v4().to_string();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;

        insert_post_search_document(
            &pool,
            &current_post_id,
            &topic_id,
            &pubkey,
            "Ｈｅｌｌｏ PG Search #Rust",
            &["rust".to_string()],
            now,
            search_normalizer::SEARCH_NORMALIZER_VERSION,
        )
        .await;

        let stale_version = search_normalizer::SEARCH_NORMALIZER_VERSION.saturating_sub(1);
        insert_post_search_document(
            &pool,
            &stale_post_id,
            &topic_id,
            &pubkey,
            "hello old normalizer",
            &[],
            now - 1,
            stale_version,
        )
        .await;

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
    }

    #[tokio::test]
    async fn search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let topic_a = format!("kukuri:search-pg-topic-a-{}", Uuid::new_v4().simple());
        let topic_b = format!("kukuri:search-pg-topic-b-{}", Uuid::new_v4().simple());
        let pubkey = Keys::generate().public_key().to_hex();
        ensure_consents(&pool, &pubkey).await;
        insert_topic_subscription(&pool, &topic_a, &pubkey).await;
        insert_topic_subscription(&pool, &topic_b, &pubkey).await;

        let shared_post_id = Uuid::new_v4().to_string();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        insert_post_search_document(
            &pool,
            &shared_post_id,
            &topic_a,
            &pubkey,
            "Shared multi topic body for topic A",
            &["shared".to_string(), "topica".to_string()],
            now,
            search_normalizer::SEARCH_NORMALIZER_VERSION,
        )
        .await;
        insert_post_search_document(
            &pool,
            &shared_post_id,
            &topic_b,
            &pubkey,
            "Shared multi topic body for topic B",
            &["shared".to_string(), "topicb".to_string()],
            now - 1,
            search_normalizer::SEARCH_NORMALIZER_VERSION,
        )
        .await;

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
        assert_eq!(
            item_a.get("content").and_then(Value::as_str),
            Some("Shared multi topic body for topic A")
        );

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
        assert_eq!(
            item_b.get("content").and_then(Value::as_str),
            Some("Shared multi topic body for topic B")
        );

        sqlx::query(
            "DELETE FROM cn_search.post_search_documents \
             WHERE post_id = $1 AND topic_id = ANY($2)",
        )
        .bind(&shared_post_id)
        .bind(vec![topic_a, topic_b])
        .execute(&pool)
        .await
        .expect("cleanup multi topic post search documents");
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
}
