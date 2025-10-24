use crate::application::ports::offline_store::OfflinePersistence;
use crate::modules::offline::models::{
    AddToSyncQueueRequest, CacheStatusResponse, CacheTypeStatus, GetOfflineActionsRequest,
    OfflineAction, SaveOfflineActionRequest, SaveOfflineActionResponse, SyncOfflineActionsRequest,
    SyncOfflineActionsResponse, UpdateCacheMetadataRequest,
};
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Pool, QueryBuilder, Row, Sqlite};
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

    async fn get_offline_action_by_id(&self, id: i64) -> Result<OfflineAction, AppError> {
        let action = sqlx::query_as::<_, OfflineAction>(
            r#"
            SELECT * FROM offline_actions
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(self.pool())
        .await?;

        Ok(action)
    }
}

#[async_trait]
impl OfflinePersistence for SqliteOfflinePersistence {
    async fn save_offline_action(
        &self,
        request: SaveOfflineActionRequest,
    ) -> Result<SaveOfflineActionResponse, AppError> {
        let local_id = Uuid::new_v4().to_string();
        let action_data = serde_json::to_string(&request.action_data)?;
        let created_at = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO offline_actions (
                user_pubkey, action_type, target_id, action_data,
                local_id, is_synced, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
        )
        .bind(&request.user_pubkey)
        .bind(&request.action_type)
        .bind(&request.target_id)
        .bind(&action_data)
        .bind(&local_id)
        .bind(created_at)
        .execute(self.pool())
        .await?;

        let id = result.last_insert_rowid();
        let action = self.get_offline_action_by_id(id).await?;

        Ok(SaveOfflineActionResponse {
            local_id: action.local_id.clone(),
            action,
        })
    }

    async fn get_offline_actions(
        &self,
        request: GetOfflineActionsRequest,
    ) -> Result<Vec<OfflineAction>, AppError> {
        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT * FROM offline_actions WHERE 1=1");

        if let Some(user_pubkey) = request.user_pubkey.as_ref() {
            builder.push(" AND user_pubkey = ");
            builder.push_bind(user_pubkey);
        }

        if let Some(is_synced) = request.is_synced {
            builder.push(" AND is_synced = ");
            builder.push_bind(if is_synced { 1 } else { 0 });
        }

        builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = request.limit {
            builder.push(" LIMIT ");
            builder.push_bind(limit);
        }

        let query = builder.build_query_as::<OfflineAction>();
        let actions = query.fetch_all(self.pool()).await?;

        Ok(actions)
    }

    async fn sync_offline_actions(
        &self,
        request: SyncOfflineActionsRequest,
    ) -> Result<SyncOfflineActionsResponse, AppError> {
        let unsynced_actions = sqlx::query_as::<_, OfflineAction>(
            r#"
            SELECT * FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(&request.user_pubkey)
        .fetch_all(self.pool())
        .await?;

        let mut synced_count = 0;
        let failed_count = 0;

        for action in unsynced_actions.iter() {
            let payload: serde_json::Value = serde_json::from_str(&action.action_data)?;
            let result = self
                .add_to_sync_queue(AddToSyncQueueRequest {
                    action_type: action.action_type.clone(),
                    payload,
                })
                .await;

            if result.is_ok() {
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
                synced_count += 1;
            }
        }

        let pending_result = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM offline_actions
            WHERE user_pubkey = ?1 AND is_synced = 0
            "#,
        )
        .bind(&request.user_pubkey)
        .fetch_one(self.pool())
        .await?;

        let pending_count: i32 = pending_result.try_get("count").unwrap_or(0);

        Ok(SyncOfflineActionsResponse {
            synced_count,
            failed_count,
            pending_count,
        })
    }

    async fn get_cache_status(&self) -> Result<CacheStatusResponse, AppError> {
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

        Ok(CacheStatusResponse {
            total_items,
            stale_items,
            cache_types,
        })
    }

    async fn add_to_sync_queue(&self, request: AddToSyncQueueRequest) -> Result<i64, AppError> {
        let payload = serde_json::to_string(&request.payload)?;
        let created_at = Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO sync_queue (action_type, payload, status, created_at, updated_at)
            VALUES (?1, ?2, 'pending', ?3, ?3)
            "#,
        )
        .bind(&request.action_type)
        .bind(&payload)
        .bind(created_at)
        .execute(self.pool())
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update_cache_metadata(
        &self,
        request: UpdateCacheMetadataRequest,
    ) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        let metadata = request
            .metadata
            .map(|value| serde_json::to_string(&value))
            .transpose()?;
        let expiry_time = request.expiry_seconds.map(|seconds| now + seconds);

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
        .bind(&request.cache_key)
        .bind(&request.cache_type)
        .bind(now)
        .bind(expiry_time)
        .bind(metadata)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn save_optimistic_update(
        &self,
        entity_type: String,
        entity_id: String,
        original_data: Option<String>,
        updated_data: String,
    ) -> Result<String, AppError> {
        let update_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO optimistic_updates (
                update_id, entity_type, entity_id, original_data,
                updated_data, is_confirmed, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            "#,
        )
        .bind(&update_id)
        .bind(&entity_type)
        .bind(&entity_id)
        .bind(&original_data)
        .bind(&updated_data)
        .bind(created_at)
        .execute(self.pool())
        .await?;

        Ok(update_id)
    }

    async fn confirm_optimistic_update(&self, update_id: String) -> Result<(), AppError> {
        let confirmed_at = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE optimistic_updates
            SET is_confirmed = 1, confirmed_at = ?1
            WHERE update_id = ?2
            "#,
        )
        .bind(confirmed_at)
        .bind(&update_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError> {
        let update = sqlx::query_as::<_, (Option<String>,)>(
            r#"
                SELECT original_data FROM optimistic_updates
                WHERE update_id = ?1
                "#,
        )
        .bind(&update_id)
        .fetch_optional(self.pool())
        .await?;

        if let Some((original_data,)) = update {
            sqlx::query(r#"DELETE FROM optimistic_updates WHERE update_id = ?1"#)
                .bind(&update_id)
                .execute(self.pool())
                .await?;

            return Ok(original_data);
        }

        Ok(None)
    }

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
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

        Ok(result.rows_affected() as i32)
    }

    async fn update_sync_status(
        &self,
        entity_type: String,
        entity_id: String,
        sync_status: String,
        conflict_data: Option<String>,
    ) -> Result<(), AppError> {
        let now = Utc::now().timestamp();

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
        .bind(&entity_type)
        .bind(&entity_id)
        .bind(now)
        .bind(&sync_status)
        .bind(&conflict_data)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
