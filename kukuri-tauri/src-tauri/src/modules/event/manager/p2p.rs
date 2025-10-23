use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tracing::error;

use super::EventManager;
use super::conversions::nostr_to_domain_event;
use crate::domain::p2p::user_topic_id;
use crate::infrastructure::database::EventRepository as InfraEventRepository;
use crate::infrastructure::p2p::GossipService;

/// フロントエンドに送信するイベントペイロード
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NostrEventPayload {
    pub id: String,
    pub author: String,
    pub content: String,
    pub created_at: u64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
}

impl EventManager {
    /// GossipServiceを接続（P2P配信用）。未設定でも動作は継続。
    pub async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>) {
        let mut gs = self.gossip_service.write().await;
        *gs = Some(gossip);
    }

    /// EventRepositoryを接続（参照トピック解決用）。未設定でも動作は継続。
    pub async fn set_event_repository(&self, repo: Arc<dyn InfraEventRepository>) {
        let mut r = self.event_repository.write().await;
        *r = Some(repo);
    }

    /// P2Pネットワークから受信したNostrイベントを処理
    pub async fn handle_p2p_event(&self, event: Event) -> Result<()> {
        if let Err(e) = self.event_handler.handle_event(event.clone()).await {
            error!("Error handling P2P event: {}", e);
            return Err(e);
        }

        if let Some(handle) = self.app_handle.read().await.clone() {
            let payload = NostrEventPayload {
                id: event.id.to_string(),
                author: event.pubkey.to_string(),
                content: event.content.clone(),
                created_at: event.created_at.as_u64(),
                kind: event.kind.as_u16() as u32,
                tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
            };
            if let Err(e) = handle.emit("nostr://event/p2p", payload) {
                error!("Failed to emit nostr event to frontend: {}", e);
            }
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

        let domain_event = nostr_to_domain_event(nostr_event)?;
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
        let domain_event = nostr_to_domain_event(nostr_event)?;
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
        if let Some(repo) = self.event_repository.read().await.as_ref().cloned() {
            match repo.get_event_topics(event_id).await {
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
