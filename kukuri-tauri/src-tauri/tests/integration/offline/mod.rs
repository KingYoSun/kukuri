#[path = "../../common/performance/offline.rs"]
mod offline_support;

use chrono::{Duration, Utc};
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineActionsQuery, OfflineServiceTrait,
};
use kukuri_lib::test_support::domain::entities::offline::{CacheMetadataUpdate, SyncStatusUpdate};
use kukuri_lib::test_support::domain::value_objects::event_gateway::PublicKey;
use kukuri_lib::test_support::domain::value_objects::offline::{
    CacheKey, CacheType, EntityId, EntityType, OfflinePayload, SyncStatus,
};
use kukuri_lib::test_support::infrastructure::offline::{
    OfflineReindexJob, SqliteOfflinePersistence,
};
use offline_support::{
    OfflineTestContext, TEST_PUBKEY_HEX, sample_save_params, setup_offline_service,
};
use serde_json::Value;
use std::sync::Arc;

#[tokio::test]
async fn save_action_persists_record() {
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    let saved = service
        .save_action(sample_save_params())
        .await
        .expect("save action");

    assert_eq!(saved.action.user_pubkey.as_hex(), TEST_PUBKEY_HEX);
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
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    let mut second_params = sample_save_params();
    second_params.entity_id = EntityId::new("post124".into()).expect("entity id");

    service
        .save_action(sample_save_params())
        .await
        .expect("save first");
    let second = service
        .save_action(second_params)
        .await
        .expect("save second");

    sqlx::query("UPDATE offline_actions SET is_synced = 1 WHERE local_id = ?")
        .bind(second.local_id.as_str())
        .execute(&pool)
        .await
        .expect("mark synced");

    let synced = service
        .list_actions(OfflineActionsQuery {
            user_pubkey: Some(PublicKey::from_hex_str(TEST_PUBKEY_HEX).expect("pubkey")),
            include_synced: Some(true),
            limit: None,
        })
        .await
        .expect("list synced");
    assert_eq!(synced.len(), 1);

    let unsynced = service
        .list_actions(OfflineActionsQuery {
            user_pubkey: Some(PublicKey::from_hex_str(TEST_PUBKEY_HEX).expect("pubkey")),
            include_synced: Some(false),
            limit: None,
        })
        .await
        .expect("list unsynced");
    assert_eq!(unsynced.len(), 1);
}

#[tokio::test]
async fn sync_actions_marks_records_and_enqueues() {
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    service
        .save_action(sample_save_params())
        .await
        .expect("save action");

    let result = service
        .sync_actions(PublicKey::from_hex_str(TEST_PUBKEY_HEX).expect("pubkey"))
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
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    let update = CacheMetadataUpdate {
        cache_key: CacheKey::new("cache:topics".into()).expect("cache key"),
        cache_type: CacheType::new("topics".into()).expect("cache type"),
        metadata: Some(serde_json::json!({"version": 1})),
        expiry: Some(Utc::now() + Duration::seconds(1)),
        is_stale: Some(false),
        doc_version: None,
        blob_hash: None,
        payload_bytes: None,
    };

    service
        .upsert_cache_metadata(update)
        .await
        .expect("upsert cache");

    sqlx::query(
        r#"
        UPDATE cache_metadata
        SET expiry_time = expiry_time - 10
        WHERE cache_key = ?1
        "#,
    )
    .bind("cache:topics")
    .execute(&pool)
    .await
    .expect("force expiry for cleanup test");

    let removed = service.cleanup_expired_cache().await.expect("cleanup");
    assert_eq!(removed, 1);

    let (remaining,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cache_metadata")
        .fetch_one(&pool)
        .await
        .expect("remaining cache rows");
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn cache_status_returns_metadata_summary() {
    let OfflineTestContext { service, .. } = setup_offline_service().await;

    let first_update = CacheMetadataUpdate {
        cache_key: CacheKey::new("sync_queue::offline_actions".into()).expect("cache key"),
        cache_type: CacheType::new("sync_queue".into()).expect("cache type"),
        metadata: Some(serde_json::json!({
            "cacheType": "offline_actions",
            "requestedAt": "2025-11-09T00:00:00Z",
            "requestedBy": "npub1first"
        })),
        expiry: None,
        is_stale: Some(true),
        doc_version: None,
        blob_hash: None,
        payload_bytes: None,
    };
    service
        .upsert_cache_metadata(first_update)
        .await
        .expect("upsert first metadata");

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let second_update = CacheMetadataUpdate {
        cache_key: CacheKey::new("sync_queue::trending".into()).expect("cache key"),
        cache_type: CacheType::new("sync_queue".into()).expect("cache type"),
        metadata: Some(serde_json::json!({
            "cacheType": "trending",
            "requestedAt": "2025-11-09T00:00:01Z",
            "requestedBy": "npub1latest",
            "queueItemId": 42
        })),
        expiry: None,
        is_stale: Some(true),
        doc_version: None,
        blob_hash: None,
        payload_bytes: None,
    };
    service
        .upsert_cache_metadata(second_update)
        .await
        .expect("upsert second metadata");

    let snapshot = service.cache_status().await.expect("cache status");
    let queue_summary = snapshot
        .cache_types
        .into_iter()
        .find(|status| status.cache_type.as_str() == "sync_queue")
        .expect("sync_queue summary");

    assert_eq!(queue_summary.item_count, 2);
    assert!(queue_summary.is_stale);
    let metadata = queue_summary.metadata.expect("metadata present");
    assert_eq!(
        metadata.get("requestedBy").and_then(|value| value.as_str()),
        Some("npub1latest")
    );
    assert_eq!(
        metadata.get("queueItemId").and_then(|value| value.as_i64()),
        Some(42)
    );
}

#[tokio::test]
async fn update_sync_status_performs_upsert() {
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    let pending = SyncStatusUpdate::new(
        EntityType::new("post".into()).expect("entity type"),
        EntityId::new("p1".into()).expect("entity id"),
        SyncStatus::from("pending"),
        Some(OfflinePayload::new(Value::String("conflict".into())).expect("payload")),
        Utc::now(),
    );
    service
        .update_sync_status(pending)
        .await
        .expect("initial update");

    let resolved = SyncStatusUpdate::new(
        EntityType::new("post".into()).expect("entity type"),
        EntityId::new("p1".into()).expect("entity id"),
        SyncStatus::from("resolved"),
        None,
        Utc::now(),
    );
    service
        .update_sync_status(resolved)
        .await
        .expect("second update");

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

#[tokio::test]
async fn cache_status_reports_per_type() {
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    service
        .upsert_cache_metadata(CacheMetadataUpdate {
            cache_key: CacheKey::new("cache:posts:1".into()).expect("cache key"),
            cache_type: CacheType::new("posts".into()).expect("cache type"),
            metadata: Some(serde_json::json!({"version": 1})),
            expiry: None,
            is_stale: Some(false),
            doc_version: None,
            blob_hash: None,
            payload_bytes: None,
        })
        .await
        .expect("upsert posts");

    service
        .upsert_cache_metadata(CacheMetadataUpdate {
            cache_key: CacheKey::new("cache:topics:1".into()).expect("cache key"),
            cache_type: CacheType::new("topics".into()).expect("cache type"),
            metadata: None,
            expiry: Some(Utc::now() + Duration::seconds(60)),
            is_stale: Some(false),
            doc_version: None,
            blob_hash: None,
            payload_bytes: None,
        })
        .await
        .expect("upsert topics");

    sqlx::query("UPDATE cache_metadata SET is_stale = 1 WHERE cache_type = ?")
        .bind("posts")
        .execute(&pool)
        .await
        .expect("mark posts stale");

    let status = service.cache_status().await.expect("cache status");
    assert_eq!(status.total_items, 2);
    assert_eq!(status.stale_items, 1);

    let mut posts_entry = None;
    let mut topics_entry = None;
    for entry in status.cache_types {
        match entry.cache_type.as_str() {
            "posts" => posts_entry = Some(entry),
            "topics" => topics_entry = Some(entry),
            _ => {}
        }
    }

    let posts = posts_entry.expect("posts entry");
    assert!(posts.is_stale);
    let topics = topics_entry.expect("topics entry");
    assert!(!topics.is_stale);
}

#[tokio::test]
async fn sync_actions_after_reindex_clears_pending() {
    let OfflineTestContext { service, pool } = setup_offline_service().await;

    service
        .save_action(sample_save_params())
        .await
        .expect("save first");

    let mut second = sample_save_params();
    second.entity_id = EntityId::new("post124".into()).expect("entity id");
    service.save_action(second).await.expect("save second");

    let persistence = Arc::new(SqliteOfflinePersistence::new(pool.clone()));
    let job = OfflineReindexJob::with_emitter(None, persistence.clone());
    let report = job.reindex_once().await.expect("reindex report");
    assert_eq!(report.queued_action_count, 2);

    let result = service
        .sync_actions(PublicKey::from_hex_str(TEST_PUBKEY_HEX).expect("pubkey"))
        .await
        .expect("sync actions");

    assert_eq!(result.synced_count, 2);
    assert_eq!(result.pending_count, 0);

    let (unsynced,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM offline_actions WHERE is_synced = 0")
            .fetch_one(&pool)
            .await
            .expect("unsynced count");
    assert_eq!(unsynced, 0);
}

mod recovery;
