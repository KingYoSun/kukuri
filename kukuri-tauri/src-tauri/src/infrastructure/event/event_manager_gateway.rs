use crate::application::ports::event_gateway::EventGateway;
#[cfg(test)]
use crate::application::ports::key_manager::KeyPair;
use crate::application::shared::mappers::{domain_event_to_nostr_event, profile_metadata_to_nostr};
use crate::domain::entities::event_gateway::{DomainEvent, ProfileMetadata};
use crate::domain::value_objects::event_gateway::{PublicKey, ReactionValue, TopicContent};
use crate::domain::value_objects::{EventId, TopicId};
use crate::infrastructure::event::manager_handle::EventManagerHandle;
use crate::infrastructure::event::metrics::{self, GatewayMetricKind};
use crate::infrastructure::p2p::metrics as p2p_metrics;
use crate::shared::{AppError, ValidationFailureKind};
use async_trait::async_trait;
use nostr_sdk::prelude::{Event as NostrEvent, EventId as NostrEventId};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tracing::error;

pub struct LegacyEventManagerGateway {
    manager: Arc<dyn EventManagerHandle>,
    app_handle: Arc<RwLock<Option<AppHandle>>>,
}

const VALIDATION_LOG_WINDOW: Duration = Duration::from_secs(60);
const VALIDATION_WARN_THRESHOLD: u32 = 3;

struct ValidationLogWindow {
    window_start: Instant,
    count: u32,
}

static VALIDATION_LOG_WINDOWS: Lazy<Mutex<HashMap<ValidationFailureKind, ValidationLogWindow>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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
            created_at: event.created_at.as_secs(),
            kind: event.kind.as_u16() as u32,
            tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::event_topic_store::EventTopicStore;
    use crate::domain::entities::EventKind;
    use crate::domain::entities::event_gateway::EventTag;
    use crate::domain::value_objects::EventId;
    use crate::infrastructure::event::manager_handle::LegacyEventManagerHandle;
    use crate::infrastructure::event::metrics;
    use crate::infrastructure::p2p::GossipService;
    use anyhow::{Result as AnyResult, anyhow};
    use chrono::Utc;
    use nostr_sdk::Timestamp;
    use nostr_sdk::prelude::{Event as NostrEvent, EventId as NostrEventId, Metadata};

    #[derive(Default)]
    struct TestEventManagerHandle {
        fail_handle_event: bool,
        fail_publish_text: bool,
    }

    impl TestEventManagerHandle {
        fn with_handle_failure() -> Self {
            Self {
                fail_handle_event: true,
                fail_publish_text: false,
            }
        }

        fn with_publish_failure() -> Self {
            Self {
                fail_handle_event: false,
                fail_publish_text: true,
            }
        }
    }

    #[async_trait]
    impl EventManagerHandle for TestEventManagerHandle {
        async fn set_gossip_service(&self, _: Arc<dyn GossipService>) {}

        async fn set_event_topic_store(&self, _: Arc<dyn EventTopicStore>) {}

        async fn set_default_p2p_topic_id(&self, _: &str) {}

        async fn set_default_p2p_topics(&self, _: Vec<String>) {}

        async fn list_default_p2p_topics(&self) -> Vec<String> {
            vec![]
        }

        async fn handle_p2p_event(&self, _: NostrEvent) -> AnyResult<()> {
            if self.fail_handle_event {
                Err(anyhow!("forced incoming failure"))
            } else {
                Ok(())
            }
        }

        async fn publish_text_note(&self, _: &str) -> AnyResult<NostrEventId> {
            if self.fail_publish_text {
                Err(anyhow!("forced publish failure"))
            } else {
                Ok(sample_nostr_event_id('1'))
            }
        }

        async fn publish_topic_post(
            &self,
            _: &str,
            _: &str,
            _: Option<NostrEventId>,
        ) -> AnyResult<NostrEventId> {
            Ok(sample_nostr_event_id('2'))
        }

        async fn publish_repost(&self, _: &NostrEventId) -> AnyResult<NostrEventId> {
            Ok(sample_nostr_event_id('4'))
        }

        async fn publish_event(&self, _: NostrEvent) -> AnyResult<NostrEventId> {
            Ok(sample_nostr_event_id('6'))
        }

        async fn send_reaction(&self, _: &NostrEventId, _: &str) -> AnyResult<NostrEventId> {
            Ok(sample_nostr_event_id('3'))
        }

        async fn update_metadata(&self, _: Metadata) -> AnyResult<NostrEventId> {
            Ok(sample_nostr_event_id('4'))
        }

        async fn delete_events(
            &self,
            _: Vec<NostrEventId>,
            _: Option<String>,
        ) -> AnyResult<NostrEventId> {
            Ok(sample_nostr_event_id('5'))
        }

        async fn disconnect(&self) -> AnyResult<()> {
            Ok(())
        }

        async fn get_public_key(&self) -> Option<nostr_sdk::prelude::PublicKey> {
            None
        }

        async fn subscribe_to_topic(&self, _: &str, _: Option<Timestamp>) -> AnyResult<()> {
            Ok(())
        }

        async fn subscribe_to_user(
            &self,
            _: nostr_sdk::prelude::PublicKey,
            _: Option<Timestamp>,
        ) -> AnyResult<()> {
            Ok(())
        }

        async fn register_event_callback(&self, _: Arc<dyn Fn(NostrEvent) + Send + Sync>) {}

        async fn initialize_with_keypair(&self, _: KeyPair) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn sample_nostr_event_id(ch: char) -> NostrEventId {
        let hex: String = std::iter::repeat(ch).take(64).collect();
        NostrEventId::from_hex(&hex).expect("valid nostr id")
    }

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

    #[tokio::test]
    async fn gateway_metrics_record_incoming_success() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let before = metrics::snapshot();
        let manager: Arc<dyn EventManagerHandle> = Arc::new(TestEventManagerHandle::default());
        let gateway = LegacyEventManagerGateway::new(manager);
        gateway
            .handle_incoming_event(sample_domain_event())
            .await
            .expect("incoming event succeeds");

        let snapshot = metrics::snapshot();
        assert_eq!(snapshot.incoming.total, before.incoming.total + 1);
        assert_eq!(snapshot.incoming.failures, before.incoming.failures);
    }

    #[tokio::test]
    async fn gateway_metrics_record_incoming_failure() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let before = metrics::snapshot();
        let manager: Arc<dyn EventManagerHandle> =
            Arc::new(TestEventManagerHandle::with_handle_failure());
        let gateway = LegacyEventManagerGateway::new(manager);
        let result = gateway.handle_incoming_event(sample_domain_event()).await;
        assert!(result.is_err());

        let snapshot = metrics::snapshot();
        assert_eq!(snapshot.incoming.failures, before.incoming.failures + 1);
        assert_eq!(snapshot.incoming.total, before.incoming.total);
    }

    #[tokio::test]
    async fn gateway_metrics_record_publish_failure() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let before = metrics::snapshot();
        let manager: Arc<dyn EventManagerHandle> =
            Arc::new(TestEventManagerHandle::with_publish_failure());
        let gateway = LegacyEventManagerGateway::new(manager);

        let result = gateway.publish_text_note("metrics-check").await;
        assert!(result.is_err());

        let snapshot = metrics::snapshot();
        assert_eq!(
            snapshot.publish_text_note.failures,
            before.publish_text_note.failures + 1
        );
        assert_eq!(
            snapshot.publish_text_note.total,
            before.publish_text_note.total
        );
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
        EventId::from_hex(&event_id.to_hex()).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid event ID returned: {err}"),
            )
        })
    }

    fn to_domain_public_key(pk: nostr_sdk::prelude::PublicKey) -> Result<PublicKey, AppError> {
        PublicKey::from_hex_str(&pk.to_hex()).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid public key: {err}"),
            )
        })
    }

    pub async fn set_app_handle(&self, handle: AppHandle) {
        let mut guard = self.app_handle.write().await;
        *guard = Some(handle);
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

fn record_p2p_broadcast_metrics<T>(result: Result<T, AppError>) -> Result<T, AppError> {
    match result {
        Ok(value) => {
            p2p_metrics::record_broadcast_success();
            Ok(value)
        }
        Err(err) => {
            p2p_metrics::record_broadcast_failure();
            Err(err)
        }
    }
}

#[async_trait]
impl EventGateway for LegacyEventManagerGateway {
    async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError> {
        let result = metrics::record_outcome(
            async {
                let nostr_event = domain_event_to_nostr_event(&event)?;
                self.manager
                    .handle_p2p_event(nostr_event.clone())
                    .await
                    .map_err(|err| AppError::NostrError(err.to_string()))?;
                self.emit_frontend_event(&nostr_event).await;
                Ok(())
            }
            .await,
            GatewayMetricKind::Incoming,
        );

        match &result {
            Ok(_) => {
                p2p_metrics::record_receive_success();
            }
            Err(err) => {
                if let Some(kind) = err.validation_kind() {
                    p2p_metrics::record_receive_failure_with_reason(kind);
                    log_validation_failure(&event, kind, err.validation_message());
                } else {
                    p2p_metrics::record_receive_failure();
                }
            }
        }

        result
    }

    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError> {
        record_p2p_broadcast_metrics(metrics::record_outcome(
            async {
                let event_id = self
                    .manager
                    .publish_text_note(content)
                    .await
                    .map_err(|err| AppError::NostrError(err.to_string()))?;
                Self::to_domain_event_id(event_id)
            }
            .await,
            GatewayMetricKind::PublishTextNote,
        ))
    }

    async fn publish_topic_post(
        &self,
        topic_id: &TopicId,
        content: &TopicContent,
        reply_to: Option<&EventId>,
    ) -> Result<EventId, AppError> {
        record_p2p_broadcast_metrics(metrics::record_outcome(
            async {
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
            .await,
            GatewayMetricKind::PublishTopicPost,
        ))
    }

    async fn send_reaction(
        &self,
        target: &EventId,
        reaction: &ReactionValue,
    ) -> Result<EventId, AppError> {
        record_p2p_broadcast_metrics(metrics::record_outcome(
            async {
                let nostr_event_id = Self::to_nostr_event_id(target)?;
                let event_id = self
                    .manager
                    .send_reaction(&nostr_event_id, reaction.as_str())
                    .await
                    .map_err(|err| AppError::NostrError(err.to_string()))?;
                Self::to_domain_event_id(event_id)
            }
            .await,
            GatewayMetricKind::Reaction,
        ))
    }

    async fn update_profile_metadata(
        &self,
        metadata: &ProfileMetadata,
    ) -> Result<EventId, AppError> {
        record_p2p_broadcast_metrics(metrics::record_outcome(
            async {
                let nostr_metadata = profile_metadata_to_nostr(metadata)?;
                let event_id = self
                    .manager
                    .update_metadata(nostr_metadata)
                    .await
                    .map_err(|err| AppError::NostrError(err.to_string()))?;
                Self::to_domain_event_id(event_id)
            }
            .await,
            GatewayMetricKind::MetadataUpdate,
        ))
    }

    async fn delete_events(
        &self,
        targets: &[EventId],
        reason: Option<&str>,
    ) -> Result<EventId, AppError> {
        record_p2p_broadcast_metrics(metrics::record_outcome(
            async {
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
            .await,
            GatewayMetricKind::DeleteEvents,
        ))
    }

    async fn publish_repost(&self, target: &EventId) -> Result<EventId, AppError> {
        record_p2p_broadcast_metrics(metrics::record_outcome(
            async {
                let nostr_id = Self::to_nostr_event_id(target)?;
                let event_id = self
                    .manager
                    .publish_repost(&nostr_id)
                    .await
                    .map_err(|err| AppError::NostrError(err.to_string()))?;
                Self::to_domain_event_id(event_id)
            }
            .await,
            GatewayMetricKind::Repost,
        ))
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        metrics::record_outcome(
            self.manager
                .disconnect()
                .await
                .map_err(|err| AppError::NostrError(err.to_string())),
            GatewayMetricKind::Disconnect,
        )
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
                    AppError::validation(
                        ValidationFailureKind::Generic,
                        format!("Invalid topic identifier returned: {err}"),
                    )
                })
            })
            .collect()
    }
}

fn log_validation_failure(event: &DomainEvent, kind: ValidationFailureKind, message: Option<&str>) {
    if let Ok(mut map) = VALIDATION_LOG_WINDOWS.lock() {
        let entry = map.entry(kind).or_insert(ValidationLogWindow {
            window_start: Instant::now(),
            count: 0,
        });
        if entry.window_start.elapsed() > VALIDATION_LOG_WINDOW {
            entry.window_start = Instant::now();
            entry.count = 0;
        }
        entry.count += 1;
        let log_message = message.unwrap_or("");
        let event_kind = u32::from(event.kind);
        let event_id = event.id.to_hex();
        if entry.count <= VALIDATION_WARN_THRESHOLD {
            tracing::warn!(
                reason = %kind,
                event_id = %event_id,
                event_kind,
                message = log_message,
                "dropped invalid nostr event",
            );
        } else {
            tracing::debug!(
                reason = %kind,
                event_id = %event_id,
                event_kind,
                message = log_message,
                "dropped invalid nostr event",
            );
        }
    }
}
