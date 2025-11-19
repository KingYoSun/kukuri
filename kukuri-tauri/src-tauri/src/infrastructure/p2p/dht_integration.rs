//! DHT統合モジュール
//! iroh-gossipとdistributed-topic-trackerの統合
use crate::domain::entities::Event;
use crate::domain::p2p::generate_topic_id;
use crate::infrastructure::p2p::dht_bootstrap::DhtGossip;
use crate::shared::error::AppError;
// use iroh_gossip::proto::Event as GossipEvent;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error};

/// DHTイベントハンドラー
pub struct DhtEventHandler {
    event_tx: mpsc::Sender<Event>,
}

impl DhtEventHandler {
    /// 新しいハンドラーを作成
    pub fn new(event_tx: mpsc::Sender<Event>) -> Self {
        Self { event_tx }
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
            .map_err(|e| {
                AppError::DeserializationError(format!("failed to deserialize DHT event: {e:?}"))
            })
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
        self.event_handler = Some(DhtEventHandler::new(event_tx));
    }

    /// トピックに参加
    pub async fn join_topic(&self, topic: &str) -> Result<(), AppError> {
        let canonical = generate_topic_id(topic);
        self.dht_gossip
            .join_topic(canonical.as_bytes(), vec![])
            .await?;
        Ok(())
    }

    /// トピックから離脱
    pub async fn leave_topic(&self, topic: &str) -> Result<(), AppError> {
        let canonical = generate_topic_id(topic);
        self.dht_gossip.leave_topic(canonical.as_bytes()).await?;
        Ok(())
    }

    /// イベントをブロードキャスト
    pub async fn broadcast_event(&self, topic: &str, event: &Event) -> Result<(), AppError> {
        // イベントをシリアライズ
        let message = bincode::serde::encode_to_vec(event, bincode::config::standard())
            .map_err(|e| AppError::SerializationError(format!("Failed to serialize: {e:?}")))?;

        // DHTにブロードキャスト
        let canonical = generate_topic_id(topic);
        self.dht_gossip
            .broadcast(canonical.as_bytes(), message)
            .await?;

        debug!(
            "Event broadcast to topic: {} (canonical: {})",
            topic, canonical
        );
        Ok(())
    }
}

/// NostrとDHTのブリッジ
pub mod bridge {
    use super::*;
    use nostr_sdk::{Event as NostrEvent, JsonUtil};
    use serde_json::json;
    use std::convert::TryFrom;

    /// NostrイベントをKukuriイベントに変換
    pub fn nostr_to_kukuri(event: &NostrEvent) -> Result<Event, AppError> {
        let timestamp_raw = event.created_at.as_u64();
        let timestamp = i64::try_from(timestamp_raw).map_err(|_| {
            AppError::DeserializationError(format!(
                "timestamp overflow when converting nostr event: {timestamp_raw}"
            ))
        })?;
        let created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).ok_or_else(|| {
                AppError::DeserializationError(format!(
                    "invalid timestamp in nostr event: {timestamp}"
                ))
            })?;

        Ok(Event {
            id: event.id.to_string(),
            pubkey: event.pubkey.to_string(),
            created_at,
            kind: event.kind.as_u16() as u32,
            tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
            content: event.content.clone(),
            sig: event.sig.to_string(),
        })
    }

    /// KukuriイベントをNostrイベントに変換
    pub fn kukuri_to_nostr(event: &Event) -> Result<NostrEvent, AppError> {
        let payload = json!({
            "id": event.id,
            "pubkey": event.pubkey,
            "created_at": event.created_at.timestamp(),
            "kind": event.kind,
            "tags": event.tags,
            "content": event.content,
            "sig": event.sig,
        });

        NostrEvent::from_json(payload.to_string())
            .map_err(|e| AppError::NostrError(format!("Failed to convert event: {e}")))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use nostr_sdk::{EventBuilder, Keys, Tag};

        #[test]
        fn convert_nostr_to_domain_event() {
            let keys = Keys::generate();
            let event = EventBuilder::text_note("hello nostr to domain")
                .tags(vec![Tag::hashtag("kukuri")])
                .sign_with_keys(&keys)
                .expect("signing succeeds");

            let converted = nostr_to_kukuri(&event).expect("conversion succeeds");
            assert_eq!(converted.id, event.id.to_string());
            assert_eq!(converted.pubkey, event.pubkey.to_string());
            assert_eq!(converted.kind, event.kind.as_u16() as u32);
            assert_eq!(converted.content, event.content);
            assert_eq!(converted.tags.len(), 1);
            assert_eq!(converted.tags[0][0], "t");
            assert_eq!(converted.sig, event.sig.to_string());
        }

        #[test]
        fn convert_domain_to_nostr_event() {
            let keys = Keys::generate();
            let nostr_event = EventBuilder::text_note("roundtrip conversion")
                .tags(vec![Tag::hashtag("kukuri")])
                .sign_with_keys(&keys)
                .expect("signing succeeds");

            let domain_event = nostr_to_kukuri(&nostr_event).expect("conversion succeeds");

            let rebuilt = kukuri_to_nostr(&domain_event).expect("rebuild succeeds");
            assert_eq!(rebuilt.id, nostr_event.id);
            assert_eq!(rebuilt.pubkey, nostr_event.pubkey);
            assert_eq!(rebuilt.kind, nostr_event.kind);
            assert_eq!(rebuilt.content, nostr_event.content);
            assert_eq!(rebuilt.tags, nostr_event.tags);
            assert_eq!(rebuilt.sig, nostr_event.sig);
            assert_eq!(rebuilt.created_at, nostr_event.created_at);
        }
    }
}
