use crate::application::ports::offline_store::OfflinePersistence;
use crate::domain::entities::offline::{
    CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionDraft, OfflineActionFilter,
    OfflineActionRecord, OptimisticUpdateDraft, SavedOfflineAction, SyncQueueItemDraft, SyncResult,
    SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    EntityId, EntityType, OfflineActionType, OfflinePayload, OptimisticUpdateId, SyncQueueId,
};
use crate::shared::error::AppError;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SaveOfflineActionParams {
    pub user_pubkey: PublicKey,
    pub action_type: OfflineActionType,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub payload: OfflinePayload,
}

#[derive(Debug, Clone, Default)]
pub struct OfflineActionsQuery {
    pub user_pubkey: Option<PublicKey>,
    pub include_synced: Option<bool>,
    pub limit: Option<u32>,
}

#[async_trait]
pub trait OfflineServiceTrait: Send + Sync {
    async fn save_action(
        &self,
        params: SaveOfflineActionParams,
    ) -> Result<SavedOfflineAction, AppError>;
    async fn list_actions(
        &self,
        query: OfflineActionsQuery,
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
}

pub struct OfflineService {
    persistence: Arc<dyn OfflinePersistence>,
}

impl OfflineService {
    pub fn new(persistence: Arc<dyn OfflinePersistence>) -> Self {
        Self { persistence }
    }

    fn build_action_draft(
        params: &SaveOfflineActionParams,
    ) -> Result<OfflineActionDraft, AppError> {
        let payload_value = params.payload.clone().into_inner();
        let mut map = match payload_value {
            Value::Object(map) => map,
            _ => {
                return Err(AppError::ValidationError(
                    "Offline action payload must be a JSON object".to_string(),
                ));
            }
        };

        map.insert(
            "entityType".to_string(),
            Value::String(params.entity_type.to_string()),
        );
        map.insert(
            "entityId".to_string(),
            Value::String(params.entity_id.to_string()),
        );

        let enriched_payload =
            OfflinePayload::new(Value::Object(map)).map_err(AppError::ValidationError)?;

        Ok(OfflineActionDraft::new(
            params.user_pubkey.clone(),
            params.action_type.clone(),
            Some(params.entity_id.clone()),
            enriched_payload,
        ))
    }

    fn filter_from_query(query: &OfflineActionsQuery) -> OfflineActionFilter {
        OfflineActionFilter::new(query.user_pubkey.clone(), query.include_synced, query.limit)
    }
}

#[async_trait]
impl OfflineServiceTrait for OfflineService {
    async fn save_action(
        &self,
        params: SaveOfflineActionParams,
    ) -> Result<SavedOfflineAction, AppError> {
        let draft = Self::build_action_draft(&params)?;
        self.persistence.save_action(draft).await
    }

    async fn list_actions(
        &self,
        query: OfflineActionsQuery,
    ) -> Result<Vec<OfflineActionRecord>, AppError> {
        let filter = Self::filter_from_query(&query);
        self.persistence.list_actions(filter).await
    }

    async fn sync_actions(&self, user_pubkey: PublicKey) -> Result<SyncResult, AppError> {
        self.persistence.sync_actions(user_pubkey).await
    }

    async fn cache_status(&self) -> Result<CacheStatusSnapshot, AppError> {
        self.persistence.cache_status().await
    }

    async fn enqueue_sync(&self, draft: SyncQueueItemDraft) -> Result<SyncQueueId, AppError> {
        self.persistence.enqueue_sync(draft).await
    }

    async fn upsert_cache_metadata(&self, update: CacheMetadataUpdate) -> Result<(), AppError> {
        self.persistence.upsert_cache_metadata(update).await
    }

    async fn save_optimistic_update(
        &self,
        draft: OptimisticUpdateDraft,
    ) -> Result<OptimisticUpdateId, AppError> {
        self.persistence.save_optimistic_update(draft).await
    }

    async fn confirm_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<(), AppError> {
        self.persistence.confirm_optimistic_update(update_id).await
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<Option<OfflinePayload>, AppError> {
        self.persistence.rollback_optimistic_update(update_id).await
    }

    async fn cleanup_expired_cache(&self) -> Result<u32, AppError> {
        self.persistence.cleanup_expired_cache().await
    }

    async fn update_sync_status(&self, update: SyncStatusUpdate) -> Result<(), AppError> {
        self.persistence.update_sync_status(update).await
    }
}
