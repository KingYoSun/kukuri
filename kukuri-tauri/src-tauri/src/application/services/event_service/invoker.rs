use crate::infrastructure::event::EventManagerHandle;
use crate::shared::error::AppError;
use async_trait::async_trait;
use nostr_sdk::prelude::{PublicKey, Timestamp};
use std::sync::Arc;

#[async_trait]
pub trait SubscriptionInvoker: Send + Sync {
    async fn subscribe_topic(
        &self,
        topic_id: &str,
        since: Option<Timestamp>,
    ) -> Result<(), AppError>;

    async fn subscribe_user(&self, pubkey: &str, since: Option<Timestamp>) -> Result<(), AppError>;
}

pub struct EventManagerSubscriptionInvoker {
    event_manager: Arc<dyn EventManagerHandle>,
}

impl EventManagerSubscriptionInvoker {
    pub fn new(event_manager: Arc<dyn EventManagerHandle>) -> Self {
        Self { event_manager }
    }
}

#[async_trait]
impl SubscriptionInvoker for EventManagerSubscriptionInvoker {
    async fn subscribe_topic(
        &self,
        topic_id: &str,
        since: Option<Timestamp>,
    ) -> Result<(), AppError> {
        self.event_manager
            .subscribe_to_topic(topic_id, since)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn subscribe_user(&self, pubkey: &str, since: Option<Timestamp>) -> Result<(), AppError> {
        let public_key =
            PublicKey::from_hex(pubkey).map_err(|e| AppError::NostrError(e.to_string()))?;
        self.event_manager
            .subscribe_to_user(public_key, since)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
}
