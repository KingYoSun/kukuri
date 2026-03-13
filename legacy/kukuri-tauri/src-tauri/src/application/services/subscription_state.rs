use crate::application::ports::subscription_state_repository::SubscriptionStateRepository;
use crate::domain::value_objects::subscription::{SubscriptionRecord, SubscriptionTarget};
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

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
    repository: Arc<dyn SubscriptionStateRepository>,
}

impl SubscriptionStateMachine {
    pub fn new(repository: Arc<dyn SubscriptionStateRepository>) -> Self {
        Self { repository }
    }

    async fn load_or_initialize(
        &self,
        target: &SubscriptionTarget,
    ) -> Result<SubscriptionRecord, AppError> {
        match self.repository.find(target).await? {
            Some(record) => Ok(record),
            None => Ok(SubscriptionRecord::new(target.clone())),
        }
    }
}

#[async_trait]
impl SubscriptionStateStore for SubscriptionStateMachine {
    async fn record_request(
        &self,
        target: SubscriptionTarget,
    ) -> Result<SubscriptionRecord, AppError> {
        let now_secs = Utc::now().timestamp();
        let mut record = self
            .repository
            .find(&target)
            .await?
            .unwrap_or_else(|| SubscriptionRecord::new(target.clone()));

        record.mark_requested(now_secs);

        self.repository.upsert(&record).await
    }

    async fn mark_subscribed(
        &self,
        target: &SubscriptionTarget,
        synced_at: i64,
    ) -> Result<(), AppError> {
        let mut record = self.load_or_initialize(target).await?;
        record.mark_subscribed(synced_at);
        self.repository.upsert(&record).await?;
        Ok(())
    }

    async fn mark_failure(&self, target: &SubscriptionTarget, error: &str) -> Result<(), AppError> {
        let now_secs = Utc::now().timestamp();
        let mut record = self.load_or_initialize(target).await?;
        record.mark_failure(now_secs, error);
        self.repository.upsert(&record).await?;
        Ok(())
    }

    async fn mark_all_need_resync(&self) -> Result<(), AppError> {
        let now_ms = Utc::now().timestamp_millis();
        self.repository.mark_all_need_resync(now_ms).await
    }

    async fn list_for_restore(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        self.repository.list_for_restore().await
    }

    async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        self.repository.list_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::subscription::SubscriptionStatus;
    use crate::infrastructure::database::{
        SqliteSubscriptionStateRepository, connection_pool::ConnectionPool,
    };

    async fn setup_state_machine() -> SubscriptionStateMachine {
        let pool = ConnectionPool::from_memory().await.unwrap();
        sqlx::migrate!("./migrations")
            .run(pool.get_pool())
            .await
            .unwrap();
        let repository = Arc::new(SqliteSubscriptionStateRepository::new(pool));
        SubscriptionStateMachine::new(repository)
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
        let records = machine.list_all().await.unwrap();
        let record = records
            .into_iter()
            .find(|record| matches!(&record.target, SubscriptionTarget::Topic(id) if id == "fail_topic"))
            .unwrap();
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
        let records = machine.list_all().await.unwrap();
        let record = records
            .into_iter()
            .find(
                |record| matches!(&record.target, SubscriptionTarget::Topic(id) if id == "resync"),
            )
            .unwrap();
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
