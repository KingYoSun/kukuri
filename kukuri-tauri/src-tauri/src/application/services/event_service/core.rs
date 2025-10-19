use super::distribution::distribute_hybrid;
use super::factory::{build_deletion_event, to_nostr_event};
use super::invoker::SubscriptionInvoker;
use crate::application::services::{SubscriptionRecord, SubscriptionStateStore};
use crate::domain::entities::{Event, EventKind};
use crate::infrastructure::crypto::SignatureService;
use crate::infrastructure::database::EventRepository;
use crate::infrastructure::p2p::EventDistributor;
use crate::modules::event::manager::EventManager;
use crate::presentation::dto::event::NostrMetadataDto;
use crate::shared::error::AppError;
use async_trait::async_trait;
use nostr_sdk::prelude::*;
use std::sync::Arc;

pub struct EventService {
    pub(crate) repository: Arc<dyn EventRepository>,
    pub(crate) signature_service: Arc<dyn SignatureService>,
    pub(crate) distributor: Arc<dyn EventDistributor>,
    pub(crate) event_manager: Option<Arc<EventManager>>,
    pub(crate) subscription_state: Arc<dyn SubscriptionStateStore>,
    pub(crate) subscription_invoker: Option<Arc<dyn SubscriptionInvoker>>,
}

impl EventService {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        signature_service: Arc<dyn SignatureService>,
        distributor: Arc<dyn EventDistributor>,
        subscription_state: Arc<dyn SubscriptionStateStore>,
    ) -> Self {
        Self {
            repository,
            signature_service,
            distributor,
            event_manager: None,
            subscription_state,
            subscription_invoker: None,
        }
    }

    /// Attach the EventManager used by this service.
    pub fn set_event_manager(&mut self, event_manager: Arc<EventManager>) {
        self.event_manager = Some(event_manager);
    }

    /// Attach the subscription invoker used to execute subscriptions.
    pub fn set_subscription_invoker(&mut self, invoker: Arc<dyn SubscriptionInvoker>) {
        self.subscription_invoker = Some(invoker);
    }

    pub(crate) fn event_manager(&self) -> Result<&Arc<EventManager>, AppError> {
        self.event_manager
            .as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))
    }

    pub(crate) fn subscription_invoker(&self) -> Result<&Arc<dyn SubscriptionInvoker>, AppError> {
        self.subscription_invoker
            .as_ref()
            .ok_or_else(|| AppError::ConfigurationError("Subscription invoker not set".to_string()))
    }

    pub async fn create_event(
        &self,
        kind: u32,
        content: String,
        pubkey: String,
        private_key: &str,
    ) -> Result<Event, AppError> {
        let mut event = Event::new(kind, content, pubkey);

        self.signature_service
            .sign_event(&mut event, private_key)
            .await?;

        self.repository.create_event(&event).await?;
        distribute_hybrid(&self.distributor, &event).await?;

        Ok(event)
    }

    pub async fn process_received_event(&self, event: Event) -> Result<(), AppError> {
        if !self.signature_service.verify_event(&event).await? {
            return Err("Invalid event signature".into());
        }

        self.repository.create_event(&event).await?;

        if let Some(event_manager) = &self.event_manager {
            if matches!(
                EventKind::from_u32(event.kind),
                Some(EventKind::TextNote)
                    | Some(EventKind::Metadata)
                    | Some(EventKind::Reaction)
                    | Some(EventKind::Repost)
            ) {
                let nostr_event = to_nostr_event(&event)?;
                event_manager
                    .handle_p2p_event(nostr_event)
                    .await
                    .map_err(|e| AppError::NostrError(e.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError> {
        self.repository.get_event(id).await
    }

    pub async fn get_events_by_kind(
        &self,
        kind: u32,
        limit: usize,
    ) -> Result<Vec<Event>, AppError> {
        self.repository.get_events_by_kind(kind, limit).await
    }

    pub async fn get_events_by_author(
        &self,
        pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Event>, AppError> {
        self.repository.get_events_by_author(pubkey, limit).await
    }

    pub async fn delete_event(
        &self,
        id: &str,
        pubkey: String,
        private_key: &str,
    ) -> Result<(), AppError> {
        let mut deletion_event = build_deletion_event(id, pubkey);

        self.signature_service
            .sign_event(&mut deletion_event, private_key)
            .await?;
        self.repository.create_event(&deletion_event).await?;
        distribute_hybrid(&self.distributor, &deletion_event).await?;

        self.repository.delete_event(id).await
    }

    pub async fn sync_pending_events(&self) -> Result<u32, AppError> {
        let unsync_events = self.repository.get_unsync_events().await?;
        let mut synced_count = 0;

        for event in unsync_events {
            distribute_hybrid(&self.distributor, &event).await?;
            self.repository.mark_event_synced(&event.id).await?;
            synced_count += 1;
        }

        Ok(synced_count)
    }
}

#[async_trait]
pub trait EventServiceTrait: Send + Sync {
    async fn initialize(&self) -> Result<(), AppError>;
    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError>;
    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<EventId, AppError>;
    async fn send_reaction(&self, event_id: &str, reaction: &str) -> Result<EventId, AppError>;
    async fn update_metadata(&self, metadata: NostrMetadataDto) -> Result<EventId, AppError>;
    async fn subscribe_to_topic(&self, topic_id: &str) -> Result<(), AppError>;
    async fn subscribe_to_user(&self, pubkey: &str) -> Result<(), AppError>;
    async fn get_public_key(&self) -> Result<Option<String>, AppError>;
    async fn delete_events(
        &self,
        event_ids: Vec<String>,
        reason: Option<String>,
    ) -> Result<EventId, AppError>;
    async fn disconnect(&self) -> Result<(), AppError>;
    async fn set_default_p2p_topic(&self, topic_id: &str) -> Result<(), AppError>;
    async fn list_subscriptions(&self) -> Result<Vec<SubscriptionRecord>, AppError>;
}

#[async_trait]
impl EventServiceTrait for EventService {
    async fn initialize(&self) -> Result<(), AppError> {
        if self.event_manager.is_none() {
            Err(AppError::ConfigurationError(
                "EventManager not set".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError> {
        let event_manager = self.event_manager()?;

        event_manager
            .publish_text_note(content)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<EventId, AppError> {
        let event_manager = self.event_manager()?;

        let reply_to_id = if let Some(reply_id) = reply_to {
            Some(EventId::from_hex(reply_id).map_err(|e| AppError::NostrError(e.to_string()))?)
        } else {
            None
        };

        event_manager
            .publish_topic_post(topic_id, content, reply_to_id)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn send_reaction(&self, event_id: &str, reaction: &str) -> Result<EventId, AppError> {
        let event_manager = self.event_manager()?;

        let event_id =
            EventId::from_hex(event_id).map_err(|e| AppError::NostrError(e.to_string()))?;

        event_manager
            .send_reaction(&event_id, reaction)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn update_metadata(&self, metadata: NostrMetadataDto) -> Result<EventId, AppError> {
        let event_manager = self.event_manager()?;

        let mut nostr_metadata = Metadata::new();
        if let Some(name) = metadata.name {
            nostr_metadata = nostr_metadata.name(name);
        }
        if let Some(display_name) = metadata.display_name {
            nostr_metadata = nostr_metadata.display_name(display_name);
        }
        if let Some(about) = metadata.about {
            nostr_metadata = nostr_metadata.about(about);
        }
        if let Some(picture) = metadata.picture {
            if let Ok(pic_url) = picture.parse() {
                nostr_metadata = nostr_metadata.picture(pic_url);
            }
        }
        if let Some(website) = metadata.website {
            nostr_metadata = nostr_metadata.website(
                website
                    .parse()
                    .map_err(|_| AppError::ValidationError("Invalid website URL".to_string()))?,
            );
        }

        event_manager
            .update_metadata(nostr_metadata)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn subscribe_to_topic(&self, topic_id: &str) -> Result<(), AppError> {
        super::subscription::subscribe_to_topic_internal(self, topic_id).await
    }

    async fn subscribe_to_user(&self, pubkey: &str) -> Result<(), AppError> {
        super::subscription::subscribe_to_user_internal(self, pubkey).await
    }

    async fn get_public_key(&self) -> Result<Option<String>, AppError> {
        let event_manager = self.event_manager()?;

        let public_key = event_manager.get_public_key().await;
        Ok(public_key.map(|pk| pk.to_hex()))
    }

    async fn delete_events(
        &self,
        event_ids: Vec<String>,
        reason: Option<String>,
    ) -> Result<EventId, AppError> {
        if event_ids.is_empty() {
            return Err(AppError::ValidationError(
                "No event IDs provided".to_string(),
            ));
        }

        let event_manager = self.event_manager()?;

        let parsed_ids = event_ids
            .iter()
            .map(|id| {
                EventId::from_hex(id)
                    .map_err(|e| AppError::ValidationError(format!("Invalid event ID: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let deletion_event_id = event_manager
            .delete_events(parsed_ids, reason)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))?;

        for event_id in event_ids {
            self.repository.delete_event(&event_id).await?;
        }

        Ok(deletion_event_id)
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        let event_manager = self.event_manager()?;

        event_manager
            .disconnect()
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn set_default_p2p_topic(&self, topic_id: &str) -> Result<(), AppError> {
        let event_manager = self.event_manager()?;
        if topic_id.is_empty() {
            return Err(AppError::ValidationError(
                "Topic ID is required".to_string(),
            ));
        }
        event_manager
            .set_default_p2p_topic_id(topic_id.to_string())
            .await;
        Ok(())
    }

    async fn list_subscriptions(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        super::subscription::list_subscriptions_internal(self).await
    }
}
