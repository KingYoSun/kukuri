use crate::domain::entities::offline::{
    CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionDraft, OfflineActionFilter,
    OfflineActionRecord, OptimisticUpdateDraft, SavedOfflineAction, SyncQueueItemDraft, SyncResult,
    SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    CacheType, EntityId, OfflineActionId, OfflineActionType, OfflinePayload, OptimisticUpdateId,
    RemoteEventId, SyncQueueId, SyncStatus,
};
use crate::modules::offline::models::{
    AddToSyncQueueRequest, CacheStatusResponse, CacheTypeStatus, GetOfflineActionsRequest,
    OfflineAction, SaveOfflineActionRequest, SaveOfflineActionResponse, SyncOfflineActionsRequest,
    SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
};
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use std::convert::TryInto;

pub fn module_save_request_from_draft(
    draft: OfflineActionDraft,
) -> Result<SaveOfflineActionRequest, AppError> {
    let OfflineActionDraft {
        user_pubkey,
        action_type,
        target_id,
        payload,
    } = draft;

    Ok(SaveOfflineActionRequest {
        user_pubkey: user_pubkey.as_hex().to_string(),
        action_type: action_type.as_str().to_string(),
        target_id: target_id.map(|id| id.to_string()),
        action_data: payload.into_inner(),
    })
}

pub fn module_get_request_from_filter(filter: OfflineActionFilter) -> GetOfflineActionsRequest {
    GetOfflineActionsRequest {
        user_pubkey: filter.user_pubkey.map(|pk| pk.as_hex().to_string()),
        is_synced: filter.include_synced,
        limit: filter.limit.and_then(|value| value.try_into().ok()),
    }
}

pub fn module_sync_request_from_pubkey(public_key: &PublicKey) -> SyncOfflineActionsRequest {
    SyncOfflineActionsRequest {
        user_pubkey: public_key.as_hex().to_string(),
    }
}

pub fn module_add_to_sync_queue_request_from_draft(
    draft: SyncQueueItemDraft,
) -> Result<AddToSyncQueueRequest, AppError> {
    Ok(AddToSyncQueueRequest {
        action_type: draft.action_type.as_str().to_string(),
        payload: draft.payload.into_inner(),
    })
}

pub fn module_cache_metadata_request_from_domain(
    update: CacheMetadataUpdate,
) -> Result<UpdateCacheMetadataRequest, AppError> {
    let ttl_seconds = update.expiry.map(|expiry| {
        let now = Utc::now();
        let diff = expiry.signed_duration_since(now).num_seconds();
        if diff > 0 { diff } else { 0 }
    });

    Ok(UpdateCacheMetadataRequest {
        cache_key: update.cache_key.to_string(),
        cache_type: update.cache_type.to_string(),
        metadata: update.metadata,
        expiry_seconds: ttl_seconds,
    })
}

pub fn module_optimistic_params_from_draft(
    draft: OptimisticUpdateDraft,
) -> Result<(String, String, Option<String>, String), AppError> {
    Ok((
        draft.entity_type.to_string(),
        draft.entity_id.to_string(),
        draft
            .original_data
            .map(|payload| serde_json::to_string(&payload.into_inner()))
            .transpose()
            .map_err(|err| AppError::SerializationError(err.to_string()))?,
        serde_json::to_string(&draft.updated_data.into_inner())
            .map_err(|err| AppError::SerializationError(err.to_string()))?,
    ))
}

pub fn module_sync_status_params_from_domain(
    update: &SyncStatusUpdate,
) -> Result<(String, String, String, Option<String>), AppError> {
    let conflict = update
        .conflict_data
        .as_ref()
        .map(|payload| serde_json::to_string(payload.as_json()))
        .transpose()
        .map_err(|err| AppError::SerializationError(err.to_string()))?;

    Ok((
        update.entity_type.to_string(),
        update.entity_id.to_string(),
        update.sync_status.as_str().to_string(),
        conflict,
    ))
}

pub fn domain_saved_action_from_module(
    response: SaveOfflineActionResponse,
) -> Result<SavedOfflineAction, AppError> {
    let action = domain_offline_action_from_module(response.action)?;
    let local_id = OfflineActionId::from_str(&response.local_id)
        .map_err(|err| AppError::ValidationError(err))?;
    Ok(SavedOfflineAction::new(local_id, action))
}

pub fn domain_offline_action_from_module(
    model: OfflineAction,
) -> Result<OfflineActionRecord, AppError> {
    let action_id =
        OfflineActionId::from_str(&model.local_id).map_err(|err| AppError::ValidationError(err))?;
    let public_key = PublicKey::from_hex_str(&model.user_pubkey)
        .map_err(|err| AppError::ValidationError(err))?;
    let action_type = OfflineActionType::new(model.action_type.clone())
        .map_err(|err| AppError::ValidationError(err))?;
    let target_id = model
        .target_id
        .map(|id| EntityId::new(id).map_err(AppError::ValidationError))
        .transpose()?;
    let payload_value: serde_json::Value = serde_json::from_str(&model.action_data)
        .map_err(|err| AppError::DeserializationError(err.to_string()))?;
    let payload =
        OfflinePayload::new(payload_value).map_err(|err| AppError::ValidationError(err))?;
    let sync_status = if model.is_synced {
        SyncStatus::FullySynced
    } else {
        SyncStatus::Pending
    };
    let created_at = timestamp_to_datetime(model.created_at);
    let synced_at = model.synced_at.map(timestamp_to_datetime);
    let remote_id = model
        .remote_id
        .map(|id| RemoteEventId::new(id).map_err(AppError::ValidationError))
        .transpose()?;

    Ok(OfflineActionRecord::new(
        Some(model.id),
        action_id,
        public_key,
        action_type,
        target_id,
        payload,
        sync_status,
        created_at,
        synced_at,
        remote_id,
    ))
}

pub fn domain_sync_result_from_module(
    response: SyncOfflineActionsResponse,
) -> Result<SyncResult, AppError> {
    Ok(SyncResult::new(
        try_i32_to_u32(response.synced_count, "synced_count")?,
        try_i32_to_u32(response.failed_count, "failed_count")?,
        try_i32_to_u32(response.pending_count, "pending_count")?,
    ))
}

pub fn domain_cache_status_from_module(
    response: CacheStatusResponse,
) -> Result<CacheStatusSnapshot, AppError> {
    let cache_types = response
        .cache_types
        .into_iter()
        .map(domain_cache_type_status_from_module)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CacheStatusSnapshot::new(
        try_i64_to_u64(response.total_items, "total_items")?,
        try_i64_to_u64(response.stale_items, "stale_items")?,
        cache_types,
    ))
}

pub fn sync_queue_id_from_i64(value: i64) -> Result<SyncQueueId, AppError> {
    SyncQueueId::new(value).map_err(AppError::ValidationError)
}

pub fn optimistic_update_id_from_string(value: String) -> Result<OptimisticUpdateId, AppError> {
    OptimisticUpdateId::new(value).map_err(AppError::ValidationError)
}

pub fn payload_from_optional_json(
    value: Option<String>,
) -> Result<Option<OfflinePayload>, AppError> {
    value
        .map(|json| {
            let parsed: serde_json::Value = serde_json::from_str(&json)
                .map_err(|err| AppError::DeserializationError(err.to_string()))?;
            OfflinePayload::new(parsed).map_err(AppError::ValidationError)
        })
        .transpose()
}

fn domain_cache_type_status_from_module(
    status: CacheTypeStatus,
) -> Result<crate::domain::entities::offline::CacheTypeStatus, AppError> {
    let cache_type = CacheType::new(status.cache_type).map_err(AppError::ValidationError)?;
    Ok(crate::domain::entities::offline::CacheTypeStatus::new(
        cache_type,
        try_i64_to_u64(status.item_count, "item_count")?,
        status.last_synced_at.map(timestamp_to_datetime),
        status.is_stale,
    ))
}

fn try_i32_to_u32(value: i32, label: &str) -> Result<u32, AppError> {
    value
        .try_into()
        .map_err(|_| AppError::ValidationError(format!("{label} cannot be negative")))
}

fn try_i64_to_u64(value: i64, label: &str) -> Result<u64, AppError> {
    value
        .try_into()
        .map_err(|_| AppError::ValidationError(format!("{label} cannot be negative")))
}

fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .or_else(|| DateTime::<Utc>::from_timestamp_millis(ts))
        .unwrap_or_else(Utc::now)
}
