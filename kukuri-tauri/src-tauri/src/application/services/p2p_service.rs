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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::p2p::{NetworkService, GossipService};
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};
    use std::collections::HashMap;

    // NetworkServiceのモック
    mock! {
        pub NetworkServ {}
        
        #[async_trait]
        impl NetworkService for NetworkServ {
            async fn connect(&self, address: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn get_node_id(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
            async fn get_addresses(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
            async fn is_connected(&self) -> bool;
        }
    }

    // GossipServiceのモック
    mock! {
        pub GossipServ {}
        
        #[async_trait]
        impl GossipService for GossipServ {
            async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn leave_topic(&self, topic_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn broadcast_message(&self, topic_id: &str, message: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn get_joined_topics(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
            async fn get_topic_peers(&self, topic_id: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
            async fn get_all_topics_stats(&self) -> Result<HashMap<String, usize>, Box<dyn std::error::Error + Send + Sync>>;
        }
    }

    #[tokio::test]
    async fn test_initialize() {
        let mock_network = MockNetworkServ::new();
        let mock_gossip = MockGossipServ::new();

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_join_topic_success() {
        let mock_network = MockNetworkServ::new();
        let mut mock_gossip = MockGossipServ::new();
        
        mock_gossip
            .expect_join_topic()
            .with(eq("test_topic"), eq(vec!["peer1".to_string(), "peer2".to_string()]))
            .times(1)
            .returning(|_, _| Ok(()));

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.join_topic("test_topic", vec!["peer1".to_string(), "peer2".to_string()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_join_topic_failure() {
        let mock_network = MockNetworkServ::new();
        let mut mock_gossip = MockGossipServ::new();
        
        mock_gossip
            .expect_join_topic()
            .with(eq("test_topic"), eq(vec![]))
            .times(1)
            .returning(|_, _| Err("Failed to join topic".into()));

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.join_topic("test_topic", vec![]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to join topic"));
    }

    #[tokio::test]
    async fn test_leave_topic() {
        let mock_network = MockNetworkServ::new();
        let mut mock_gossip = MockGossipServ::new();
        
        mock_gossip
            .expect_leave_topic()
            .with(eq("test_topic"))
            .times(1)
            .returning(|_| Ok(()));

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.leave_topic("test_topic").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let mock_network = MockNetworkServ::new();
        let mut mock_gossip = MockGossipServ::new();
        
        let test_content = "Test message";
        mock_gossip
            .expect_broadcast_message()
            .with(eq("test_topic"), eq(test_content.as_bytes()))
            .times(1)
            .returning(|_, _| Ok(()));

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.broadcast_message("test_topic", test_content).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_status() {
        let mut mock_network = MockNetworkServ::new();
        mock_network
            .expect_get_node_id()
            .times(1)
            .returning(|| Ok("node123".to_string()));

        let mut mock_gossip = MockGossipServ::new();
        mock_gossip
            .expect_get_joined_topics()
            .times(1)
            .returning(|| Ok(vec!["topic1".to_string(), "topic2".to_string()]));
        
        let mut stats = HashMap::new();
        stats.insert("topic1".to_string(), 5);
        stats.insert("topic2".to_string(), 3);
        
        mock_gossip
            .expect_get_all_topics_stats()
            .times(1)
            .returning(move || Ok(stats.clone()));

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.get_status().await;
        assert!(result.is_ok());
        
        let status = result.unwrap();
        assert_eq!(status.endpoint_id, "node123");
        assert!(status.connected);
        assert_eq!(status.active_topics.len(), 2);
        assert_eq!(status.peer_count, 8); // 5 + 3
    }

    #[tokio::test]
    async fn test_get_node_addresses() {
        let mut mock_network = MockNetworkServ::new();
        mock_network
            .expect_get_addresses()
            .times(1)
            .returning(|| Ok(vec![
                "/ip4/127.0.0.1/tcp/4001".to_string(),
                "/ip4/192.168.1.10/tcp/4001".to_string(),
            ]));

        let mock_gossip = MockGossipServ::new();

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let result = service.get_node_addresses().await;
        assert!(result.is_ok());
        
        let addresses = result.unwrap();
        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains(&"/ip4/127.0.0.1/tcp/4001".to_string()));
    }

    #[tokio::test]
    async fn test_generate_topic_id() {
        let mock_network = MockNetworkServ::new();
        let mock_gossip = MockGossipServ::new();

        let service = P2PService::new(
            Arc::new(mock_network),
            Arc::new(mock_gossip),
        );

        let topic_id1 = service.generate_topic_id("test_topic");
        let topic_id2 = service.generate_topic_id("test_topic");
        let topic_id3 = service.generate_topic_id("different_topic");
        
        // 同じトピック名から同じIDが生成される
        assert_eq!(topic_id1, topic_id2);
        // 異なるトピック名からは異なるIDが生成される
        assert_ne!(topic_id1, topic_id3);
    }
}