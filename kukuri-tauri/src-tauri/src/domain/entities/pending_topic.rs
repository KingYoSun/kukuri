use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PendingTopicStatus {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "synced")]
    Synced,
    #[serde(rename = "failed")]
    Failed,
}

impl PendingTopicStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PendingTopicStatus::Queued => "queued",
            PendingTopicStatus::Synced => "synced",
            PendingTopicStatus::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "synced" => PendingTopicStatus::Synced,
            "failed" => PendingTopicStatus::Failed,
            _ => PendingTopicStatus::Queued,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTopic {
    pub pending_id: String,
    pub user_pubkey: String,
    pub name: String,
    pub description: Option<String>,
    pub status: PendingTopicStatus,
    pub offline_action_id: String,
    pub synced_topic_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PendingTopic {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pending_id: String,
        user_pubkey: String,
        name: String,
        description: Option<String>,
        status: PendingTopicStatus,
        offline_action_id: String,
        synced_topic_id: Option<String>,
        error_message: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            pending_id,
            user_pubkey,
            name,
            description,
            status,
            offline_action_id,
            synced_topic_id,
            error_message,
            created_at,
            updated_at,
        }
    }

    pub fn with_status(mut self, status: PendingTopicStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }
}
