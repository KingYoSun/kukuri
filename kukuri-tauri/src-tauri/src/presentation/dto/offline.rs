use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAction {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub action_type: String,
    pub payload: String,
    pub status: String,
    pub created_at: i64,
    pub synced_at: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOfflineActionRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub action_type: String,
    pub payload: String,
}

impl Validate for SaveOfflineActionRequest {
    fn validate(&self) -> Result<(), String> {
        if self.entity_type.is_empty() {
            return Err("Entity type is required".to_string());
        }
        if self.entity_id.is_empty() {
            return Err("Entity ID is required".to_string());
        }
        if self.action_type.is_empty() {
            return Err("Action type is required".to_string());
        }
        if self.payload.is_empty() {
            return Err("Payload is required".to_string());
        }
        if self.payload.len() > 100000 {
            return Err("Payload is too large (max 100KB)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOfflineActionResponse {
    pub success: bool,
    pub action_id: i64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOfflineActionsRequest {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i32>,
}

impl Validate for GetOfflineActionsRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit {
            if limit <= 0 || limit > 1000 {
                return Err("Limit must be between 1 and 1000".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOfflineActionsRequest {
    pub action_ids: Option<Vec<i64>>,
}

impl Validate for SyncOfflineActionsRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(ids) = &self.action_ids {
            if ids.len() > 100 {
                return Err("Too many actions to sync (max 100)".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOfflineActionsResponse {
    pub success: bool,
    pub synced_count: usize,
    pub failed_count: usize,
    pub failed_actions: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatusResponse {
    pub total_size: i64,
    pub item_count: i32,
    pub oldest_item: Option<i64>,
    pub newest_item: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddToSyncQueueRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub operation: String,
    pub data: String,
    pub priority: Option<i32>,
}

impl Validate for AddToSyncQueueRequest {
    fn validate(&self) -> Result<(), String> {
        if self.entity_type.is_empty() {
            return Err("Entity type is required".to_string());
        }
        if self.entity_id.is_empty() {
            return Err("Entity ID is required".to_string());
        }
        if self.operation.is_empty() {
            return Err("Operation is required".to_string());
        }
        if self.data.is_empty() {
            return Err("Data is required".to_string());
        }
        if let Some(priority) = self.priority {
            if priority < 0 || priority > 10 {
                return Err("Priority must be between 0 and 10".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCacheMetadataRequest {
    pub key: String,
    pub metadata: String,
    pub ttl: Option<i64>,
}

impl Validate for UpdateCacheMetadataRequest {
    fn validate(&self) -> Result<(), String> {
        if self.key.is_empty() {
            return Err("Key is required".to_string());
        }
        if self.metadata.is_empty() {
            return Err("Metadata is required".to_string());
        }
        if let Some(ttl) = self.ttl {
            if ttl <= 0 {
                return Err("TTL must be positive".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        // 同期ステータスの有効な値をチェック
        let valid_statuses = ["pending", "syncing", "synced", "failed", "conflict"];
        if !valid_statuses.contains(&self.sync_status.as_str()) {
            return Err(format!("Invalid sync status. Must be one of: {:?}", valid_statuses));
        }
        Ok(())
    }
}