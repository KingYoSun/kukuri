use super::distribution::distribute_hybrid;
use super::factory::build_deletion_event;
use crate::application::ports::event_gateway::EventGateway;
use crate::application::ports::repositories::EventRepository;
use crate::application::ports::subscription_invoker::SubscriptionInvoker;
use crate::application::services::{SubscriptionRecord, SubscriptionStateStore};
use crate::application::shared::mappers::{
    domain_event_from_event, dto_to_profile_metadata, parse_event_id, parse_event_ids,
    parse_optional_event_id,
};
use crate::domain::entities::{Event, EventKind};
use crate::domain::value_objects::event_gateway::{ReactionValue, TopicContent};
use crate::domain::value_objects::{EventId, TopicId};
use crate::infrastructure::crypto::SignatureService;
use crate::infrastructure::p2p::EventDistributor;
use crate::presentation::dto::event::NostrMetadataDto;
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;

pub struct EventService {
    pub(crate) repository: Arc<dyn EventRepository>,
    pub(crate) signature_service: Arc<dyn SignatureService>,
    pub(crate) distributor: Arc<dyn EventDistributor>,
    pub(crate) event_gateway: Arc<dyn EventGateway>,
    pub(crate) subscription_state: Arc<dyn SubscriptionStateStore>,
    pub(crate) subscription_invoker: Option<Arc<dyn SubscriptionInvoker>>,
}

impl EventService {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        signature_service: Arc<dyn SignatureService>,
        distributor: Arc<dyn EventDistributor>,
        event_gateway: Arc<dyn EventGateway>,
        subscription_state: Arc<dyn SubscriptionStateStore>,
    ) -> Self {
        Self {
            repository,
            signature_service,
            distributor,
            event_gateway,
            subscription_state,
            subscription_invoker: None,
        }
    }

    /// Attach the subscription invoker used to execute subscriptions.
    pub fn set_subscription_invoker(&mut self, invoker: Arc<dyn SubscriptionInvoker>) {
        self.subscription_invoker = Some(invoker);
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

        if matches!(
            EventKind::from_u32(event.kind),
            Some(EventKind::TextNote)
                | Some(EventKind::Metadata)
                | Some(EventKind::Reaction)
                | Some(EventKind::Repost)
        ) {
            let domain_event = domain_event_from_event(&event)?;
            self.event_gateway
                .handle_incoming_event(domain_event)
                .await?;
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
    async fn boost_post(&self, event_id: &str) -> Result<EventId, AppError>;
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
        Ok(())
    }

    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError> {
        self.event_gateway.publish_text_note(content).await
    }

    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<EventId, AppError> {
        let topic = TopicId::new(topic_id.to_string())
            .map_err(|err| AppError::ValidationError(format!("Invalid topic ID: {err}")))?;
        let topic_content = TopicContent::parse(content)
            .map_err(|err| AppError::ValidationError(format!("Invalid topic content: {err}")))?;
        let reply_to_id = parse_optional_event_id(reply_to)?;
        self.event_gateway
            .publish_topic_post(&topic, &topic_content, reply_to_id.as_ref())
            .await
    }

    async fn send_reaction(&self, event_id: &str, reaction: &str) -> Result<EventId, AppError> {
        let event_id = parse_event_id(event_id)?;
        let reaction_value = ReactionValue::parse(reaction)
            .map_err(|err| AppError::ValidationError(format!("Invalid reaction value: {err}")))?;
        self.event_gateway
            .send_reaction(&event_id, &reaction_value)
            .await
    }

    async fn update_metadata(&self, metadata: NostrMetadataDto) -> Result<EventId, AppError> {
        let profile = dto_to_profile_metadata(metadata)?;
        self.event_gateway.update_profile_metadata(&profile).await
    }

    async fn subscribe_to_topic(&self, topic_id: &str) -> Result<(), AppError> {
        super::subscription::subscribe_to_topic_internal(self, topic_id).await
    }

    async fn subscribe_to_user(&self, pubkey: &str) -> Result<(), AppError> {
        super::subscription::subscribe_to_user_internal(self, pubkey).await
    }

    async fn get_public_key(&self) -> Result<Option<String>, AppError> {
        self.event_gateway
            .get_public_key()
            .await
            .map(|key| key.map(|pk| pk.as_hex().to_string()))
    }

    async fn boost_post(&self, event_id: &str) -> Result<EventId, AppError> {
        let target_id = parse_event_id(event_id)?;
        self.event_gateway.publish_repost(&target_id).await
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

        let parsed_ids = parse_event_ids(&event_ids)?;
        let deletion_event_id = self
            .event_gateway
            .delete_events(&parsed_ids, reason.as_deref())
            .await?;

        for event_id in event_ids {
            self.repository.delete_event(&event_id).await?;
        }

        Ok(deletion_event_id)
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        self.event_gateway.disconnect().await
    }

    async fn set_default_p2p_topic(&self, topic_id: &str) -> Result<(), AppError> {
        if topic_id.is_empty() {
            return Err(AppError::ValidationError(
                "Topic ID is required".to_string(),
            ));
        }
        let topic = TopicId::new(topic_id.to_string())
            .map_err(|err| AppError::ValidationError(format!("Invalid topic ID: {err}")))?;
        self.event_gateway
            .set_default_topics(std::slice::from_ref(&topic))
            .await?;
        Ok(())
    }

    async fn list_subscriptions(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        super::subscription::list_subscriptions_internal(self).await
    }
}

#[async_trait]
impl super::super::sync_service::SyncParticipant for EventService {
    async fn sync_pending(&self) -> Result<u32, AppError> {
        self.sync_pending_events().await
    }
}
