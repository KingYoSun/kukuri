use crate::application::ports::subscription_state_repository::SubscriptionStateRepository;
use crate::domain::value_objects::subscription::{
    SubscriptionRecord, SubscriptionStatus, SubscriptionTarget,
};
use crate::infrastructure::database::connection_pool::ConnectionPool;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};

#[derive(Clone)]
pub struct SqliteSubscriptionStateRepository {
    pool: ConnectionPool,
}

impl SqliteSubscriptionStateRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    fn pool(&self) -> &SqlitePool {
        self.pool.get_pool()
    }

    fn row_to_record(row: SqliteRow) -> Result<SubscriptionRecord, AppError> {
        let target_value: String = row.get("target");
        let target_type: String = row.get("target_type");
        let status: String = row.get("status");
        let last_synced_at: Option<i64> = row.get("last_synced_at");
        let last_attempt_at: Option<i64> = row.get("last_attempt_at");
        let failure_count: i64 = row.get("failure_count");
        let error_message: Option<String> = row.get("error_message");

        let target = SubscriptionTarget::from_parts(&target_type, target_value)?;
        let status = SubscriptionStatus::parse(&status)?;

        Ok(SubscriptionRecord {
            target,
            status,
            last_synced_at,
            last_attempt_at,
            failure_count,
            error_message,
        })
    }
}

#[async_trait]
impl SubscriptionStateRepository for SqliteSubscriptionStateRepository {
    async fn upsert(&self, record: &SubscriptionRecord) -> Result<SubscriptionRecord, AppError> {
        let (target_type, target_value) = record.target.as_parts();
        let now_ms = Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            INSERT INTO nostr_subscriptions (
                target,
                target_type,
                status,
                last_synced_at,
                last_attempt_at,
                failure_count,
                error_message,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ON CONFLICT(target, target_type) DO UPDATE SET
                status = excluded.status,
                last_synced_at = excluded.last_synced_at,
                last_attempt_at = excluded.last_attempt_at,
                failure_count = excluded.failure_count,
                error_message = excluded.error_message,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(target_value)
        .bind(target_type)
        .bind(record.status.as_str())
        .bind(record.last_synced_at)
        .bind(record.last_attempt_at)
        .bind(record.failure_count)
        .bind(record.error_message.clone())
        .bind(now_ms)
        .execute(self.pool())
        .await?;

        self.find(&record.target)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription record missing after upsert".into()))
    }

    async fn find(
        &self,
        target: &SubscriptionTarget,
    ) -> Result<Option<SubscriptionRecord>, AppError> {
        let (target_type, target_value) = target.as_parts();

        let row = sqlx::query(
            r#"
            SELECT target, target_type, status, last_synced_at, last_attempt_at, failure_count, error_message
            FROM nostr_subscriptions
            WHERE target_type = ?1 AND target = ?2
            "#,
        )
        .bind(target_type)
        .bind(target_value)
        .fetch_optional(self.pool())
        .await?;

        match row {
            Some(row) => Ok(Some(Self::row_to_record(row)?)),
            None => Ok(None),
        }
    }

    async fn mark_all_need_resync(&self, updated_at_ms: i64) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE nostr_subscriptions
            SET status = 'needs_resync',
                updated_at = ?1,
                error_message = NULL
            WHERE status = 'subscribed'
            "#,
        )
        .bind(updated_at_ms)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn list_for_restore(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT target, target_type, status, last_synced_at, last_attempt_at, failure_count, error_message
            FROM nostr_subscriptions
            WHERE status IN ('pending', 'needs_resync')
            ORDER BY updated_at ASC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(Self::row_to_record).collect()
    }

    async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT target, target_type, status, last_synced_at, last_attempt_at, failure_count, error_message
            FROM nostr_subscriptions
            ORDER BY target_type ASC, target ASC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(Self::row_to_record).collect()
    }
}
