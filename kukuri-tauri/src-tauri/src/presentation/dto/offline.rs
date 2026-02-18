use crate::domain::value_objects::offline::SyncStatus;
use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOfflineActionRequest {
    pub user_pubkey: String,
    pub action_type: String,
    pub entity_type: String,
    pub entity_id: String,
    pub data: String,
}

impl Validate for SaveOfflineActionRequest {
    fn validate(&self) -> Result<(), String> {
        if self.user_pubkey.is_empty() {
            return Err("User pubkey is required".to_string());
        }
        if self.action_type.is_empty() {
            return Err("Action type is required".to_string());
        }
        if self.entity_type.is_empty() {
            return Err("Entity type is required".to_string());
        }
        if self.entity_id.is_empty() {
            return Err("Entity ID is required".to_string());
        }
        if self.data.is_empty() {
            return Err("Data is required".to_string());
        }
        if self.data.len() > 200_000 {
            return Err("Data is too large (max 200KB)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOfflineActionResponse {
    pub local_id: String,
    pub action: OfflineAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOfflineActionsRequest {
    pub user_pubkey: Option<String>,
    pub is_synced: Option<bool>,
    pub limit: Option<i32>,
}

impl Validate for GetOfflineActionsRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit
            && (limit <= 0 || limit > 1000)
        {
            return Err("Limit must be between 1 and 1000".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOfflineActionsRequest {
    pub user_pubkey: String,
}

impl Validate for SyncOfflineActionsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.user_pubkey.is_empty() {
            return Err("User pubkey is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOfflineActionsResponse {
    pub synced_count: i32,
    pub failed_count: i32,
    pub pending_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheTypeStatus {
    pub cache_type: String,
    pub item_count: i64,
    pub last_synced_at: Option<i64>,
    pub is_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatusResponse {
    pub total_items: i64,
    pub stale_items: i64,
    pub cache_types: Vec<CacheTypeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSyncQueueItemsRequest {
    pub limit: Option<i32>,
}

impl Validate for ListSyncQueueItemsRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit
            && !(1..=200).contains(&limit)
        {
            return Err("Limit must be between 1 and 200".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncQueueItemResponse {
    pub id: i64,
    pub action_type: String,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub synced_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddToSyncQueueRequest {
    pub action_type: String,
    pub payload: serde_json::Value,
    pub priority: Option<i32>,
}

impl Validate for AddToSyncQueueRequest {
    fn validate(&self) -> Result<(), String> {
        if self.action_type.is_empty() {
            return Err("Action type is required".to_string());
        }
        if let Some(priority) = self.priority
            && !(0..=10).contains(&priority)
        {
            return Err("Priority must be between 0 and 10".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCacheMetadataRequest {
    pub cache_key: String,
    pub cache_type: String,
    pub metadata: Option<serde_json::Value>,
    pub expiry_seconds: Option<i64>,
    pub is_stale: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_bytes: Option<i64>,
}

impl Validate for UpdateCacheMetadataRequest {
    fn validate(&self) -> Result<(), String> {
        if self.cache_key.is_empty() {
            return Err("Cache key is required".to_string());
        }
        if self.cache_type.is_empty() {
            return Err("Cache type is required".to_string());
        }
        if let Some(ttl) = self.expiry_seconds
            && ttl <= 0
        {
            return Err("Expiry seconds must be positive".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimisticUpdateRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub original_data: Option<String>,
    pub updated_data: String,
}

impl Validate for OptimisticUpdateRequest {
    fn validate(&self) -> Result<(), String> {
        if self.entity_type.is_empty() {
            return Err("Entity type is required".to_string());
        }
        if self.entity_id.is_empty() {
            return Err("Entity ID is required".to_string());
        }
        if self.updated_data.is_empty() {
            return Err("Updated data is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSyncStatusRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub sync_status: String,
    pub conflict_data: Option<String>,
}

impl Validate for UpdateSyncStatusRequest {
    fn validate(&self) -> Result<(), String> {
        if self.entity_type.is_empty() {
            return Err("Entity type is required".to_string());
        }
        if self.entity_id.is_empty() {
            return Err("Entity ID is required".to_string());
        }
        if self.sync_status.is_empty() {
            return Err("Sync status is required".to_string());
        }
        let status = self.sync_status.as_str();
        let parsed = SyncStatus::from(status);
        let legacy_allowed = matches!(status, "syncing" | "synced");
        if matches!(parsed, SyncStatus::Unknown(_)) && !legacy_allowed {
            return Err("Invalid sync status. Supported values include pending, sent_to_nostr, sent_to_p2p, fully_synced, failed, conflict, invalid:<reason>".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineRetryMetricsResponse {
    pub total_success: u64,
    pub total_failure: u64,
    pub consecutive_failure: u64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
    pub last_outcome: Option<String>,
    pub last_job_id: Option<String>,
    pub last_job_reason: Option<String>,
    pub last_trigger: Option<String>,
    pub last_user_pubkey: Option<String>,
    pub last_retry_count: Option<i32>,
    pub last_max_retries: Option<i32>,
    pub last_backoff_ms: Option<u64>,
    pub last_duration_ms: Option<u64>,
    pub last_success_count: Option<i32>,
    pub last_failure_count: Option<i32>,
    pub last_timestamp_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOfflineRetryOutcomeRequest {
    pub job_id: Option<String>,
    pub status: String,
    pub job_reason: Option<String>,
    pub trigger: Option<String>,
    pub user_pubkey: Option<String>,
    pub retry_count: Option<i32>,
    pub max_retries: Option<i32>,
    pub backoff_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub success_count: Option<i32>,
    pub failure_count: Option<i32>,
    pub timestamp_ms: Option<u64>,
}

impl Validate for RecordOfflineRetryOutcomeRequest {
    fn validate(&self) -> Result<(), String> {
        match self.status.as_str() {
            "success" | "failure" => {}
            other => return Err(format!("Unsupported status: {other}")),
        }
        if let Some(value) = self.retry_count
            && value < 0
        {
            return Err("retry_count must be >= 0".to_string());
        }
        if let Some(value) = self.max_retries
            && value <= 0
        {
            return Err("max_retries must be > 0".to_string());
        }
        Ok(())
    }
}
