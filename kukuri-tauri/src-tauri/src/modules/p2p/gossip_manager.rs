use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use iroh::{Endpoint, protocol::Router, Watcher};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN_BYTES};
use iroh::SecretKey;

use crate::modules::p2p::error::{P2PError, Result as P2PResult};
use crate::modules::p2p::message::GossipMessage;


pub struct GossipManager {
    endpoint: Endpoint,
    gossip: Gossip,
    router: Router,
    topics: Arc<RwLock<HashMap<String, TopicHandle>>>,
    secret_key: SecretKey,
}

pub struct TopicHandle {
    topic_id: String,
    // gossip topic subscription will be added here
}

impl GossipManager {
    /// 新しいGossipManagerを作成
    pub async fn new(secret_key: SecretKey) -> P2PResult<Self> {
        // Endpointの作成
        let endpoint = Endpoint::builder()
            .secret_key(secret_key.clone())
            .discovery_n0()
            .bind()
            .await
            .map_err(|e| P2PError::EndpointInit(e.to_string()))?;
        
        // Gossipインスタンスの作成
        let gossip = Gossip::builder()
            .spawn(endpoint.clone());
        
        // Routerの作成
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN_BYTES.to_vec(), gossip.clone())
            .spawn();
        
        Ok(Self {
            endpoint,
            gossip,
            router,
            topics: Arc::new(RwLock::new(HashMap::new())),
            secret_key,
        })
    }
    
    /// 自身のNodeIDを取得
    pub fn node_id(&self) -> String {
        self.endpoint.node_id().to_string()
    }
    
    /// 自身のアドレス情報を取得
    pub async fn node_addr(&self) -> P2PResult<Vec<String>> {
        let node_addr = self.endpoint.node_addr();
        let addrs = match node_addr.get() {
            Ok(Some(addr)) => addr,
            Ok(None) => return Err(P2PError::Internal("Node address not available".to_string())),
            Err(e) => return Err(P2PError::Internal(format!("Failed to get node address: {}", e))),
        };
        
        Ok(addrs
            .direct_addresses()
            .map(|addr| addr.to_string())
            .collect())
    }
    
    /// トピックに参加
    pub async fn join_topic(&self, topic_id: &str, _initial_peers: Vec<String>) -> P2PResult<()> {
        let mut topics = self.topics.write().await;
        
        if topics.contains_key(topic_id) {
            return Ok(()); // 既に参加済み
        }
        
        // TODO: iroh-gossipのsubscribe実装
        let handle = TopicHandle {
            topic_id: topic_id.to_string(),
        };
        
        topics.insert(topic_id.to_string(), handle);
        
        tracing::info!("Joined topic: {}", topic_id);
        Ok(())
    }
    
    /// トピックから離脱
    pub async fn leave_topic(&self, topic_id: &str) -> P2PResult<()> {
        let mut topics = self.topics.write().await;
        
        if topics.remove(topic_id).is_none() {
            return Err(P2PError::TopicNotFound(topic_id.to_string()));
        }
        
        tracing::info!("Left topic: {}", topic_id);
        Ok(())
    }
    
    /// メッセージをブロードキャスト
    pub async fn broadcast(&self, topic_id: &str, _message: GossipMessage) -> P2PResult<()> {
        let topics = self.topics.read().await;
        
        if !topics.contains_key(topic_id) {
            return Err(P2PError::TopicNotFound(topic_id.to_string()));
        }
        
        // TODO: iroh-gossipのbroadcast実装
        
        tracing::debug!("Broadcast message to topic: {}", topic_id);
        Ok(())
    }
    
    /// アクティブなトピックのリストを取得
    pub async fn active_topics(&self) -> Vec<String> {
        let topics = self.topics.read().await;
        topics.keys().cloned().collect()
    }
    
    /// シャットダウン
    pub async fn shutdown(&self) -> P2PResult<()> {
        // すべてのトピックから離脱
        let topic_ids: Vec<String> = {
            let topics = self.topics.read().await;
            topics.keys().cloned().collect()
        };
        
        for topic_id in topic_ids {
            let _ = self.leave_topic(&topic_id).await;
        }
        
        // Routerとエンドポイントのシャットダウン
        self.router.shutdown().await;
        self.endpoint.close().await;
        
        tracing::info!("GossipManager shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_gossip_manager_initialization() {
        let secret_key = SecretKey::generate(rand::thread_rng());
        let manager = GossipManager::new(secret_key).await;
        assert!(manager.is_ok());
    }
}