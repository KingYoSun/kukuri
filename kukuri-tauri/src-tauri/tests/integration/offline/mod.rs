use std::sync::Arc;

use chrono::{Duration, Utc};
use kukuri_lib::test_support::application::ports::offline_store::OfflinePersistence;
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineActionsQuery,
    OfflineService,
    OfflineServiceTrait,
    SaveOfflineActionParams,
};
use kukuri_lib::test_support::domain::entities::offline::{CacheMetadataUpdate, SyncStatusUpdate};
use kukuri_lib::test_support::domain::value_objects::event_gateway::PublicKey;
use kukuri_lib::test_support::domain::value_objects::offline::{
    CacheKey,
    CacheType,
    EntityId,
    EntityType,
    OfflineActionType,
    OfflinePayload,
    SyncStatus,
};
use kukuri_lib::test_support::infrastructure::offline::SqliteOfflinePersistence;
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Executor, Pool, Sqlite};

const PUBKEY_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

async fn setup_service() -> (OfflineService, Pool<Sqlite>) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:?cache=shared")
        .await
        .expect("in-memory sqlite");

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

fn sample_save_params() -> SaveOfflineActionParams {
    SaveOfflineActionParams {
        user_pubkey: PublicKey::from_hex_str(PUBKEY_HEX).expect("pubkey"),
        action_type: OfflineActionType::new("create_post".into()).expect("action type"),
        entity_type: EntityType::new("post".into()).expect("entity type"),
        entity_id: EntityId::new("post123".into()).expect("entity id"),
        payload: OfflinePayload::from_json_str(r#"{"content":"Hello"}"#).expect("payload"),
    }
}

#[tokio::test]
async fn save_action_persists_record() {
    let (service, pool) = setup_service().await;

    let saved = service.save_action(sample_save_params()).await.expect("save");

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
        .expect("offline_actions count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn list_actions_applies_sync_filter() {
    let (service, pool) = setup_service().await;

    let first = service.save_action(sample_save_params()).await.expect("save first");
    let mut second_params = sample_save_params();
    second_params.entity_id = EntityId::new("post124".into()).expect("entity id");
    service.save_action(second_params).await.expect("save second");

    sqlx::query("UPDATE offline_actions SET is_synced = 1 WHERE id = ?1")
        .bind(first.action.record_id.expect("record id"))
        .execute(&pool)
        .await
        .expect("mark synced");

    let synced = service
        .list_actions(OfflineActionsQuery {
            user_pubkey: Some(PublicKey::from_hex_str(PUBKEY_HEX).expect("pubkey")),
            include_synced: Some(true),
            limit: None,
        })
        .await
        .expect("list synced");
    assert_eq!(synced.len(), 1);

    let unsynced = service
        .list_actions(OfflineActionsQuery {
            user_pubkey: Some(PublicKey::from_hex_str(PUBKEY_HEX).expect("pubkey")),
            include_synced: Some(false),
            limit: None,
        })
        .await
        .expect("list unsynced");
    assert_eq!(unsynced.len(), 1);
}

#[tokio::test]
async fn sync_actions_marks_records_and_enqueues() {
    let (service, pool) = setup_service().await;

    service.save_action(sample_save_params()).await.expect("save action");

    let result = service
        .sync_actions(PublicKey::from_hex_str(PUBKEY_HEX).expect("pubkey"))
        .await
        .expect("sync actions");
    assert_eq!(result.synced_count, 1);
    assert_eq!(result.failed_count, 0);

    let (is_synced,): (i64,) = sqlx::query_as("SELECT is_synced FROM offline_actions LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("synced flag");
    assert_eq!(is_synced, 1);

    let (queue_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sync_queue")
        .fetch_one(&pool)
        .await
        .expect("queue count");
    assert_eq!(queue_count, 1);
}

#[tokio::test]
async fn cache_metadata_upsert_and_cleanup() {
    let (service, pool) = setup_service().await;

    let update = CacheMetadataUpdate {
        cache_key: CacheKey::new("cache:topics".into()).expect("cache key"),
        cache_type: CacheType::new("topics".into()).expect("cache type"),
        metadata: Some(serde_json::json!({"version": 1})),
        expiry: Some(Utc::now() + Duration::seconds(1)),
    };

    service.upsert_cache_metadata(update).await.expect("upsert cache");

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let removed = service.cleanup_expired_cache().await.expect("cleanup");
    assert_eq!(removed, 1);

    let (remaining,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cache_metadata")
        .fetch_one(&pool)
        .await
        .expect("remaining cache rows");
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn update_sync_status_performs_upsert() {
    let (service, pool) = setup_service().await;

    let pending = SyncStatusUpdate::new(
        EntityType::new("post".into()).expect("entity type"),
        EntityId::new("p1".into()).expect("entity id"),
        SyncStatus::from("pending"),
        Some(OfflinePayload::new(Value::String("conflict".into())).expect("payload")),
        Utc::now(),
    );
    service.update_sync_status(pending).await.expect("initial update");

    let resolved = SyncStatusUpdate::new(
        EntityType::new("post".into()).expect("entity type"),
        EntityId::new("p1".into()).expect("entity id"),
        SyncStatus::from("resolved"),
        None,
        Utc::now(),
    );
    service.update_sync_status(resolved).await.expect("second update");

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
        .expect("sync_status row");

    assert_eq!(local_version, 2);
    assert_eq!(sync_status, "resolved");
    assert!(conflict_data.is_none());
}
