use crate::modules::offline::{
    OfflineManager,
    models::{
        AddToSyncQueueRequest as ManagerAddToSyncQueueRequest,
        CacheStatusResponse as ManagerCacheStatusResponse,
        CacheTypeStatus as ManagerCacheTypeStatus,
        GetOfflineActionsRequest as ManagerGetOfflineActionsRequest,
        OfflineAction as ManagerOfflineAction,
        SaveOfflineActionRequest as ManagerSaveOfflineActionRequest,
        SaveOfflineActionResponse as ManagerSaveOfflineActionResponse,
        SyncOfflineActionsRequest as ManagerSyncOfflineActionsRequest,
        UpdateCacheMetadataRequest as ManagerUpdateCacheMetadataRequest,
    },
};
use crate::shared::error::AppError;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct OfflineActionRecord {
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
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SavedOfflineAction {
    pub local_id: String,
    pub action: OfflineActionRecord,
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub synced_count: i32,
    pub failed_count: i32,
    pub pending_count: i32,
}

#[derive(Debug, Clone)]
pub struct CacheTypeStatusData {
    pub cache_type: String,
    pub item_count: i64,
    pub last_synced_at: Option<i64>,
    pub is_stale: bool,
}

#[derive(Debug, Clone)]
pub struct CacheStatusData {
    pub total_items: i64,
    pub stale_items: i64,
    pub cache_types: Vec<CacheTypeStatusData>,
}

#[async_trait]
pub trait OfflineServiceTrait: Send + Sync {
    async fn save_action(
        &self,
        user_pubkey: String,
        action_type: String,
        entity_type: String,
        entity_id: String,
        data: String,
    ) -> Result<SavedOfflineAction, AppError>;

    async fn get_actions(
        &self,
        user_pubkey: Option<String>,
        is_synced: Option<bool>,
        limit: Option<i32>,
    ) -> Result<Vec<OfflineActionRecord>, AppError>;

    async fn sync_actions(&self, user_pubkey: String) -> Result<SyncResult, AppError>;

    async fn get_cache_status(&self) -> Result<CacheStatusData, AppError>;

    async fn add_to_sync_queue(
        &self,
        action_type: String,
        payload: Value,
        priority: Option<i32>,
    ) -> Result<i64, AppError>;

    async fn update_cache_metadata(
        &self,
        cache_key: String,
        cache_type: String,
        metadata: Option<Value>,
        expiry_seconds: Option<i64>,
    ) -> Result<(), AppError>;

    async fn save_optimistic_update(
        &self,
        entity_type: String,
        entity_id: String,
        original_data: Option<String>,
        updated_data: String,
    ) -> Result<String, AppError>;

    async fn confirm_optimistic_update(&self, update_id: String) -> Result<(), AppError>;

    async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError>;

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError>;

    async fn update_sync_status(
        &self,
        entity_type: String,
        entity_id: String,
        sync_status: String,
        conflict_data: Option<String>,
    ) -> Result<(), AppError>;
}

pub struct OfflineService {
    manager: Arc<OfflineManager>,
}

impl OfflineService {
    pub fn new(manager: Arc<OfflineManager>) -> Self {
        Self { manager }
    }

    fn map_offline_action(action: ManagerOfflineAction) -> OfflineActionRecord {
        OfflineActionRecord {
            id: action.id,
            user_pubkey: action.user_pubkey,
            action_type: action.action_type,
            target_id: action.target_id,
            action_data: action.action_data,
            local_id: action.local_id,
            remote_id: action.remote_id,
            is_synced: action.is_synced,
            created_at: action.created_at,
            synced_at: action.synced_at,
            error_message: None,
        }
    }

    fn map_cache_status(status: ManagerCacheStatusResponse) -> CacheStatusData {
        CacheStatusData {
            total_items: status.total_items,
            stale_items: status.stale_items,
            cache_types: status
                .cache_types
                .into_iter()
                .map(|t: ManagerCacheTypeStatus| CacheTypeStatusData {
                    cache_type: t.cache_type,
                    item_count: t.item_count,
                    last_synced_at: t.last_synced_at,
                    is_stale: t.is_stale,
                })
                .collect(),
        }
    }

    fn build_action_payload(
        data: String,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Value, AppError> {
        let value = serde_json::from_str::<Value>(&data).map_err(|e| {
            AppError::ValidationError(format!("Invalid data payload. Expected JSON: {e}"))
        })?;

        let mut map = value.as_object().cloned().ok_or_else(|| {
            AppError::ValidationError("Data payload must be a JSON object".to_string())
        })?;

        map.insert(
            "entityType".to_string(),
            Value::String(entity_type.to_string()),
        );
        map.insert("entityId".to_string(), Value::String(entity_id.to_string()));

        Ok(Value::Object(map))
    }

    fn to_saved_action(response: ManagerSaveOfflineActionResponse) -> SavedOfflineAction {
        SavedOfflineAction {
            local_id: response.local_id,
            action: Self::map_offline_action(response.action),
        }
    }

    fn filter_and_limit(
        actions: Vec<ManagerOfflineAction>,
        user_pubkey: Option<String>,
        is_synced: Option<bool>,
        limit: Option<i32>,
    ) -> Vec<OfflineActionRecord> {
        let mut filtered = actions
            .into_iter()
            .filter(|action| {
                if let Some(ref user) = user_pubkey {
                    if action.user_pubkey != *user {
                        return false;
                    }
                }
                if let Some(flag) = is_synced {
                    if action.is_synced != flag {
                        return false;
                    }
                }
                true
            })
            .map(Self::map_offline_action)
            .collect::<Vec<_>>();

        if let Some(limit) = limit {
            let limit = limit.max(0) as usize;
            if filtered.len() > limit {
                filtered.truncate(limit);
            }
        }

        filtered
    }
}

#[async_trait]
impl OfflineServiceTrait for OfflineService {
    async fn save_action(
        &self,
        user_pubkey: String,
        action_type: String,
        entity_type: String,
        entity_id: String,
        data: String,
    ) -> Result<SavedOfflineAction, AppError> {
        let payload = Self::build_action_payload(data, &entity_type, &entity_id)?;

        let response = self
            .manager
            .save_offline_action(ManagerSaveOfflineActionRequest {
                user_pubkey,
                action_type,
                target_id: Some(entity_id),
                action_data: payload,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Self::to_saved_action(response))
    }

    async fn get_actions(
        &self,
        user_pubkey: Option<String>,
        is_synced: Option<bool>,
        limit: Option<i32>,
    ) -> Result<Vec<OfflineActionRecord>, AppError> {
        let manager_response = self
            .manager
            .get_offline_actions(ManagerGetOfflineActionsRequest {
                user_pubkey: user_pubkey.clone(),
                is_synced,
                limit,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Self::filter_and_limit(
            manager_response,
            user_pubkey,
            is_synced,
            limit,
        ))
    }

    async fn sync_actions(&self, user_pubkey: String) -> Result<SyncResult, AppError> {
        let response = self
            .manager
            .sync_offline_actions(ManagerSyncOfflineActionsRequest { user_pubkey })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(SyncResult {
            synced_count: response.synced_count,
            failed_count: response.failed_count,
            pending_count: response.pending_count,
        })
    }

    async fn get_cache_status(&self) -> Result<CacheStatusData, AppError> {
        let status = self
            .manager
            .get_cache_status()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(Self::map_cache_status(status))
    }

    async fn add_to_sync_queue(
        &self,
        action_type: String,
        payload: Value,
        _priority: Option<i32>,
    ) -> Result<i64, AppError> {
        self.manager
            .add_to_sync_queue(ManagerAddToSyncQueueRequest {
                action_type,
                payload,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn update_cache_metadata(
        &self,
        cache_key: String,
        cache_type: String,
        metadata: Option<Value>,
        expiry_seconds: Option<i64>,
    ) -> Result<(), AppError> {
        self.manager
            .update_cache_metadata(ManagerUpdateCacheMetadataRequest {
                cache_key,
                cache_type,
                metadata,
                expiry_seconds,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn save_optimistic_update(
        &self,
        entity_type: String,
        entity_id: String,
        original_data: Option<String>,
        updated_data: String,
    ) -> Result<String, AppError> {
        self.manager
            .save_optimistic_update(entity_type, entity_id, original_data, updated_data)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn confirm_optimistic_update(&self, update_id: String) -> Result<(), AppError> {
        self.manager
            .confirm_optimistic_update(update_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError> {
        self.manager
            .rollback_optimistic_update(update_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        self.manager
            .cleanup_expired_cache()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn update_sync_status(
        &self,
        entity_type: String,
        entity_id: String,
        sync_status: String,
        conflict_data: Option<String>,
    ) -> Result<(), AppError> {
        self.manager
            .update_sync_status(entity_type, entity_id, sync_status, conflict_data)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}
