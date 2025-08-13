use crate::shared::error::AppError;
use crate::infrastructure::p2p::{NetworkService, GossipService};
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// P2Pネットワークのステータス情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PStatus {
    pub connected: bool,
    pub endpoint_id: String,
    pub active_topics: Vec<TopicInfo>,
    pub peer_count: usize,
}

/// トピック情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    pub id: String,
    pub peer_count: usize,
    pub message_count: usize,
    pub last_activity: i64,
}

/// P2Pサービスのトレイト
#[async_trait]
pub trait P2PServiceTrait: Send + Sync {
    /// P2Pネットワークを初期化
    async fn initialize(&self) -> Result<(), AppError>;
    
    /// トピックに参加
    async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
    
    /// トピックから離脱
    async fn leave_topic(&self, topic_id: &str) -> Result<(), AppError>;
    
    /// メッセージをブロードキャスト
    async fn broadcast_message(&self, topic_id: &str, content: &str) -> Result<(), AppError>;
    
    /// P2Pステータスを取得
    async fn get_status(&self) -> Result<P2PStatus, AppError>;
    
    /// ノードアドレスを取得
    async fn get_node_addresses(&self) -> Result<Vec<String>, AppError>;
    
    /// トピックIDを生成
    fn generate_topic_id(&self, topic_name: &str) -> String;
}

/// P2Pサービスの実装
pub struct P2PService {
    network_service: Arc<dyn NetworkService>,
    gossip_service: Arc<dyn GossipService>,
}

impl P2PService {
    pub fn new(
        network_service: Arc<dyn NetworkService>,
        gossip_service: Arc<dyn GossipService>,
    ) -> Self {
        Self {
            network_service,
            gossip_service,
        }
    }
}

#[async_trait]
impl P2PServiceTrait for P2PService {
    async fn initialize(&self) -> Result<(), AppError> {
        // P2Pネットワークの初期化処理
        // 既にstate.rsのinitialize_p2pで初期化されている場合はチェックのみ
        Ok(())
    }
    
    async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), AppError> {
        self.gossip_service.join_topic(topic_id, initial_peers).await
            .map_err(|e| AppError::P2PError(e.to_string()))
    }
    
    async fn leave_topic(&self, topic_id: &str) -> Result<(), AppError> {
        self.gossip_service.leave_topic(topic_id).await
            .map_err(|e| AppError::P2PError(e.to_string()))
    }
    
    async fn broadcast_message(&self, topic_id: &str, content: &str) -> Result<(), AppError> {
        self.gossip_service.broadcast_message(topic_id, content.as_bytes()).await
            .map_err(|e| AppError::P2PError(e.to_string()))
    }
    
    async fn get_status(&self) -> Result<P2PStatus, AppError> {
        // ステータス情報を収集
        let endpoint_id = self.network_service.get_node_id().await
            .map_err(|e| AppError::P2PError(e.to_string()))?;
            
        // 実際のトピック情報を取得
        let joined_topics = self.gossip_service.get_joined_topics().await
            .map_err(|e| AppError::P2PError(e.to_string()))?;
        
        let mut active_topics = Vec::new();
        let mut total_peer_count = 0;
        
        for topic_id in joined_topics {
            let peers = self.gossip_service.get_topic_peers(&topic_id).await
                .map_err(|e| AppError::P2PError(e.to_string()))?;
            let peer_count = peers.len();
            total_peer_count += peer_count;
            
            active_topics.push(TopicInfo {
                id: topic_id,
                peer_count,
                message_count: 0, // TODO: メッセージカウントの実装
                last_activity: chrono::Utc::now().timestamp(),
            });
        }
        
        Ok(P2PStatus {
            connected: true,
            endpoint_id,
            active_topics,
            peer_count: total_peer_count,
        })
    }
    
    async fn get_node_addresses(&self) -> Result<Vec<String>, AppError> {
        self.network_service.get_addresses().await
            .map_err(|e| AppError::P2PError(e.to_string()))
    }
    
    fn generate_topic_id(&self, topic_name: &str) -> String {
        // トピック名からIDを生成（例：ハッシュを使用）
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(topic_name.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}