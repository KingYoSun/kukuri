//! #616 プロバイダ疎通確認の期限付き保存の Postgres integration テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - slot 単位の upsert（同一 slot は上書き）。
//! - 保存されるのは判定と要約のみ（読み戻しで往復が安定する）。

use anyhow::Result;
use chrono::{TimeZone, Utc};
use kukuri_cn_core::{
    ReadinessProbeRecord, TestDatabase, connect_postgres, initialize_database,
    list_readiness_probes, upsert_readiness_probe,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

#[tokio::test]
async fn probe_cache_upserts_by_slot_and_round_trips() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping readiness probe test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_readiness_probe").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;

    let first = ReadinessProbeRecord {
        provider_slot: "known_csam".to_string(),
        provider: "project-arachnid-shield".to_string(),
        pass: false,
        detail: "認証拒否 (HTTP 401)".to_string(),
        checked_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
    };
    upsert_readiness_probe(&pool, &first).await?;

    // 同一 slot は上書きされ、行が増えない。
    let second = ReadinessProbeRecord {
        pass: true,
        detail: "認証と応答受信に成功".to_string(),
        checked_at: Utc.timestamp_opt(1_700_000_600, 0).unwrap(),
        ..first.clone()
    };
    upsert_readiness_probe(&pool, &second).await?;

    let third = ReadinessProbeRecord {
        provider_slot: "general".to_string(),
        provider: "openai-compatible-vlm".to_string(),
        pass: true,
        detail: "接続と応答形式の解析に成功".to_string(),
        checked_at: Utc.timestamp_opt(1_700_000_700, 0).unwrap(),
    };
    upsert_readiness_probe(&pool, &third).await?;

    let records = list_readiness_probes(&pool).await?;
    assert_eq!(records, vec![third, second]);
    Ok(())
}
