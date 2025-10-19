use crate::application::services::{SubscriptionRecord, SubscriptionTarget};
use crate::shared::error::AppError;
use chrono::Utc;
use tracing::warn;

use super::EventService;

impl EventService {
    pub async fn handle_network_disconnected(&self) -> Result<(), AppError> {
        self.subscription_state.mark_all_need_resync().await
    }

    pub async fn handle_network_connected(&self) -> Result<(), AppError> {
        self.restore_subscriptions().await
    }

    async fn restore_subscriptions(&self) -> Result<(), AppError> {
        let invoker = self.subscription_invoker()?;

        let records = self.subscription_state.list_for_restore().await?;
        let mut failure_message: Option<String> = None;

        for record in records {
            let target = record.target.clone();
            let since = record.since_timestamp();
            let target_label = match &target {
                SubscriptionTarget::Topic(t) => format!("topic:{t}"),
                SubscriptionTarget::User(u) => format!("user:{u}"),
            };

            let result = match &target {
                SubscriptionTarget::Topic(topic_id) => {
                    invoker.subscribe_topic(topic_id, since).await
                }
                SubscriptionTarget::User(pubkey) => invoker.subscribe_user(pubkey, since).await,
            };

            match result {
                Ok(_) => {
                    self.subscription_state
                        .mark_subscribed(&target, Utc::now().timestamp())
                        .await?;
                }
                Err(err) => {
                    let err_message = err.to_string();
                    if let Err(store_err) = self
                        .subscription_state
                        .mark_failure(&target, &err_message)
                        .await
                    {
                        warn!(
                            "Failed to record subscription failure for {}: {}",
                            target_label, store_err
                        );
                    }
                    warn!(
                        "Failed to restore subscription for {}: {}",
                        target_label, err_message
                    );
                    failure_message = Some(err_message);
                }
            }
        }

        if let Some(message) = failure_message {
            Err(AppError::NostrError(message))
        } else {
            Ok(())
        }
    }
}

pub(crate) async fn subscribe_to_topic_internal(
    service: &EventService,
    topic_id: &str,
) -> Result<(), AppError> {
    if topic_id.is_empty() {
        return Err(AppError::ValidationError(
            "Topic ID is required".to_string(),
        ));
    }

    let invoker = service.subscription_invoker()?;

    let target = SubscriptionTarget::Topic(topic_id.to_string());
    let record = service
        .subscription_state
        .record_request(target.clone())
        .await?;
    let since = record.since_timestamp();

    match invoker.subscribe_topic(topic_id, since).await {
        Ok(_) => {
            service
                .subscription_state
                .mark_subscribed(&target, Utc::now().timestamp())
                .await?;
            Ok(())
        }
        Err(err) => {
            let err_message = err.to_string();
            if let Err(store_err) = service
                .subscription_state
                .mark_failure(&target, &err_message)
                .await
            {
                warn!(
                    "Failed to record subscription failure for topic {}: {}",
                    topic_id, store_err
                );
            }
            Err(err)
        }
    }
}

pub(crate) async fn subscribe_to_user_internal(
    service: &EventService,
    pubkey: &str,
) -> Result<(), AppError> {
    if pubkey.is_empty() {
        return Err(AppError::ValidationError(
            "Public key is required".to_string(),
        ));
    }

    let invoker = service.subscription_invoker()?;

    let target = SubscriptionTarget::User(pubkey.to_string());
    let record = service
        .subscription_state
        .record_request(target.clone())
        .await?;
    let since = record.since_timestamp();

    match invoker.subscribe_user(pubkey, since).await {
        Ok(_) => {
            service
                .subscription_state
                .mark_subscribed(&target, Utc::now().timestamp())
                .await?;
            Ok(())
        }
        Err(err) => {
            let err_message = err.to_string();
            if let Err(store_err) = service
                .subscription_state
                .mark_failure(&target, &err_message)
                .await
            {
                warn!(
                    "Failed to record subscription failure for user {}: {}",
                    pubkey, store_err
                );
            }
            Err(err)
        }
    }
}

pub(crate) async fn list_subscriptions_internal(
    service: &EventService,
) -> Result<Vec<SubscriptionRecord>, AppError> {
    service.subscription_state.list_all().await
}
