use crate::shared::validation::ValidationFailureKind;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    SentToNostr,
    SentToP2P,
    FullySynced,
    Failed,
    Conflict,
    Invalid(ValidationFailureKind),
    Unknown(String),
}

impl SyncStatus {
    pub fn as_str(&self) -> Cow<'static, str> {
        match self {
            SyncStatus::Pending => Cow::Borrowed("pending"),
            SyncStatus::SentToNostr => Cow::Borrowed("sent_to_nostr"),
            SyncStatus::SentToP2P => Cow::Borrowed("sent_to_p2p"),
            SyncStatus::FullySynced => Cow::Borrowed("fully_synced"),
            SyncStatus::Failed => Cow::Borrowed("failed"),
            SyncStatus::Conflict => Cow::Borrowed("conflict"),
            SyncStatus::Invalid(kind) => Cow::Owned(format!("invalid:{}", kind.as_str())),
            SyncStatus::Unknown(value) => Cow::Owned(value.clone()),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SyncStatus::FullySynced
                | SyncStatus::Failed
                | SyncStatus::Conflict
                | SyncStatus::Invalid(_)
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
            value if value.starts_with("invalid:") => {
                let reason = &value[8..];
                let kind = reason.parse().unwrap_or(ValidationFailureKind::Generic);
                SyncStatus::Invalid(kind)
            }
            other => SyncStatus::Unknown(other.to_string()),
        }
    }
}
