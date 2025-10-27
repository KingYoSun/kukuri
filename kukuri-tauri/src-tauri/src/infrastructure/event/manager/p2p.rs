use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use nostr_sdk::prelude::*;
use tracing::error;

use super::EventManager;
use crate::application::ports::event_topic_store::EventTopicStore;
use crate::application::shared::mappers::nostr_event_to_domain_event;
use crate::domain::p2p::user_topic_id;
use crate::infrastructure::p2p::GossipService;

impl EventManager {
    /// GossipServiceを接続（P2P配信用）。未設定でも動作は継続。
    pub async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>) {
        let mut gs = self.gossip_service.write().await;
        *gs = Some(gossip);
    }

    /// EventTopicStoreを接続（参照トピック解決用）。未設定でも動作は継続。
    pub async fn set_event_topic_store(&self, store: Arc<dyn EventTopicStore>) {
        let mut r = self.event_topic_store.write().await;
        *r = Some(store);
    }

    /// P2Pネットワークから受信したNostrイベントを処理
    pub async fn handle_p2p_event(&self, event: Event) -> Result<()> {
        if let Err(e) = self.event_handler.handle_event(event.clone()).await {
            error!("Error handling P2P event: {}", e);
            return Err(e);
        }

        Ok(())
    }

    /// 複数トピックへ冪等Join + 重複排除つきでブロードキャスト
    pub(crate) async fn broadcast_to_topics(
        &self,
        gossip: &Arc<dyn GossipService>,
        topics: &[String],
        nostr_event: &Event,
    ) -> Result<()> {
        let mut uniq: HashSet<String> = HashSet::new();
        for t in topics {
            if !t.is_empty() {
                uniq.insert(t.clone());
            }
        }
        if uniq.is_empty() {
            return Ok(());
        }

        let domain_event =
            nostr_event_to_domain_event(nostr_event).map_err(|err| anyhow!(err.to_string()))?;
        for topic in uniq.into_iter() {
            let _ = gossip.join_topic(&topic, vec![]).await;
            if let Err(e) = gossip.broadcast(&topic, &domain_event).await {
                error!("Failed to broadcast to topic {}: {}", topic, e);
            }
        }
        Ok(())
    }

    pub(crate) async fn broadcast_to_topic(
        &self,
        gossip: &Arc<dyn GossipService>,
        topic_id: &str,
        nostr_event: &Event,
    ) -> Result<()> {
        let domain_event =
            nostr_event_to_domain_event(nostr_event).map_err(|err| anyhow!(err.to_string()))?;
        let _ = gossip.join_topic(topic_id, vec![]).await;
        gossip
            .broadcast(topic_id, &domain_event)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    pub(crate) async fn resolve_topics_for_referenced_event(
        &self,
        event_id: &str,
    ) -> Option<Vec<String>> {
        if let Some(store) = self.event_topic_store.read().await.as_ref().cloned() {
            match store.get_event_topics(event_id).await {
                Ok(v) if !v.is_empty() => return Some(v),
                _ => {}
            }
        }
        None
    }

    pub(crate) async fn default_topics_with_user_topic(&self) -> Vec<String> {
        let mut topics = self.default_topics.snapshot().await;
        if let Some(pk) = self.get_public_key().await {
            topics.insert(user_topic_id(&pk.to_string()));
        }
        topics.into_iter().collect()
    }
}
