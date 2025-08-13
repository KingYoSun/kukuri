use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct P2PStatusResponse {
    pub node_id: String,
    pub connected_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub stats: P2PStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct P2PStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}