use crate::application::ports::event_gateway::EventGateway;
use crate::application::shared::mappers::{domain_event_to_nostr_event, profile_metadata_to_nostr};
use crate::domain::entities::event_gateway::{DomainEvent, ProfileMetadata};
use crate::domain::value_objects::event_gateway::{PublicKey, ReactionValue, TopicContent};
use crate::domain::value_objects::{EventId, TopicId};
use crate::infrastructure::event::manager_handle::EventManagerHandle;
use crate::shared::error::AppError;
use async_trait::async_trait;
use nostr_sdk::prelude::{Event as NostrEvent, EventId as NostrEventId};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tracing::error;

pub struct LegacyEventManagerGateway {
    manager: Arc<dyn EventManagerHandle>,
    app_handle: Arc<RwLock<Option<AppHandle>>>,
}

#[derive(Debug, Clone, Serialize)]
struct FrontendEventPayload {
    id: String,
    author: String,
    content: String,
    created_at: u64,
    kind: u32,
    tags: Vec<Vec<String>>,
}

impl From<&NostrEvent> for FrontendEventPayload {
    fn from(event: &NostrEvent) -> Self {
        Self {
            id: event.id.to_string(),
            author: event.pubkey.to_string(),
            content: event.content.clone(),
            created_at: event.created_at.as_u64(),
            kind: event.kind.as_u16() as u32,
            tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::EventKind;
    use crate::domain::entities::event_gateway::EventTag;
    use crate::domain::value_objects::EventId;
    use crate::infrastructure::event::manager_handle::LegacyEventManagerHandle;
    use chrono::Utc;

    fn repeating_hex(ch: char, len: usize) -> String {
        std::iter::repeat(ch).take(len).collect()
    }

    fn sample_domain_event() -> DomainEvent {
        let event_id = EventId::from_hex(&repeating_hex('a', 64)).expect("valid event id");
        let public_key = PublicKey::from_hex_str(&repeating_hex('b', 64)).expect("valid pubkey");
        let tags = vec![
            EventTag::new("p", vec![repeating_hex('c', 64)]).expect("valid p tag"),
            EventTag::new("t", vec!["sample".to_string()]).expect("valid t tag"),
        ];
        DomainEvent::new(
            event_id,
            public_key,
            EventKind::TextNote,
            Utc::now(),
            "sample content".to_string(),
            tags,
            repeating_hex('d', 128),
        )
        .expect("valid domain event")
    }

    #[tokio::test]
    async fn handle_incoming_event_without_app_handle_succeeds() {
        let manager: Arc<dyn EventManagerHandle> = Arc::new(LegacyEventManagerHandle::new());
        let gateway = LegacyEventManagerGateway::new(manager);
        let event = sample_domain_event();

        let result = gateway.handle_incoming_event(event).await;
        assert!(result.is_ok());
    }

    #[test]
    fn frontend_payload_matches_nostr_event() {
        let event = sample_domain_event();
        let nostr_event = domain_event_to_nostr_event(&event).expect("domain to nostr");
        let payload = FrontendEventPayload::from(&nostr_event);

        assert_eq!(payload.id, nostr_event.id.to_string());
        assert_eq!(payload.author, nostr_event.pubkey.to_string());
        assert_eq!(payload.content, nostr_event.content);
        assert_eq!(payload.kind, nostr_event.kind.as_u16() as u32);
        assert_eq!(payload.tags.len(), nostr_event.tags.len());
    }
}

impl LegacyEventManagerGateway {
    pub fn new(manager: Arc<dyn EventManagerHandle>) -> Self {
        Self {
            manager,
            app_handle: Arc::new(RwLock::new(None)),
        }
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

    pub async fn set_app_handle(&self, handle: AppHandle) {
        let mut guard = self.app_handle.write().await;
        *guard = Some(handle);
    }

    #[allow(dead_code)]
    pub async fn clear_app_handle(&self) {
        let mut guard = self.app_handle.write().await;
        *guard = None;
    }

    async fn emit_frontend_event(&self, event: &NostrEvent) {
        let handle = self.app_handle.read().await.clone();
        if let Some(handle) = handle {
            let payload = FrontendEventPayload::from(event);

            if let Err(err) = handle.emit("nostr://event/p2p", payload) {
                error!("Failed to emit nostr event to frontend: {}", err);
            }
        }
    }
}

#[async_trait]
impl EventGateway for LegacyEventManagerGateway {
    async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError> {
        let nostr_event = domain_event_to_nostr_event(&event)?;
        self.manager
            .handle_p2p_event(nostr_event.clone())
            .await
            .map_err(|err| AppError::NostrError(err.to_string()))?;
        self.emit_frontend_event(&nostr_event).await;
        Ok(())
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
