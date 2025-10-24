use crate::application::ports::offline_store::OfflinePersistence;
use crate::modules::offline::manager::OfflineManager;
use crate::modules::offline::models::{
    AddToSyncQueueRequest, CacheStatusResponse, GetOfflineActionsRequest, OfflineAction,
    SaveOfflineActionRequest, SaveOfflineActionResponse, SyncOfflineActionsRequest,
    SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
};
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;

pub struct LegacyOfflineManagerAdapter {
    manager: Arc<OfflineManager>,
}

impl LegacyOfflineManagerAdapter {
    pub fn new(manager: Arc<OfflineManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl OfflinePersistence for LegacyOfflineManagerAdapter {
    async fn save_offline_action(
        &self,
        request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse, AppError> {
        self.manager
            .save_offline_action(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError> {
        self.manager
            .get_offline_actions(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError> {
        self.manager
            .sync_offline_actions(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError> {
        self.manager
            .get_cache_status()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn add_to_sync_queue(&self, request: AddToSyncQueueRequest) -> Result<i64, AppError> {
        self.manager
            .add_to_sync_queue(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn update_cache_metadata(
        &self,
        request: UpdateCacheMetadataRequest,
    ) -> Result<(), AppError> {
        self.manager
            .update_cache_metadata(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
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
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn confirm_optimistic_update(&self, update_id: String) -> Result<(), AppError> {
        self.manager
            .confirm_optimistic_update(update_id)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError> {
        self.manager
            .rollback_optimistic_update(update_id)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        self.manager
            .cleanup_expired_cache()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
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
            .map_err(|err| AppError::Internal(err.to_string()))
    }
}
