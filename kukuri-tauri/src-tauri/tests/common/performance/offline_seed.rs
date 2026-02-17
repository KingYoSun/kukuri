use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use kukuri_lib::test_support::application::services::offline_service::{
    OfflineService, OfflineServiceTrait,
};
use kukuri_lib::test_support::domain::entities::offline::CacheMetadataUpdate;
use kukuri_lib::test_support::domain::value_objects::offline::{CacheKey, CacheType};
use serde_json::json;

use super::offline_support::build_params_for_index;

pub async fn seed_offline_actions(service: &OfflineService, count: usize) -> Result<()> {
    for i in 0..count {
        service
            .save_action(build_params_for_index(i))
            .await
            .map_err(|err| anyhow!("{err}"))?;
    }
    Ok(())
}

pub async fn seed_cache_metadata(service: &OfflineService, count: usize) -> Result<()> {
    for i in 0..count {
        let update = CacheMetadataUpdate {
            cache_key: CacheKey::new(format!("cache:test:{i}")).expect("cache key"),
            cache_type: CacheType::new("posts".into()).expect("cache type"),
            metadata: Some(json!({ "version": i })),
            expiry: Some(Utc::now() + Duration::seconds((i as i64 % 3) + 1)),
            is_stale: Some(false),
            doc_version: None,
            blob_hash: None,
            payload_bytes: None,
        };
        service
            .upsert_cache_metadata(update)
            .await
            .map_err(|err| anyhow!("{err}"))?;
    }
    Ok(())
}
