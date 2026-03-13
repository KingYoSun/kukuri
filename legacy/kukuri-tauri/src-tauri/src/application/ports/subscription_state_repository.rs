use crate::domain::value_objects::subscription::{SubscriptionRecord, SubscriptionTarget};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait SubscriptionStateRepository: Send + Sync {
    async fn upsert(&self, record: &SubscriptionRecord) -> Result<SubscriptionRecord, AppError>;

    async fn find(
        &self,
        target: &SubscriptionTarget,
    ) -> Result<Option<SubscriptionRecord>, AppError>;

    async fn mark_all_need_resync(&self, updated_at_ms: i64) -> Result<(), AppError>;

    async fn list_for_restore(&self) -> Result<Vec<SubscriptionRecord>, AppError>;

    async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError>;
}
