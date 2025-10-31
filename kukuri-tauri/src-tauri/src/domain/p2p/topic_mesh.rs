use crate::domain::p2p::{
    error::Result as P2PResult,
    message::{GossipMessage, MessageId},
};
use lru::LruCache;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, mpsc};

const DEFAULT_SUBSCRIBER_BUFFER: usize = 128;

#[derive(Clone)]
pub struct TopicMesh {
    #[allow(dead_code)]
    topic_id: Arc<String>,
    peers: Arc<RwLock<HashSet<Vec<u8>>>>, // PublicKeyのバイト表現
    message_cache: Arc<RwLock<LruCache<MessageId, GossipMessage>>>,
    subscribers: Arc<RwLock<HashMap<u64, mpsc::Sender<GossipMessage>>>>,
    next_subscription_id: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Default)]
pub struct TopicStats {
    pub peer_count: usize,
    pub message_count: usize,
    pub last_activity: i64,
}

impl TopicMesh {
    /// 新しいTopicMeshを作成
    pub fn new(topic_id: String) -> Self {
        let cache_size = NonZeroUsize::new(1000).unwrap(); // 最大1000メッセージをキャッシュ

        Self {
            topic_id: Arc::new(topic_id),
            peers: Arc::new(RwLock::new(HashSet::new())),
            message_cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            next_subscription_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// メッセージの受信処理
    pub async fn handle_message(&self, message: GossipMessage) -> P2PResult<()> {
        // 重複チェック
        if self.is_duplicate(&message.id).await {
            return Ok(()); // 重複メッセージは無視
        }

        // メッセージをキャッシュに追加
        let mut cache = self.message_cache.write().await;
        cache.put(message.id, message.clone());

        // ピアリストに送信者を追加
        let mut peers = self.peers.write().await;
        peers.insert(message.sender.clone());
        drop(peers);

        self.notify_subscribers(&message).await;

        Ok(())
    }

    /// ピアの接続状態管理
    pub async fn update_peer_status(&self, peer: Vec<u8>, connected: bool) {
        let mut peers = self.peers.write().await;
        if connected {
            peers.insert(peer);
        } else {
            peers.remove(&peer);
        }
    }

    /// メッセージの重複チェック
    pub async fn is_duplicate(&self, message_id: &MessageId) -> bool {
        let cache = self.message_cache.read().await;
        cache.contains(message_id)
    }

    /// トピックの統計情報を取得
    pub async fn get_stats(&self) -> TopicStats {
        let peers = self.peers.read().await;
        let cache = self.message_cache.read().await;

        let last_activity = cache
            .iter()
            .map(|(_, msg)| msg.timestamp)
            .max()
            .unwrap_or(0);

        TopicStats {
            peer_count: peers.len(),
            message_count: cache.len(),
            last_activity,
        }
    }

    /// 接続中のピアのリストを取得
    #[allow(dead_code)]
    pub async fn get_peers(&self) -> Vec<Vec<u8>> {
        let peers = self.peers.read().await;
        peers.iter().cloned().collect()
    }

    /// キャッシュされたメッセージを取得（最新順）
    #[allow(dead_code)]
    pub async fn get_recent_messages(&self, limit: usize) -> Vec<GossipMessage> {
        let cache = self.message_cache.read().await;
        let mut messages: Vec<_> = cache.iter().map(|(_, msg)| msg.clone()).collect();

        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        messages.into_iter().take(limit).collect()
    }

    /// キャッシュをクリア
    #[allow(dead_code)]
    pub async fn clear_cache(&self) {
        let mut cache = self.message_cache.write().await;
        cache.clear();
    }

    /// Gossipメッセージ購読用のチャネルを生成
    pub async fn subscribe(&self) -> TopicMeshSubscription {
        let (tx, rx) = mpsc::channel(DEFAULT_SUBSCRIBER_BUFFER);
        let subscription_id = self.next_subscription_id.fetch_add(1, Ordering::Relaxed);

        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscription_id, tx);

        TopicMeshSubscription {
            id: subscription_id,
            receiver: rx,
        }
    }

    /// 指定された購読IDを解除
    pub async fn unsubscribe(&self, subscription_id: u64) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(&subscription_id);
    }

    async fn notify_subscribers(&self, message: &GossipMessage) {
        let subscribers = self.subscribers.read().await;
        if subscribers.is_empty() {
            return;
        }

        let senders: Vec<(u64, mpsc::Sender<GossipMessage>)> = subscribers
            .iter()
            .map(|(&id, sender)| (id, sender.clone()))
            .collect();
        drop(subscribers);

        let mut closed_ids = Vec::new();
        for (id, sender) in senders {
            match sender.try_send(message.clone()) {
                Ok(_) => {}
                Err(mpsc::error::TrySendError::Full(pending)) => {
                    if sender.send(pending).await.is_err() {
                        closed_ids.push(id);
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => closed_ids.push(id),
            }
        }

        if !closed_ids.is_empty() {
            let mut subscribers = self.subscribers.write().await;
            for id in closed_ids {
                subscribers.remove(&id);
            }
        }
    }

    #[cfg(test)]
    pub async fn subscriber_count(&self) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers.len()
    }
}

pub struct TopicMeshSubscription {
    pub id: u64,
    pub receiver: mpsc::Receiver<GossipMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::p2p::message::MessageType;

    #[tokio::test]
    async fn test_duplicate_detection() {
        let mesh = TopicMesh::new("test_topic".to_string());
        let message = GossipMessage::new(
            MessageType::TopicSync,
            vec![1, 2, 3],
            vec![0x02; 33], // 33バイトの公開鍵
        );
        let id = message.id;

        mesh.handle_message(message.clone()).await.unwrap();
        assert!(mesh.is_duplicate(&id).await);

        // もう一度同じメッセージを処理
        let result = mesh.handle_message(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_peer_management() {
        let mesh = TopicMesh::new("test_topic".to_string());
        let peer = vec![0x02; 33];

        mesh.update_peer_status(peer.clone(), true).await;
        let peers = mesh.get_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], peer);

        mesh.update_peer_status(peer.clone(), false).await;
        let peers = mesh.get_peers().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let mesh = TopicMesh::new("test_topic".to_string());

        for i in 0..5 {
            let mut message =
                GossipMessage::new(MessageType::TopicSync, vec![i as u8], vec![0x02; 33]);
            message.timestamp = i;
            mesh.handle_message(message).await.unwrap();
        }

        let stats = mesh.get_stats().await;
        assert_eq!(stats.peer_count, 1);
        assert_eq!(stats.message_count, 5);
        assert_eq!(stats.last_activity, 4);
    }

    #[tokio::test]
    async fn test_subscribe_and_receive_messages() {
        let mesh = TopicMesh::new("topic_subscribe".into());
        let mut subscription = mesh.subscribe().await;
        assert_eq!(mesh.subscriber_count().await, 1);

        let message = GossipMessage::new(MessageType::NostrEvent, vec![42, 24], vec![0x02; 33]);
        mesh.handle_message(message.clone()).await.unwrap();

        let received = subscription
            .receiver
            .recv()
            .await
            .expect("subscriber should receive message");
        assert_eq!(received.payload, message.payload);
        assert_eq!(received.msg_type as u8, MessageType::NostrEvent as u8);
    }

    #[tokio::test]
    async fn test_unsubscribe_removes_channel() {
        let mesh = TopicMesh::new("topic_unsubscribe".into());
        let subscription = mesh.subscribe().await;
        let subscription_id = subscription.id;
        assert_eq!(mesh.subscriber_count().await, 1);

        mesh.unsubscribe(subscription_id).await;
        assert_eq!(mesh.subscriber_count().await, 0);

        // Drop receiver without explicit unsubscribe: should be cleaned up on notify
        let subscription = mesh.subscribe().await;
        let dropped_id = subscription.id;
        drop(subscription);

        let message = GossipMessage::new(MessageType::TopicSync, vec![1, 2, 3], vec![0x02; 33]);
        mesh.handle_message(message).await.unwrap();

        // 送信エラー処理が走る時間を確保
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let subscribers = mesh.subscribers.read().await;
        assert!(
            !subscribers.contains_key(&dropped_id),
            "closed channel should be removed automatically"
        );
    }
}
