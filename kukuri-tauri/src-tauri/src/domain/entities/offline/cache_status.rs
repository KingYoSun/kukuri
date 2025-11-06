use crate::domain::value_objects::{CacheKey, CacheType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheTypeStatus {
    pub cache_type: CacheType,
    pub item_count: u64,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub is_stale: bool,
}

impl CacheTypeStatus {
    pub fn new(
        cache_type: CacheType,
        item_count: u64,
        last_synced_at: Option<DateTime<Utc>>,
        is_stale: bool,
    ) -> Self {
        Self {
            cache_type,
            item_count,
            last_synced_at,
            is_stale,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheStatusSnapshot {
    pub total_items: u64,
    pub stale_items: u64,
    pub cache_types: Vec<CacheTypeStatus>,
}

impl CacheStatusSnapshot {
    pub fn new(total_items: u64, stale_items: u64, cache_types: Vec<CacheTypeStatus>) -> Self {
        Self {
            total_items,
            stale_items,
            cache_types,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheMetadataUpdate {
    pub cache_key: CacheKey,
    pub cache_type: CacheType,
    pub metadata: Option<Value>,
    pub expiry: Option<DateTime<Utc>>,
    pub is_stale: Option<bool>,
}
