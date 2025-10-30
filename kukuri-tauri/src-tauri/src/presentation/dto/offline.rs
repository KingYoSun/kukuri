use crate::domain::value_objects::offline::SyncStatus;
use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};

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
        if let Some(limit) = self.limit {
            if limit <= 0 || limit > 1000 {
                return Err("Limit must be between 1 and 1000".to_string());
            }
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
        if let Some(priority) = self.priority {
            if !(0..=10).contains(&priority) {
                return Err("Priority must be between 0 and 10".to_string());
            }
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
}

impl Validate for UpdateCacheMetadataRequest {
    fn validate(&self) -> Result<(), String> {
        if self.cache_key.is_empty() {
            return Err("Cache key is required".to_string());
        }
        if self.cache_type.is_empty() {
            return Err("Cache type is required".to_string());
        }
        if let Some(ttl) = self.expiry_seconds {
            if ttl <= 0 {
                return Err("Expiry seconds must be positive".to_string());
            }
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
