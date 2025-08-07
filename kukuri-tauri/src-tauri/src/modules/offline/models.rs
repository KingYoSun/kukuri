use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncQueueItem {
    pub id: i64,
    pub action_type: String,
    pub payload: String,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub synced_at: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OfflineAction {
    pub id: i64,
    pub user_pubkey: String,
    pub action_type: String,
    pub target_id: Option<String>,
    pub action_data: String,
    pub local_id: String,
    pub remote_id: Option<String>,
    pub is_synced: bool,
    pub created_at: i64,
    pub synced_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CacheMetadata {
    pub id: i64,
    pub cache_key: String,
    pub cache_type: String,
    pub last_synced_at: Option<i64>,
    pub last_accessed_at: Option<i64>,
    pub data_version: i32,
    pub is_stale: bool,
    pub expiry_time: Option<i64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OptimisticUpdate {
    pub id: i64,
    pub update_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub original_data: Option<String>,
    pub updated_data: String,
    pub is_confirmed: bool,
    pub created_at: i64,
    pub confirmed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncStatus {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub local_version: i32,
    pub remote_version: Option<i32>,
    pub last_local_update: i64,
    pub last_remote_sync: Option<i64>,
    pub sync_status: String,
    pub conflict_data: Option<String>,
}

// API用のリクエスト/レスポンス型

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOfflineActionRequest {
    pub user_pubkey: String,
    pub action_type: String,
    pub target_id: Option<String>,
    pub action_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOfflineActionResponse {
    pub local_id: String,
    pub action: OfflineAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOfflineActionsRequest {
    pub user_pubkey: Option<String>,
    pub is_synced: Option<bool>,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOfflineActionsRequest {
    pub user_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOfflineActionsResponse {
    pub synced_count: i32,
    pub failed_count: i32,
    pub pending_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatusResponse {
    pub total_items: i64,
    pub stale_items: i64,
    pub cache_types: Vec<CacheTypeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTypeStatus {
    pub cache_type: String,
    pub item_count: i64,
    pub last_synced_at: Option<i64>,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddToSyncQueueRequest {
    pub action_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCacheMetadataRequest {
    pub cache_key: String,
    pub cache_type: String,
    pub metadata: Option<serde_json::Value>,
    pub expiry_seconds: Option<i64>,
}