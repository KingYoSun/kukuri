use crate::domain::entities::offline::{
    CacheMetadataRecord, CacheStatusSnapshot, OfflineActionRecord, OptimisticUpdateRecord,
    SyncQueueItem, SyncResult, SyncStatusRecord,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    CacheKey, CacheType, EntityId, EntityType, OfflineActionId, OfflineActionType, OfflinePayload,
    OptimisticUpdateId, RemoteEventId, SyncQueueId, SyncQueueStatus, SyncStatus,
};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::rows::{
    CacheMetadataRow, OfflineActionRow, OptimisticUpdateRow, SyncQueueItemRow, SyncStatusRow,
};

#[derive(Debug, Deserialize, FromRow)]
pub struct CacheTypeAggregate {
    pub cache_type: String,
    pub item_count: i64,
    pub last_synced_at: Option<i64>,
    pub is_stale: bool,
}

pub fn offline_action_from_row(row: OfflineActionRow) -> Result<OfflineActionRecord, AppError> {
    let action_id = OfflineActionId::parse(&row.local_id)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let public_key = PublicKey::from_hex_str(&row.user_pubkey)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let action_type = OfflineActionType::new(row.action_type.clone())
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let target_id = row
        .target_id
        .map(|id| {
            EntityId::new(id).map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
        })
        .transpose()?;
    let payload_value: serde_json::Value = serde_json::from_str(&row.action_data)
        .map_err(|err| AppError::DeserializationError(err.to_string()))?;
    let payload = OfflinePayload::new(payload_value)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let sync_status = if row.is_synced {
        SyncStatus::FullySynced
    } else {
        SyncStatus::Pending
    };
    let created_at = timestamp_to_datetime(row.created_at);
    let synced_at = row.synced_at.map(timestamp_to_datetime);
    let remote_id = row
        .remote_id
        .map(|id| {
            RemoteEventId::new(id)
                .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
        })
        .transpose()?;

    Ok(OfflineActionRecord::new(
        Some(row.id),
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

pub fn cache_status_from_aggregates(
    total_items: i64,
    stale_items: i64,
    aggregates: Vec<CacheTypeAggregate>,
) -> Result<CacheStatusSnapshot, AppError> {
    let cache_types = aggregates
        .into_iter()
        .map(|aggregate| {
            let cache_type = CacheType::new(aggregate.cache_type)
                .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
            Ok(crate::domain::entities::offline::CacheTypeStatus::new(
                cache_type,
                try_i64_to_u64(aggregate.item_count, "item_count")?,
                aggregate.last_synced_at.map(timestamp_to_datetime),
                aggregate.is_stale,
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(CacheStatusSnapshot::new(
        try_i64_to_u64(total_items, "total_items")?,
        try_i64_to_u64(stale_items, "stale_items")?,
        cache_types,
    ))
}

pub fn sync_result_from_counts(
    synced: i32,
    failed: i32,
    pending: i32,
) -> Result<SyncResult, AppError> {
    Ok(SyncResult::new(
        try_i32_to_u32(synced, "synced_count")?,
        try_i32_to_u32(failed, "failed_count")?,
        try_i32_to_u32(pending, "pending_count")?,
    ))
}

pub fn sync_queue_item_from_row(row: SyncQueueItemRow) -> Result<SyncQueueItem, AppError> {
    let id = SyncQueueId::new(row.id)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let action_type = OfflineActionType::new(row.action_type)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let payload_value: serde_json::Value = serde_json::from_str(&row.payload)
        .map_err(|err| AppError::DeserializationError(err.to_string()))?;
    let payload = OfflinePayload::new(payload_value)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let status = SyncQueueStatus::from(row.status.as_str());
    let created_at = timestamp_to_datetime(row.created_at);
    let updated_at = timestamp_to_datetime(row.updated_at);
    let synced_at = row.synced_at.map(timestamp_to_datetime);

    Ok(SyncQueueItem::new(
        id,
        action_type,
        payload,
        status,
        try_i32_to_u32(row.retry_count, "retry_count")?,
        try_i32_to_u32(row.max_retries, "max_retries")?,
        created_at,
        updated_at,
        synced_at,
        row.error_message,
    ))
}

pub fn cache_metadata_from_row(row: CacheMetadataRow) -> Result<CacheMetadataRecord, AppError> {
    let cache_key = CacheKey::new(row.cache_key)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let cache_type = CacheType::new(row.cache_type)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let last_synced_at = row.last_synced_at.map(timestamp_to_datetime);
    let last_accessed_at = row.last_accessed_at.map(timestamp_to_datetime);
    let expiry_time = row.expiry_time.map(timestamp_to_datetime);
    let metadata = row
        .metadata
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|err| AppError::DeserializationError(err.to_string()))?;

    Ok(CacheMetadataRecord {
        record_id: row.id,
        cache_key,
        cache_type,
        last_synced_at,
        last_accessed_at,
        data_version: row.data_version,
        is_stale: row.is_stale,
        expiry_time,
        metadata,
    })
}

pub fn optimistic_update_from_row(
    row: OptimisticUpdateRow,
) -> Result<OptimisticUpdateRecord, AppError> {
    let update_id = OptimisticUpdateId::new(row.update_id)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let entity_type = EntityType::new(row.entity_type)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let entity_id = EntityId::new(row.entity_id)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let original_data = row
        .original_data
        .map(|value| payload_from_json_str(&value))
        .transpose()?;
    let updated_data = payload_from_json_str(&row.updated_data)?;
    let created_at = timestamp_to_datetime(row.created_at);
    let confirmed_at = row.confirmed_at.map(timestamp_to_datetime);

    Ok(OptimisticUpdateRecord {
        record_id: row.id,
        update_id,
        entity_type,
        entity_id,
        original_data,
        updated_data,
        is_confirmed: row.is_confirmed,
        created_at,
        confirmed_at,
    })
}

pub fn sync_status_from_row(row: SyncStatusRow) -> Result<SyncStatusRecord, AppError> {
    let entity_type = EntityType::new(row.entity_type)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let entity_id = EntityId::new(row.entity_id)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
    let sync_status = SyncStatus::from(row.sync_status.as_str());
    let last_local_update = timestamp_to_datetime(row.last_local_update);
    let last_remote_sync = row.last_remote_sync.map(timestamp_to_datetime);
    let conflict_data = row
        .conflict_data
        .map(|value| payload_from_json_str(&value))
        .transpose()?;

    Ok(SyncStatusRecord {
        record_id: row.id,
        entity_type,
        entity_id,
        local_version: row.local_version,
        remote_version: row.remote_version,
        last_local_update,
        last_remote_sync,
        sync_status,
        conflict_data,
    })
}

pub fn sync_queue_id_from_i64(value: i64) -> Result<SyncQueueId, AppError> {
    SyncQueueId::new(value).map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

pub fn optimistic_update_id_from_string(value: String) -> Result<OptimisticUpdateId, AppError> {
    OptimisticUpdateId::new(value)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

pub fn payload_from_optional_json_str(
    value: Option<String>,
) -> Result<Option<OfflinePayload>, AppError> {
    value.map(|json| payload_from_json_str(&json)).transpose()
}

pub fn payload_from_json_str(json: &str) -> Result<OfflinePayload, AppError> {
    OfflinePayload::from_json_str(json)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

pub fn payload_to_string(payload: &OfflinePayload) -> Result<String, AppError> {
    serde_json::to_string(payload.as_json())
        .map_err(|err| AppError::SerializationError(err.to_string()))
}

pub fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .or_else(|| DateTime::<Utc>::from_timestamp_millis(ts))
        .unwrap_or_else(Utc::now)
}

pub fn try_i32_to_u32(value: i32, label: &str) -> Result<u32, AppError> {
    value.try_into().map_err(|_| {
        AppError::validation(
            ValidationFailureKind::Generic,
            format!("{label} cannot be negative"),
        )
    })
}

pub fn try_i64_to_u64(value: i64, label: &str) -> Result<u64, AppError> {
    value.try_into().map_err(|_| {
        AppError::validation(
            ValidationFailureKind::Generic,
            format!("{label} cannot be negative"),
        )
    })
}
