use crate::application::services::p2p_service::{
    ConnectionStatus as ServiceConnectionStatus, PeerStatus as ServicePeerStatus,
};
use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatusResponse {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

impl From<ServiceConnectionStatus> for ConnectionStatusResponse {
    fn from(value: ServiceConnectionStatus) -> Self {
        match value {
            ServiceConnectionStatus::Connected => ConnectionStatusResponse::Connected,
            ServiceConnectionStatus::Connecting => ConnectionStatusResponse::Connecting,
            ServiceConnectionStatus::Disconnected => ConnectionStatusResponse::Disconnected,
            ServiceConnectionStatus::Error => ConnectionStatusResponse::Error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStatusResponse {
    pub node_id: String,
    pub address: String,
    pub connected_at: i64,
    pub last_seen: i64,
}

impl From<ServicePeerStatus> for PeerStatusResponse {
    fn from(value: ServicePeerStatus) -> Self {
        Self {
            node_id: value.node_id,
            address: value.address,
            connected_at: value.connected_at,
            last_seen: value.last_seen,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PStatusResponse {
    pub connected: bool,
    pub connection_status: ConnectionStatusResponse,
    pub endpoint_id: String,
    pub active_topics: Vec<TopicStatus>,
    pub peer_count: usize,
    pub peers: Vec<PeerStatusResponse>,
    pub metrics_summary: GossipMetricsSummaryResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStatus {
    pub topic_id: String,
    pub peer_count: usize,
    pub message_count: usize,
    pub last_activity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinTopicRequest {
    pub topic_id: String,
    pub initial_peers: Vec<String>,
}

impl Validate for JoinTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.is_empty() {
            return Err("Topic ID is required".to_string());
        }
        // 初期ピアのフォーマット検証は省略（実際には必要に応じて追加）
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveTopicRequest {
    pub topic_id: String,
}

impl Validate for LeaveTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.is_empty() {
            return Err("Topic ID is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastRequest {
    pub topic_id: String,
    pub content: String,
}

impl Validate for BroadcastRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.is_empty() {
            return Err("Topic ID is required".to_string());
        }
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }
        if self.content.len() > 50000 {
            return Err("Content is too large (max 50000 bytes)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAddressResponse {
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfigResponse {
    pub mode: String,
    pub nodes: Vec<String>,
    pub effective_nodes: Vec<String>,
    pub source: String,
    pub env_locked: bool,
    #[serde(default)]
    pub cli_nodes: Vec<String>,
    #[serde(default)]
    pub cli_updated_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStatusResponse {
    pub url: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMetricsResponse {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
    pub join_details: GossipMetricDetailsResponse,
    pub leave_details: GossipMetricDetailsResponse,
    pub broadcast_details: GossipMetricDetailsResponse,
    pub receive_details: GossipMetricDetailsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMetricDetailsResponse {
    pub total: u64,
    pub failures: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainlineMetricsResponse {
    pub connected_peers: u64,
    pub connection_attempts: u64,
    pub connection_successes: u64,
    pub connection_failures: u64,
    pub connection_last_success_ms: Option<u64>,
    pub connection_last_failure_ms: Option<u64>,
    pub routing_attempts: u64,
    pub routing_successes: u64,
    pub routing_failures: u64,
    pub routing_success_rate: f64,
    pub routing_last_success_ms: Option<u64>,
    pub routing_last_failure_ms: Option<u64>,
    pub reconnect_attempts: u64,
    pub reconnect_successes: u64,
    pub reconnect_failures: u64,
    pub last_reconnect_success_ms: Option<u64>,
    pub last_reconnect_failure_ms: Option<u64>,
    pub bootstrap: BootstrapMetricsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PMetricsResponse {
    pub gossip: GossipMetricsResponse,
    pub mainline: MainlineMetricsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapMetricsResponse {
    pub env_uses: u64,
    pub user_uses: u64,
    pub bundle_uses: u64,
    pub fallback_uses: u64,
    pub last_source: Option<String>,
    pub last_applied_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMetricsSummaryResponse {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
}
