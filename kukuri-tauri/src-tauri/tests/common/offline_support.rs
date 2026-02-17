use std::sync::Arc;

use chrono::Utc;
use kukuri_lib::test_support::application::ports::offline_store::OfflinePersistence;
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineService, SaveOfflineActionParams,
};
use kukuri_lib::test_support::domain::value_objects::event_gateway::PublicKey;
use kukuri_lib::test_support::domain::value_objects::offline::{
    EntityId, EntityType, OfflineActionType, OfflinePayload,
};
use kukuri_lib::test_support::infrastructure::offline::SqliteOfflinePersistence;
use serde_json::json;
use sqlx::{Executor, Pool, Sqlite, sqlite::SqlitePoolOptions};

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
        entity_id: EntityId::new(format!("post_{index:04}")).expect("entity id"),
        payload: OfflinePayload::from_json_str(&payload.to_string()).expect("payload"),
    }
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
            metadata TEXT,
            doc_version INTEGER,
            blob_hash TEXT,
            payload_bytes INTEGER
        )
        "#,
    )
    .await
    .expect("cache_metadata table");

    pool.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_cache_metadata_doc_version ON cache_metadata(doc_version);
        "#,
    )
    .await
    .expect("cache_metadata doc_version index");

    pool.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_cache_metadata_blob_hash ON cache_metadata(blob_hash);
        "#,
    )
    .await
    .expect("cache_metadata blob_hash index");

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
