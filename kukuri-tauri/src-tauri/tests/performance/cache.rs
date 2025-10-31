use std::time::Instant;

use super::performance_common::{
    offline::{
        OfflineTestContext, TEST_PUBKEY_HEX, build_params_for_index, seed_cache_metadata,
        setup_offline_service,
    },
    recorder::{PerformanceRecorder, duration_secs},
};
use anyhow::{Result, anyhow};
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineActionsQuery, OfflineServiceTrait,
};
use kukuri_lib::test_support::domain::value_objects::event_gateway::PublicKey;
use tokio::time::{Duration, sleep};

#[tokio::test]
#[ignore]
async fn offline_action_save_throughput() -> Result<()> {
    const ACTION_COUNT: usize = 200;
    let OfflineTestContext { service, pool: _ } = setup_offline_service().await;

    let save_started = Instant::now();
    for index in 0..ACTION_COUNT {
        service.save_action(build_params_for_index(index)).await?;
    }
    let save_elapsed = save_started.elapsed();
    let user_pubkey = PublicKey::from_hex_str(TEST_PUBKEY_HEX).map_err(|err| anyhow!(err))?;
    let list_started = Instant::now();
    let records = service
        .list_actions(OfflineActionsQuery {
            user_pubkey: Some(user_pubkey),
            include_synced: Some(false),
            limit: None,
        })
        .await?;
    let list_elapsed = list_started.elapsed();

    let save_secs = duration_secs(save_elapsed);
    let list_secs = duration_secs(list_elapsed);

    PerformanceRecorder::new("offline_action_save_throughput")
        .iterations(ACTION_COUNT as u64)
        .metric("save_total_ms", save_secs * 1_000.0)
        .metric("save_throughput_per_sec", ACTION_COUNT as f64 / save_secs)
        .metric("list_total_ms", list_secs * 1_000.0)
        .metric("list_throughput_per_sec", records.len() as f64 / list_secs)
        .note(
            "description",
            "OfflineService::save_action and list_actions against in-memory SQLite",
        )
        .note(
            "environment",
            std::env::var("CI").unwrap_or_else(|_| "local".into()),
        )
        .write()?;

    assert_eq!(records.len(), ACTION_COUNT);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn cache_cleanup_latency() -> Result<()> {
    const CACHE_ENTRIES: usize = 50;
    let OfflineTestContext { service, .. } = setup_offline_service().await;
    seed_cache_metadata(&service, CACHE_ENTRIES).await?;

    // expiry values are 1-3 seconds; wait long enough for at least two buckets.
    sleep(Duration::from_secs(4)).await;

    let cleanup_started = Instant::now();
    let removed = service.cleanup_expired_cache().await?;
    let cleanup_elapsed = cleanup_started.elapsed();
    let cleanup_secs = duration_secs(cleanup_elapsed);

    PerformanceRecorder::new("offline_cache_cleanup_latency")
        .iterations(CACHE_ENTRIES as u64)
        .metric("removed_entries", removed as f64)
        .metric("cleanup_total_ms", cleanup_secs * 1_000.0)
        .metric("cleanup_throughput_per_sec", removed as f64 / cleanup_secs)
        .note(
            "description",
            "OfflineService::cleanup_expired_cache after staged expiry seeding",
        )
        .note(
            "environment",
            std::env::var("CI").unwrap_or_else(|_| "local".into()),
        )
        .write()?;

    Ok(())
}
