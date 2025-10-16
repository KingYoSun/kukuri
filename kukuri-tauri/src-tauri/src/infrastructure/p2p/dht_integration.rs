//! DHT統合モジュール
//! iroh-gossipとdistributed-topic-trackerの統合
use crate::domain::entities::Event;
use crate::infrastructure::p2p::dht_bootstrap::DhtGossip;
use crate::shared::error::AppError;
// use iroh_gossip::proto::Event as GossipEvent;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error};

/// DHTイベントハンドラー
pub struct DhtEventHandler {
    event_tx: mpsc::Sender<Event>,
    dht_gossip: Arc<DhtGossip>,
}

impl DhtEventHandler {
    /// 新しいハンドラーを作成
    pub fn new(event_tx: mpsc::Sender<Event>, dht_gossip: Arc<DhtGossip>) -> Self {
        Self {
            event_tx,
            dht_gossip,
        }
    }

    /// Gossipメッセージを処理
    pub async fn handle_message(&self, data: &[u8], from: Option<String>) -> Result<(), AppError> {
        debug!("Received message from {:?}", from);

        // メッセージをデシリアライズ
        if let Ok(event) = self.deserialize_message(data).await {
            // イベントチャンネルに送信
            if let Err(e) = self.event_tx.send(event).await {
                error!("Failed to send event: {:?}", e);
            }
        }
        Ok(())
    }

    /// メッセージをデシリアライズ
    async fn deserialize_message(&self, data: &[u8]) -> Result<Event, AppError> {
        bincode::serde::decode_from_slice::<Event, _>(data, bincode::config::standard())
            .map(|(event, _)| event)
            .map_err(|e| AppError::DeserializationError(format!("Failed to deserialize: {e:?}")))
    }
}

/// DHT統合マネージャー
pub struct DhtIntegration {
    dht_gossip: Arc<DhtGossip>,
    event_handler: Option<DhtEventHandler>,
}

impl DhtIntegration {
    /// 新しい統合マネージャーを作成
    pub fn new(dht_gossip: Arc<DhtGossip>) -> Self {
        Self {
            dht_gossip,
            event_handler: None,
        }
    }

    /// イベントハンドラーを設定
    pub fn set_event_handler(&mut self, event_tx: mpsc::Sender<Event>) {
        self.event_handler = Some(DhtEventHandler::new(event_tx, Arc::clone(&self.dht_gossip)));
    }

    /// トピックに参加
    pub async fn join_topic(&self, topic: &str) -> Result<(), AppError> {
        self.dht_gossip.join_topic(topic.as_bytes(), vec![]).await?;
        Ok(())
    }

    /// トピックから離脱
    pub async fn leave_topic(&self, topic: &str) -> Result<(), AppError> {
        self.dht_gossip.leave_topic(topic.as_bytes()).await?;
        Ok(())
    }

    /// イベントをブロードキャスト
    pub async fn broadcast_event(&self, topic: &str, event: &Event) -> Result<(), AppError> {
        // イベントをシリアライズ
        let message = bincode::serde::encode_to_vec(event, bincode::config::standard())
            .map_err(|e| AppError::SerializationError(format!("Failed to serialize: {e:?}")))?;

        // DHTにブロードキャスト
        self.dht_gossip.broadcast(topic.as_bytes(), message).await?;

        debug!("Event broadcast to topic: {}", topic);
        Ok(())
    }
}

/// NostrとDHTのブリッジ
pub mod bridge {
    use super::*;
    use nostr_sdk::Event as NostrEvent;

    /// NostrイベントをKukuriイベントに変換
    pub fn nostr_to_kukuri(_event: &NostrEvent) -> Result<Event, AppError> {
        // TODO: 実装
        Err(AppError::NotImplemented(
            "Conversion not implemented".to_string(),
        ))
    }

    /// KukuriイベントをNostrイベントに変換
    pub fn kukuri_to_nostr(_event: &Event) -> Result<NostrEvent, AppError> {
        // TODO: 実装
        Err(AppError::NotImplemented(
            "Conversion not implemented".to_string(),
        ))
    }
}
