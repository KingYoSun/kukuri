use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    SentToNostr,
    SentToP2P,
    FullySynced,
    Failed,
    Conflict,
    Unknown(String),
}

impl SyncStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SyncStatus::Pending => "pending",
            SyncStatus::SentToNostr => "sent_to_nostr",
            SyncStatus::SentToP2P => "sent_to_p2p",
            SyncStatus::FullySynced => "fully_synced",
            SyncStatus::Failed => "failed",
            SyncStatus::Conflict => "conflict",
            SyncStatus::Unknown(value) => value.as_str(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SyncStatus::FullySynced | SyncStatus::Failed | SyncStatus::Conflict
        )
    }
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for SyncStatus {
    fn from(value: &str) -> Self {
        match value {
            "pending" => SyncStatus::Pending,
            "sent_to_nostr" => SyncStatus::SentToNostr,
            "sent_to_p2p" => SyncStatus::SentToP2P,
            "fully_synced" => SyncStatus::FullySynced,
            "failed" => SyncStatus::Failed,
            "conflict" => SyncStatus::Conflict,
            other => SyncStatus::Unknown(other.to_string()),
        }
    }
}
