use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use kukuri_lib::test_support::application::ports::offline_store::OfflinePersistence;
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineService, OfflineServiceTrait, SaveOfflineActionParams,
};
use kukuri_lib::test_support::domain::entities::offline::{
    CacheMetadataUpdate, SyncStatusUpdate,
};
use kukuri_lib::test_support::domain::value_objects::event_gateway::PublicKey;
use kukuri_lib::test_support::domain::value_objects::offline::{
    CacheKey, CacheType, EntityId, EntityType, OfflineActionType, OfflinePayload, SyncStatus,
};
use kukuri_lib::test_support::infrastructure::offline::SqliteOfflinePersistence;
use serde_json::{json, Value};
use sqlx::{sqlite::SqlitePoolOptions, Executor, Pool, Sqlite};

pub const TEST_PUBKEY_HEX: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub struct OfflineTestContext {
    pub service: OfflineService,
    pub pool: Pool<Sqlite>,
}

pub async fn setup_offline_service() -> OfflineTestContext {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:?cache=shared")
        .await
        .expect("in-memory sqlite");

    initialize_schema(&pool).await;

    let persistence: Arc<dyn OfflinePersistence> =
        Arc::new(SqliteOfflinePersistence::new(pool.clone()));

    OfflineTestContext {
        service: OfflineService::new(persistence),
        pool,
    }
}

#[allow(dead_code)]
pub fn sample_save_params() -> SaveOfflineActionParams {
    SaveOfflineActionParams {
        user_pubkey: PublicKey::from_hex_str(TEST_PUBKEY_HEX).expect("pubkey"),
        action_type: OfflineActionType::new("create_post".into()).expect("action type"),
        entity_type: EntityType::new("post".into()).expect("entity type"),
        entity_id: EntityId::new("post123".into()).expect("entity id"),
        payload: OfflinePayload::from_json_str(r#"{"content":"Hello"}"#).expect("payload"),
    }
}

#[allow(dead_code)]
pub fn build_params_for_index(index: usize) -> SaveOfflineActionParams {
    let payload = json!({
        "content": format!("Post {index}"),
        "topicId": format!("topic-{}", index % 8),
        "created_at": Utc::now().timestamp()
    });

    SaveOfflineActionParams {
        user_pubkey: PublicKey::from_hex_str(TEST_PUBKEY_HEX).expect("pubkey"),
        action_type: OfflineActionType::new("create_post".into()).expect("action type"),
        entity_type: EntityType::new("post".into()).expect("entity type"),
        entity_id: EntityId::new(format!("post_{index:04}").into()).expect("entity id"),
        payload: OfflinePayload::from_json_str(&payload.to_string()).expect("payload"),
    }
}

#[allow(dead_code)]
pub async fn seed_offline_actions(service: &OfflineService, count: usize) -> Result<()> {
    for i in 0..count {
        service
            .save_action(build_params_for_index(i))
            .await
            .map_err(|err| anyhow!(err))?;
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn seed_cache_metadata(service: &OfflineService, count: usize) -> Result<()> {
    for i in 0..count {
        let update = CacheMetadataUpdate {
            cache_key: CacheKey::new(format!("cache:test:{i}").into()).expect("cache key"),
            cache_type: CacheType::new("posts".into()).expect("cache type"),
            metadata: Some(json!({ "version": i })),
            expiry: Some(Utc::now() + Duration::seconds((i as i64 % 3) + 1)),
        };
        service
            .upsert_cache_metadata(update)
            .await
            .map_err(|err| anyhow!(err))?;
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn mark_actions_synced(pool: &Pool<Sqlite>, ids: &[i64]) {
    for id in ids {
        sqlx::query("UPDATE offline_actions SET is_synced = 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .expect("mark offline action synced");
    }
}

#[allow(dead_code)]
pub async fn insert_sync_status(
    service: &OfflineService,
    entity: (&str, &str),
    status: SyncStatus,
    conflict: Option<Value>,
) -> Result<()> {
    let payload = conflict
        .map(|value| {
            OfflinePayload::from_json_str(&value.to_string()).map_err(|err| anyhow!(err))
        })
        .transpose()?;

    let update = SyncStatusUpdate::new(
        EntityType::new(entity.0.into()).map_err(|err| anyhow!(err))?,
        EntityId::new(entity.1.into()).map_err(|err| anyhow!(err))?,
        status,
        payload,
        Utc::now(),
    );
    service
        .update_sync_status(update)
        .await
        .map_err(|err| anyhow!(err))?;
    Ok(())
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
    .expect("offline_actions table");

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
    .expect("sync_queue table");

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
    .expect("cache_metadata table");

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
    .expect("optimistic_updates table");

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
    .expect("sync_status table");
}
