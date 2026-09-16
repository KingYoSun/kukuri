use async_trait::async_trait;
use kukuri_cn_safety::ScanError;
use kukuri_cn_safety_openai::{BudgetConfig, ModerationBudget};
use sqlx::{PgPool, Row};
use std::time::Duration;

/// Shared by every OpenAI client of this node, including readiness/CLI and restarts.
#[derive(Clone, Debug)]
pub struct PgModerationBudget {
    pool: PgPool,
}
impl PgModerationBudget {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModerationBudget for PgModerationBudget {
    async fn reserve(&self, tokens: u32, limits: &BudgetConfig) -> Result<Duration, ScanError> {
        limits.validate()?;
        if tokens == 0 || tokens > limits.tokens_per_minute {
            return Err(ScanError::Protocol(
                "moderation input exceeds token reservation budget".into(),
            ));
        }
        self.reserve_inner(tokens, limits).await.map_err(|_| {
            ScanError::Unavailable("persistent moderation budget is unavailable".into())
        })
    }
}

impl PgModerationBudget {
    async fn reserve_inner(&self, tokens: u32, limits: &BudgetConfig) -> anyhow::Result<Duration> {
        let mut tx = self.pool.begin().await?;
        // Fixed namespace deliberately spans config/credential rotations. Neither keys nor content
        // are part of the budget identity; changing a model must not reset the day's usage.
        sqlx::query("SELECT pg_advisory_xact_lock(1060,1)")
            .execute(&mut *tx)
            .await?;
        let now: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM cn_safety.moderation_budget_reservations
            WHERE budget_key='openai-moderation' AND admitted_at<=$1::timestamptz-interval '24 hours'")
            .bind(now).execute(&mut *tx).await?;
        let row=sqlx::query("SELECT COUNT(*) AS daily,
            COUNT(*) FILTER (WHERE admitted_at>$1::timestamptz-interval '60 seconds') AS minute,
            COALESCE(SUM(tokens) FILTER (WHERE admitted_at>$1::timestamptz-interval '60 seconds'),0)::bigint AS tokens,
            EXTRACT(EPOCH FROM (MIN(admitted_at)+interval '24 hours'-$1::timestamptz))::double precision AS day_wait,
            EXTRACT(EPOCH FROM ((MIN(admitted_at) FILTER (WHERE admitted_at>$1::timestamptz-interval '60 seconds'))+interval '60 seconds'-$1::timestamptz))::double precision AS minute_wait
            FROM cn_safety.moderation_budget_reservations WHERE budget_key='openai-moderation'")
            .bind(now).fetch_one(&mut *tx).await?;
        let wait = if row.try_get::<i64, _>("daily")? >= i64::from(limits.requests_per_day) {
            row.try_get::<Option<f64>, _>("day_wait")?
                .unwrap_or(86400.0)
        } else if row.try_get::<i64, _>("minute")? >= i64::from(limits.requests_per_minute)
            || row.try_get::<i64, _>("tokens")? + i64::from(tokens)
                > i64::from(limits.tokens_per_minute)
        {
            row.try_get::<Option<f64>, _>("minute_wait")?
                .unwrap_or(60.0)
        } else {
            0.0
        };
        if wait > 0.0 {
            tx.commit().await?;
            return Ok(Duration::from_secs_f64(wait.max(0.001)));
        }
        sqlx::query("INSERT INTO cn_safety.moderation_budget_reservations(budget_key,tokens,admitted_at) VALUES('openai-moderation',$1,$2)")
            .bind(tokens as i32).bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Duration::ZERO)
    }
}
