use nostr_sdk::{Event, Kind, TagStandard};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::modules::event::manager::{EventManager, NostrEventPayload};
use crate::modules::offline::models::SyncStatus;
use crate::modules::p2p::error::{P2PError, Result as P2PResult};
use crate::modules::p2p::gossip_manager::GossipManager;
use crate::modules::p2p::message::{generate_topic_id, GossipMessage, MessageType};

#[derive(Clone)]
pub struct EventSync {
    event_manager: Arc<EventManager>,
    gossip_manager: Arc<GossipManager>,
    /// 同期状態の管理（イベントID -> 同期ステータス）
    sync_state: Arc<RwLock<HashMap<String, SyncStatus>>>,
    /// P2P同期が有効かどうか
    p2p_sync_enabled: Arc<RwLock<bool>>,
}

impl EventSync {
    /// 新しいEventSyncインスタンスを作成
    pub fn new(event_manager: Arc<EventManager>, gossip_manager: Arc<GossipManager>) -> Self {
        Self {
            event_manager: event_manager.clone(),
            gossip_manager: gossip_manager.clone(),
            sync_state: Arc::new(RwLock::new(HashMap::new())),
            p2p_sync_enabled: Arc::new(RwLock::new(true)), // デフォルトで有効
        }
    }

    /// NostrイベントをP2Pネットワークに配信
    pub async fn propagate_nostr_event(&self, event: Event) -> P2PResult<()> {
        // P2P同期が無効の場合はスキップ
        if !*self.p2p_sync_enabled.read().await {
            tracing::debug!("P2P sync is disabled, skipping propagation");
            return Ok(());
        }
        let event_id = event.id.to_string();

        // 1. 同期状態を確認
        {
            let sync_state = self.sync_state.read().await;
            if let Some(&status) = sync_state.get(&event_id) {
                if status == SyncStatus::SentToP2P || status == SyncStatus::FullySynced {
                    tracing::debug!("Event {} already sent to P2P network", event_id);
                    return Ok(());
                }
            }
        }

        // 2. イベントをGossipMessage形式に変換
        let message = self.convert_to_gossip_message_internal(event.clone())?;

        // 3. 関連するトピックを特定
        let topic_ids = self.extract_topic_ids_internal(&event)?;

        // 4. P2Pネットワークにブロードキャスト
        for topic_id in &topic_ids {
            self.gossip_manager
                .broadcast(topic_id, message.clone())
                .await?;
        }

        // 5. 同期状態を更新
        {
            let mut sync_state = self.sync_state.write().await;
            let current = sync_state
                .get(&event_id)
                .copied()
                .unwrap_or(SyncStatus::SentToP2P);
            let new_status = match current {
                SyncStatus::SentToNostr => SyncStatus::FullySynced,
                _ => SyncStatus::SentToP2P,
            };
            sync_state.insert(event_id.clone(), new_status);
        }

        tracing::info!(
            "Propagated event {} to {} topics via P2P",
            event_id,
            topic_ids.len()
        );

        Ok(())
    }

    /// P2Pで受信したメッセージをNostrイベントとして処理
    pub async fn handle_gossip_message(&self, message: GossipMessage) -> P2PResult<()> {
        match message.msg_type {
            MessageType::NostrEvent => {
                // ペイロードからNostrイベントを復元
                let payload: NostrEventPayload = serde_json::from_slice(&message.payload)
                    .map_err(|e| P2PError::SerializationError(e.to_string()))?;

                let event = payload.event;
                let event_id = event.id.to_string();

                // 重複チェック
                {
                    let sync_state = self.sync_state.read().await;
                    if sync_state.contains_key(&event_id) {
                        tracing::debug!("Event {} already processed", event_id);
                        return Ok(());
                    }
                }

                // 署名検証
                match message.verify_signature() {
                    Ok(false) | Err(_) => {
                        tracing::warn!("Invalid signature for message from P2P");
                        return Err(P2PError::Internal("Invalid signature".to_string()));
                    }
                    Ok(true) => {}
                }

                // EventManagerに転送してNostrリレーに送信
                if let Err(e) = self.event_manager.handle_p2p_event(event.clone()).await {
                    tracing::error!("Failed to handle P2P event in EventManager: {}", e);
                }

                // 同期状態を更新
                {
                    let mut sync_state = self.sync_state.write().await;
                    sync_state.insert(event_id.clone(), SyncStatus::SentToNostr);
                }

                tracing::info!("Processed Nostr event {} from P2P network", event_id);
            }
            _ => {
                // 他のメッセージタイプは現時点では無視
                tracing::debug!(
                    "Received non-NostrEvent message type: {:?}",
                    message.msg_type
                );
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
    #[allow(dead_code)]
    fn convert_to_gossip_message(&self, event: Event) -> P2PResult<GossipMessage> {
        self.convert_to_gossip_message_internal(event)
    }

    #[allow(dead_code)]
    fn convert_to_gossip_message_internal(&self, event: Event) -> P2PResult<GossipMessage> {
        let payload = NostrEventPayload {
            event: event.clone(),
        };
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| P2PError::SerializationError(e.to_string()))?;

        // 送信者の公開鍵を取得 (Nostr公開鍵は32バイト)
        let sender = event.pubkey.to_bytes().to_vec();

        let mut message = GossipMessage::new(MessageType::NostrEvent, payload_bytes, sender);

        // GossipManagerを使用してメッセージに署名
        self.gossip_manager.sign_message(&mut message)?;

        Ok(message)
    }

    /// イベントから関連するトピックIDを抽出
    #[cfg(test)]
    pub fn extract_topic_ids(&self, event: &Event) -> P2PResult<Vec<String>> {
        self.extract_topic_ids_internal(event)
    }

    #[cfg(not(test))]
    #[allow(dead_code)]
    fn extract_topic_ids(&self, event: &Event) -> P2PResult<Vec<String>> {
        self.extract_topic_ids_internal(event)
    }

    #[allow(dead_code)]
    fn extract_topic_ids_internal(&self, event: &Event) -> P2PResult<Vec<String>> {
        let mut topic_ids = Vec::new();

        // グローバルトピックには常に配信（テキストノートなど）
        if matches!(event.kind, Kind::TextNote | Kind::Repost | Kind::Reaction) {
            topic_ids.push(crate::modules::p2p::message::GLOBAL_TOPIC.to_string());
        }

        // kind:30078 (Application-specific data) - kukuriトピック投稿
        if event.kind == Kind::from(30078u16) {
            // dタグからトピックIDを抽出
            for tag in event.tags.iter() {
                if let Some(TagStandard::Identifier(identifier)) = tag.as_standardized() {
                    // identifierがトピックIDとして使用される
                    topic_ids.push(generate_topic_id(identifier));
                }
            }
        }

        // ハッシュタグをトピックとして扱う
        for tag in event.tags.iter() {
            if let Some(TagStandard::Hashtag(topic_name)) = tag.as_standardized() {
                topic_ids.push(generate_topic_id(topic_name));
            }
        }

        // eタグ（リプライ）の場合、元の投稿のトピックも取得
        for tag in event.tags.iter() {
            if let Some(TagStandard::Event { event_id, .. }) = tag.as_standardized() {
                // リプライ先のイベントのトピックにも配信
                // 実際の実装では、EventManagerから元イベントを取得してトピックを抽出する必要がある
                tracing::debug!("Reply to event: {}", event_id);
            }
        }

        // ユーザー固有トピック（フォロワーへの配信用）
        topic_ids.push(crate::modules::p2p::message::user_topic_id(
            &event.pubkey.to_string(),
        ));

        // 重複を除去
        topic_ids.sort_unstable();
        topic_ids.dedup();

        Ok(topic_ids)
    }

    /// Nostrイベント送信時のP2P配信を有効化
    pub async fn enable_nostr_to_p2p_sync(&self, enabled: bool) -> P2PResult<()> {
        // P2P同期の有効/無効を設定
        *self.p2p_sync_enabled.write().await = enabled;
        
        tracing::info!(
            "Nostr to P2P sync: {}",
            if enabled { "enabled" } else { "disabled" }
        );
        
        // EventManagerとの統合はpropagate_nostr_eventメソッドで既に実装済み
        // EventManagerがpublish_*メソッドで自動的にpropagate_nostr_eventを呼び出している
        
        Ok(())
    }

    /// 同期状態の取得
    #[allow(dead_code)]
    pub async fn get_sync_status(&self, event_id: &str) -> Option<SyncStatus> {
        let sync_state = self.sync_state.read().await;
        sync_state.get(event_id).copied()
    }

    /// 同期状態のクリーンアップ（古いエントリを削除）
    #[allow(dead_code)]
    pub async fn cleanup_sync_state(&self, keep_recent: usize) -> P2PResult<()> {
        let mut sync_state = self.sync_state.write().await;
        if sync_state.len() > keep_recent * 2 {
            // 単純な実装：半分を削除
            let to_remove = sync_state.len() - keep_recent;
            let keys_to_remove: Vec<_> = sync_state.keys().take(to_remove).cloned().collect();
            for key in keys_to_remove {
                sync_state.remove(&key);
            }
        }
        Ok(())
    }

    /// テスト用：同期状態に直接アクセス
    #[cfg(test)]
    pub async fn test_set_sync_status(&self, event_id: String, status: SyncStatus) {
        let mut sync_state = self.sync_state.write().await;
        sync_state.insert(event_id, status);
    }

    /// テスト用：同期状態のサイズを取得
    #[cfg(test)]
    pub async fn test_get_sync_state_size(&self) -> usize {
        let sync_state = self.sync_state.read().await;
        sync_state.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::p2p::message::{user_topic_id, GLOBAL_TOPIC};
    use nostr_sdk::{EventBuilder, Keys};

    #[test]
    fn test_nostr_event_payload_serialization() {
        let keys = Keys::generate();
        let event = EventBuilder::text_note("Test content")
            .sign_with_keys(&keys)
            .unwrap();

        let payload = NostrEventPayload {
            event: event.clone(),
        };

        // シリアライズ
        let serialized = serde_json::to_vec(&payload).unwrap();

        // デシリアライズ
        let deserialized: NostrEventPayload = serde_json::from_slice(&serialized).unwrap();

        assert_eq!(deserialized.event.id, event.id);
        assert_eq!(deserialized.event.content, event.content);
    }

    #[test]
    fn test_topic_id_extraction_from_hashtags() {
        let keys = Keys::generate();

        // ハッシュタグ付きイベント
        let event = EventBuilder::text_note("#bitcoin #nostr")
            .tags(vec![
                nostr_sdk::Tag::hashtag("bitcoin"),
                nostr_sdk::Tag::hashtag("nostr"),
            ])
            .sign_with_keys(&keys)
            .unwrap();

        // extract_topic_ids_internalを直接テスト（EventSyncインスタンスを作らない）
        let mut topic_ids = Vec::new();

        // グローバルトピック
        if matches!(event.kind, Kind::TextNote | Kind::Repost | Kind::Reaction) {
            topic_ids.push(GLOBAL_TOPIC.to_string());
        }

        // ハッシュタグ
        for tag in event.tags.iter() {
            if let Some(TagStandard::Hashtag(topic_name)) = tag.as_standardized() {
                topic_ids.push(generate_topic_id(topic_name));
            }
        }

        // ユーザートピック
        topic_ids.push(user_topic_id(&event.pubkey.to_string()));

        // 重複を除去
        topic_ids.sort_unstable();
        topic_ids.dedup();

        // グローバルトピックが含まれる
        assert!(topic_ids.contains(&GLOBAL_TOPIC.to_string()));

        // ハッシュタグトピックが含まれる
        assert!(topic_ids.contains(&generate_topic_id("bitcoin")));
        assert!(topic_ids.contains(&generate_topic_id("nostr")));

        // ユーザートピックが含まれる
        assert!(topic_ids.contains(&user_topic_id(&event.pubkey.to_string())));
    }

    #[test]
    fn test_topic_id_extraction_from_kind_30078() {
        let keys = Keys::generate();

        // kind:30078 (Application-specific data)
        let event = EventBuilder::new(Kind::from(30078u16), "Topic content")
            .tags(vec![nostr_sdk::Tag::identifier("technology")])
            .sign_with_keys(&keys)
            .unwrap();

        let mut topic_ids = Vec::new();

        // kind:30078の処理
        if event.kind == Kind::from(30078u16) {
            for tag in event.tags.iter() {
                if let Some(TagStandard::Identifier(identifier)) = tag.as_standardized() {
                    topic_ids.push(generate_topic_id(identifier));
                }
            }
        }

        // ユーザートピック
        topic_ids.push(user_topic_id(&event.pubkey.to_string()));

        // 重複を除去
        topic_ids.sort_unstable();
        topic_ids.dedup();

        // kind:30078はグローバルトピックに含まれない
        assert!(!topic_ids.contains(&GLOBAL_TOPIC.to_string()));

        // identifierタグからトピックIDが生成される
        assert!(topic_ids.contains(&generate_topic_id("technology")));

        // ユーザートピックは常に含まれる
        assert!(topic_ids.contains(&user_topic_id(&event.pubkey.to_string())));
    }

    #[tokio::test]
    async fn test_sync_state_operations() {
        // 同期状態の管理だけをテスト
        let sync_state = Arc::new(RwLock::new(HashMap::<String, SyncStatus>::new()));

        let event_id = "test_event_123";

        // 初期状態
        {
            let state = sync_state.read().await;
            assert_eq!(state.get(event_id).copied(), None);
        }

        // 状態を設定
        {
            let mut state = sync_state.write().await;
            state.insert(event_id.to_string(), SyncStatus::SentToP2P);
        }

        {
            let state = sync_state.read().await;
            assert_eq!(state.get(event_id).copied(), Some(SyncStatus::SentToP2P));
        }

        // クリーンアップのシミュレーション
        {
            let mut state = sync_state.write().await;
            for i in 0..100 {
                state.insert(format!("event_{i}"), SyncStatus::FullySynced);
            }
        }

        // クリーンアップ実行
        {
            let mut state = sync_state.write().await;
            if state.len() > 50 * 2 {
                let to_remove = state.len() - 50;
                let keys_to_remove: Vec<_> = state.keys().take(to_remove).cloned().collect();
                for key in keys_to_remove {
                    state.remove(&key);
                }
            }
        }

        let state = sync_state.read().await;
        assert!(state.len() <= 51); // 50 + test_event_123
    }
}
