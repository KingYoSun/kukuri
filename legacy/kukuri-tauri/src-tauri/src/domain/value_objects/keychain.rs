use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const LEDGER_VERSION: u32 = 1;

/// 個々の鍵素材を表すレコード。公開情報のみを保持する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMaterialRecord {
    pub npub: String,
    pub public_key: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
}

impl KeyMaterialRecord {
    pub fn new(npub: String, public_key: String) -> Self {
        let now = Utc::now();
        Self {
            npub,
            public_key,
            created_at: now,
            last_used: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_used = Utc::now();
    }
}

/// KeyManager が参照する鍵素材の台帳。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMaterialLedger {
    pub version: u32,
    pub records: HashMap<String, KeyMaterialRecord>,
    pub current_npub: Option<String>,
}

impl Default for KeyMaterialLedger {
    fn default() -> Self {
        Self {
            version: LEDGER_VERSION,
            records: HashMap::new(),
            current_npub: None,
        }
    }
}

impl KeyMaterialLedger {
    pub fn upsert(&mut self, record: KeyMaterialRecord) {
        let npub = record.npub.clone();
        self.records.insert(npub, record);
    }

    pub fn remove(&mut self, npub: &str) -> bool {
        let removed = self.records.remove(npub).is_some();
        if removed && self.current_npub.as_deref() == Some(npub) {
            self.current_npub = None;
        }
        removed
    }

    pub fn touch_current(&mut self, npub: &str) {
        if let Some(record) = self.records.get_mut(npub) {
            record.touch();
            self.current_npub = Some(npub.to_string());
        }
    }
}
