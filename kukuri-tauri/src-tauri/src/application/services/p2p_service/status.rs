use serde::{Deserialize, Serialize};

use super::metrics::GossipMetricsSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStatus {
    pub node_id: String,
    pub address: String,
    pub connected_at: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PStatus {
    pub connected: bool,
    pub connection_status: ConnectionStatus,
    pub endpoint_id: String,
    pub active_topics: Vec<TopicInfo>,
    pub peer_count: usize,
    pub peers: Vec<PeerStatus>,
    pub metrics_summary: GossipMetricsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    pub id: String,
    pub peer_count: usize,
    pub message_count: usize,
    pub last_activity: i64,
}
