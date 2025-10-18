use crate::infrastructure::database::connection_pool::ConnectionPool;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;

const RESYNC_BACKOFF_SECS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionTarget {
    Topic(String),
    User(String),
}

impl SubscriptionTarget {
    pub fn as_parts(&self) -> (&str, &str) {
        match self {
            SubscriptionTarget::Topic(id) => ("topic", id.as_str()),
            SubscriptionTarget::User(id) => ("user", id.as_str()),
        }
    }

    pub fn from_parts(target_type: &str, target: String) -> Result<Self, AppError> {
        match target_type {
            "topic" => Ok(SubscriptionTarget::Topic(target)),
            "user" => Ok(SubscriptionTarget::User(target)),
            other => Err(AppError::ValidationError(format!(
                "Unknown subscription target type: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Pending,
    Subscribed,
    NeedsResync,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::Pending => "pending",
            SubscriptionStatus::Subscribed => "subscribed",
            SubscriptionStatus::NeedsResync => "needs_resync",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(SubscriptionStatus::Pending),
            "subscribed" => Some(SubscriptionStatus::Subscribed),
            "needs_resync" => Some(SubscriptionStatus::NeedsResync),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubscriptionRecord {
    pub target: SubscriptionTarget,
    pub status: SubscriptionStatus,
    pub last_synced_at: Option<i64>,
    pub last_attempt_at: Option<i64>,
    pub failure_count: i64,
    pub error_message: Option<String>,
}

impl SubscriptionRecord {
    pub fn since_timestamp(&self) -> Option<nostr_sdk::prelude::Timestamp> {
        let last_synced = self.last_synced_at?;
        let adjusted = last_synced.saturating_sub(RESYNC_BACKOFF_SECS);
        Some(nostr_sdk::prelude::Timestamp::from(adjusted as u64))
    }
}

#[async_trait]
pub trait SubscriptionStateStore: Send + Sync {
    async fn record_request(
        &self,
        target: SubscriptionTarget,
    ) -> Result<SubscriptionRecord, AppError>;

    async fn mark_subscribed(
        &self,
        target: &SubscriptionTarget,
        synced_at: i64,
    ) -> Result<(), AppError>;

    async fn mark_failure(&self, target: &SubscriptionTarget, error: &str) -> Result<(), AppError>;

    async fn mark_all_need_resync(&self) -> Result<(), AppError>;

    async fn list_for_restore(&self) -> Result<Vec<SubscriptionRecord>, AppError>;

    async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError>;
}

#[derive(Clone)]
pub struct SubscriptionStateMachine {
    pool: ConnectionPool,
}

impl SubscriptionStateMachine {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    async fn fetch_record(
        &self,
        target_type: &str,
        target: &str,
    ) -> Result<SubscriptionRecord, AppError> {
        let row = sqlx::query(
            r#"
            SELECT target, target_type, status, last_synced_at, last_attempt_at, failure_count, error_message
            FROM nostr_subscriptions
            WHERE target_type = ? AND target = ?
            "#,
        )
        .bind(target_type)
        .bind(target)
        .fetch_one(self.pool.get_pool())
        .await?;

        self.row_to_record(row)
    }

    fn row_to_record(&self, row: sqlx::sqlite::SqliteRow) -> Result<SubscriptionRecord, AppError> {
        let target: String = row.try_get("target")?;
        let target_type: String = row.try_get("target_type")?;
        let status_str: String = row.try_get("status")?;
        let last_synced_at: Option<i64> = row.try_get("last_synced_at")?;
        let last_attempt_at: Option<i64> = row.try_get("last_attempt_at")?;
        let failure_count: i64 = row.try_get("failure_count")?;
        let error_message: Option<String> = row.try_get("error_message")?;

        let status = SubscriptionStatus::from_str(&status_str).ok_or_else(|| {
            AppError::ValidationError(format!("Unknown subscription status: {status_str}"))
        })?;

        let target = SubscriptionTarget::from_parts(&target_type, target)?;

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
impl SubscriptionStateStore for SubscriptionStateMachine {
    async fn record_request(
        &self,
        target: SubscriptionTarget,
    ) -> Result<SubscriptionRecord, AppError> {
        let (target_type, target_value) = target.as_parts();
        let now_ms = Utc::now().timestamp_millis();
        let now_secs = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO nostr_subscriptions (
                target, target_type, status, last_synced_at, last_attempt_at,
                failure_count, error_message, created_at, updated_at
            )
            VALUES (?, ?, 'pending', NULL, ?, 0, NULL, ?, ?)
            "#,
        )
        .bind(target_value)
        .bind(target_type)
        .bind(now_secs)
        .bind(now_ms)
        .bind(now_ms)
        .execute(self.pool.get_pool())
        .await?;

        sqlx::query(
            r#"
            UPDATE nostr_subscriptions
            SET status = 'pending',
                last_attempt_at = ?,
                updated_at = ?,
                error_message = NULL
            WHERE target_type = ? AND target = ?
            "#,
        )
        .bind(now_secs)
        .bind(now_ms)
        .bind(target_type)
        .bind(target_value)
        .execute(self.pool.get_pool())
        .await?;

        self.fetch_record(target_type, target_value).await
    }

    async fn mark_subscribed(
        &self,
        target: &SubscriptionTarget,
        synced_at: i64,
    ) -> Result<(), AppError> {
        let (target_type, target_value) = target.as_parts();
        let now_ms = Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            UPDATE nostr_subscriptions
            SET status = 'subscribed',
                last_synced_at = ?,
                updated_at = ?,
                failure_count = 0,
                error_message = NULL
            WHERE target_type = ? AND target = ?
            "#,
        )
        .bind(synced_at)
        .bind(now_ms)
        .bind(target_type)
        .bind(target_value)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn mark_failure(&self, target: &SubscriptionTarget, error: &str) -> Result<(), AppError> {
        let (target_type, target_value) = target.as_parts();
        let now_ms = Utc::now().timestamp_millis();
        let now_secs = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE nostr_subscriptions
            SET status = 'needs_resync',
                failure_count = failure_count + 1,
                last_attempt_at = ?,
                updated_at = ?,
                error_message = ?
            WHERE target_type = ? AND target = ?
            "#,
        )
        .bind(now_secs)
        .bind(now_ms)
        .bind(error)
        .bind(target_type)
        .bind(target_value)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn mark_all_need_resync(&self) -> Result<(), AppError> {
        let now_ms = Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            UPDATE nostr_subscriptions
            SET status = 'needs_resync',
                updated_at = ?,
                error_message = NULL
            WHERE status = 'subscribed'
            "#,
        )
        .bind(now_ms)
        .execute(self.pool.get_pool())
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
        .fetch_all(self.pool.get_pool())
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_record(row))
            .collect()
    }

    async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT target, target_type, status, last_synced_at, last_attempt_at, failure_count, error_message
            FROM nostr_subscriptions
            ORDER BY target_type ASC, target ASC
            "#,
        )
        .fetch_all(self.pool.get_pool())
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_record(row))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_state_machine() -> SubscriptionStateMachine {
        let pool = ConnectionPool::from_memory().await.unwrap();
        let machine = SubscriptionStateMachine::new(pool.clone());
        sqlx::migrate!("./migrations")
            .run(pool.get_pool())
            .await
            .unwrap();
        machine
    }

    #[tokio::test]
    async fn record_request_inserts_and_updates() {
        let machine = setup_state_machine().await;
        let record = machine
            .record_request(SubscriptionTarget::Topic("test".into()))
            .await
            .unwrap();
        assert_eq!(record.failure_count, 0);
        assert_eq!(record.status, SubscriptionStatus::Pending);

        let record_again = machine
            .record_request(SubscriptionTarget::Topic("test".into()))
            .await
            .unwrap();
        assert_eq!(record_again.status, SubscriptionStatus::Pending);
    }

    #[tokio::test]
    async fn mark_subscribed_updates_status() {
        let machine = setup_state_machine().await;
        let target = SubscriptionTarget::Topic("topic".into());
        machine.record_request(target.clone()).await.unwrap();
        machine.mark_subscribed(&target, 100).await.unwrap();
        let all = machine.list_all().await.unwrap();
        assert_eq!(all[0].status, SubscriptionStatus::Subscribed);
        assert_eq!(all[0].last_synced_at, Some(100));
    }

    #[tokio::test]
    async fn mark_failure_increments_counter() {
        let machine = setup_state_machine().await;
        let target = SubscriptionTarget::Topic("fail_topic".into());
        machine.record_request(target.clone()).await.unwrap();
        machine.mark_failure(&target, "error").await.unwrap();
        let record = machine.fetch_record("topic", "fail_topic").await.unwrap();
        assert_eq!(record.status, SubscriptionStatus::NeedsResync);
        assert_eq!(record.failure_count, 1);
        assert_eq!(record.error_message.as_deref(), Some("error"));
    }

    #[tokio::test]
    async fn mark_all_need_resync_updates_subscribed() {
        let machine = setup_state_machine().await;
        let target = SubscriptionTarget::Topic("resync".into());
        machine.record_request(target.clone()).await.unwrap();
        machine.mark_subscribed(&target, 200).await.unwrap();
        machine.mark_all_need_resync().await.unwrap();
        let record = machine.fetch_record("topic", "resync").await.unwrap();
        assert_eq!(record.status, SubscriptionStatus::NeedsResync);
    }

    #[tokio::test]
    async fn list_for_restore_filters_status() {
        let machine = setup_state_machine().await;
        let pending_target = SubscriptionTarget::Topic("pending".into());
        machine.record_request(pending_target).await.unwrap();
        let subscribed_target = SubscriptionTarget::User("user".into());
        machine
            .record_request(subscribed_target.clone())
            .await
            .unwrap();
        machine
            .mark_subscribed(&subscribed_target, 100)
            .await
            .unwrap();
        machine.mark_all_need_resync().await.unwrap();

        let restore = machine.list_for_restore().await.unwrap();
        assert_eq!(restore.len(), 2);
    }
}
