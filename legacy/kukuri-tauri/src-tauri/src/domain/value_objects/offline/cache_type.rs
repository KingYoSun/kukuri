use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheType(String);

impl CacheType {
    pub fn new(value: String) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("Cache type cannot be empty".to_string());
        }
        Ok(())
    }
}

impl fmt::Display for CacheType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CacheType> for String {
    fn from(value: CacheType) -> Self {
        value.0
    }
}
