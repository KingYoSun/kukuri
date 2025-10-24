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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::offline::{CacheKey, CacheType, SyncStatus};
    use crate::infrastructure::offline::SqliteOfflinePersistence;
    use chrono::{Duration, Utc};
    use sqlx::{Executor, Pool, Sqlite, sqlite::SqlitePoolOptions};

    const PUBKEY_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    async fn setup_service() -> (OfflineService, Pool<Sqlite>) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:?cache=shared")
            .await
            .unwrap();

        initialize_schema(&pool).await;

        let persistence: Arc<dyn OfflinePersistence> =
            Arc::new(SqliteOfflinePersistence::new(pool.clone()));
        (OfflineService::new(persistence), pool)
    }

    async fn initialize_schema(pool: &Pool<Sqlite>) {
        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS offline_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_pubkey TEXT NOT NULL,
                action_type TEXT NOT NULL,
                target_id TEXT,
                action_data TEXT NOT NULL,
                local_id TEXT NOT NULL,
                remote_id TEXT,
                is_synced INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                synced_at INTEGER
            )
            "#,
        )
        .await
        .unwrap();

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sync_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                synced_at INTEGER,
                error_message TEXT
            )
            "#,
        )
        .await
        .unwrap();

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS cache_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cache_key TEXT NOT NULL UNIQUE,
                cache_type TEXT NOT NULL,
                last_synced_at INTEGER,
                last_accessed_at INTEGER,
                data_version INTEGER DEFAULT 1,
                is_stale INTEGER DEFAULT 0,
                expiry_time INTEGER,
                metadata TEXT
            )
            "#,
        )
        .await
        .unwrap();

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS optimistic_updates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                update_id TEXT NOT NULL UNIQUE,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                original_data TEXT,
                updated_data TEXT NOT NULL,
                is_confirmed INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                confirmed_at INTEGER
            )
            "#,
        )
        .await
        .unwrap();

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sync_status (
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                local_version INTEGER NOT NULL,
                last_local_update INTEGER NOT NULL,
                sync_status TEXT NOT NULL,
                conflict_data TEXT,
                PRIMARY KEY (entity_type, entity_id)
            )
            "#,
        )
        .await
        .unwrap();
    }

    fn sample_save_params() -> SaveOfflineActionParams {
        SaveOfflineActionParams {
            user_pubkey: PublicKey::from_hex_str(PUBKEY_HEX).unwrap(),
            action_type: OfflineActionType::new("create_post".into()).unwrap(),
            entity_type: EntityType::new("post".into()).unwrap(),
            entity_id: EntityId::new("post123".into()).unwrap(),
            payload: OfflinePayload::from_json_str(r#"{"content":"Hello"}"#).unwrap(),
        }
    }

    #[tokio::test]
    async fn test_save_action_persists_record() {
        let (service, pool) = setup_service().await;

        let saved = service.save_action(sample_save_params()).await.unwrap();

        assert_eq!(saved.action.user_pubkey.as_hex(), PUBKEY_HEX);
        assert_eq!(
            saved
                .action
                .target_id
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("post123")
        );
        assert_eq!(saved.action.action_type.as_str(), "create_post");

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM offline_actions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_list_actions_filters_by_sync_state() {
        let (service, pool) = setup_service().await;

        let first = service.save_action(sample_save_params()).await.unwrap();
        let mut second_params = sample_save_params();
        second_params.entity_id = EntityId::new("post124".into()).unwrap();
        service.save_action(second_params).await.unwrap();

        sqlx::query("UPDATE offline_actions SET is_synced = 1 WHERE id = ?1")
            .bind(first.action.record_id.expect("record id"))
            .execute(&pool)
            .await
            .unwrap();

        let synced = service
            .list_actions(OfflineActionsQuery {
                user_pubkey: Some(PublicKey::from_hex_str(PUBKEY_HEX).unwrap()),
                include_synced: Some(true),
                limit: None,
            })
            .await
            .unwrap();
        assert_eq!(synced.len(), 1);

        let unsynced = service
            .list_actions(OfflineActionsQuery {
                user_pubkey: Some(PublicKey::from_hex_str(PUBKEY_HEX).unwrap()),
                include_synced: Some(false),
                limit: None,
            })
            .await
            .unwrap();
        assert_eq!(unsynced.len(), 1);
    }

    #[tokio::test]
    async fn test_sync_actions_marks_entries_and_enqueues() {
        let (service, pool) = setup_service().await;

        service.save_action(sample_save_params()).await.unwrap();

        let result = service
            .sync_actions(PublicKey::from_hex_str(PUBKEY_HEX).unwrap())
            .await
            .unwrap();
        assert_eq!(result.synced_count, 1);
        assert_eq!(result.failed_count, 0);

        let (is_synced,): (i64,) = sqlx::query_as("SELECT is_synced FROM offline_actions LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(is_synced, 1);

        let (queue_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sync_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(queue_count, 1);
    }

    #[tokio::test]
    async fn test_upsert_cache_metadata_and_cleanup() {
        let (service, pool) = setup_service().await;

        let update = CacheMetadataUpdate {
            cache_key: CacheKey::new("cache:topics".into()).unwrap(),
            cache_type: CacheType::new("topics".into()).unwrap(),
            metadata: Some(serde_json::json!({"version":1})),
            expiry: Some(Utc::now() + Duration::seconds(1)),
        };

        service.upsert_cache_metadata(update).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let removed = service.cleanup_expired_cache().await.unwrap();
        assert_eq!(removed, 1);

        let (remaining,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cache_metadata")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[tokio::test]
    async fn test_update_sync_status_upserts_record() {
        let (service, pool) = setup_service().await;

        let update = SyncStatusUpdate::new(
            EntityType::new("post".into()).unwrap(),
            EntityId::new("p1".into()).unwrap(),
            SyncStatus::from("pending"),
            Some(OfflinePayload::new(Value::String("conflict".into())).unwrap()),
            Utc::now(),
        );
        service.update_sync_status(update).await.unwrap();

        let update_resolved = SyncStatusUpdate::new(
            EntityType::new("post".into()).unwrap(),
            EntityId::new("p1".into()).unwrap(),
            SyncStatus::from("resolved"),
            None,
            Utc::now(),
        );
        service.update_sync_status(update_resolved).await.unwrap();

        let (local_version, sync_status, conflict_data): (i64, String, Option<String>) =
            sqlx::query_as(
                r#"
                SELECT local_version, sync_status, conflict_data
                FROM sync_status
                WHERE entity_type = 'post' AND entity_id = 'p1'
                "#,
            )
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(local_version, 2);
        assert_eq!(sync_status, "resolved");
        assert!(conflict_data.is_none());
    }
}
