//! cn-indexer 常駐 runtime（#413 / T4）。
//!
//! 起動フローの要は **relay validation の起動 gate**（ADR 0025 §6.4）: 自前 relay も外部 relay も
//! 設定されていなければ indexing を起動しない（fail-closed）。gate を通ったら Postgres の scope
//! 管理 state を真実源に docs replica sync participant を立ち上げる。
//!
//! safety provider（#391 / #411）はまだ実装が無いため、scan orchestrator を構成できない構成では
//! ingest を起動しない（unscanned を index しない fail-closed と整合。`CommunityIndex` capability が
//! `Availability::Planned` である現状と一致する）。relay gate 自体はそれとは独立に起動時へ適用する。

use anyhow::{Context, Result};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::IndexerConfig;

/// 環境変数から設定を読み、relay validation 起動 gate を適用して cn-indexer を起動する。
///
/// 現段階では relay gate の適用と scope state の準備確認までを行う。safety provider が実装されて
/// ingest が実運用可能になった段階（#391 / #411, `CommunityIndex` 昇格）で ingest loop を有効化する。
pub async fn run_from_env() -> Result<()> {
    init_tracing();
    let config = IndexerConfig::from_env()?;
    run(config).await
}

async fn run(config: IndexerConfig) -> Result<()> {
    // fail-closed の起動 gate: 自前 relay も外部 relay も無ければ indexing を起動しない。
    let validation = config.relay.validate_for_startup().context(
        "cn-indexer startup blocked: no validated relay (ADR 0025 §6.4 fail-closed startup gate)",
    )?;
    info!(?validation, "relay validation passed; starting cn-indexer");

    // scope 管理 state（supported set / request / channel secret）を持つ DB が ready であること。
    let pool = kukuri_cn_core::connect_postgres(config.database_url.as_str()).await?;
    kukuri_cn_core::ensure_database_ready(&pool)
        .await
        .context("community-node database is not ready for cn-indexer")?;

    // channel secret 復号鍵を検証する（不正なら早期に失敗させる）。
    let _cipher =
        kukuri_cn_core::ChannelSecretCipher::from_key_material(config.channel_secret_key.as_str())
            .context("invalid COMMUNITY_NODE_CHANNEL_SECRET_KEY")?;

    info!(
        data_dir = %config.data_dir.display(),
        arcadedb_url = %config.arcadedb.base_url,
        arcadedb_database = %config.arcadedb.database,
        "cn-indexer configuration validated"
    );

    // NOTE: safety provider（#391 / #411）が実装され `CommunityIndex` が昇格するまで、実 ingest loop は
    // 起動しない。ここまでで relay 起動 gate と scope state の準備確認は完了しており、participant /
    // ingest pipeline（`crate::participant` / `crate::ingest`）は provider 実装後に本 runtime へ結線する。
    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kukuri_cn_indexer=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}
