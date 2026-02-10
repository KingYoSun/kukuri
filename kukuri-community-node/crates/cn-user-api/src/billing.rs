use anyhow::Result;
use axum::http::StatusCode;
use cn_core::metrics;
use serde_json::json;
use sqlx::{Pool, Postgres};

use crate::{ApiError, ApiResult};

const DEFAULT_PLAN_ID: &str = "free";

pub async fn ensure_default_plan(pool: &Pool<Postgres>) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_user.plans          (plan_id, name, is_active)          VALUES ($1, $2, TRUE)          ON CONFLICT (plan_id) DO NOTHING",
    )
    .bind(DEFAULT_PLAN_ID)
    .bind("Free")
    .execute(pool)
    .await?;

    let limits = vec![
        ("max_topics", "limit", 1_i64),
        ("index.search_requests", "day", 100),
        ("index.trending_requests", "day", 100),
        ("trust.requests", "day", 100),
        ("moderation.report_submits", "day", 20),
        ("invite.redeem_attempts", "day", 50),
    ];

    for (metric, window, limit) in limits {
        sqlx::query(
            "INSERT INTO cn_user.plan_limits              (plan_id, metric, \"window\", \"limit\")              VALUES ($1, $2, $3, $4)              ON CONFLICT DO NOTHING",
        )
        .bind(DEFAULT_PLAN_ID)
        .bind(metric)
        .bind(window)
        .bind(limit)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub(crate) async fn check_topic_limit(pool: &Pool<Postgres>, pubkey: &str) -> ApiResult<()> {
    let plan_id = active_plan_id(pool, pubkey).await?;
    let limit = sqlx::query_scalar::<_, i64>(
        "SELECT \"limit\" FROM cn_user.plan_limits WHERE plan_id = $1 AND metric = 'max_topics' AND \"window\" = 'limit'",
    )
    .bind(&plan_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let Some(limit) = limit else {
        return Ok(());
    };

    let current = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = $1 AND status = 'active'",
    )
    .bind(pubkey)
    .fetch_one(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if current >= limit {
        metrics::inc_quota_exceeded(crate::SERVICE_NAME, "max_topics");
        return Err(ApiError::new(
            StatusCode::PAYMENT_REQUIRED,
            "QUOTA_EXCEEDED",
            "topic limit reached",
        )
        .with_details(json!({
            "metric": "max_topics",
            "current": current,
            "limit": limit,
        })));
    }

    Ok(())
}

pub(crate) async fn consume_quota(
    pool: &Pool<Postgres>,
    pubkey: &str,
    metric: &str,
    units: i64,
    request_id: Option<&str>,
) -> ApiResult<()> {
    let plan_id = active_plan_id(pool, pubkey).await?;
    let limit = sqlx::query_scalar::<_, i64>(
        "SELECT \"limit\" FROM cn_user.plan_limits WHERE plan_id = $1 AND metric = $2 AND \"window\" = 'day'",
    )
    .bind(&plan_id)
    .bind(metric)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let day = chrono::Utc::now().date_naive();
    let mut tx = pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    if let Some(request_id) = request_id {
        let outcome = sqlx::query_scalar::<_, String>(
            "SELECT outcome FROM cn_user.usage_events WHERE subscriber_pubkey = $1 AND metric = $2 AND request_id = $3",
        )
        .bind(pubkey)
        .bind(metric)
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
        if let Some(outcome) = outcome {
            if outcome == "rejected" {
                let current = sqlx::query_scalar::<_, i64>(
                    "SELECT count FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1 AND metric = $2 AND day = $3",
                )
                .bind(pubkey)
                .bind(metric)
                .bind(day)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|err| {
                    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string())
                })?
                .unwrap_or(0);
                let limit = limit.unwrap_or(i64::MAX);
                return Err(quota_exceeded_error(metric, current, limit));
            }
            tx.rollback().await.ok();
            return Ok(());
        }
    }

    let current = sqlx::query_scalar::<_, i64>(
        "SELECT count FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1 AND metric = $2 AND day = $3",
    )
    .bind(pubkey)
    .bind(metric)
    .bind(day)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .unwrap_or(0);

    let limit = limit.unwrap_or(i64::MAX);
    if current.saturating_add(units) > limit {
        sqlx::query(
            "INSERT INTO cn_user.usage_events                  (subscriber_pubkey, metric, day, request_id, units, outcome)                  VALUES ($1, $2, $3, $4, $5, 'rejected')",
        )
        .bind(pubkey)
        .bind(metric)
        .bind(day)
        .bind(request_id)
        .bind(units)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
        tx.commit().await.ok();

        metrics::inc_quota_exceeded(crate::SERVICE_NAME, metric);
        return Err(quota_exceeded_error(metric, current, limit));
    }

    sqlx::query(
        "INSERT INTO cn_user.usage_counters_daily          (subscriber_pubkey, metric, day, count)          VALUES ($1, $2, $3, $4)          ON CONFLICT (subscriber_pubkey, metric, day) DO UPDATE SET count = cn_user.usage_counters_daily.count + EXCLUDED.count",
    )
    .bind(pubkey)
    .bind(metric)
    .bind(day)
    .bind(units)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_user.usage_events          (subscriber_pubkey, metric, day, request_id, units, outcome)          VALUES ($1, $2, $3, $4, $5, 'ok')",
    )
    .bind(pubkey)
    .bind(metric)
    .bind(day)
    .bind(request_id)
    .bind(units)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    tx.commit().await.ok();
    Ok(())
}

fn quota_exceeded_error(metric: &str, current: i64, limit: i64) -> ApiError {
    ApiError::new(
        StatusCode::PAYMENT_REQUIRED,
        "QUOTA_EXCEEDED",
        "quota exceeded",
    )
    .with_details(json!({
        "metric": metric,
        "current": current,
        "limit": limit,
        "reset_at": quota_reset_at_timestamp(),
    }))
}

fn quota_reset_at_timestamp() -> i64 {
    let now = chrono::Utc::now();
    now.date_naive()
        .succ_opt()
        .unwrap_or_else(|| now.date_naive())
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| now.date_naive().and_hms_opt(0, 0, 0).unwrap())
        .and_utc()
        .timestamp()
}

async fn active_plan_id(pool: &Pool<Postgres>, pubkey: &str) -> ApiResult<String> {
    let plan_id = sqlx::query_scalar::<_, String>(
        "SELECT plan_id FROM cn_user.subscriptions WHERE subscriber_pubkey = $1 AND status = 'active' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(pubkey)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    Ok(plan_id.unwrap_or_else(|| DEFAULT_PLAN_ID.to_string()))
}
