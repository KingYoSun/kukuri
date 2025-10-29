use crate::application::ports::offline_store::OfflinePersistence;
use crate::domain::entities::offline::{
    CacheMetadataRecord, CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionRecord,
    OptimisticUpdateDraft, OptimisticUpdateRecord, SyncQueueItem, SyncQueueItemDraft, SyncResult,
    SyncStatusRecord, SyncStatusUpdate,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{OfflinePayload, OptimisticUpdateId, SyncQueueId};
use crate::infrastructure::offline::mappers::{
    CacheTypeAggregate, cache_metadata_from_row, cache_status_from_aggregates,
    offline_action_from_row, optimistic_update_from_row, optimistic_update_id_from_string,
    payload_from_optional_json_str, payload_to_string, sync_queue_id_from_i64,
    sync_queue_item_from_row, sync_status_from_row,
};
use crate::infrastructure::offline::rows::{
    CacheMetadataRow, OfflineActionRow, OptimisticUpdateRow, SyncQueueItemRow, SyncStatusRow,
};
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

    pub async fn enqueue_if_missing(&self, action: &OfflineActionRecord) -> Result<bool, AppError> {
        let payload = payload_to_string(&action.payload)?;

        let existing = sqlx::query(
            r#"
            SELECT id FROM sync_queue
            WHERE action_type = ?1 AND payload = ?2
              AND status IN ('pending', 'failed')
            LIMIT 1
            "#,
        )
        .bind(action.action_type.as_str())
        .bind(&payload)
        .fetch_optional(self.pool())
        .await?;

        if existing.is_some() {
            return Ok(false);
        }

        self.enqueue_sync(SyncQueueItemDraft::new(
            action.action_type.clone(),
            action.payload.clone(),
            None,
        ))
        .await?;

        Ok(true)
    }

    pub async fn list_pending_sync_queue(&self) -> Result<Vec<SyncQueueItem>, AppError> {
        let rows = sqlx::query_as::<_, SyncQueueItemRow>(
            r#"
            SELECT * FROM sync_queue
            WHERE status IN ('pending', 'failed')
            ORDER BY updated_at ASC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(sync_queue_item_from_row).collect()
    }

    pub async fn list_stale_cache_entries(&self) -> Result<Vec<CacheMetadataRecord>, AppError> {
        let now = Utc::now().timestamp();
        let rows = sqlx::query_as::<_, CacheMetadataRow>(
            r#"
            SELECT * FROM cache_metadata
            WHERE is_stale = 1
               OR (expiry_time IS NOT NULL AND expiry_time < ?1)
            ORDER BY COALESCE(last_synced_at, 0) ASC
            "#,
        )
        .bind(now)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(cache_metadata_from_row).collect()
    }

    pub async fn list_unconfirmed_updates(&self) -> Result<Vec<OptimisticUpdateRecord>, AppError> {
        let rows = sqlx::query_as::<_, OptimisticUpdateRow>(
            r#"
            SELECT * FROM optimistic_updates
            WHERE is_confirmed = 0
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(optimistic_update_from_row).collect()
    }

    pub async fn list_sync_conflicts(&self) -> Result<Vec<SyncStatusRecord>, AppError> {
        let rows = sqlx::query_as::<_, SyncStatusRow>(
            r#"
            SELECT
                rowid as id,
                entity_type,
                entity_id,
                local_version,
                NULL AS remote_version,
                last_local_update,
                NULL AS last_remote_sync,
                sync_status,
                conflict_data
            FROM sync_status
            WHERE sync_status IN ('conflict', 'failed', 'pending')
            ORDER BY last_local_update DESC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(sync_status_from_row).collect()
    }

    async fn get_offline_action_by_id(&self, id: i64) -> Result<OfflineActionRecord, AppError> {
        let action = sqlx::query_as::<_, OfflineActionRow>(
            r#"
            SELECT * FROM offline_actions
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(self.pool())
        .await?;

        offline_action_from_row(action)
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
        let action_data = payload_to_string(&payload)?;
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

        let query = builder.build_query_as::<OfflineActionRow>();
        let actions = query.fetch_all(self.pool()).await?;

        actions.into_iter().map(offline_action_from_row).collect()
    }

    async fn sync_actions(&self, user_pubkey: PublicKey) -> Result<SyncResult, AppError> {
        let unsynced_actions = sqlx::query_as::<_, OfflineActionRow>(
            r#"
            SELECT * FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_pubkey.as_hex())
        .fetch_all(self.pool())
        .await?;

        let domain_actions = unsynced_actions
            .into_iter()
            .map(offline_action_from_row)
            .collect::<Result<Vec<_>, AppError>>()?;

        let mut synced_count: u32 = 0;

        for action in domain_actions.iter() {
            let enqueue_result = self
                .enqueue_sync(SyncQueueItemDraft::new(
                    action.action_type.clone(),
                    action.payload.clone(),
                    None,
                ))
                .await;

            if enqueue_result.is_ok() {
                let Some(record_id) = action.record_id else {
                    continue;
                };
                let synced_at = Utc::now().timestamp();
                sqlx::query(
                    r#"
                    UPDATE offline_actions
                    SET is_synced = 1, synced_at = ?1
                    WHERE id = ?2
                    "#,
                )
                .bind(synced_at)
                .bind(record_id)
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

        let cache_types = sqlx::query_as::<_, CacheTypeAggregate>(
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

        cache_status_from_aggregates(total_items, stale_items, cache_types)
    }

    async fn enqueue_sync(&self, draft: SyncQueueItemDraft) -> Result<SyncQueueId, AppError> {
        let payload_json = payload_to_string(&draft.payload)?;
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
            .map(|payload| payload_to_string(&payload))
            .transpose()?;
        let updated = payload_to_string(&draft.updated_data)?;

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

            return payload_from_optional_json_str(original_data);
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
            .map(|payload| payload_to_string(&payload))
            .transpose()?;
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

    async fn enqueue_if_missing(&self, action: &OfflineActionRecord) -> Result<bool, AppError> {
        SqliteOfflinePersistence::enqueue_if_missing(self, action).await
    }

    async fn pending_sync_items(&self) -> Result<Vec<SyncQueueItem>, AppError> {
        SqliteOfflinePersistence::list_pending_sync_queue(self).await
    }

    async fn stale_cache_entries(&self) -> Result<Vec<CacheMetadataRecord>, AppError> {
        SqliteOfflinePersistence::list_stale_cache_entries(self).await
    }

    async fn unconfirmed_updates(&self) -> Result<Vec<OptimisticUpdateRecord>, AppError> {
        SqliteOfflinePersistence::list_unconfirmed_updates(self).await
    }

    async fn sync_conflicts(&self) -> Result<Vec<SyncStatusRecord>, AppError> {
        SqliteOfflinePersistence::list_sync_conflicts(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::offline::{
        CacheMetadataUpdate, OfflineActionDraft, OfflineActionFilter, OptimisticUpdateDraft,
        SyncStatusUpdate,
    };
    use crate::domain::value_objects::event_gateway::PublicKey;
    use crate::domain::value_objects::offline::{
        CacheKey, CacheType, EntityId, EntityType, OfflineActionType, OfflinePayload,
        SyncQueueStatus, SyncStatus,
    };
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;

    const PUBKEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    async fn setup_persistence() -> SqliteOfflinePersistence {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        SqliteOfflinePersistence::new(pool)
    }

    fn sample_draft() -> OfflineActionDraft {
        OfflineActionDraft::new(
            PublicKey::from_hex_str(PUBKEY).unwrap(),
            OfflineActionType::new("create_post".to_string()).unwrap(),
            Some(EntityId::new("post_123".to_string()).unwrap()),
            OfflinePayload::from_json_str(r#"{"content":"test"}"#).unwrap(),
        )
    }

    #[tokio::test]
    async fn test_save_offline_action() {
        let persistence = setup_persistence().await;
        let saved = persistence.save_action(sample_draft()).await.unwrap();

        assert_eq!(saved.action.user_pubkey.as_hex(), PUBKEY);
        assert_eq!(saved.action.sync_status, SyncStatus::Pending);
        assert!(!saved.action.payload.as_json().is_null());
    }

    #[tokio::test]
    async fn test_list_offline_actions() {
        let persistence = setup_persistence().await;
        persistence.save_action(sample_draft()).await.unwrap();

        let actions = persistence
            .list_actions(OfflineActionFilter::new(
                Some(PublicKey::from_hex_str(PUBKEY).unwrap()),
                Some(false),
                None,
            ))
            .await
            .unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].user_pubkey.as_hex(), PUBKEY);
    }

    #[tokio::test]
    async fn test_enqueue_and_pending_queue() {
        let persistence = setup_persistence().await;
        persistence.save_action(sample_draft()).await.unwrap();

        let unsynced = persistence
            .list_actions(OfflineActionFilter::new(None, Some(false), None))
            .await
            .unwrap();
        let action = unsynced.first().unwrap();

        let inserted = persistence.enqueue_if_missing(action).await.unwrap();
        assert!(inserted);

        // 重複登録は false を返す
        let duplicated = persistence.enqueue_if_missing(action).await.unwrap();
        assert!(!duplicated);

        let pending = persistence.list_pending_sync_queue().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, SyncQueueStatus::Pending);
    }

    #[tokio::test]
    async fn test_cache_metadata_and_status() {
        let persistence = setup_persistence().await;
        persistence
            .upsert_cache_metadata(CacheMetadataUpdate {
                cache_key: CacheKey::new("posts".to_string()).unwrap(),
                cache_type: CacheType::new("topic".to_string()).unwrap(),
                metadata: Some(serde_json::json!({"last_id": "1"})),
                expiry: Some(Utc::now()),
            })
            .await
            .unwrap();

        let status = persistence.cache_status().await.unwrap();
        assert_eq!(status.total_items, 1);
    }

    #[tokio::test]
    async fn test_optimistic_update_lifecycle() {
        let persistence = setup_persistence().await;
        let draft = OptimisticUpdateDraft::new(
            EntityType::new("post".to_string()).unwrap(),
            EntityId::new("post_1".to_string()).unwrap(),
            Some(OfflinePayload::from_json_str(r#"{"likes":10}"#).unwrap()),
            OfflinePayload::from_json_str(r#"{"likes":11}"#).unwrap(),
        );

        let update_id = persistence.save_optimistic_update(draft).await.unwrap();
        persistence
            .confirm_optimistic_update(update_id.clone())
            .await
            .unwrap();

        let rollback_id = persistence
            .save_optimistic_update(OptimisticUpdateDraft::new(
                EntityType::new("post".to_string()).unwrap(),
                EntityId::new("post_2".to_string()).unwrap(),
                Some(OfflinePayload::from_json_str(r#"{"likes":1}"#).unwrap()),
                OfflinePayload::from_json_str(r#"{"likes":2}"#).unwrap(),
            ))
            .await
            .unwrap();

        let rolled_back = persistence
            .rollback_optimistic_update(rollback_id)
            .await
            .unwrap();
        assert!(rolled_back.is_some());
    }

    #[tokio::test]
    async fn test_sync_status_update() {
        let persistence = setup_persistence().await;
        let update = SyncStatusUpdate::new(
            EntityType::new("post".to_string()).unwrap(),
            EntityId::new("post_3".to_string()).unwrap(),
            SyncStatus::Pending,
            None,
            Utc::now(),
        );
        persistence.update_sync_status(update).await.unwrap();

        let conflicts = persistence.list_sync_conflicts().await.unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].entity_id.to_string(), "post_3");
    }
}
