use crate::shared::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub address: String,
    pub connected_at: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub bandwidth_up: u64,
    pub bandwidth_down: u64,
}

#[async_trait]
pub trait NetworkService: Send + Sync {
    // Type conversion helper for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    async fn connect(&self) -> Result<(), AppError>;
    async fn disconnect(&self) -> Result<(), AppError>;
    async fn get_peers(&self) -> Result<Vec<Peer>, AppError>;
    async fn add_peer(&self, address: &str) -> Result<(), AppError>;
    async fn remove_peer(&self, peer_id: &str) -> Result<(), AppError>;
    async fn get_stats(&self) -> Result<NetworkStats, AppError>;
    async fn is_connected(&self) -> bool;
    async fn get_node_id(&self) -> Result<String, AppError>;
    async fn get_addresses(&self) -> Result<Vec<String>, AppError>;

    async fn join_dht_topic(&self, _topic: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn leave_dht_topic(&self, _topic: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn broadcast_dht(&self, _topic: &str, _message: Vec<u8>) -> Result<(), AppError> {
        Ok(())
    }
}
