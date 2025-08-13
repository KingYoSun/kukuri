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
    
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_peers(&self) -> Result<Vec<Peer>, Box<dyn std::error::Error + Send + Sync>>;
    async fn add_peer(&self, address: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn remove_peer(&self, peer_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_stats(&self) -> Result<NetworkStats, Box<dyn std::error::Error + Send + Sync>>;
    async fn is_connected(&self) -> bool;
}