use crate::modules::offline::models::{
    AddToSyncQueueRequest, CacheStatusResponse, GetOfflineActionsRequest, OfflineAction,
    SaveOfflineActionRequest, SaveOfflineActionResponse, SyncOfflineActionsRequest,
    SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait OfflinePersistence: Send + Sync {
    async fn save_offline_action(
        &self,
        request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse, AppError>;
    async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError>;
    async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError>;
    async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError>;
    async fn add_to_sync_queue(&self, request: AddToSyncQueueRequest) -> Result<i64, AppError>;
    async fn update_cache_metadata(
        &self,
        request: UpdateCacheMetadataRequest,
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
