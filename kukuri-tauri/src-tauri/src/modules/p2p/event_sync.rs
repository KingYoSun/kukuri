use std::sync::Arc;
use nostr_sdk::Event;
use serde::{Deserialize, Serialize};

use crate::modules::event::manager::EventManager;
use crate::modules::p2p::gossip_manager::GossipManager;
use crate::modules::p2p::message::{GossipMessage, MessageType, generate_topic_id};
use crate::modules::p2p::error::{P2PError, Result as P2PResult};

pub struct EventSync {
    event_manager: Arc<EventManager>,
    gossip_manager: Arc<GossipManager>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrEventPayload {
    pub event: Event,
}

impl EventSync {
    /// 新しいEventSyncインスタンスを作成
    pub fn new(
        event_manager: Arc<EventManager>,
        gossip_manager: Arc<GossipManager>,
    ) -> Self {
        Self {
            event_manager,
            gossip_manager,
        }
    }
    
    /// NostrイベントをP2Pネットワークに配信
    pub async fn propagate_nostr_event(&self, event: Event) -> P2PResult<()> {
        // 1. イベントをGossipMessage形式に変換
        let message = self.convert_to_gossip_message_internal(event.clone())?;
        
        // 2. 関連するトピックを特定
        let topic_ids = self.extract_topic_ids_internal(&event)?;
        
        // 3. P2Pネットワークにブロードキャスト
        for topic_id in topic_ids {
            self.gossip_manager.broadcast(&topic_id, message.clone()).await?;
        }
        
        Ok(())
    }
    
    /// P2Pで受信したメッセージをNostrイベントとして処理
    pub async fn handle_gossip_message(&self, message: GossipMessage) -> P2PResult<()> {
        match message.msg_type {
            MessageType::NostrEvent => {
                // ペイロードからNostrイベントを復元
                let payload: NostrEventPayload = serde_json::from_slice(&message.payload)
                    .map_err(|e| P2PError::SerializationError(e.to_string()))?;
                
                // TODO: EventManagerとの統合
                // 現時点では単純にログに記録
                tracing::info!("Received Nostr event via P2P: {:?}", payload.event.id);
            },
            _ => {
                // 他のメッセージタイプは現時点では無視
                tracing::debug!("Received non-NostrEvent message type: {:?}", message.msg_type);
            }
        }
        
        Ok(())
    }
    
    /// NostrイベントをGossipMessageに変換
    #[cfg(test)]
    pub fn convert_to_gossip_message(&self, event: Event) -> P2PResult<GossipMessage> {
        self.convert_to_gossip_message_internal(event)
    }
    
    #[cfg(not(test))]
    fn convert_to_gossip_message(&self, event: Event) -> P2PResult<GossipMessage> {
        self.convert_to_gossip_message_internal(event)
    }
    
    fn convert_to_gossip_message_internal(&self, event: Event) -> P2PResult<GossipMessage> {
        let payload = NostrEventPayload { event: event.clone() };
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| P2PError::SerializationError(e.to_string()))?;
        
        // 送信者の公開鍵を取得 (Nostr公開鍵は32バイト)
        let sender = event.pubkey.to_bytes().to_vec();
        
        let message = GossipMessage::new(
            MessageType::NostrEvent,
            payload_bytes,
            sender,
        );
        
        // TODO: メッセージに署名を追加
        
        Ok(message)
    }
    
    /// イベントから関連するトピックIDを抽出
    #[cfg(test)]
    pub fn extract_topic_ids(&self, event: &Event) -> P2PResult<Vec<String>> {
        self.extract_topic_ids_internal(event)
    }
    
    #[cfg(not(test))]
    fn extract_topic_ids(&self, event: &Event) -> P2PResult<Vec<String>> {
        self.extract_topic_ids_internal(event)
    }
    
    fn extract_topic_ids_internal(&self, event: &Event) -> P2PResult<Vec<String>> {
        let mut topic_ids = Vec::new();
        
        // グローバルトピックには常に配信
        topic_ids.push(crate::modules::p2p::message::GLOBAL_TOPIC.to_string());
        
        // tタグ（トピックタグ）を確認
        for tag in event.tags.iter() {
            if let Some(tag_kind) = tag.as_standardized() {
                if matches!(tag_kind, nostr_sdk::TagStandard::Hashtag(_)) {
                    // ハッシュタグをトピックとして扱う
                    if let nostr_sdk::TagStandard::Hashtag(topic_name) = tag_kind {
                        topic_ids.push(generate_topic_id(&topic_name));
                    }
                }
            }
        }
        
        // ユーザー固有トピック
        topic_ids.push(crate::modules::p2p::message::user_topic_id(&event.pubkey.to_string()));
        
        Ok(topic_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_topic_extraction() {
        // TODO: テスト実装
    }
}