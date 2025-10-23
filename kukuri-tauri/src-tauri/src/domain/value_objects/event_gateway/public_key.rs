use serde::{Deserialize, Serialize};
use std::fmt;

/// Nostr の公開鍵（hex 64文字）を表現する値オブジェクト。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(String);

impl PublicKey {
    /// 64桁の16進文字列から `PublicKey` を生成する。
    pub fn new(value: String) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// 64桁の16進文字列から `PublicKey` を生成する。
    pub fn from_hex_str(value: &str) -> Result<Self, String> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }

    /// 内部の16進文字列を参照で取得する。
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), String> {
        if value.len() != 64 {
            return Err("Public key must be 64 hex characters".to_string());
        }
        if !value.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Public key must contain only hex characters".to_string());
        }
        Ok(())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<PublicKey> for String {
    fn from(value: PublicKey) -> Self {
        value.0
    }
}

impl TryFrom<&str> for PublicKey {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_hex_str(value)
    }
}
