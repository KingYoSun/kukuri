use crate::application::ports::subscription_invoker::SubscriptionInvoker;
use crate::infrastructure::event::EventManagerHandle;
use crate::shared::error::AppError;
use async_trait::async_trait;
use nostr_sdk::prelude::{PublicKey, Timestamp};
use std::sync::Arc;

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
            .map_err(|err| AppError::NostrError(err.to_string()))
    }

    async fn subscribe_user(&self, pubkey: &str, since: Option<Timestamp>) -> Result<(), AppError> {
        let public_key = PublicKey::from_hex(pubkey)?;
        self.event_manager
            .subscribe_to_user(public_key, since)
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))
    }
}
