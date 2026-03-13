use crate::domain::entities::DirectMessage;
use crate::shared::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait DirectMessageNotifier: Send + Sync {
    async fn notify(&self, owner_npub: &str, message: &DirectMessage) -> Result<(), AppError>;
}
