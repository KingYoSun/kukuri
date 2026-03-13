use crate::domain::constants::DEFAULT_PUBLIC_TOPIC_ID;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopicId(String);

impl TopicId {
    pub fn new(value: String) -> Result<Self, String> {
        if value.is_empty() {
            return Err("Topic ID cannot be empty".to_string());
        }
        Ok(Self(value))
    }

    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn public() -> Self {
        Self(DEFAULT_PUBLIC_TOPIC_ID.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_public(&self) -> bool {
        self.0 == DEFAULT_PUBLIC_TOPIC_ID
    }
}

impl fmt::Display for TopicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TopicId> for String {
    fn from(id: TopicId) -> Self {
        id.0
    }
}

impl Default for TopicId {
    fn default() -> Self {
        Self::public()
    }
}
