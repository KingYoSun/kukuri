use crate::application::ports::offline_store::OfflinePersistence;
use crate::modules::offline::models::{
    AddToSyncQueueRequest as ManagerAddToSyncQueueRequest,
    CacheStatusResponse as ManagerCacheStatusResponse, CacheTypeStatus as ManagerCacheTypeStatus,
    GetOfflineActionsRequest as ManagerGetOfflineActionsRequest,
    OfflineAction as ManagerOfflineAction,
    SaveOfflineActionRequest as ManagerSaveOfflineActionRequest,
    SaveOfflineActionResponse as ManagerSaveOfflineActionResponse,
    SyncOfflineActionsRequest as ManagerSyncOfflineActionsRequest,
    UpdateCacheMetadataRequest as ManagerUpdateCacheMetadataRequest,
};
use crate::shared::error::AppError;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct OfflineActionRecord {
    pub id: i64,
    pub user_pubkey: String,
    pub action_type: String,
    pub target_id: Option<String>,
    pub action_data: String,
    pub local_id: String,
    pub remote_id: Option<String>,
    pub is_synced: bool,
    pub created_at: i64,
    pub synced_at: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SavedOfflineAction {
    pub local_id: String,
    pub action: OfflineActionRecord,
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub synced_count: i32,
    pub failed_count: i32,
    pub pending_count: i32,
}

#[derive(Debug, Clone)]
pub struct CacheTypeStatusData {
    pub cache_type: String,
    pub item_count: i64,
    pub last_synced_at: Option<i64>,
    pub is_stale: bool,
}

#[derive(Debug, Clone)]
pub struct CacheStatusData {
    pub total_items: i64,
    pub stale_items: i64,
    pub cache_types: Vec<CacheTypeStatusData>,
}

#[async_trait]
pub trait OfflineServiceTrait: Send + Sync {
    async fn save_action(
        &self,
        user_pubkey: String,
        action_type: String,
        entity_type: String,
        entity_id: String,
        data: String,
    ) -> Result<SavedOfflineAction, AppError>;

    async fn get_actions(
        &self,
        user_pubkey: Option<String>,
        is_synced: Option<bool>,
        limit: Option<i32>,
    ) -> Result<Vec<OfflineActionRecord>, AppError>;

    async fn sync_actions(&self, user_pubkey: String) -> Result<SyncResult, AppError>;

    async fn get_cache_status(&self) -> Result<CacheStatusData, AppError>;

    async fn add_to_sync_queue(
        &self,
        action_type: String,
        payload: Value,
        priority: Option<i32>,
    ) -> Result<i64, AppError>;

    async fn update_cache_metadata(
        &self,
        cache_key: String,
        cache_type: String,
        metadata: Option<Value>,
        expiry_seconds: Option<i64>,
    ) -> Result<(), AppError>;

    async fn save_optimistic_update(
        &self,
        entity_type: String,
        entity_id: String,
        original_data: Option<String>,
        updated_data: String,
    ) -> Result<String, AppError>;

    async fn confirm_optimistic_update(&self, update_id: String) -> Result<(), AppError>;

    async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError>;

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError>;

    async fn update_sync_status(
        &self,
        entity_type: String,
        entity_id: String,
        sync_status: String,
        conflict_data: Option<String>,
    ) -> Result<(), AppError>;
}

pub struct OfflineService {
    persistence: Arc<dyn OfflinePersistence>,
}

impl OfflineService {
    pub fn new(persistence: Arc<dyn OfflinePersistence>) -> Self {
        Self { persistence }
    }

    fn map_offline_action(action: ManagerOfflineAction) -> OfflineActionRecord {
        OfflineActionRecord {
            id: action.id,
            user_pubkey: action.user_pubkey,
            action_type: action.action_type,
            target_id: action.target_id,
            action_data: action.action_data,
            local_id: action.local_id,
            remote_id: action.remote_id,
            is_synced: action.is_synced,
            created_at: action.created_at,
            synced_at: action.synced_at,
            error_message: None,
        }
    }

    fn map_cache_status(status: ManagerCacheStatusResponse) -> CacheStatusData {
        CacheStatusData {
            total_items: status.total_items,
            stale_items: status.stale_items,
            cache_types: status
                .cache_types
                .into_iter()
                .map(|t: ManagerCacheTypeStatus| CacheTypeStatusData {
                    cache_type: t.cache_type,
                    item_count: t.item_count,
                    last_synced_at: t.last_synced_at,
                    is_stale: t.is_stale,
                })
                .collect(),
        }
    }

    fn build_action_payload(
        data: String,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Value, AppError> {
        let value = serde_json::from_str::<Value>(&data).map_err(|e| {
            AppError::ValidationError(format!("Invalid data payload. Expected JSON: {e}"))
        })?;

        let mut map = value.as_object().cloned().ok_or_else(|| {
            AppError::ValidationError("Data payload must be a JSON object".to_string())
        })?;

        map.insert(
            "entityType".to_string(),
            Value::String(entity_type.to_string()),
        );
        map.insert("entityId".to_string(), Value::String(entity_id.to_string()));

        Ok(Value::Object(map))
    }

    fn to_saved_action(response: ManagerSaveOfflineActionResponse) -> SavedOfflineAction {
        SavedOfflineAction {
            local_id: response.local_id,
            action: Self::map_offline_action(response.action),
        }
    }

    fn filter_and_limit(
        actions: Vec<ManagerOfflineAction>,
        user_pubkey: Option<String>,
        is_synced: Option<bool>,
        limit: Option<i32>,
    ) -> Vec<OfflineActionRecord> {
        let mut filtered = actions
            .into_iter()
            .filter(|action| {
                if let Some(ref user) = user_pubkey {
                    if action.user_pubkey != *user {
                        return false;
                    }
                }
                if let Some(flag) = is_synced {
                    if action.is_synced != flag {
                        return false;
                    }
                }
                true
            })
            .map(Self::map_offline_action)
            .collect::<Vec<_>>();

        if let Some(limit) = limit {
            let limit = limit.max(0) as usize;
            if filtered.len() > limit {
                filtered.truncate(limit);
            }
        }

        filtered
    }
}

#[async_trait]
impl OfflineServiceTrait for OfflineService {
    async fn save_action(
        &self,
        user_pubkey: String,
        action_type: String,
        entity_type: String,
        entity_id: String,
        data: String,
    ) -> Result<SavedOfflineAction, AppError> {
        let payload = Self::build_action_payload(data, &entity_type, &entity_id)?;

        let response = self
            .persistence
            .save_offline_action(ManagerSaveOfflineActionRequest {
                user_pubkey,
                action_type,
                target_id: Some(entity_id),
                action_data: payload,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let saved = Self::to_saved_action(response);
        let _domain_saved = converters::saved_action(&saved).ok();
        // TODO(OFF-S0-02): Stage 1 will swap the return type to domain::OfflineActionRecord.
        Ok(saved)
    }

    async fn get_actions(
        &self,
        user_pubkey: Option<String>,
        is_synced: Option<bool>,
        limit: Option<i32>,
    ) -> Result<Vec<OfflineActionRecord>, AppError> {
        let manager_response = self
            .persistence
            .get_offline_actions(ManagerGetOfflineActionsRequest {
                user_pubkey: user_pubkey.clone(),
                is_synced,
                limit,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let filtered = Self::filter_and_limit(manager_response, user_pubkey, is_synced, limit);
        let _domain_actions: Vec<_> = filtered
            .iter()
            .filter_map(|record| converters::offline_action_record(record).ok())
            .collect();
        // TODO(OFF-S0-02): Stage 1 will return domain::OfflineActionRecord values.
        Ok(filtered)
    }

    async fn sync_actions(&self, user_pubkey: String) -> Result<SyncResult, AppError> {
        let response = self
            .persistence
            .sync_offline_actions(ManagerSyncOfflineActionsRequest { user_pubkey })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let result = SyncResult {
            synced_count: response.synced_count,
            failed_count: response.failed_count,
            pending_count: response.pending_count,
        };
        let _domain_result = converters::sync_result(&result).ok();
        // TODO(OFF-S0-02): Stage 1 will return domain::SyncResult directly.
        Ok(result)
    }

    async fn get_cache_status(&self) -> Result<CacheStatusData, AppError> {
        let status = self
            .persistence
            .get_cache_status()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let mapped = Self::map_cache_status(status);
        let _domain_snapshot = converters::cache_status(&mapped).ok();
        // TODO(OFF-S0-02): Stage 1 will return domain::CacheStatusSnapshot.
        Ok(mapped)
    }

    async fn add_to_sync_queue(
        &self,
        action_type: String,
        payload: Value,
        _priority: Option<i32>,
    ) -> Result<i64, AppError> {
        self.persistence
            .add_to_sync_queue(ManagerAddToSyncQueueRequest {
                action_type,
                payload,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn update_cache_metadata(
        &self,
        cache_key: String,
        cache_type: String,
        metadata: Option<Value>,
        expiry_seconds: Option<i64>,
    ) -> Result<(), AppError> {
        self.persistence
            .update_cache_metadata(ManagerUpdateCacheMetadataRequest {
                cache_key,
                cache_type,
                metadata,
                expiry_seconds,
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn save_optimistic_update(
        &self,
        entity_type: String,
        entity_id: String,
        original_data: Option<String>,
        updated_data: String,
    ) -> Result<String, AppError> {
        self.persistence
            .save_optimistic_update(entity_type, entity_id, original_data, updated_data)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn confirm_optimistic_update(&self, update_id: String) -> Result<(), AppError> {
        self.persistence
            .confirm_optimistic_update(update_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn rollback_optimistic_update(
        &self,
        update_id: String,
    ) -> Result<Option<String>, AppError> {
        self.persistence
            .rollback_optimistic_update(update_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        self.persistence
            .cleanup_expired_cache()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn update_sync_status(
        &self,
        entity_type: String,
        entity_id: String,
        sync_status: String,
        conflict_data: Option<String>,
    ) -> Result<(), AppError> {
        self.persistence
            .update_sync_status(entity_type, entity_id, sync_status, conflict_data)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::offline::SqliteOfflinePersistence;
    use sqlx::{Executor, Pool, Sqlite, sqlite::SqlitePoolOptions};

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

    #[tokio::test]
    async fn test_save_action_persists_record() {
        let (service, pool) = setup_service().await;

        let saved = service
            .save_action(
                "npub1".into(),
                "create_post".into(),
                "post".into(),
                "post123".into(),
                r#"{"content":"Hello"}"#.into(),
            )
            .await
            .unwrap();

        assert_eq!(saved.action.user_pubkey, "npub1");
        assert_eq!(saved.action.target_id.as_deref(), Some("post123"));

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM offline_actions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let (action_data,): (String,) =
            sqlx::query_as("SELECT action_data FROM offline_actions LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(action_data.contains("\"entityType\":\"post\""));
        assert!(action_data.contains("\"entityId\":\"post123\""));
    }

    #[tokio::test]
    async fn test_get_actions_filters_by_user_and_sync_state() {
        let (service, pool) = setup_service().await;

        let first = service
            .save_action(
                "npub1".into(),
                "create".into(),
                "post".into(),
                "p1".into(),
                r#"{"content":"A"}"#.into(),
            )
            .await
            .unwrap();

        let _second = service
            .save_action(
                "npub2".into(),
                "create".into(),
                "post".into(),
                "p2".into(),
                r#"{"content":"B"}"#.into(),
            )
            .await
            .unwrap();

        // Mark first as synced
        sqlx::query("UPDATE offline_actions SET is_synced = 1 WHERE id = ?1")
            .bind(first.action.id)
            .execute(&pool)
            .await
            .unwrap();

        let synced_actions = service
            .get_actions(Some("npub1".into()), Some(true), None)
            .await
            .unwrap();
        assert_eq!(synced_actions.len(), 1);
        assert_eq!(synced_actions[0].local_id, first.action.local_id);

        let unsynced = service
            .get_actions(Some("npub2".into()), Some(false), None)
            .await
            .unwrap();
        assert_eq!(unsynced.len(), 1);
        assert_eq!(unsynced[0].user_pubkey, "npub2");
    }

    #[tokio::test]
    async fn test_sync_actions_marks_entries_and_enqueues() {
        let (service, pool) = setup_service().await;

        service
            .save_action(
                "npub1".into(),
                "create".into(),
                "post".into(),
                "p1".into(),
                r#"{"content":"sync"}"#.into(),
            )
            .await
            .unwrap();

        let result = service.sync_actions("npub1".into()).await.unwrap();
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
    async fn test_update_cache_metadata_and_cleanup() {
        let (service, pool) = setup_service().await;

        service
            .update_cache_metadata(
                "cache:topics".into(),
                "topics".into(),
                Some(serde_json::json!({"version":1})),
                Some(1),
            )
            .await
            .unwrap();

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

        service
            .update_sync_status(
                "post".into(),
                "p1".into(),
                "pending".into(),
                Some("conflict".into()),
            )
            .await
            .unwrap();

        service
            .update_sync_status("post".into(), "p1".into(), "resolved".into(), None)
            .await
            .unwrap();

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

mod converters {
    use super::{
        CacheStatusData, CacheTypeStatusData, OfflineActionRecord, SavedOfflineAction, SyncResult,
    };
    use crate::domain::entities::offline as domain_offline;
    use crate::domain::value_objects::event_gateway::PublicKey;
    use crate::domain::value_objects::offline::{
        CacheType, EntityId, OfflineActionId, OfflineActionType, OfflinePayload, RemoteEventId,
        SyncStatus,
    };
    use crate::shared::error::AppError;
    use chrono::{DateTime, Utc};
    use std::convert::TryInto;

    pub(super) fn offline_action_record(
        record: &OfflineActionRecord,
    ) -> Result<domain_offline::OfflineActionRecord, AppError> {
        let action_id = OfflineActionId::from_str(&record.local_id).map_err(|err| {
            AppError::ValidationError(format!("Invalid offline action id: {err}"))
        })?;
        let public_key = PublicKey::from_hex_str(&record.user_pubkey).map_err(|err| {
            AppError::ValidationError(format!("Invalid offline action pubkey: {err}"))
        })?;
        let action_type = OfflineActionType::new(record.action_type.clone()).map_err(|err| {
            AppError::ValidationError(format!("Invalid offline action type: {err}"))
        })?;
        let target_id = match &record.target_id {
            Some(value) if !value.trim().is_empty() => {
                Some(EntityId::new(value.clone()).map_err(|err| {
                    AppError::ValidationError(format!("Invalid offline action target id: {err}"))
                })?)
            }
            _ => None,
        };
        let payload = OfflinePayload::from_json_str(&record.action_data).map_err(|err| {
            AppError::DeserializationError(format!("Invalid offline action payload: {err}"))
        })?;
        let sync_status = if record.is_synced {
            SyncStatus::FullySynced
        } else {
            SyncStatus::Pending
        };
        let created_at = timestamp_to_datetime(record.created_at);
        let synced_at = record.synced_at.map(timestamp_to_datetime);
        let remote_id = record
            .remote_id
            .as_ref()
            .map(|value| {
                RemoteEventId::new(value.clone()).map_err(|err| {
                    AppError::ValidationError(format!("Invalid remote event id: {err}"))
                })
            })
            .transpose()?;

        let domain_record = domain_offline::OfflineActionRecord::new(
            Some(record.id),
            action_id,
            public_key,
            action_type,
            target_id,
            payload,
            sync_status,
            created_at,
            synced_at,
            remote_id,
        )
        .with_error_message(record.error_message.clone());

        Ok(domain_record)
    }

    pub(super) fn saved_action(
        saved: &SavedOfflineAction,
    ) -> Result<domain_offline::SavedOfflineAction, AppError> {
        let action = offline_action_record(&saved.action)?;
        let local_id = OfflineActionId::from_str(&saved.local_id)
            .map_err(|err| AppError::ValidationError(format!("Invalid saved action id: {err}")))?;
        Ok(domain_offline::SavedOfflineAction::new(local_id, action))
    }

    pub(super) fn sync_result(result: &SyncResult) -> Result<domain_offline::SyncResult, AppError> {
        Ok(domain_offline::SyncResult::new(
            try_i32_to_u32(result.synced_count, "synced_count")?,
            try_i32_to_u32(result.failed_count, "failed_count")?,
            try_i32_to_u32(result.pending_count, "pending_count")?,
        ))
    }

    pub(super) fn cache_status(
        status: &CacheStatusData,
    ) -> Result<domain_offline::CacheStatusSnapshot, AppError> {
        let cache_types = status
            .cache_types
            .iter()
            .map(cache_type_status)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(domain_offline::CacheStatusSnapshot::new(
            try_i64_to_u64(status.total_items, "total_items")?,
            try_i64_to_u64(status.stale_items, "stale_items")?,
            cache_types,
        ))
    }

    fn cache_type_status(
        status: &CacheTypeStatusData,
    ) -> Result<domain_offline::CacheTypeStatus, AppError> {
        let cache_type = CacheType::new(status.cache_type.clone()).map_err(|err| {
            AppError::ValidationError(format!("Invalid cache type identifier: {err}"))
        })?;
        Ok(domain_offline::CacheTypeStatus::new(
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
        chrono::DateTime::<Utc>::from_timestamp(ts, 0)
            .or_else(|| chrono::DateTime::<Utc>::from_timestamp_millis(ts))
            .unwrap_or_else(Utc::now)
    }
}
