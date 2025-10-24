use crate::application::ports::event_gateway::EventGateway;
use crate::application::shared::mappers::{domain_event_to_nostr_event, profile_metadata_to_nostr};
use crate::domain::entities::event_gateway::{DomainEvent, ProfileMetadata};
use crate::domain::value_objects::event_gateway::{PublicKey, ReactionValue, TopicContent};
use crate::domain::value_objects::{EventId, TopicId};
use crate::modules::event::manager::EventManager;
use crate::shared::error::AppError;
use async_trait::async_trait;
use nostr_sdk::prelude::EventId as NostrEventId;
use std::sync::Arc;

pub struct LegacyEventManagerGateway {
    manager: Arc<EventManager>,
}

impl LegacyEventManagerGateway {
    pub fn new(manager: Arc<EventManager>) -> Self {
        Self { manager }
    }

    fn to_nostr_event_id(event_id: &EventId) -> Result<NostrEventId, AppError> {
        NostrEventId::from_hex(event_id.as_str())
            .map_err(|err| AppError::NostrError(err.to_string()))
    }

    fn to_domain_event_id(event_id: NostrEventId) -> Result<EventId, AppError> {
        EventId::from_hex(&event_id.to_hex())
            .map_err(|err| AppError::ValidationError(format!("Invalid event ID returned: {err}")))
    }

    fn to_domain_public_key(pk: nostr_sdk::prelude::PublicKey) -> Result<PublicKey, AppError> {
        PublicKey::from_hex_str(&pk.to_hex())
            .map_err(|err| AppError::ValidationError(format!("Invalid public key: {err}")))
    }
}

#[async_trait]
impl EventGateway for LegacyEventManagerGateway {
    async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError> {
        let nostr_event = domain_event_to_nostr_event(&event)?;
        self.manager
            .handle_p2p_event(nostr_event)
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))
    }

    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError> {
        let event_id = self
            .manager
            .publish_text_note(content)
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))?;
        Self::to_domain_event_id(event_id)
    }

    async fn publish_topic_post(
        &self,
        topic_id: &TopicId,
        content: &TopicContent,
        reply_to: Option<&EventId>,
    ) -> Result<EventId, AppError> {
        let reply_to_converted = if let Some(reply) = reply_to {
            Some(Self::to_nostr_event_id(reply)?)
        } else {
            None
        };

        let event_id = self
            .manager
            .publish_topic_post(topic_id.as_str(), content.as_str(), reply_to_converted)
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))?;
        Self::to_domain_event_id(event_id)
    }

    async fn send_reaction(
        &self,
        target: &EventId,
        reaction: &ReactionValue,
    ) -> Result<EventId, AppError> {
        let nostr_event_id = Self::to_nostr_event_id(target)?;
        let event_id = self
            .manager
            .send_reaction(&nostr_event_id, reaction.as_str())
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))?;
        Self::to_domain_event_id(event_id)
    }

    async fn update_profile_metadata(
        &self,
        metadata: &ProfileMetadata,
    ) -> Result<EventId, AppError> {
        let nostr_metadata = profile_metadata_to_nostr(metadata)?;
        let event_id = self
            .manager
            .update_metadata(nostr_metadata)
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))?;
        Self::to_domain_event_id(event_id)
    }

    async fn delete_events(
        &self,
        targets: &[EventId],
        reason: Option<&str>,
    ) -> Result<EventId, AppError> {
        let nostr_ids = targets
            .iter()
            .map(Self::to_nostr_event_id)
            .collect::<Result<Vec<_>, _>>()?;
        let event_id = self
            .manager
            .delete_events(nostr_ids, reason.map(|value| value.to_string()))
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))?;
        Self::to_domain_event_id(event_id)
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        self.manager
            .disconnect()
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))
    }

    async fn get_public_key(&self) -> Result<Option<PublicKey>, AppError> {
        let maybe_pk = self.manager.get_public_key().await;
        maybe_pk.map(Self::to_domain_public_key).transpose()
    }

    async fn set_default_topics(&self, topics: &[TopicId]) -> Result<(), AppError> {
        let topic_strings: Vec<String> = topics
            .iter()
            .map(|topic| topic.as_str().to_string())
            .collect();
        self.manager.set_default_p2p_topics(topic_strings).await;
        Ok(())
    }

    async fn list_default_topics(&self) -> Result<Vec<TopicId>, AppError> {
        let topics = self.manager.list_default_p2p_topics().await;
        topics
            .into_iter()
            .map(|t| {
                TopicId::new(t).map_err(|err| {
                    AppError::ValidationError(format!("Invalid topic identifier returned: {err}"))
                })
            })
            .collect()
    }
}
