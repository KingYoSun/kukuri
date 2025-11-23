use std::collections::HashSet;

use anyhow::{Result, anyhow};
use nostr_sdk::prelude::*;
use tracing::error;

use super::EventManager;

impl EventManager {
    /// テキストノートを投稿
    pub async fn publish_text_note(&self, content: &str) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_text_note(content, vec![])?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let event_id = match client_manager.publish_event(event.clone()).await {
            Ok(id) => id,
            Err(e) => {
                if std::env::var("KUKURI_ALLOW_NO_RELAY")
                    .map(|value| value == "1")
                    .unwrap_or(false)
                    && e.to_string().contains("no relays specified")
                {
                    event.id
                } else {
                    return Err(e);
                }
            }
        };
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topics = self.default_topics_with_user_topic().await;
            if let Err(e) = self.broadcast_to_topics(&gossip, &topics, &event).await {
                error!("Failed to broadcast to P2P (text_note): {}", e);
            }
        }

        Ok(event_id)
    }

    /// トピック投稿を作成・送信
    pub async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<EventId>,
    ) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_topic_post(topic_id, content, reply_to)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let event_id = match client_manager.publish_event(event.clone()).await {
            Ok(id) => id,
            Err(e) => {
                let msg = e.to_string();
                let allow_no_relay = std::env::var("KUKURI_ALLOW_NO_RELAY")
                    .map(|value| value == "1")
                    .unwrap_or(false)
                    || msg.contains("no relays specified")
                    || msg.contains("not connected to any relays");

                if allow_no_relay {
                    tracing::warn!(
                        target: "event_manager",
                        "publish_event skipped (no relay connected): {msg}"
                    );
                    event.id
                } else {
                    return Err(e);
                }
            }
        };
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            if let Err(e) = self.broadcast_to_topic(&gossip, topic_id, &event).await {
                error!("Failed to broadcast to P2P (topic {}): {}", topic_id, e);
            }
        }

        if let Some(store) = self.event_topic_store.read().await.as_ref().cloned() {
            tracing::debug!(
                target: "event_manager",
                "adding event_topic mapping for {}",
                event.id.to_hex()
            );
            let _ = store.add_event_topic(&event.id.to_string(), topic_id).await;
        }

        Ok(event_id)
    }

    /// リアクションを送信
    pub async fn send_reaction(&self, event_id: &EventId, reaction: &str) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_reaction(event_id, reaction)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topic_list = if let Some(resolved_topics) = self
                .resolve_topics_for_referenced_event(&event_id.to_hex())
                .await
            {
                if resolved_topics.is_empty() {
                    self.default_topics_with_user_topic().await
                } else {
                    let unique: HashSet<_> = resolved_topics.into_iter().collect();
                    unique.into_iter().collect()
                }
            } else {
                self.default_topics_with_user_topic().await
            };

            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast reaction to P2P: {}", e);
            }
        }

        Ok(result_id)
    }

    /// 投稿を再配信 (Repost) する
    pub async fn publish_repost(&self, target: &EventId) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_repost(target)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topic_list = if let Some(resolved_topics) = self
                .resolve_topics_for_referenced_event(&target.to_hex())
                .await
            {
                if resolved_topics.is_empty() {
                    self.default_topics_with_user_topic().await
                } else {
                    let unique: HashSet<_> = resolved_topics.into_iter().collect();
                    unique.into_iter().collect()
                }
            } else {
                self.default_topics_with_user_topic().await
            };

            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast repost to P2P: {}", e);
            }
        }

        Ok(result_id)
    }

    /// 指定したイベントを削除するための削除イベントを発行
    pub async fn delete_events(
        &self,
        target_ids: Vec<EventId>,
        reason: Option<String>,
    ) -> Result<EventId> {
        self.ensure_initialized().await?;
        if target_ids.is_empty() {
            return Err(anyhow!("No event IDs provided"));
        }

        let publisher = self.event_publisher.read().await;
        let deletion_event = publisher.create_deletion(target_ids.clone(), reason.as_deref())?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let deletion_event_id = client_manager.publish_event(deletion_event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let mut topics: HashSet<String> = HashSet::new();
            for event_id in &target_ids {
                if let Some(resolved_topics) = self
                    .resolve_topics_for_referenced_event(&event_id.to_hex())
                    .await
                {
                    topics.extend(resolved_topics);
                }
            }

            if topics.is_empty() {
                topics.extend(self.default_topics_with_user_topic().await);
            }

            let topic_list: Vec<String> = topics.into_iter().collect();
            if let Err(e) = self
                .broadcast_to_topics(&gossip, &topic_list, &deletion_event)
                .await
            {
                error!("Failed to broadcast deletion to P2P: {}", e);
            }
        }

        Ok(deletion_event_id)
    }

    /// リポスト（ブースト）を送信
    /// 任意のイベントを発行
    pub async fn publish_event(&self, event: Event) -> Result<EventId> {
        self.ensure_initialized().await?;

        let client_manager = self.client_manager.read().await;
        let event_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topics = self.default_topics_with_user_topic().await;
            if let Err(e) = self.broadcast_to_topics(&gossip, &topics, &event).await {
                error!("Failed to broadcast generic event to P2P: {}", e);
            }
        }

        Ok(event_id)
    }

    /// メタデータを更新
    pub async fn update_metadata(&self, metadata: Metadata) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_metadata(metadata)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topics = self.default_topics_with_user_topic().await;
            if let Err(e) = self.broadcast_to_topics(&gossip, &topics, &event).await {
                error!("Failed to broadcast metadata to P2P: {}", e);
            }
        }

        Ok(result_id)
    }
}
