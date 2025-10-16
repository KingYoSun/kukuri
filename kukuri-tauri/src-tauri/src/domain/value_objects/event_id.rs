use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    pub fn new(value: String) -> Result<Self, String> {
        if value.is_empty() {
            return Err("Event ID cannot be empty".to_string());
        }
        // Validate hex format (64 characters)
        if value.len() != 64 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid event ID format: must be 64 hex characters".to_string());
        }
        Ok(Self(value))
    }

    pub fn generate() -> Self {
        use sha2::{Digest, Sha256};
        let random_bytes = uuid::Uuid::new_v4().as_bytes().to_vec();
        let hash = Sha256::digest(&random_bytes);
        Self(format!("{hash:x}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_hex(hex: &str) -> Result<Self, String> {
        Self::new(hex.to_string())
    }

    pub fn to_hex(&self) -> String {
        self.0.clone()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<EventId> for String {
    fn from(id: EventId) -> Self {
        id.0
    }
}
