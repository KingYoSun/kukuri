use std::{sync::Arc, time::Instant};

use super::performance_common::{
    offline::{OfflineTestContext, TEST_PUBKEY_HEX, seed_offline_actions, setup_offline_service},
    recorder::{PerformanceRecorder, duration_secs},
};
use anyhow::{Result, anyhow};
use kukuri_lib::test_support::application::ports::offline_store::OfflinePersistence;
use kukuri_lib::test_support::application::services::offline_service::OfflineServiceTrait;
use kukuri_lib::test_support::domain::value_objects::event_gateway::PublicKey;
use kukuri_lib::test_support::infrastructure::offline::{
    OfflineReindexJob, SqliteOfflinePersistence,
};
use sqlx::Row;

#[tokio::test]
#[ignore]
async fn offline_reindex_throughput() -> Result<()> {
    const ACTION_COUNT: usize = 120;

    let OfflineTestContext { service, pool } = setup_offline_service().await;
    seed_offline_actions(&service, ACTION_COUNT).await?;

    let persistence: Arc<dyn OfflinePersistence> =
        Arc::new(SqliteOfflinePersistence::new(pool.clone()));
    let job = OfflineReindexJob::with_emitter(None, persistence);

    let started = Instant::now();
    let report = job.reindex_once().await?;
    let elapsed = started.elapsed();
    let secs = duration_secs(elapsed);

    PerformanceRecorder::new("offline_reindex_once")
        .iterations(ACTION_COUNT as u64)
        .metric("duration_ms", secs * 1_000.0)
        .metric("actions_per_sec", report.offline_action_count as f64 / secs)
        .metric("queued_actions", report.queued_action_count as f64)
        .metric("pending_queue", report.pending_queue_count as f64)
        .note(
            "description",
            "OfflineReindexJob::reindex_once after seeding offline actions",
        )
        .note(
            "environment",
            std::env::var("CI").unwrap_or_else(|_| "local".into()),
        )
        .write()?;

    assert_eq!(report.offline_action_count as usize, ACTION_COUNT);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn offline_sync_actions_throughput() -> Result<()> {
    const ACTION_COUNT: usize = 120;

    let OfflineTestContext { service, pool } = setup_offline_service().await;
    seed_offline_actions(&service, ACTION_COUNT).await?;

    let user_pubkey = PublicKey::from_hex_str(TEST_PUBKEY_HEX).map_err(|err| anyhow!(err))?;
    let started = Instant::now();
    let result = service.sync_actions(user_pubkey).await?;
    let elapsed = started.elapsed();
    let secs = duration_secs(elapsed);

    PerformanceRecorder::new("offline_sync_actions")
        .iterations(ACTION_COUNT as u64)
        .metric("duration_ms", secs * 1_000.0)
        .metric("synced_count", result.synced_count as f64)
        .metric("failed_count", result.failed_count as f64)
        .metric("throughput_per_sec", result.synced_count as f64 / secs)
        .note(
            "description",
            "OfflineService::sync_actions with seeded offline queue entries",
        )
        .note(
            "environment",
            std::env::var("CI").unwrap_or_else(|_| "local".into()),
        )
        .write()?;

    assert_eq!(result.synced_count as usize, ACTION_COUNT);
    assert_eq!(result.failed_count, 0);

    let queue_count: i64 = sqlx::query("SELECT COUNT(*) FROM sync_queue")
        .fetch_one(&pool)
        .await?
        .get(0);
    assert_eq!(queue_count as usize, ACTION_COUNT);

    Ok(())
}
