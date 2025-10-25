use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// リアクション（例: 👍, ❤️）の値を表現する値オブジェクト。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionValue(String);

impl ReactionValue {
    const MAX_LENGTH: usize = 20;

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
            return Err("Reaction cannot be empty".to_string());
        }
        if value.chars().count() > Self::MAX_LENGTH {
            return Err(format!(
                "Reaction is too long (max {} characters)",
                Self::MAX_LENGTH
            ));
        }
        Ok(())
    }
}

impl fmt::Display for ReactionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ReactionValue> for String {
    fn from(value: ReactionValue) -> Self {
        value.0
    }
}

impl TryFrom<&str> for ReactionValue {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl FromStr for ReactionValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
