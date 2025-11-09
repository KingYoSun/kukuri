use crate::application::services::offline_service::{
    OfflineActionsQuery, OfflineServiceTrait, SaveOfflineActionParams,
};
use crate::domain::entities::offline::{
    CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionRecord, OptimisticUpdateDraft,
    SyncQueueItem, SyncQueueItemDraft, SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    CacheKey, CacheType, EntityId, EntityType, OfflineActionType, OfflinePayload,
    OptimisticUpdateId, SyncQueueId, SyncStatus,
};
use crate::presentation::dto::Validate;
use crate::presentation::dto::offline::{
    AddToSyncQueueRequest, CacheStatusResponse, CacheTypeStatus, GetOfflineActionsRequest,
    ListSyncQueueItemsRequest, OfflineAction, OptimisticUpdateRequest, SaveOfflineActionRequest,
    SaveOfflineActionResponse, SyncOfflineActionsRequest, SyncOfflineActionsResponse,
    SyncQueueItemResponse, UpdateCacheMetadataRequest, UpdateSyncStatusRequest,
};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;

pub struct OfflineHandler {
    offline_service: Arc<dyn OfflineServiceTrait>,
}

impl OfflineHandler {
    pub fn new(offline_service: Arc<dyn OfflineServiceTrait>) -> Self {
        Self { offline_service }
    }
    pub async fn save_offline_action(
        &self,
        request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse, AppError> {
        request.validate()?;

        let params = SaveOfflineActionParams {
            user_pubkey: parse_public_key(&request.user_pubkey)?,
            action_type: parse_action_type(&request.action_type)?,
            entity_type: parse_entity_type(&request.entity_type)?,
            entity_id: parse_entity_id(&request.entity_id)?,
            payload: parse_payload(&request.data)?,
        };

        let saved = self.offline_service.save_action(params).await?;
        let action = map_action_record(&saved.action)?;

        Ok(SaveOfflineActionResponse {
            local_id: saved.local_id.to_string(),
            action,
        })
    }

    pub async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError> {
        request.validate()?;

        let query = OfflineActionsQuery {
            user_pubkey: match request.user_pubkey.as_deref() {
                Some(value) => Some(parse_public_key(value)?),
                None => None,
            },
            include_synced: request.is_synced,
            limit: request.limit.map(|value| value as u32),
        };

        let actions = self.offline_service.list_actions(query).await?;
        actions
            .iter()
            .map(map_action_record)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError> {
        request.validate()?;

        let pubkey = parse_public_key(&request.user_pubkey)?;
        let result = self.offline_service.sync_actions(pubkey).await?;

        Ok(SyncOfflineActionsResponse {
            synced_count: i32::try_from(result.synced_count)
                .map_err(|_| AppError::Internal("Synced count overflowed i32".to_string()))?,
            failed_count: i32::try_from(result.failed_count)
                .map_err(|_| AppError::Internal("Failed count overflowed i32".to_string()))?,
            pending_count: i32::try_from(result.pending_count)
                .map_err(|_| AppError::Internal("Pending count overflowed i32".to_string()))?,
        })
    }

    pub async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError> {
        let snapshot = self.offline_service.cache_status().await?;
        map_cache_status(snapshot)
    }

    pub async fn add_to_sync_queue(&self, request: AddToSyncQueueRequest) -> Result<i64, AppError> {
        request.validate()?;

        let draft = SyncQueueItemDraft::new(
            parse_action_type(&request.action_type)?,
            OfflinePayload::new(request.payload.clone())
                .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?,
            request
                .priority
                .map(|value| {
                    u8::try_from(value).map_err(|_| {
                        AppError::validation(
                            ValidationFailureKind::Generic,
                            "Priority must fit in u8",
                        )
                    })
                })
                .transpose()?,
        );
        let queue_id = self.offline_service.enqueue_sync(draft).await?;

        if let Some(cache_type) = request
            .payload
            .get("cacheType")
            .and_then(|value| value.as_str())
        {
            if let Err(err) = self
                .record_sync_queue_metadata(cache_type, &request.payload, queue_id)
                .await
            {
                tracing::warn!(
                    target: "offline::handler",
                    error = %err,
                    "failed to update sync_queue metadata for {cache_type}"
                );
            }
        }
        Ok(queue_id.value())
    }

    pub async fn list_sync_queue_items(
        &self,
        request: ListSyncQueueItemsRequest,
    ) -> Result<Vec<SyncQueueItemResponse>, AppError> {
        request.validate()?;
        let limit = request
            .limit
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    AppError::validation(
                        ValidationFailureKind::Generic,
                        "Limit must fit in u32".to_string(),
                    )
                })
            })
            .transpose()?;

        let items = self.offline_service.recent_sync_queue_items(limit).await?;

        items
            .iter()
            .map(map_sync_queue_item)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn update_cache_metadata(
        &self,
        request: UpdateCacheMetadataRequest,
    ) -> Result<Value, AppError> {
        request.validate()?;

        let update = CacheMetadataUpdate {
            cache_key: CacheKey::new(request.cache_key)
                .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?,
            cache_type: CacheType::new(request.cache_type)
                .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?,
            metadata: request.metadata,
            is_stale: request.is_stale,
            expiry: request
                .expiry_seconds
                .map(|seconds| {
                    if seconds <= 0 {
                        return Err(AppError::validation(
                            ValidationFailureKind::Generic,
                            "Expiry seconds must be positive".to_string(),
                        ));
                    }
                    Ok(Utc::now() + Duration::seconds(seconds))
                })
                .transpose()?,
        };

        self.offline_service.upsert_cache_metadata(update).await?;
        Ok(json!({ "success": true }))
    }

    pub async fn save_optimistic_update(
        &self,
        request: OptimisticUpdateRequest,
    ) -> Result<String, AppError> {
        request.validate()?;

        let draft = OptimisticUpdateDraft::new(
            parse_entity_type(&request.entity_type)?,
            parse_entity_id(&request.entity_id)?,
            match request.original_data {
                Some(ref data) => Some(parse_payload(data)?),
                None => None,
            },
            parse_payload(&request.updated_data)?,
        );

        let update_id = self.offline_service.save_optimistic_update(draft).await?;
        Ok(update_id.to_string())
    }

    pub async fn confirm_optimistic_update(&self, update_id: String) -> Result<Value, AppError> {
        if update_id.is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Update ID is required".to_string(),
            ));
        }

        let id = OptimisticUpdateId::new(update_id)
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        self.offline_service.confirm_optimistic_update(id).await?;

        Ok(json!({ "success": true }))
    }

    pub async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError> {
        if update_id.is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Update ID is required".to_string(),
            ));
        }

        let id = OptimisticUpdateId::new(update_id)
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        let original = self.offline_service.rollback_optimistic_update(id).await?;

        let serialized = original
            .map(|payload| serde_json::to_string(&payload.into_inner()))
            .transpose()
            .map_err(|err| AppError::SerializationError(err.to_string()))?;

        Ok(serialized)
    }

    pub async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        let cleaned = self.offline_service.cleanup_expired_cache().await?;
        cleaned
            .try_into()
            .map_err(|_| AppError::Internal("Cleanup count overflowed i32".to_string()))
    }

    pub async fn update_sync_status(
        &self,
        request: UpdateSyncStatusRequest,
    ) -> Result<Value, AppError> {
        request.validate()?;

        let update = SyncStatusUpdate::new(
            parse_entity_type(&request.entity_type)?,
            parse_entity_id(&request.entity_id)?,
            map_sync_status(&request.sync_status),
            parse_optional_payload(request.conflict_data)?,
            Utc::now(),
        );

        self.offline_service.update_sync_status(update).await?;
        Ok(json!({ "success": true }))
    }
}

fn parse_public_key(value: &str) -> Result<PublicKey, AppError> {
    PublicKey::from_hex_str(value)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

fn parse_action_type(value: &str) -> Result<OfflineActionType, AppError> {
    OfflineActionType::new(value.to_string())
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

fn parse_entity_type(value: &str) -> Result<EntityType, AppError> {
    EntityType::new(value.to_string())
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

fn parse_entity_id(value: &str) -> Result<EntityId, AppError> {
    EntityId::new(value.to_string())
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

fn parse_payload(data: &str) -> Result<OfflinePayload, AppError> {
    OfflinePayload::from_json_str(data)
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
}

fn parse_optional_payload(data: Option<String>) -> Result<Option<OfflinePayload>, AppError> {
    match data {
        Some(raw) => {
            let parsed =
                serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| Value::String(raw.clone()));
            OfflinePayload::new(parsed)
                .map(Some)
                .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))
        }
        None => Ok(None),
    }
}

impl OfflineHandler {
    async fn record_sync_queue_metadata(
        &self,
        cache_type_str: &str,
        payload: &Value,
        queue_id: SyncQueueId,
    ) -> Result<(), AppError> {
        let cache_key = CacheKey::new(format!("sync_queue::{cache_type_str}"))
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        let cache_type = CacheType::new("sync_queue".to_string())
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        // NOTE: 現状は sync_queue を仮想的なキャッシュ種別としてまとめている。
        // 将来的にキャッシュ種類ごとのキュー状況を分離する場合はここで cache_type を切り替える。

        let requested_at = payload
            .get("requestedAt")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        let metadata = json!({
            "cacheType": cache_type_str,
            "requestedAt": requested_at,
            "requestedBy": payload.get("userPubkey").and_then(|value| value.as_str()),
            "source": payload.get("source").and_then(|value| value.as_str()).unwrap_or("unknown"),
            "queueItemId": queue_id.value(),
        });

        let update = CacheMetadataUpdate {
            cache_key,
            cache_type,
            metadata: Some(metadata),
            expiry: Some(Utc::now() + Duration::minutes(30)),
            is_stale: Some(true),
        };

        self.offline_service.upsert_cache_metadata(update).await
    }
}

fn map_action_record(record: &OfflineActionRecord) -> Result<OfflineAction, AppError> {
    Ok(OfflineAction {
        id: record.record_id.unwrap_or_default(),
        user_pubkey: record.user_pubkey.as_hex().to_string(),
        action_type: record.action_type.as_str().to_string(),
        target_id: record.target_id.as_ref().map(ToString::to_string),
        action_data: serde_json::to_string(record.payload.as_json())
            .map_err(|err| AppError::SerializationError(err.to_string()))?,
        local_id: record.action_id.to_string(),
        remote_id: record.remote_id.as_ref().map(ToString::to_string),
        is_synced: matches!(record.sync_status, SyncStatus::FullySynced),
        created_at: record.created_at.timestamp(),
        synced_at: record.synced_at.map(|ts| ts.timestamp()),
        error_message: record.error_message.clone(),
    })
}

fn map_sync_queue_item(item: &SyncQueueItem) -> Result<SyncQueueItemResponse, AppError> {
    Ok(SyncQueueItemResponse {
        id: item.id.value(),
        action_type: item.action_type.as_str().to_string(),
        status: item.status.as_str().to_string(),
        retry_count: i32::try_from(item.retry_count)
            .map_err(|_| AppError::Internal("retry_count overflowed i32".to_string()))?,
        max_retries: i32::try_from(item.max_retries)
            .map_err(|_| AppError::Internal("max_retries overflowed i32".to_string()))?,
        created_at: item.created_at.timestamp(),
        updated_at: item.updated_at.timestamp(),
        synced_at: item.synced_at.map(|ts| ts.timestamp()),
        error_message: item.error_message.clone(),
        payload: item.payload.as_json().clone(),
    })
}

fn map_cache_status(snapshot: CacheStatusSnapshot) -> Result<CacheStatusResponse, AppError> {
    let cache_types = snapshot
        .cache_types
        .into_iter()
        .map(|status| {
            Ok(CacheTypeStatus {
                cache_type: status.cache_type.to_string(),
                item_count: status.item_count.try_into().map_err(|_| {
                    AppError::Internal("Cache item count overflowed i64".to_string())
                })?,
                last_synced_at: status.last_synced_at.map(|dt| dt.timestamp()),
                is_stale: status.is_stale,
                metadata: status.metadata,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(CacheStatusResponse {
        total_items: snapshot
            .total_items
            .try_into()
            .map_err(|_| AppError::Internal("Total items overflowed i64".to_string()))?,
        stale_items: snapshot
            .stale_items
            .try_into()
            .map_err(|_| AppError::Internal("Stale items overflowed i64".to_string()))?,
        cache_types,
    })
}

fn map_sync_status(value: &str) -> SyncStatus {
    match value {
        "pending" => SyncStatus::Pending,
        "syncing" => SyncStatus::SentToP2P,
        "synced" => SyncStatus::FullySynced,
        "failed" => SyncStatus::Failed,
        "conflict" => SyncStatus::Conflict,
        other => SyncStatus::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::offline_store::OfflinePersistence;
    use crate::application::services::offline_service::OfflineService;
    use crate::infrastructure::offline::sqlite_store::SqliteOfflinePersistence;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::{Pool, Sqlite};
    use std::sync::Arc;

    async fn setup_handler() -> (OfflineHandler, Pool<Sqlite>) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:?cache=shared")
            .await
            .expect("in-memory sqlite");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        let persistence: Arc<dyn OfflinePersistence> =
            Arc::new(SqliteOfflinePersistence::new(pool.clone()));
        let service = OfflineService::new(persistence);
        (OfflineHandler::new(Arc::new(service)), pool)
    }

    #[tokio::test]
    async fn add_to_sync_queue_records_metadata_entry() {
        let (handler, pool) = setup_handler().await;
        let request = AddToSyncQueueRequest {
            action_type: "manual_sync_refresh".to_string(),
            payload: serde_json::json!({
                "cacheType": "offline_actions",
                "requestedAt": "2025-11-09T00:00:00Z",
                "source": "sync_status_indicator",
                "userPubkey": "npub1testexample"
            }),
            priority: Some(5),
        };

        let queue_id = handler.add_to_sync_queue(request).await.expect("queue id");

        let (cache_key, cache_type, metadata): (String, String, Option<String>) = sqlx::query_as(
            r#"
            SELECT cache_key, cache_type, metadata
            FROM cache_metadata
            WHERE cache_key = 'sync_queue::offline_actions'
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("metadata row");

        assert_eq!(cache_key, "sync_queue::offline_actions");
        assert_eq!(cache_type, "sync_queue");

        let json = metadata.expect("metadata json");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse metadata");
        assert_eq!(
            parsed.get("queueItemId").and_then(|value| value.as_i64()),
            Some(queue_id)
        );
        assert_eq!(
            parsed.get("requestedBy").and_then(|value| value.as_str()),
            Some("npub1testexample")
        );
        assert_eq!(
            parsed.get("source").and_then(|value| value.as_str()),
            Some("sync_status_indicator")
        );
    }

    #[tokio::test]
    async fn list_sync_queue_items_returns_recent_rows() {
        let (handler, pool) = setup_handler().await;

        let first_id = handler
            .add_to_sync_queue(AddToSyncQueueRequest {
                action_type: "manual_sync_refresh".to_string(),
                payload: serde_json::json!({
                    "cacheType": "offline_actions",
                    "source": "sync_status_indicator"
                }),
                priority: Some(3),
            })
            .await
            .expect("first queue id");

        let second_id = handler
            .add_to_sync_queue(AddToSyncQueueRequest {
                action_type: "manual_sync_refresh".to_string(),
                payload: serde_json::json!({
                    "cacheType": "cache_metadata",
                    "source": "sync_status_indicator"
                }),
                priority: Some(2),
            })
            .await
            .expect("second queue id");

        sqlx::query(
            r#"
            UPDATE sync_queue
            SET status = 'failed',
                error_message = 'timeout',
                updated_at = strftime('%s','now')
            WHERE id = ?1
            "#,
        )
        .bind(first_id)
        .execute(&pool)
        .await
        .expect("update queue row");

        let items = handler
            .list_sync_queue_items(ListSyncQueueItemsRequest { limit: Some(10) })
            .await
            .expect("queue items");

        assert!(
            items.len() >= 2,
            "expected at least two queue items, got {}",
            items.len()
        );

        let failed = items
            .iter()
            .find(|item| item.id == first_id)
            .expect("failed queue item present");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.error_message.as_deref(), Some("timeout"));

        assert!(
            items.iter().any(|item| item.id == second_id),
            "second queue id should be present"
        );
    }
}
