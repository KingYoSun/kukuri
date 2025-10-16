use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PStatusResponse {
    pub connected: bool,
    pub endpoint_id: String,
    pub active_topics: Vec<TopicStatus>,
    pub peer_count: usize,
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
pub struct GossipMetricsSummaryResponse {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinTopicByNameRequest {
    pub topic_name: String,
    pub initial_peers: Vec<String>,
}

impl Validate for JoinTopicByNameRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_name.is_empty() {
            return Err("Topic name is required".to_string());
        }
        if self.topic_name.len() > 100 {
            return Err("Topic name is too long (max 100 characters)".to_string());
        }
        Ok(())
    }
}
