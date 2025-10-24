use crate::application::ports::offline_store::OfflinePersistence;
use crate::domain::entities::offline::{
    CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionRecord, OptimisticUpdateDraft,
    SyncQueueItemDraft, SyncResult, SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    OfflineActionType, OfflinePayload, OptimisticUpdateId, SyncQueueId,
};
use crate::infrastructure::offline::mappers::{
    domain_cache_status_from_module, domain_offline_action_from_module,
    optimistic_update_id_from_string, sync_queue_id_from_i64,
};
use crate::modules::offline::models::{CacheStatusResponse, CacheTypeStatus, OfflineAction};
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Pool, QueryBuilder, Row, Sqlite};
use std::convert::TryInto;
use uuid::Uuid;

pub struct SqliteOfflinePersistence {
    pool: Pool<Sqlite>,
}

impl SqliteOfflinePersistence {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    async fn get_offline_action_by_id(&self, id: i64) -> Result<OfflineActionRecord, AppError> {
        let action = sqlx::query_as::<_, OfflineAction>(
            r#"
            SELECT * FROM offline_actions
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(self.pool())
        .await?;

        domain_offline_action_from_module(action)
    }
}

#[async_trait]
impl OfflinePersistence for SqliteOfflinePersistence {
    async fn save_action(
        &self,
        draft: crate::domain::entities::offline::OfflineActionDraft,
    ) -> Result<crate::domain::entities::offline::SavedOfflineAction, AppError> {
        use crate::domain::entities::offline::{OfflineActionDraft, SavedOfflineAction};

        let OfflineActionDraft {
            user_pubkey,
            action_type,
            target_id,
            payload,
        } = draft;

        let local_id = Uuid::new_v4().to_string();
        let payload_value = payload.into_inner();
        let action_data = serde_json::to_string(&payload_value)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let created_at = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO offline_actions (
                user_pubkey, action_type, target_id, action_data,
                local_id, is_synced, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
        )
        .bind(user_pubkey.as_hex())
        .bind(action_type.as_str())
        .bind(target_id.as_ref().map(|value| value.to_string()))
        .bind(&action_data)
        .bind(&local_id)
        .bind(created_at)
        .execute(self.pool())
        .await?;

        let id = result.last_insert_rowid();
        let action = self.get_offline_action_by_id(id).await?;

        Ok(SavedOfflineAction::new(action.action_id.clone(), action))
    }

    async fn list_actions(
        &self,
        filter: crate::domain::entities::offline::OfflineActionFilter,
    ) -> Result<Vec<crate::domain::entities::offline::OfflineActionRecord>, AppError> {
        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT * FROM offline_actions WHERE 1=1");

        if let Some(user_pubkey) = filter.user_pubkey.as_ref() {
            builder.push(" AND user_pubkey = ");
            builder.push_bind(user_pubkey.as_hex());
        }

        if let Some(is_synced) = filter.include_synced {
            builder.push(" AND is_synced = ");
            builder.push_bind(if is_synced { 1 } else { 0 });
        }

        builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            builder.push(" LIMIT ");
            builder.push_bind(limit as i64);
        }

        let query = builder.build_query_as::<OfflineAction>();
        let actions = query.fetch_all(self.pool()).await?;

        actions
            .into_iter()
            .map(domain_offline_action_from_module)
            .collect()
    }

    async fn sync_actions(&self, user_pubkey: PublicKey) -> Result<SyncResult, AppError> {
        let unsynced_actions = sqlx::query_as::<_, OfflineAction>(
            r#"
            SELECT * FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_pubkey.as_hex())
        .fetch_all(self.pool())
        .await?;

        let mut synced_count: u32 = 0;

        for action in unsynced_actions.iter() {
            let payload_value: serde_json::Value = serde_json::from_str(&action.action_data)
                .map_err(|err| AppError::DeserializationError(err.to_string()))?;
            let action_type = OfflineActionType::new(action.action_type.clone())
                .map_err(AppError::ValidationError)?;
            let payload_vo =
                OfflinePayload::new(payload_value).map_err(AppError::ValidationError)?;
            let enqueue_result = self
                .enqueue_sync(SyncQueueItemDraft::new(action_type, payload_vo, None))
                .await;

            if enqueue_result.is_ok() {
                let synced_at = Utc::now().timestamp();
                sqlx::query(
                    r#"
                    UPDATE offline_actions
                    SET is_synced = 1, synced_at = ?1
                    WHERE id = ?2
                    "#,
                )
                .bind(synced_at)
                .bind(action.id)
                .execute(self.pool())
                .await?;
                synced_count = synced_count.saturating_add(1);
            }
        }

        let pending_result = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            "#,
        )
        .bind(user_pubkey.as_hex())
        .fetch_one(self.pool())
        .await?;

        let pending_count: u32 = pending_result
            .try_get::<i32, _>("count")
            .unwrap_or(0)
            .try_into()
            .unwrap_or(0);

        Ok(SyncResult::new(synced_count, 0, pending_count))
    }

    async fn cache_status(&self) -> Result<CacheStatusSnapshot, AppError> {
        let total_result = sqlx::query(r#"SELECT COUNT(*) as count FROM cache_metadata"#)
            .fetch_one(self.pool())
            .await?;
        let total_items: i64 = total_result.try_get("count").unwrap_or(0);

        let stale_result =
            sqlx::query(r#"SELECT COUNT(*) as count FROM cache_metadata WHERE is_stale = 1"#)
                .fetch_one(self.pool())
                .await?;
        let stale_items: i64 = stale_result.try_get("count").unwrap_or(0);

        let cache_types_rows = sqlx::query(
            r#"
            SELECT
                cache_type,
                COUNT(*) as item_count,
                MAX(last_synced_at) as last_synced_at,
                MAX(is_stale) as is_stale
            FROM cache_metadata
            GROUP BY cache_type
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        let cache_types: Vec<CacheTypeStatus> = cache_types_rows
            .into_iter()
            .map(|row| CacheTypeStatus {
                cache_type: row.try_get("cache_type").unwrap_or_default(),
                item_count: row.try_get("item_count").unwrap_or(0),
                last_synced_at: row.try_get("last_synced_at").ok(),
                is_stale: row.try_get::<i32, _>("is_stale").unwrap_or(0) > 0,
            })
            .collect();

        let response = CacheStatusResponse {
            total_items,
            stale_items,
            cache_types,
        };

        domain_cache_status_from_module(response)
    }

    async fn enqueue_sync(&self, draft: SyncQueueItemDraft) -> Result<SyncQueueId, AppError> {
        let payload_json = serde_json::to_string(&draft.payload.into_inner())
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let created_at = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO sync_queue (action_type, payload, status, created_at, updated_at)
            VALUES (?1, ?2, 'pending', ?3, ?3)
            "#,
        )
        .bind(draft.action_type.as_str())
        .bind(&payload_json)
        .bind(created_at)
        .execute(self.pool())
        .await?;

        sync_queue_id_from_i64(result.last_insert_rowid())
    }

    async fn upsert_cache_metadata(&self, update: CacheMetadataUpdate) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        let metadata = update
            .metadata
            .map(|value| serde_json::to_string(&value))
            .transpose()
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let expiry_time = update.expiry.map(|expiry| {
            let seconds = expiry.signed_duration_since(Utc::now()).num_seconds();
            now + seconds.max(0)
        });

        sqlx::query(
            r#"
            INSERT INTO cache_metadata (
                cache_key, cache_type, last_synced_at, last_accessed_at,
                data_version, is_stale, expiry_time, metadata
            ) VALUES (?1, ?2, ?3, ?3, 1, 0, ?4, ?5)
            ON CONFLICT(cache_key) DO UPDATE SET
                cache_type = excluded.cache_type,
                last_synced_at = excluded.last_synced_at,
                last_accessed_at = excluded.last_accessed_at,
                data_version = data_version + 1,
                expiry_time = excluded.expiry_time,
                metadata = excluded.metadata
            "#,
        )
        .bind(update.cache_key.as_str())
        .bind(update.cache_type.as_str())
        .bind(now)
        .bind(expiry_time)
        .bind(metadata)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn save_optimistic_update(
        &self,
        draft: OptimisticUpdateDraft,
    ) -> Result<OptimisticUpdateId, AppError> {
        let update_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().timestamp();
        let original = draft
            .original_data
            .map(|payload| serde_json::to_string(&payload.into_inner()))
            .transpose()
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let updated = serde_json::to_string(&draft.updated_data.into_inner())
            .map_err(|err| AppError::SerializationError(err.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO optimistic_updates (
                update_id, entity_type, entity_id, original_data,
                updated_data, is_confirmed, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
        )
        .bind(&update_id)
        .bind(draft.entity_type.as_str())
        .bind(draft.entity_id.as_str())
        .bind(&original)
        .bind(&updated)
        .bind(created_at)
        .execute(self.pool())
        .await?;

        optimistic_update_id_from_string(update_id)
    }

    async fn confirm_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<(), AppError> {
        let confirmed_at = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE optimistic_updates
            SET is_confirmed = 1, confirmed_at = ?1
            WHERE update_id = ?2
            "#,
        )
        .bind(confirmed_at)
        .bind(update_id.as_str())
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: OptimisticUpdateId,
    ) -> Result<Option<OfflinePayload>, AppError> {
        let update = sqlx::query_as::<_, (Option<String>,)>(
            r#"
                SELECT original_data FROM optimistic_updates
                WHERE update_id = ?1
                "#,
        )
        .bind(update_id.as_str())
        .fetch_optional(self.pool())
        .await?;

        if let Some((original_data,)) = update {
            sqlx::query(r#"DELETE FROM optimistic_updates WHERE update_id = ?1"#)
                .bind(update_id.as_str())
                .execute(self.pool())
                .await?;

            return crate::infrastructure::offline::mappers::payload_from_optional_json(
                original_data,
            );
        }

        Ok(None)
    }

    async fn cleanup_expired_cache(&self) -> Result<u32, AppError> {
        let now = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            DELETE FROM cache_metadata
            WHERE expiry_time IS NOT NULL AND expiry_time < ?1
            "#,
        )
        .bind(now)
        .execute(self.pool())
        .await?;

        result
            .rows_affected()
            .try_into()
            .map_err(|_| AppError::Internal("Cleanup count overflowed u32".to_string()))
    }

    async fn update_sync_status(&self, update: SyncStatusUpdate) -> Result<(), AppError> {
        let conflict_data = update
            .conflict_data
            .map(|payload| serde_json::to_string(&payload.into_inner()))
            .transpose()
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let updated_at = update.updated_at.timestamp();

        sqlx::query(
            r#"
            INSERT INTO sync_status (
                entity_type, entity_id, local_version, last_local_update,
                sync_status, conflict_data
            ) VALUES (?1, ?2, 1, ?3, ?4, ?5)
            ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                local_version = local_version + 1,
                last_local_update = excluded.last_local_update,
                sync_status = excluded.sync_status,
                conflict_data = excluded.conflict_data
            "#,
        )
        .bind(update.entity_type.as_str())
        .bind(update.entity_id.as_str())
        .bind(updated_at)
        .bind(update.sync_status.as_str())
        .bind(&conflict_data)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
