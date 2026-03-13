use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// アプリケーションが参照するオフラインアクションの識別子（`local_id` 相当）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OfflineActionId(String);

impl OfflineActionId {
    pub fn new(value: String) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("Offline action ID cannot be empty".to_string());
        }
        Ok(())
    }
}

impl fmt::Display for OfflineActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<OfflineActionId> for String {
    fn from(id: OfflineActionId) -> Self {
        id.0
    }
}

impl FromStr for OfflineActionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
