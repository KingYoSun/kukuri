use std::collections::HashSet;
use std::sync::Arc;
use std::num::NonZeroUsize;
use tokio::sync::RwLock;
use lru::LruCache;

use crate::modules::p2p::message::{GossipMessage, MessageId};
use crate::modules::p2p::error::Result as P2PResult;

pub struct TopicMesh {
    topic_id: String,
    // TODO: iroh-gossipのsubscription実装
    peers: Arc<RwLock<HashSet<Vec<u8>>>>, // PublicKeyのバイト表現
    message_cache: Arc<RwLock<LruCache<MessageId, GossipMessage>>>,
}

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
            topic_id,
            peers: Arc::new(RwLock::new(HashSet::new())),
            message_cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
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
    pub async fn get_peers(&self) -> Vec<Vec<u8>> {
        let peers = self.peers.read().await;
        peers.iter().cloned().collect()
    }
    
    /// キャッシュされたメッセージを取得（最新順）
    pub async fn get_recent_messages(&self, limit: usize) -> Vec<GossipMessage> {
        let cache = self.message_cache.read().await;
        let mut messages: Vec<_> = cache.iter()
            .map(|(_, msg)| msg.clone())
            .collect();
        
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        messages.into_iter().take(limit).collect()
    }
    
    /// キャッシュをクリア
    pub async fn clear_cache(&self) {
        let mut cache = self.message_cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::p2p::message::MessageType;
    
    #[tokio::test]
    async fn test_duplicate_detection() {
        let mesh = TopicMesh::new("test-topic".to_string());
        
        let message = GossipMessage::new(
            MessageType::NostrEvent,
            vec![1, 2, 3],
            vec![0; 33], // 公開鍵は33バイト
        );
        
        // 最初のメッセージは重複ではない
        assert!(!mesh.is_duplicate(&message.id).await);
        
        // メッセージを処理
        mesh.handle_message(message.clone()).await.unwrap();
        
        // 同じメッセージは重複として検出される
        assert!(mesh.is_duplicate(&message.id).await);
    }
    
    #[tokio::test]
    async fn test_peer_management() {
        let mesh = TopicMesh::new("test-topic".to_string());
        
        let peer1 = vec![1; 33];
        let peer2 = vec![2; 33];
        
        // ピアを追加
        mesh.update_peer_status(peer1.clone(), true).await;
        mesh.update_peer_status(peer2.clone(), true).await;
        
        let peers = mesh.get_peers().await;
        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&peer1));
        assert!(peers.contains(&peer2));
        
        // ピアを削除
        mesh.update_peer_status(peer1.clone(), false).await;
        
        let peers = mesh.get_peers().await;
        assert_eq!(peers.len(), 1);
        assert!(!peers.contains(&peer1));
        assert!(peers.contains(&peer2));
    }
    
    #[tokio::test]
    async fn test_message_cache() {
        let mesh = TopicMesh::new("test-topic".to_string());
        
        // 複数のメッセージを追加
        for i in 0..5 {
            let message = GossipMessage::new(
                MessageType::NostrEvent,
                vec![i],
                vec![i; 33],
            );
            mesh.handle_message(message).await.unwrap();
        }
        
        // 統計情報を確認
        let stats = mesh.get_stats().await;
        assert_eq!(stats.message_count, 5);
        assert_eq!(stats.peer_count, 5); // 各メッセージは異なる送信者から
        
        // 最新のメッセージを取得
        let recent = mesh.get_recent_messages(3).await;
        assert_eq!(recent.len(), 3);
        
        // キャッシュをクリア
        mesh.clear_cache().await;
        let stats = mesh.get_stats().await;
        assert_eq!(stats.message_count, 0);
    }
}