use crate::application::ports::event_topic_store::EventTopicStore;
use crate::infrastructure::database::connection_pool::ConnectionPool;
use crate::infrastructure::event::EventManager;
use crate::infrastructure::p2p::GossipService;
use anyhow::Result;
use async_trait::async_trait;
use nostr_sdk::Timestamp;
use nostr_sdk::prelude::{Event as NostrEvent, EventId as NostrEventId, Metadata, PublicKey};
use std::sync::Arc;

#[async_trait]
pub trait EventManagerHandle: Send + Sync {
    async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>);
    async fn set_event_topic_store(&self, store: Arc<dyn EventTopicStore>);
    async fn set_default_p2p_topic_id(&self, topic_id: &str);
    async fn set_default_p2p_topics(&self, topics: Vec<String>);
    async fn list_default_p2p_topics(&self) -> Vec<String>;
    async fn handle_p2p_event(&self, event: NostrEvent) -> Result<()>;
    async fn publish_text_note(&self, content: &str) -> Result<NostrEventId>;
    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<NostrEventId>,
    ) -> Result<NostrEventId>;
    async fn send_reaction(&self, target: &NostrEventId, reaction: &str) -> Result<NostrEventId>;
    async fn publish_repost(&self, target: &NostrEventId) -> Result<NostrEventId>;
    async fn publish_event(&self, event: NostrEvent) -> Result<NostrEventId>;
    async fn update_metadata(&self, metadata: Metadata) -> Result<NostrEventId>;
    async fn delete_events(
        &self,
        target_ids: Vec<NostrEventId>,
        reason: Option<String>,
    ) -> Result<NostrEventId>;
    async fn disconnect(&self) -> Result<()>;
    async fn get_public_key(&self) -> Option<PublicKey>;
    async fn subscribe_to_topic(&self, topic_id: &str, since: Option<Timestamp>) -> Result<()>;
    async fn subscribe_to_user(&self, pubkey: PublicKey, since: Option<Timestamp>) -> Result<()>;
}

#[derive(Clone)]
pub struct LegacyEventManagerHandle {
    inner: Arc<EventManager>,
}

impl LegacyEventManagerHandle {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(EventManager::new()),
        }
    }

    pub fn new_with_connection_pool(pool: ConnectionPool) -> Self {
        Self {
            inner: Arc::new(EventManager::new_with_connection_pool(pool)),
        }
    }

    #[allow(dead_code)]
    pub fn from_arc(inner: Arc<EventManager>) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub fn into_trait_arc(self) -> Arc<dyn EventManagerHandle> {
        Arc::new(self)
    }

    #[allow(dead_code)]
    pub fn as_event_manager(&self) -> Arc<EventManager> {
        Arc::clone(&self.inner)
    }
}

impl Default for LegacyEventManagerHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventManagerHandle for LegacyEventManagerHandle {
    async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>) {
        self.inner.set_gossip_service(gossip).await;
    }

    async fn set_event_topic_store(&self, store: Arc<dyn EventTopicStore>) {
        self.inner.set_event_topic_store(store).await;
    }

    async fn set_default_p2p_topic_id(&self, topic_id: &str) {
        self.inner.set_default_p2p_topic_id(topic_id).await;
    }

    async fn set_default_p2p_topics(&self, topics: Vec<String>) {
        self.inner.set_default_p2p_topics(topics).await;
    }

    async fn list_default_p2p_topics(&self) -> Vec<String> {
        self.inner.list_default_p2p_topics().await
    }

    async fn handle_p2p_event(&self, event: NostrEvent) -> Result<()> {
        self.inner.handle_p2p_event(event).await
    }

    async fn publish_text_note(&self, content: &str) -> Result<NostrEventId> {
        self.inner.publish_text_note(content).await
    }

    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<NostrEventId>,
    ) -> Result<NostrEventId> {
        self.inner
            .publish_topic_post(topic_id, content, reply_to)
            .await
    }

    async fn publish_repost(&self, target: &NostrEventId) -> Result<NostrEventId> {
        self.inner.publish_repost(target).await
    }

    async fn publish_event(&self, event: NostrEvent) -> Result<NostrEventId> {
        self.inner.publish_event(event).await
    }

    async fn send_reaction(&self, target: &NostrEventId, reaction: &str) -> Result<NostrEventId> {
        self.inner.send_reaction(target, reaction).await
    }

    async fn update_metadata(&self, metadata: Metadata) -> Result<NostrEventId> {
        self.inner.update_metadata(metadata).await
    }

    async fn delete_events(
        &self,
        target_ids: Vec<NostrEventId>,
        reason: Option<String>,
    ) -> Result<NostrEventId> {
        self.inner.delete_events(target_ids, reason).await
    }

    async fn disconnect(&self) -> Result<()> {
        self.inner.disconnect().await
    }

    async fn get_public_key(&self) -> Option<PublicKey> {
        self.inner.get_public_key().await
    }

    async fn subscribe_to_topic(&self, topic_id: &str, since: Option<Timestamp>) -> Result<()> {
        self.inner.subscribe_to_topic(topic_id, since).await
    }

    async fn subscribe_to_user(&self, pubkey: PublicKey, since: Option<Timestamp>) -> Result<()> {
        self.inner.subscribe_to_user(pubkey, since).await
    }
}
