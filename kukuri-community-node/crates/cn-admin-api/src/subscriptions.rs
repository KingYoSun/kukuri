use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize)]
pub struct SubscriptionRequestQuery {
    pub status: Option<String>,
}

#[derive(Serialize)]
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

#[derive(Deserialize)]
pub struct ReviewRequest {
    pub review_note: Option<String>,
}

#[derive(Serialize)]
pub struct NodeSubscription {
    pub topic_id: String,
    pub enabled: bool,
    pub ref_count: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
pub struct NodeSubscriptionUpdate {
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct Plan {
    pub plan_id: String,
    pub name: String,
    pub is_active: bool,
    pub limits: Vec<PlanLimit>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanLimit {
    pub metric: String,
    pub window: String,
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct PlanRequest {
    pub plan_id: String,
    pub name: String,
    pub is_active: bool,
    pub limits: Vec<PlanLimit>,
}

#[derive(Serialize)]
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

#[derive(Deserialize)]
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

#[derive(Serialize)]
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

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let row = sqlx::query(
        "SELECT requester_pubkey, topic_id FROM cn_user.topic_subscription_requests WHERE request_id = $1",
    )
    .bind(&request_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "request not found"));
    };
    let requester_pubkey: String = row.try_get("requester_pubkey")?;
    let topic_id: String = row.try_get("topic_id")?;

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

    tx.commit().await.ok();

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "subscription_request.approve",
        &format!("subscription_request:{request_id}"),
        Some(serde_json::json!({ "topic_id": topic_id })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "status": "approved" })))
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
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "request not found"));
    }

    cn_core::admin::log_audit(
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
    .await
    .ok();

    Ok(Json(serde_json::json!({ "status": "rejected" })))
}

pub async fn list_node_subscriptions(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<NodeSubscription>>> {
    require_admin(&state, &jar).await?;

    let rows = sqlx::query(
        "SELECT topic_id, enabled, ref_count, updated_at FROM cn_admin.node_subscriptions ORDER BY updated_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut subscriptions = Vec::new();
    for row in rows {
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        subscriptions.push(NodeSubscription {
            topic_id: row.try_get("topic_id")?,
            enabled: row.try_get("enabled")?,
            ref_count: row.try_get("ref_count")?,
            updated_at: updated_at.timestamp(),
        });
    }

    Ok(Json(subscriptions))
}

pub async fn update_node_subscription(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(topic_id): Path<String>,
    Json(payload): Json<NodeSubscriptionUpdate>,
) -> ApiResult<Json<NodeSubscription>> {
    let admin = require_admin(&state, &jar).await?;
    let row = sqlx::query(
        "UPDATE cn_admin.node_subscriptions          SET enabled = $1, updated_at = NOW()          WHERE topic_id = $2          RETURNING topic_id, enabled, ref_count, updated_at",
    )
    .bind(payload.enabled)
    .bind(&topic_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", "topic not found"));
    };

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "node_subscription.update",
        &format!("topic:{topic_id}"),
        Some(serde_json::json!({ "enabled": payload.enabled })),
        None,
    )
    .await
    .ok();

    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(Json(NodeSubscription {
        topic_id: row.try_get("topic_id")?,
        enabled: row.try_get("enabled")?,
        ref_count: row.try_get("ref_count")?,
        updated_at: updated_at.timestamp(),
    }))
}

pub async fn list_plans(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<Plan>>> {
    require_admin(&state, &jar).await?;

    let rows = sqlx::query("SELECT plan_id, name, is_active FROM cn_user.plans")
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let limit_rows =
        sqlx::query("SELECT plan_id, metric, \"window\", \"limit\" FROM cn_user.plan_limits")
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let mut limit_map: std::collections::HashMap<String, Vec<PlanLimit>> = std::collections::HashMap::new();
    for row in limit_rows {
        let plan_id: String = row.try_get("plan_id")?;
        limit_map
            .entry(plan_id)
            .or_default()
            .push(PlanLimit {
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
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_user.plans (plan_id, name, is_active) VALUES ($1, $2, $3)",
    )
    .bind(&payload.plan_id)
    .bind(&payload.name)
    .bind(payload.is_active)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

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

    tx.commit().await.ok();

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "plan.create",
        &format!("plan:{}", payload.plan_id),
        Some(serde_json::json!({ "name": payload.name })),
        None,
    )
    .await
    .ok();

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
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.plans SET name = $1, is_active = $2 WHERE plan_id = $3",
    )
    .bind(&payload.name)
    .bind(payload.is_active)
    .bind(&plan_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_user.plan_limits WHERE plan_id = $1")
        .bind(&plan_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

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

    tx.commit().await.ok();

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "plan.update",
        &format!("plan:{plan_id}"),
        Some(serde_json::json!({ "name": payload.name })),
        None,
    )
    .await
    .ok();

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

    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "subscription.update",
        &format!("subscription:{subscriber_pubkey}"),
        Some(serde_json::json!({ "plan_id": payload.plan_id, "status": payload.status })),
        None,
    )
    .await
    .ok();

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
