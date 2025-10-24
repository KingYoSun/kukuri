use crate::application::ports::offline_store::OfflinePersistence;
use crate::domain::entities::offline::{
    CacheMetadataUpdate, OfflineActionDraft, OfflineActionFilter, OptimisticUpdateDraft,
    SyncQueueItemDraft, SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{OfflinePayload, OptimisticUpdateId, SyncQueueId};
use crate::infrastructure::offline::mappers::{
    domain_cache_status_from_module, domain_offline_action_from_module,
    domain_saved_action_from_module, domain_sync_result_from_module,
    module_add_to_sync_queue_request_from_draft, module_cache_metadata_request_from_domain,
    module_get_request_from_filter, module_optimistic_params_from_draft,
    module_save_request_from_draft, module_sync_request_from_pubkey,
    module_sync_status_params_from_domain, optimistic_update_id_from_string,
    payload_from_optional_json, sync_queue_id_from_i64,
};
use crate::modules::offline::manager::OfflineManager;
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::convert::TryInto;
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
    async fn save_action(
        &self,
        draft: OfflineActionDraft,
    ) -> Result<crate::domain::entities::offline::SavedOfflineAction, AppError> {
        let request = module_save_request_from_draft(draft)?;
        let response = self
            .manager
            .save_offline_action(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        domain_saved_action_from_module(response)
    }

    async fn list_actions(
        &self,
        filter: OfflineActionFilter,
    ) -> Result<Vec<crate::domain::entities::offline::OfflineActionRecord>, AppError> {
        let request = module_get_request_from_filter(filter);
        let actions = self
            .manager
            .get_offline_actions(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        actions
            .into_iter()
            .map(domain_offline_action_from_module)
            .collect()
    }

    async fn sync_actions(
        &self,
        user_pubkey: PublicKey,
    ) -> Result<crate::domain::entities::offline::SyncResult, AppError> {
        let request = module_sync_request_from_pubkey(&user_pubkey);
        let response = self
            .manager
            .sync_offline_actions(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        domain_sync_result_from_module(response)
    }

    async fn cache_status(
        &self,
    ) -> Result<crate::domain::entities::offline::CacheStatusSnapshot, AppError> {
        let response = self
            .manager
            .get_cache_status()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        domain_cache_status_from_module(response)
    }

    async fn enqueue_sync(&self, draft: SyncQueueItemDraft) -> Result<SyncQueueId, AppError> {
        let request = module_add_to_sync_queue_request_from_draft(draft)?;
        let queue_id = self
            .manager
            .add_to_sync_queue(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        sync_queue_id_from_i64(queue_id)
    }

    async fn upsert_cache_metadata(&self, update: CacheMetadataUpdate) -> Result<(), AppError> {
        let request = module_cache_metadata_request_from_domain(update)?;
        self.manager
            .update_cache_metadata(request)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn save_optimistic_update(
        &self,
        draft: OptimisticUpdateDraft,
    ) -> Result<OptimisticUpdateId, AppError> {
        let (entity_type, entity_id, original_data, updated_data) =
            module_optimistic_params_from_draft(draft)?;
        let id = self
            .manager
            .save_optimistic_update(entity_type, entity_id, original_data, updated_data)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        optimistic_update_id_from_string(id)
    }

    async fn confirm_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<(), AppError> {
        self.manager
            .confirm_optimistic_update(update_id.to_string())
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<Option<OfflinePayload>, AppError> {
        let payload = self
            .manager
            .rollback_optimistic_update(update_id.to_string())
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        payload_from_optional_json(payload)
    }

    async fn cleanup_expired_cache(&self) -> Result<u32, AppError> {
        let removed = self
            .manager
            .cleanup_expired_cache()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        removed
            .try_into()
            .map_err(|_| AppError::Internal("Cleanup count was negative".to_string()))
    }

    async fn update_sync_status(&self, update: SyncStatusUpdate) -> Result<(), AppError> {
        let (entity_type, entity_id, sync_status, conflict_data) =
            module_sync_status_params_from_domain(&update)?;
        self.manager
            .update_sync_status(entity_type, entity_id, sync_status, conflict_data)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }
}
