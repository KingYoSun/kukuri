use crate::domain::entities::offline::{
    CacheMetadataRecord, CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionDraft,
    OfflineActionFilter, OfflineActionRecord, OptimisticUpdateDraft, OptimisticUpdateRecord,
    SavedOfflineAction, SyncQueueItem, SyncQueueItemDraft, SyncResult, SyncStatusRecord,
    SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{OfflinePayload, OptimisticUpdateId, SyncQueueId};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait OfflinePersistence: Send + Sync {
    async fn save_action(&self, draft: OfflineActionDraft) -> Result<SavedOfflineAction, AppError>;

    async fn list_actions(
        &self,
        filter: OfflineActionFilter,
    ) -> Result<Vec<OfflineActionRecord>, AppError>;

    async fn sync_actions(&self, user_pubkey: PublicKey) -> Result<SyncResult, AppError>;

    async fn cache_status(&self) -> Result<CacheStatusSnapshot, AppError>;

    async fn enqueue_sync(&self, draft: SyncQueueItemDraft) -> Result<SyncQueueId, AppError>;

    async fn upsert_cache_metadata(&self, update: CacheMetadataUpdate) -> Result<(), AppError>;

    async fn save_optimistic_update(
        &self,
        draft: OptimisticUpdateDraft,
    ) -> Result<OptimisticUpdateId, AppError>;

    async fn confirm_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<(), AppError>;

    async fn rollback_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<Option<OfflinePayload>, AppError>;

    async fn cleanup_expired_cache(&self) -> Result<u32, AppError>;

    async fn update_sync_status(&self, update: SyncStatusUpdate) -> Result<(), AppError>;

    async fn enqueue_if_missing(&self, action: &OfflineActionRecord) -> Result<bool, AppError>;

    async fn recent_sync_queue_items(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<SyncQueueItem>, AppError>;

    async fn pending_sync_items(&self) -> Result<Vec<SyncQueueItem>, AppError>;

    async fn stale_cache_entries(&self) -> Result<Vec<CacheMetadataRecord>, AppError>;

    async fn unconfirmed_updates(&self) -> Result<Vec<OptimisticUpdateRecord>, AppError>;

    async fn sync_conflicts(&self) -> Result<Vec<SyncStatusRecord>, AppError>;
}
