use crate::application::ports::offline_store::OfflinePersistence;
use crate::modules::offline::models::{
    AddToSyncQueueRequest, CacheStatusResponse, GetOfflineActionsRequest, OfflineAction,
    SaveOfflineActionRequest, SaveOfflineActionResponse, SyncOfflineActionsRequest,
    SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
};
use crate::shared::error::AppError;
use async_trait::async_trait;

/// Placeholder implementation for the new infrastructure-backed offline persistence layer.
/// The concrete SQLx-backed implementation will be provided in WSA-02 Stage 2.
#[allow(dead_code)]
pub struct SqliteOfflinePersistence;

#[async_trait]
impl OfflinePersistence for SqliteOfflinePersistence {
    async fn save_offline_action(
        &self,
        _request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse, AppError> {
        todo!("SqliteOfflinePersistence::save_offline_action is not implemented yet")
    }

    async fn get_offline_actions(
        &self,
        _request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError> {
        todo!("SqliteOfflinePersistence::get_offline_actions is not implemented yet")
    }

    async fn sync_offline_actions(
        &self,
        _request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError> {
        todo!("SqliteOfflinePersistence::sync_offline_actions is not implemented yet")
    }

    async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError> {
        todo!("SqliteOfflinePersistence::get_cache_status is not implemented yet")
    }

    async fn add_to_sync_queue(&self, _request: AddToSyncQueueRequest) -> Result<i64, AppError> {
        todo!("SqliteOfflinePersistence::add_to_sync_queue is not implemented yet")
    }

    async fn update_cache_metadata(
        &self,
        _request: UpdateCacheMetadataRequest,
    ) -> Result<(), AppError> {
        todo!("SqliteOfflinePersistence::update_cache_metadata is not implemented yet")
    }

    async fn save_optimistic_update(
        &self,
        _entity_type: String,
        _entity_id: String,
        _original_data: Option<String>,
        _updated_data: String,
    ) -> Result<String, AppError> {
        todo!("SqliteOfflinePersistence::save_optimistic_update is not implemented yet")
    }

    async fn confirm_optimistic_update(&self, _update_id: String) -> Result<(), AppError> {
        todo!("SqliteOfflinePersistence::confirm_optimistic_update is not implemented yet")
    }

    async fn rollback_optimistic_update(
        &self,
        _update_id: String,
    ) -> Result<Option<String>, AppError> {
        todo!("SqliteOfflinePersistence::rollback_optimistic_update is not implemented yet")
    }

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        todo!("SqliteOfflinePersistence::cleanup_expired_cache is not implemented yet")
    }

    async fn update_sync_status(
        &self,
        _entity_type: String,
        _entity_id: String,
        _sync_status: String,
        _conflict_data: Option<String>,
    ) -> Result<(), AppError> {
        todo!("SqliteOfflinePersistence::update_sync_status is not implemented yet")
    }
}
