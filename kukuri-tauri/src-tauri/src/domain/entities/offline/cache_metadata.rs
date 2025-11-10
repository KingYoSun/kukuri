use crate::domain::value_objects::{CacheKey, CacheType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheMetadataRecord {
    pub record_id: i64,
    pub cache_key: CacheKey,
    pub cache_type: CacheType,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub data_version: i32,
    pub is_stale: bool,
    pub expiry_time: Option<DateTime<Utc>>,
    pub metadata: Option<Value>,
    pub doc_version: Option<i64>,
    pub blob_hash: Option<String>,
    pub payload_bytes: Option<i64>,
}
