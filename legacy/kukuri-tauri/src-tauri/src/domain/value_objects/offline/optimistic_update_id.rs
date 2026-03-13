use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OptimisticUpdateId(String);

impl OptimisticUpdateId {
    pub fn new(value: String) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("Optimistic update ID cannot be empty".to_string());
        }
        Ok(())
    }
}

impl fmt::Display for OptimisticUpdateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<OptimisticUpdateId> for String {
    fn from(value: OptimisticUpdateId) -> Self {
        value.0
    }
}
