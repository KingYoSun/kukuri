use crate::application::services::offline_service::OfflineServiceTrait;
use crate::presentation::dto::offline::{
    SaveOfflineActionRequest, SaveOfflineActionResponse,
    GetOfflineActionsRequest, SyncOfflineActionsRequest, SyncOfflineActionsResponse,
    CacheStatusResponse, AddToSyncQueueRequest, UpdateCacheMetadataRequest,
    OptimisticUpdateRequest, OfflineAction, UpdateSyncStatusRequest
};
use crate::presentation::dto::Validate;
use crate::shared::error::AppError;
use serde_json::json;
use std::sync::Arc;

pub struct OfflineHandler {
    offline_service: Arc<dyn OfflineServiceTrait>,
}

impl OfflineHandler {
    pub fn new(offline_service: Arc<dyn OfflineServiceTrait>) -> Self {
        Self { offline_service }
    }

    /// オフラインアクションを保存
    pub async fn save_offline_action(
        &self,
        request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse, AppError> {
        request.validate()?;
        
        let action_id = self.offline_service
            .save_action(
                request.entity_type,
                request.entity_id,
                request.action_type,
                request.payload,
            )
            .await?;
        
        Ok(SaveOfflineActionResponse {
            success: true,
            action_id,
            message: Some("Offline action saved successfully".to_string()),
        })
    }

    /// オフラインアクションを取得
    pub async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError> {
        request.validate()?;
        
        let actions = self.offline_service
            .get_actions(
                request.entity_type,
                request.entity_id,
                request.status,
                request.limit,
            )
            .await?;
        
        Ok(actions.into_iter().map(|a| OfflineAction {
            id: a.id,
            entity_type: a.entity_type,
            entity_id: a.entity_id,
            action_type: a.action_type,
            payload: a.payload,
            status: a.status,
            created_at: a.created_at,
            synced_at: a.synced_at,
            error_message: a.error_message,
        }).collect())
    }

    /// オフラインアクションを同期
    pub async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError> {
        request.validate()?;
        
        let result = self.offline_service
            .sync_actions(request.action_ids)
            .await?;
        
        Ok(SyncOfflineActionsResponse {
            success: true,
            synced_count: result.synced_count,
            failed_count: result.failed_count,
            failed_actions: result.failed_actions,
        })
    }

    /// キャッシュステータスを取得
    pub async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError> {
        let status = self.offline_service.get_cache_status().await?;
        
        Ok(CacheStatusResponse {
            total_size: status.total_size,
            item_count: status.item_count,
            oldest_item: status.oldest_item,
            newest_item: status.newest_item,
        })
    }

    /// 同期キューに追加
    pub async fn add_to_sync_queue(
        &self,
        request: AddToSyncQueueRequest,
    ) -> Result<i64, AppError> {
        request.validate()?;
        
        let queue_id = self.offline_service
            .add_to_sync_queue(
                request.entity_type,
                request.entity_id,
                request.operation,
                request.data,
                request.priority,
            )
            .await?;
        
        Ok(queue_id)
    }

    /// キャッシュメタデータを更新
    pub async fn update_cache_metadata(
        &self,
        request: UpdateCacheMetadataRequest,
    ) -> Result<serde_json::Value, AppError> {
        request.validate()?;
        
        self.offline_service
            .update_cache_metadata(
                request.key,
                request.metadata,
                request.ttl,
            )
            .await?;
        
        Ok(json!({ "success": true }))
    }

    /// 楽観的更新を保存
    pub async fn save_optimistic_update(
        &self,
        request: OptimisticUpdateRequest,
    ) -> Result<String, AppError> {
        request.validate()?;
        
        let update_id = self.offline_service
            .save_optimistic_update(
                request.entity_type,
                request.entity_id,
                request.original_data,
                request.updated_data,
            )
            .await?;
        
        Ok(update_id)
    }

    /// 楽観的更新を確定
    pub async fn confirm_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<serde_json::Value, AppError> {
        if update_id.is_empty() {
            return Err(AppError::ValidationError("Update ID is required".to_string()));
        }
        
        self.offline_service
            .confirm_optimistic_update(update_id)
            .await?;
        
        Ok(json!({ "success": true }))
    }

    /// 楽観的更新をロールバック
    pub async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError> {
        if update_id.is_empty() {
            return Err(AppError::ValidationError("Update ID is required".to_string()));
        }
        
        let original_data = self.offline_service
            .rollback_optimistic_update(update_id)
            .await?;
        
        Ok(original_data)
    }

    /// 期限切れキャッシュをクリーンアップ
    pub async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        let cleaned_count = self.offline_service
            .cleanup_expired_cache()
            .await?;
        
        Ok(cleaned_count)
    }

    /// 同期ステータスを更新
    pub async fn update_sync_status(
        &self,
        request: UpdateSyncStatusRequest,
    ) -> Result<serde_json::Value, AppError> {
        request.validate()?;
        
        self.offline_service
            .update_sync_status(
                request.entity_type,
                request.entity_id,
                request.sync_status,
                request.conflict_data,
            )
            .await?;
        
        Ok(json!({ "success": true }))
    }
}