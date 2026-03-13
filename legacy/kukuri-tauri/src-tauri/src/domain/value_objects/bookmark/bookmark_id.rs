use serde::{Deserialize, Serialize};
use std::fmt;

/// Bookmark エンティティの識別子。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookmarkId(String);

impl BookmarkId {
    /// 既存の識別子文字列から `BookmarkId` を生成する。
    pub fn new(value: String) -> Result<Self, String> {
        if value.is_empty() {
            return Err("BookmarkId cannot be empty".to_string());
        }
        uuid::Uuid::parse_str(&value).map_err(|err| format!("Invalid BookmarkId format: {err}"))?;
        Ok(Self(value))
    }

    /// 新規 BookmarkId を生成する。
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// 内部の文字列を参照する。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BookmarkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<BookmarkId> for String {
    fn from(value: BookmarkId) -> Self {
        value.0
    }
}
