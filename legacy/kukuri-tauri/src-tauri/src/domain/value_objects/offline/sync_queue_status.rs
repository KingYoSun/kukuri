use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncQueueStatus {
    Pending,
    Processing,
    Failed,
    Completed,
    Unknown(String),
}

impl SyncQueueStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SyncQueueStatus::Pending => "pending",
            SyncQueueStatus::Processing => "processing",
            SyncQueueStatus::Failed => "failed",
            SyncQueueStatus::Completed => "completed",
            SyncQueueStatus::Unknown(value) => value.as_str(),
        }
    }
}

impl From<&str> for SyncQueueStatus {
    fn from(value: &str) -> Self {
        match value {
            "pending" => SyncQueueStatus::Pending,
            "processing" => SyncQueueStatus::Processing,
            "failed" => SyncQueueStatus::Failed,
            "completed" => SyncQueueStatus::Completed,
            other => SyncQueueStatus::Unknown(other.to_string()),
        }
    }
}
