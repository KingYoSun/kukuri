use serde::{Deserialize, Serialize};
use std::fmt;

/// オフラインアクションの種類（例: `publish_text_note`、`send_reaction`）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OfflineActionType(String);

impl OfflineActionType {
    pub fn new(value: String) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("Offline action type cannot be empty".to_string());
        }
        Ok(())
    }
}

impl fmt::Display for OfflineActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<OfflineActionType> for String {
    fn from(kind: OfflineActionType) -> Self {
        kind.0
    }
}
