use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// トピック投稿など、長文コンテンツを扱う値オブジェクト。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicContent(String);

impl TopicContent {
    const MAX_LENGTH: usize = 10_000;

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

    pub fn into_string(self) -> String {
        self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("Content cannot be empty".to_string());
        }
        if value.chars().count() > Self::MAX_LENGTH {
            return Err(format!(
                "Content is too long (max {} characters)",
                Self::MAX_LENGTH
            ));
        }
        Ok(())
    }
}

impl fmt::Display for TopicContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TopicContent> for String {
    fn from(value: TopicContent) -> Self {
        value.0
    }
}

impl TryFrom<&str> for TopicContent {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl FromStr for TopicContent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
