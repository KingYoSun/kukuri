use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::{BTreeSet, HashMap};
use utoipa::ToSchema;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

const NODE_SUBSCRIPTION_LIMIT_LOCK_CONTEXT: &[u8] =
    b"cn-admin-api.subscription-request.approve.node-subscription-limit";

#[derive(Deserialize)]
pub struct SubscriptionRequestQuery {
    pub status: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionRequestRow {
    pub request_id: String,
    pub requester_pubkey: String,
    pub topic_id: String,
    pub requested_services: Value,
    pub status: String,
    pub review_note: Option<String>,
    pub created_at: i64,
    pub reviewed_at: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReviewRequest {
    pub review_note: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct NodeSubscription {
    pub topic_id: String,
    pub enabled: bool,
    pub ref_count: i64,
    pub ingest_policy: Option<NodeSubscriptionIngestPolicy>,
    #[serde(default)]
    pub connected_nodes: Vec<String>,
    #[serde(default)]
    pub connected_node_count: i64,
    #[serde(default)]
    pub connected_users: Vec<String>,
    #[serde(default)]
    pub connected_user_count: i64,
    pub updated_at: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NodeSubscriptionCreate {
    pub topic_id: String,
    pub enabled: Option<bool>,
    pub ingest_policy: Option<NodeSubscriptionIngestPolicy>,
}

#[derive(Deserialize, ToSchema)]
pub struct NodeSubscriptionUpdate {
    pub enabled: bool,
    pub ingest_policy: Option<NodeSubscriptionIngestPolicy>,
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct NodeSubscriptionIngestPolicy {
    pub retention_days: Option<i64>,
    pub max_events: Option<i64>,
    pub max_bytes: Option<i64>,
    pub allow_backfill: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct Plan {
    pub plan_id: String,
    pub name: String,
    pub is_active: bool,
    pub limits: Vec<PlanLimit>,
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct PlanLimit {
    pub metric: String,
    pub window: String,
    pub limit: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct PlanRequest {
    pub plan_id: String,
    pub name: String,
    pub is_active: bool,
    pub limits: Vec<PlanLimit>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionRow {
    pub subscription_id: String,
    pub subscriber_pubkey: String,
    pub plan_id: String,
    pub status: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Deserialize)]
pub struct SubscriptionQuery {
    pub pubkey: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SubscriptionUpdate {
    pub plan_id: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct UsageQuery {
    pub pubkey: String,
    pub metric: Option<String>,
    pub days: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct UsageRow {
    pub metric: String,
    pub day: String,
    pub count: i64,
}

pub async fn list_subscription_requests(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<SubscriptionRequestQuery>,
) -> ApiResult<Json<Vec<SubscriptionRequestRow>>> {
    require_admin(&state, &jar).await?;

    let rows = if let Some(status) = query.status {
        sqlx::query(
            "SELECT request_id, requester_pubkey, topic_id, requested_services, status, review_note, created_at, reviewed_at              FROM cn_user.topic_subscription_requests WHERE status = $1 ORDER BY created_at DESC",
        )
        .bind(status)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query(
            "SELECT request_id, requester_pubkey, topic_id, requested_services, status, review_note, created_at, reviewed_at              FROM cn_user.topic_subscription_requests ORDER BY created_at DESC",
        )
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut requests = Vec::new();
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let reviewed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("reviewed_at")?;
        requests.push(SubscriptionRequestRow {
            request_id: row.try_get("request_id")?,
            requester_pubkey: row.try_get("requester_pubkey")?,
            topic_id: row.try_get("topic_id")?,
            requested_services: row.try_get("requested_services")?,
            status: row.try_get("status")?,
            review_note: row.try_get("review_note").ok(),
            created_at: created_at.timestamp(),
            reviewed_at: reviewed_at.map(|value| value.timestamp()),
        });
    }

    Ok(Json(requests))
}

pub async fn approve_subscription_request(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(request_id): Path<String>,
    Json(payload): Json<ReviewRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let admin = require_admin(&state, &jar).await?;

    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let row = sqlx::query(
        "SELECT requester_pubkey, topic_id FROM cn_user.topic_subscription_requests WHERE request_id = $1",
    )
    .bind(&request_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "request not found",
        ));
    };
    let requester_pubkey: String = row.try_get("requester_pubkey")?;
    let topic_id: String = row.try_get("topic_id")?;

    enforce_node_subscription_topic_limit(&mut tx, &topic_id).await?;

    sqlx::query(
        "UPDATE cn_user.topic_subscription_requests          SET status = 'approved', review_note = $1, reviewed_at = NOW()          WHERE request_id = $2",
    )
    .bind(&payload.review_note)
    .bind(&request_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_user.topic_subscriptions          (topic_id, subscriber_pubkey, status)          VALUES ($1, $2, 'active')          ON CONFLICT (topic_id, subscriber_pubkey) DO UPDATE SET status = 'active', ended_at = NULL",
    )
    .bind(&topic_id)
    .bind(&requester_pubkey)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions          (topic_id, enabled, ref_count)          VALUES ($1, TRUE, 1)          ON CONFLICT (topic_id) DO UPDATE SET ref_count = cn_admin.node_subscriptions.ref_count + 1, enabled = TRUE, updated_at = NOW()",
    )
    .bind(&topic_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    crate::log_admin_audit_tx(
        &mut tx,
        &admin.admin_user_id,
        "subscription_request.approve",
        &format!("subscription_request:{request_id}"),
        Some(serde_json::json!({ "topic_id": topic_id })),
        None,
    )
    .await?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(Json(serde_json::json!({ "status": "approved" })))
}

async fn enforce_node_subscription_topic_limit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    topic_id: &str,
) -> ApiResult<()> {
    let (lock_key_high, lock_key_low) = advisory_lock_keys_for_node_subscription_limit();
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(lock_key_high)
        .bind(lock_key_low)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let existing_enabled = sqlx::query_scalar::<_, bool>(
        "SELECT enabled FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(topic_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    if existing_enabled == Some(true) {
        return Ok(());
    }

    let relay_config = sqlx::query_scalar::<_, Value>(
        "SELECT config_json FROM cn_admin.service_configs WHERE service = 'relay'",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    let max_topics = relay_config
        .as_ref()
        .map(cn_core::service_config::max_concurrent_node_topics_from_json)
        .unwrap_or(cn_core::service_config::DEFAULT_MAX_CONCURRENT_NODE_TOPICS);

    let current_enabled_topics = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.node_subscriptions WHERE enabled = TRUE",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    if current_enabled_topics >= max_topics {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED",
            "node-level concurrent topic ingest limit reached",
        )
        .with_details(json!({
            "metric": "node_subscriptions.enabled_topics",
            "scope": "node",
            "current": current_enabled_topics,
            "limit": max_topics
        })));
    }

    Ok(())
}

fn advisory_lock_keys_for_node_subscription_limit() -> (i32, i32) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(NODE_SUBSCRIPTION_LIMIT_LOCK_CONTEXT);
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();
    (
        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
    )
}

pub async fn reject_subscription_request(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(request_id): Path<String>,
    Json(payload): Json<ReviewRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let admin = require_admin(&state, &jar).await?;
    let result = sqlx::query(
        "UPDATE cn_user.topic_subscription_requests          SET status = 'rejected', review_note = $1, reviewed_at = NOW()          WHERE request_id = $2",
    )
    .bind(&payload.review_note)
    .bind(&request_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "request not found",
        ));
    }

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "subscription_request.reject",
        &format!("subscription_request:{request_id}"),
        payload
            .review_note
            .as_ref()
            .map(|note| serde_json::json!({ "note": note })),
        None,
    )
    .await?;

    Ok(Json(serde_json::json!({ "status": "rejected" })))
}

pub async fn list_node_subscriptions(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<NodeSubscription>>> {
    require_admin(&state, &jar).await?;
    let connected_nodes_by_topic = load_connected_nodes_by_topic(&state.pool, None).await?;
    let connected_users_by_topic = load_connected_users_by_topic(&state.pool, None).await?;
    let relay_runtime = load_relay_runtime_connectivity(&state.pool).await?;

    let rows = sqlx::query(
        "SELECT topic_id, enabled, ref_count, ingest_policy, updated_at \
         FROM cn_admin.node_subscriptions \
         ORDER BY updated_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let mut subscriptions = Vec::new();
    for row in rows {
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        let topic_id: String = row.try_get("topic_id")?;
        let enabled: bool = row.try_get("enabled")?;
        let ref_count: i64 = row.try_get("ref_count")?;
        let connected_nodes = connected_nodes_by_topic
            .get(&topic_id)
            .cloned()
            .unwrap_or_default();
        let connected_users = connected_users_by_topic
            .get(&topic_id)
            .cloned()
            .unwrap_or_default();
        let (connected_nodes, connected_node_count, connected_users, connected_user_count) =
            apply_runtime_connectivity_fallback(
                enabled,
                ref_count,
                connected_nodes,
                connected_users,
                &relay_runtime,
                &topic_id,
            );
        subscriptions.push(NodeSubscription {
            topic_id,
            enabled,
            ref_count,
            ingest_policy: parse_node_ingest_policy(row.try_get("ingest_policy")?)?,
            connected_node_count,
            connected_nodes,
            connected_user_count,
            connected_users,
            updated_at: updated_at.timestamp(),
        });
    }

    Ok(Json(subscriptions))
}

pub async fn create_node_subscription(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(payload): Json<NodeSubscriptionCreate>,
) -> ApiResult<Json<NodeSubscription>> {
    let admin = require_admin(&state, &jar).await?;
    let topic_id = normalize_topic_id_input(&payload.topic_id)?;
    let enabled = payload.enabled.unwrap_or(true);
    let ingest_policy_json = payload
        .ingest_policy
        .as_ref()
        .map(validate_and_normalize_ingest_policy)
        .transpose()?;

    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let existing = sqlx::query_scalar::<_, String>(
        "SELECT topic_id FROM cn_admin.node_subscriptions WHERE topic_id = $1",
    )
    .bind(&topic_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    if existing.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            "topic already exists",
        ));
    }

    if enabled {
        enforce_node_subscription_topic_limit(&mut tx, &topic_id).await?;
    }

    let row = sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions \
         (topic_id, enabled, ref_count, ingest_policy, updated_at) \
         VALUES ($1, $2, 0, $3::jsonb, NOW()) \
         RETURNING topic_id, enabled, ref_count, ingest_policy, updated_at",
    )
    .bind(&topic_id)
    .bind(enabled)
    .bind(ingest_policy_json.clone())
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    crate::log_admin_audit_tx(
        &mut tx,
        &admin.admin_user_id,
        "node_subscription.create",
        &format!("topic:{topic_id}"),
        Some(serde_json::json!({
            "enabled": enabled,
            "ingest_policy": ingest_policy_json
        })),
        None,
    )
    .await?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let connected_nodes = load_connected_nodes_by_topic(&state.pool, Some(&topic_id))
        .await?
        .remove(&topic_id)
        .unwrap_or_default();
    let connected_users = load_connected_users_by_topic(&state.pool, Some(&topic_id))
        .await?
        .remove(&topic_id)
        .unwrap_or_default();
    let relay_runtime = load_relay_runtime_connectivity(&state.pool).await?;
    let enabled: bool = row.try_get("enabled")?;
    let ref_count: i64 = row.try_get("ref_count")?;
    let (connected_nodes, connected_node_count, connected_users, connected_user_count) =
        apply_runtime_connectivity_fallback(
            enabled,
            ref_count,
            connected_nodes,
            connected_users,
            &relay_runtime,
            &topic_id,
        );
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(Json(NodeSubscription {
        topic_id: row.try_get("topic_id")?,
        enabled,
        ref_count,
        ingest_policy: parse_node_ingest_policy(row.try_get("ingest_policy")?)?,
        connected_node_count,
        connected_nodes,
        connected_user_count,
        connected_users,
        updated_at: updated_at.timestamp(),
    }))
}

pub async fn update_node_subscription(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(topic_id): Path<String>,
    Json(payload): Json<NodeSubscriptionUpdate>,
) -> ApiResult<Json<NodeSubscription>> {
    let admin = require_admin(&state, &jar).await?;
    let ingest_policy_json = payload
        .ingest_policy
        .as_ref()
        .map(validate_and_normalize_ingest_policy)
        .transpose()?;
    let row = sqlx::query(
        "UPDATE cn_admin.node_subscriptions \
         SET enabled = $1, \
             ingest_policy = COALESCE($2::jsonb, ingest_policy), \
             updated_at = NOW() \
         WHERE topic_id = $3 \
         RETURNING topic_id, enabled, ref_count, ingest_policy, updated_at",
    )
    .bind(payload.enabled)
    .bind(ingest_policy_json.clone())
    .bind(&topic_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "topic not found",
        ));
    };

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "node_subscription.update",
        &format!("topic:{topic_id}"),
        Some(serde_json::json!({
            "enabled": payload.enabled,
            "ingest_policy": ingest_policy_json
        })),
        None,
    )
    .await?;

    let connected_nodes = load_connected_nodes_by_topic(&state.pool, Some(&topic_id))
        .await?
        .remove(&topic_id)
        .unwrap_or_default();
    let connected_users = load_connected_users_by_topic(&state.pool, Some(&topic_id))
        .await?
        .remove(&topic_id)
        .unwrap_or_default();
    let relay_runtime = load_relay_runtime_connectivity(&state.pool).await?;
    let enabled: bool = row.try_get("enabled")?;
    let ref_count: i64 = row.try_get("ref_count")?;
    let (connected_nodes, connected_node_count, connected_users, connected_user_count) =
        apply_runtime_connectivity_fallback(
            enabled,
            ref_count,
            connected_nodes,
            connected_users,
            &relay_runtime,
            &topic_id,
        );
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(Json(NodeSubscription {
        topic_id: row.try_get("topic_id")?,
        enabled,
        ref_count,
        ingest_policy: parse_node_ingest_policy(row.try_get("ingest_policy")?)?,
        connected_node_count,
        connected_nodes,
        connected_user_count,
        connected_users,
        updated_at: updated_at.timestamp(),
    }))
}

pub async fn delete_node_subscription(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let admin = require_admin(&state, &jar).await?;
    let topic_id = normalize_topic_id_input(&topic_id)?;
    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let ref_count = sqlx::query_scalar::<_, i64>(
        "SELECT ref_count FROM cn_admin.node_subscriptions WHERE topic_id = $1 FOR UPDATE",
    )
    .bind(&topic_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    let Some(ref_count) = ref_count else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "topic not found",
        ));
    };
    if ref_count > 0 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "NODE_SUBSCRIPTION_IN_USE",
            "node subscription is still referenced by active subscriptions",
        )
        .with_details(json!({
            "topic_id": topic_id,
            "ref_count": ref_count,
        })));
    }

    sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = $1")
        .bind(&topic_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    crate::log_admin_audit_tx(
        &mut tx,
        &admin.admin_user_id,
        "node_subscription.delete",
        &format!("topic:{topic_id}"),
        Some(serde_json::json!({ "ref_count": ref_count })),
        None,
    )
    .await?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "topic_id": topic_id
    })))
}

fn parse_node_ingest_policy(raw: Option<Value>) -> ApiResult<Option<NodeSubscriptionIngestPolicy>> {
    match raw {
        None => Ok(None),
        Some(value) => serde_json::from_value::<NodeSubscriptionIngestPolicy>(value)
            .map(Some)
            .map_err(|err| {
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    format!("invalid ingest_policy payload: {err}"),
                )
            }),
    }
}

fn validate_and_normalize_ingest_policy(policy: &NodeSubscriptionIngestPolicy) -> ApiResult<Value> {
    if let Some(retention_days) = policy.retention_days {
        if retention_days < 0 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INGEST_POLICY",
                "retention_days must be 0 or greater",
            ));
        }
    }
    if let Some(max_events) = policy.max_events {
        if max_events < 1 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INGEST_POLICY",
                "max_events must be 1 or greater",
            ));
        }
    }
    if let Some(max_bytes) = policy.max_bytes {
        if max_bytes < 1 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INGEST_POLICY",
                "max_bytes must be 1 or greater",
            ));
        }
    }

    Ok(serde_json::json!({
        "retention_days": policy.retention_days,
        "max_events": policy.max_events,
        "max_bytes": policy.max_bytes,
        "allow_backfill": policy.allow_backfill.unwrap_or(true),
    }))
}

fn normalize_topic_id_input(topic_id: &str) -> ApiResult<String> {
    cn_core::topic::normalize_topic_id(topic_id)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_TOPIC_ID", err.to_string()))
}

#[derive(Default)]
struct RelayRuntimeConnectivity {
    bootstrap_nodes: Vec<String>,
    ws_connections: i64,
}

async fn load_relay_runtime_connectivity(
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> ApiResult<RelayRuntimeConnectivity> {
    let details = sqlx::query_scalar::<_, Value>(
        "SELECT details_json FROM cn_admin.service_health WHERE service = 'relay'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let Some(details) = details else {
        return Ok(RelayRuntimeConnectivity::default());
    };

    let ws_connections = details
        .get("auth_transition")
        .and_then(Value::as_object)
        .and_then(|auth_transition| auth_transition.get("ws_connections"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let bootstrap_nodes = details
        .get("p2p_runtime")
        .and_then(Value::as_object)
        .and_then(|runtime| runtime.get("bootstrap_nodes"))
        .and_then(Value::as_array)
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|node| !node.is_empty())
                .map(std::string::ToString::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(RelayRuntimeConnectivity {
        bootstrap_nodes,
        ws_connections,
    })
}

fn apply_runtime_connectivity_fallback(
    enabled: bool,
    ref_count: i64,
    mut connected_nodes: Vec<String>,
    connected_users: Vec<String>,
    relay_runtime: &RelayRuntimeConnectivity,
    topic_id: &str,
) -> (Vec<String>, i64, Vec<String>, i64) {
    if enabled
        && ref_count > 0
        && connected_nodes.is_empty()
        && !relay_runtime.bootstrap_nodes.is_empty()
    {
        connected_nodes = relay_runtime.bootstrap_nodes.clone();
    }
    let connected_node_count = connected_nodes.len() as i64;

    let runtime_ws_fallback_applied = enabled
        && ref_count > 0
        && connected_users.is_empty()
        && relay_runtime.ws_connections > 0
        && topic_id == cn_core::topic::DEFAULT_PUBLIC_TOPIC_ID;
    let connected_user_count = if runtime_ws_fallback_applied {
        relay_runtime.ws_connections
    } else {
        connected_users.len() as i64
    };

    (
        connected_nodes,
        connected_node_count,
        connected_users,
        connected_user_count,
    )
}

async fn load_connected_nodes_by_topic(
    pool: &sqlx::Pool<sqlx::Postgres>,
    topic_id_filter: Option<&str>,
) -> ApiResult<HashMap<String, Vec<String>>> {
    let descriptor_rows = sqlx::query(
        "SELECT event_json FROM cn_bootstrap.events WHERE kind = 39000 AND is_active = TRUE",
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
    let mut host_port_by_node_id = HashMap::new();
    for row in descriptor_rows {
        let event_json: Value = row.try_get("event_json")?;
        let Ok(raw_event) = cn_core::nostr::parse_event(&event_json) else {
            continue;
        };
        if let Some(host_port) = extract_descriptor_host_port(&raw_event) {
            host_port_by_node_id.insert(raw_event.pubkey, host_port);
        }
    }

    let topic_rows = if let Some(topic_id) = topic_id_filter {
        sqlx::query(
            "SELECT event_json, topic_id \
             FROM cn_bootstrap.events \
             WHERE kind = 39001 AND role = 'relay' AND is_active = TRUE AND topic_id = $1",
        )
        .bind(topic_id)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT event_json, topic_id \
             FROM cn_bootstrap.events \
             WHERE kind = 39001 AND role = 'relay' AND is_active = TRUE",
        )
        .fetch_all(pool)
        .await
    }
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let mut grouped = HashMap::<String, BTreeSet<String>>::new();
    for row in topic_rows {
        let event_json: Value = row.try_get("event_json")?;
        let Ok(raw_event) = cn_core::nostr::parse_event(&event_json) else {
            continue;
        };
        let topic_id = row
            .try_get::<Option<String>, _>("topic_id")?
            .or_else(|| raw_event.first_tag_value("t"))
            .unwrap_or_default();
        if topic_id.is_empty() {
            continue;
        }
        let host_port = host_port_by_node_id
            .get(&raw_event.pubkey)
            .cloned()
            .unwrap_or_else(|| "unknown:0".to_string());
        grouped
            .entry(topic_id)
            .or_default()
            .insert(format!("{}@{}", raw_event.pubkey, host_port));
    }

    Ok(grouped
        .into_iter()
        .map(|(topic_id, nodes)| (topic_id, nodes.into_iter().collect()))
        .collect())
}

async fn load_connected_users_by_topic(
    pool: &sqlx::Pool<sqlx::Postgres>,
    topic_id_filter: Option<&str>,
) -> ApiResult<HashMap<String, Vec<String>>> {
    let rows = if let Some(topic_id) = topic_id_filter {
        sqlx::query(
            "SELECT topic_id, subscriber_pubkey \
             FROM cn_user.topic_subscriptions \
             WHERE status = 'active' AND ended_at IS NULL AND topic_id = $1",
        )
        .bind(topic_id)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT topic_id, subscriber_pubkey \
             FROM cn_user.topic_subscriptions \
             WHERE status = 'active' AND ended_at IS NULL",
        )
        .fetch_all(pool)
        .await
    }
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let mut grouped = HashMap::<String, BTreeSet<String>>::new();
    for row in rows {
        let topic_id: String = row.try_get("topic_id")?;
        let subscriber_pubkey: String = row.try_get("subscriber_pubkey")?;
        if topic_id.is_empty() || subscriber_pubkey.trim().is_empty() {
            continue;
        }
        grouped
            .entry(topic_id)
            .or_default()
            .insert(subscriber_pubkey.trim().to_string());
    }

    Ok(grouped
        .into_iter()
        .map(|(topic_id, users)| (topic_id, users.into_iter().collect()))
        .collect())
}

fn extract_descriptor_host_port(event: &cn_core::nostr::RawEvent) -> Option<String> {
    let content: Value = serde_json::from_str(&event.content).ok()?;
    let endpoints = content.get("endpoints")?;
    match endpoints {
        Value::String(endpoint) => parse_host_port_from_endpoint(endpoint),
        Value::Object(map) => {
            const PRIORITY_KEYS: [&str; 5] = ["p2p", "gossip", "relay", "ws", "http"];
            for key in PRIORITY_KEYS {
                if let Some(endpoint) = map.get(key).and_then(Value::as_str) {
                    if let Some(host_port) = parse_host_port_from_endpoint(endpoint) {
                        return Some(host_port);
                    }
                }
            }
            for value in map.values() {
                if let Some(endpoint) = value.as_str() {
                    if let Some(host_port) = parse_host_port_from_endpoint(endpoint) {
                        return Some(host_port);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_host_port_from_endpoint(endpoint: &str) -> Option<String> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((_, rest)) = trimmed.rsplit_once('@') {
        if rest != trimmed {
            return parse_host_port_from_endpoint(rest);
        }
    }

    if let Ok(url) = reqwest::Url::parse(trimmed) {
        let host = url.host_str()?;
        let port = url.port_or_known_default()?;
        return Some(format!("{host}:{port}"));
    }

    let host_port = trimmed.split('/').next().unwrap_or(trimmed);
    if let Some((host, port)) = host_port.rsplit_once(':') {
        if host.is_empty() {
            return None;
        }
        if let Ok(parsed_port) = port.parse::<u16>() {
            return Some(format!("{host}:{parsed_port}"));
        }
    }

    None
}

pub async fn list_plans(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<Plan>>> {
    require_admin(&state, &jar).await?;

    let rows = sqlx::query("SELECT plan_id, name, is_active FROM cn_user.plans")
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let limit_rows =
        sqlx::query("SELECT plan_id, metric, \"window\", \"limit\" FROM cn_user.plan_limits")
            .fetch_all(&state.pool)
            .await
            .map_err(|err| {
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    err.to_string(),
                )
            })?;
    let mut limit_map: std::collections::HashMap<String, Vec<PlanLimit>> =
        std::collections::HashMap::new();
    for row in limit_rows {
        let plan_id: String = row.try_get("plan_id")?;
        limit_map.entry(plan_id).or_default().push(PlanLimit {
            metric: row.try_get("metric")?,
            window: row.try_get("window")?,
            limit: row.try_get("limit")?,
        });
    }

    let mut plans = Vec::new();
    for row in rows {
        let plan_id: String = row.try_get("plan_id")?;
        plans.push(Plan {
            plan_id: plan_id.clone(),
            name: row.try_get("name")?,
            is_active: row.try_get("is_active")?,
            limits: limit_map.remove(&plan_id).unwrap_or_default(),
        });
    }

    Ok(Json(plans))
}

pub async fn create_plan(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(payload): Json<PlanRequest>,
) -> ApiResult<Json<Plan>> {
    let admin = require_admin(&state, &jar).await?;
    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    sqlx::query("INSERT INTO cn_user.plans (plan_id, name, is_active) VALUES ($1, $2, $3)")
        .bind(&payload.plan_id)
        .bind(&payload.name)
        .bind(payload.is_active)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    for limit in payload.limits.iter() {
        sqlx::query(
            "INSERT INTO cn_user.plan_limits (plan_id, metric, \"window\", \"limit\") VALUES ($1, $2, $3, $4)",
        )
        .bind(&payload.plan_id)
        .bind(&limit.metric)
        .bind(&limit.window)
        .bind(limit.limit)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    crate::log_admin_audit_tx(
        &mut tx,
        &admin.admin_user_id,
        "plan.create",
        &format!("plan:{}", payload.plan_id),
        Some(serde_json::json!({ "name": payload.name })),
        None,
    )
    .await?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(Json(Plan {
        plan_id: payload.plan_id,
        name: payload.name,
        is_active: payload.is_active,
        limits: payload.limits,
    }))
}

pub async fn update_plan(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(plan_id): Path<String>,
    Json(payload): Json<PlanRequest>,
) -> ApiResult<Json<Plan>> {
    let admin = require_admin(&state, &jar).await?;
    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    sqlx::query("UPDATE cn_user.plans SET name = $1, is_active = $2 WHERE plan_id = $3")
        .bind(&payload.name)
        .bind(payload.is_active)
        .bind(&plan_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.plan_limits WHERE plan_id = $1")
        .bind(&plan_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    for limit in payload.limits.iter() {
        sqlx::query(
            "INSERT INTO cn_user.plan_limits (plan_id, metric, \"window\", \"limit\") VALUES ($1, $2, $3, $4)",
        )
        .bind(&plan_id)
        .bind(&limit.metric)
        .bind(&limit.window)
        .bind(limit.limit)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    crate::log_admin_audit_tx(
        &mut tx,
        &admin.admin_user_id,
        "plan.update",
        &format!("plan:{plan_id}"),
        Some(serde_json::json!({ "name": payload.name })),
        None,
    )
    .await?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(Json(Plan {
        plan_id,
        name: payload.name,
        is_active: payload.is_active,
        limits: payload.limits,
    }))
}

pub async fn list_subscriptions(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<SubscriptionQuery>,
) -> ApiResult<Json<Vec<SubscriptionRow>>> {
    require_admin(&state, &jar).await?;

    let rows = if let Some(pubkey) = query.pubkey {
        sqlx::query(
            "SELECT subscription_id, subscriber_pubkey, plan_id, status, started_at, ended_at              FROM cn_user.subscriptions WHERE subscriber_pubkey = $1 ORDER BY started_at DESC",
        )
        .bind(pubkey)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query(
            "SELECT subscription_id, subscriber_pubkey, plan_id, status, started_at, ended_at              FROM cn_user.subscriptions ORDER BY started_at DESC",
        )
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut subscriptions = Vec::new();
    for row in rows {
        let started_at: chrono::DateTime<chrono::Utc> = row.try_get("started_at")?;
        let ended_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("ended_at")?;
        subscriptions.push(SubscriptionRow {
            subscription_id: row.try_get("subscription_id")?,
            subscriber_pubkey: row.try_get("subscriber_pubkey")?,
            plan_id: row.try_get("plan_id")?,
            status: row.try_get("status")?,
            started_at: started_at.timestamp(),
            ended_at: ended_at.map(|value| value.timestamp()),
        });
    }

    Ok(Json(subscriptions))
}

pub async fn upsert_subscription(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(subscriber_pubkey): Path<String>,
    Json(payload): Json<SubscriptionUpdate>,
) -> ApiResult<Json<serde_json::Value>> {
    let admin = require_admin(&state, &jar).await?;
    let status = payload.status.as_str();
    let existing_id = sqlx::query_scalar::<_, String>(
        "SELECT subscription_id FROM cn_user.subscriptions WHERE subscriber_pubkey = $1 ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&subscriber_pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if let Some(subscription_id) = existing_id {
        sqlx::query(
            "UPDATE cn_user.subscriptions              SET plan_id = $1, status = $2, started_at = NOW(), ended_at = CASE WHEN $2 = 'active' THEN NULL ELSE NOW() END              WHERE subscription_id = $3",
        )
        .bind(&payload.plan_id)
        .bind(status)
        .bind(&subscription_id)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    } else {
        let subscription_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO cn_user.subscriptions              (subscription_id, subscriber_pubkey, plan_id, status, started_at, ended_at)              VALUES ($1, $2, $3, $4, NOW(), CASE WHEN $4 = 'active' THEN NULL ELSE NOW() END)",
        )
        .bind(&subscription_id)
        .bind(&subscriber_pubkey)
        .bind(&payload.plan_id)
        .bind(status)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "subscription.update",
        &format!("subscription:{subscriber_pubkey}"),
        Some(serde_json::json!({ "plan_id": payload.plan_id, "status": payload.status })),
        None,
    )
    .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub async fn list_usage(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<UsageQuery>,
) -> ApiResult<Json<Vec<UsageRow>>> {
    require_admin(&state, &jar).await?;

    let days = query.days.unwrap_or(30).clamp(1, 365);
    let since = chrono::Utc::now().date_naive() - chrono::Duration::days(days);

    let rows = if let Some(metric) = query.metric {
        sqlx::query(
            "SELECT metric, day, count FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1 AND metric = $2 AND day >= $3 ORDER BY day DESC",
        )
        .bind(&query.pubkey)
        .bind(metric)
        .bind(since)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query(
            "SELECT metric, day, count FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1 AND day >= $2 ORDER BY day DESC",
        )
        .bind(&query.pubkey)
        .bind(since)
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut usage = Vec::new();
    for row in rows {
        let day: chrono::NaiveDate = row.try_get("day")?;
        usage.push(UsageRow {
            metric: row.try_get("metric")?,
            day: day.to_string(),
            count: row.try_get("count")?,
        });
    }

    Ok(Json(usage))
}
