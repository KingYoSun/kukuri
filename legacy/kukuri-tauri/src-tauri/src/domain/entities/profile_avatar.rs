use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// プロフィールアバターの共有範囲
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAvatarAccessLevel {
    #[default]
    Public,
    ContactsOnly,
    Private,
}

impl ProfileAvatarAccessLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileAvatarAccessLevel::Public => "public",
            ProfileAvatarAccessLevel::ContactsOnly => "contacts_only",
            ProfileAvatarAccessLevel::Private => "private",
        }
    }
}

impl FromStr for ProfileAvatarAccessLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(ProfileAvatarAccessLevel::Public),
            "contacts_only" => Ok(ProfileAvatarAccessLevel::ContactsOnly),
            "private" => Ok(ProfileAvatarAccessLevel::Private),
            _ => Err("invalid access level"),
        }
    }
}

/// Doc に保存するプロフィールアバターのメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAvatarDocEntry {
    pub npub: String,
    pub blob_hash: String,
    pub format: String,
    pub size_bytes: u64,
    pub access_level: ProfileAvatarAccessLevel,
    pub share_ticket: String,
    pub encrypted_key: String,
    pub key_nonce: String,
    pub encryption_nonce: String,
    pub content_sha256: String,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
}
