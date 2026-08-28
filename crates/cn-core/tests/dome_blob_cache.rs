//! Issue #793 Dome manifest/asset cache の Postgres pin/GC integration tests.

use anyhow::Result;
use kukuri_cn_core::{
    DOME_BLOB_CACHE_GC_GRACE_MILLIS, StagedDomeBlob, TestDatabase, activate_dome_hosting_blob_pins,
    collect_dome_blob_cache_garbage, connect_postgres, initialize_database,
    release_dome_hosting_blob_pins, stage_dome_hosting_blobs,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

fn blob(data: &[u8]) -> StagedDomeBlob {
    StagedDomeBlob {
        blob_hash: blake3::hash(data).to_hex().to_string(),
        data: data.to_vec(),
    }
}

#[tokio::test]
async fn dome_blob_cache_activation_is_idempotent_and_keeps_shared_rollback_assets() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping Dome blob cache test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_dome_blob_cache").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    let shared_asset = blob(b"shared-asset");
    let manifest_v1 = blob(b"manifest-v1");
    stage_dome_hosting_blobs(
        &pool,
        "dome-1:1",
        &[manifest_v1.clone(), shared_asset.clone()],
        1_000,
    )
    .await?;
    activate_dome_hosting_blob_pins(&pool, "dome-1", "dome-1:1", 1_100).await?;
    activate_dome_hosting_blob_pins(&pool, "dome-1", "dome-1:1", 1_200).await?;

    let manifest_v2 = blob(b"manifest-v2");
    stage_dome_hosting_blobs(
        &pool,
        "dome-1:2",
        &[manifest_v2.clone(), shared_asset.clone()],
        2_000,
    )
    .await?;
    activate_dome_hosting_blob_pins(&pool, "dome-1", "dome-1:2", 2_100).await?;

    let cache_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cn_metaverse.dome_blob_cache")
        .fetch_one(&pool)
        .await?;
    assert_eq!(cache_rows, 3, "shared content hash must be stored once");

    let old_reasons: Vec<String> = sqlx::query_scalar(
        "SELECT reason FROM cn_metaverse.dome_blob_pins
         WHERE blob_hash = $1 AND reference_id = 'dome-1:1' ORDER BY reason",
    )
    .bind(&manifest_v1.blob_hash)
    .fetch_all(&pool)
    .await?;
    assert_eq!(old_reasons, vec!["rollback"]);

    let new_reasons: Vec<String> = sqlx::query_scalar(
        "SELECT reason FROM cn_metaverse.dome_blob_pins
         WHERE blob_hash = $1 AND reference_id = 'dome-1:2' ORDER BY reason",
    )
    .bind(&manifest_v2.blob_hash)
    .fetch_all(&pool)
    .await?;
    assert_eq!(new_reasons, vec!["active_lease", "current"]);

    release_dome_hosting_blob_pins(&pool, "dome-1:2", 3_000).await?;
    assert_eq!(
        collect_dome_blob_cache_garbage(&pool, 3_000 + DOME_BLOB_CACHE_GC_GRACE_MILLIS).await?,
        0,
        "current and rollback pins must prevent collection"
    );
    Ok(())
}
