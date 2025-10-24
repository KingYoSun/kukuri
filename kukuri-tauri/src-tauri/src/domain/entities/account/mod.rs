use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SecureStorage に保存されるアカウント情報を表現するドメインエンティティ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub npub: String,
    pub pubkey: String,
    pub name: String,
    pub display_name: String,
    pub picture: Option<String>,
    pub last_used: DateTime<Utc>,
}

impl AccountMetadata {
    pub fn mark_used(&mut self, timestamp: DateTime<Utc>) {
        self.last_used = timestamp;
    }
}

/// SecureStorage に保存されるアカウント一覧と現在アカウントのメタデータ。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountsMetadata {
    pub accounts: HashMap<String, AccountMetadata>,
    pub current_npub: Option<String>,
}

/// アカウント登録時に必要な情報。
#[derive(Debug, Clone)]
pub struct AccountRegistration {
    pub npub: String,
    pub nsec: String,
    pub pubkey: String,
    pub name: String,
    pub display_name: String,
    pub picture: Option<String>,
}

impl AccountRegistration {
    pub fn into_metadata(self) -> (AccountMetadata, String) {
        let AccountRegistration {
            npub,
            nsec,
            pubkey,
            name,
            display_name,
            picture,
        } = self;

        let metadata = AccountMetadata {
            npub: npub.clone(),
            pubkey,
            name,
            display_name,
            picture,
            last_used: Utc::now(),
        };

        (metadata, nsec)
    }
}

/// 現在のアカウントのメタデータと秘密鍵。
#[derive(Debug, Clone)]
pub struct CurrentAccountSecret {
    pub metadata: AccountMetadata,
    pub nsec: String,
}
