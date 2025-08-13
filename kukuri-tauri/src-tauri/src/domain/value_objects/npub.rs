use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Npub(String);

impl Npub {
    pub fn new(value: String) -> Result<Self, String> {
        if !value.starts_with("npub1") {
            return Err("Invalid npub format: must start with 'npub1'".to_string());
        }
        if value.len() != 63 {
            return Err("Invalid npub format: incorrect length".to_string());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_pubkey(&self) -> Result<String, String> {
        // This would normally use bech32 decoding
        // For now, return a placeholder
        Ok(format!("pubkey_from_{}", self.0))
    }
}

impl fmt::Display for Npub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Npub> for String {
    fn from(npub: Npub) -> Self {
        npub.0
    }
}