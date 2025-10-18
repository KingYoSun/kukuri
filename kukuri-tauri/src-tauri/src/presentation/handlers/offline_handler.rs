use crate::application::services::offline_service::OfflineServiceTrait;
use crate::presentation::dto::Validate;
use crate::presentation::dto::offline::{
    AddToSyncQueueRequest, CacheStatusResponse, CacheTypeStatus, GetOfflineActionsRequest,
    OfflineAction, OptimisticUpdateRequest, SaveOfflineActionRequest, SaveOfflineActionResponse,
    SyncOfflineActionsRequest, SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
    UpdateSyncStatusRequest,
};
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

        let saved = self
            .offline_service
            .save_action(
                request.user_pubkey,
                request.action_type,
                request.entity_type,
                request.entity_id,
                request.data,
            )
            .await?;

        Ok(SaveOfflineActionResponse {
            local_id: saved.local_id,
            action: OfflineAction {
                id: saved.action.id,
                user_pubkey: saved.action.user_pubkey,
                action_type: saved.action.action_type,
                target_id: saved.action.target_id,
                action_data: saved.action.action_data,
                local_id: saved.action.local_id,
                remote_id: saved.action.remote_id,
                is_synced: saved.action.is_synced,
                created_at: saved.action.created_at,
                synced_at: saved.action.synced_at,
                error_message: saved.action.error_message,
            },
        })
    }

    /// オフラインアクションを取得
    pub async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError> {
        request.validate()?;

        let actions = self
            .offline_service
            .get_actions(request.user_pubkey, request.is_synced, request.limit)
            .await?;

        Ok(actions
            .into_iter()
            .map(|a| OfflineAction {
                id: a.id,
                user_pubkey: a.user_pubkey,
                action_type: a.action_type,
                target_id: a.target_id,
                action_data: a.action_data,
                local_id: a.local_id,
                remote_id: a.remote_id,
                is_synced: a.is_synced,
                created_at: a.created_at,
                synced_at: a.synced_at,
                error_message: a.error_message,
            })
            .collect())
    }

    /// オフラインアクションを同期
    pub async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError> {
        request.validate()?;

        let result = self
            .offline_service
            .sync_actions(request.user_pubkey)
            .await?;

        Ok(SyncOfflineActionsResponse {
            synced_count: result.synced_count,
            failed_count: result.failed_count,
            pending_count: result.pending_count,
        })
    }

    /// キャッシュステータスを取得
    pub async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError> {
        let status = self.offline_service.get_cache_status().await?;

        Ok(CacheStatusResponse {
            total_items: status.total_items,
            stale_items: status.stale_items,
            cache_types: status
                .cache_types
                .into_iter()
                .map(|t| CacheTypeStatus {
                    cache_type: t.cache_type,
                    item_count: t.item_count,
                    last_synced_at: t.last_synced_at,
                    is_stale: t.is_stale,
                })
                .collect(),
        })
    }

    /// 同期キューに追加
    pub async fn add_to_sync_queue(&self, request: AddToSyncQueueRequest) -> Result<i64, AppError> {
        request.validate()?;

        let queue_id = self
            .offline_service
            .add_to_sync_queue(request.action_type, request.payload, request.priority)
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
                request.cache_key,
                request.cache_type,
                request.metadata,
                request.expiry_seconds,
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

        let update_id = self
            .offline_service
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
            return Err(AppError::ValidationError(
                "Update ID is required".to_string(),
            ));
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
            return Err(AppError::ValidationError(
                "Update ID is required".to_string(),
            ));
        }

        let original_data = self
            .offline_service
            .rollback_optimistic_update(update_id)
            .await?;

        Ok(original_data)
    }

    /// 期限切れキャッシュをクリーンアップ
    pub async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        let cleaned_count = self.offline_service.cleanup_expired_cache().await?;

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
