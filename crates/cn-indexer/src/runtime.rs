//! cn-indexer 常駐 runtime（#413 / T4、safety runtime 結線 #406）。
//!
//! 起動フローの要は **relay validation の起動 gate**（ADR 0025 §6.4）: 自前 relay も外部 relay も
//! 設定されていなければ indexing を起動しない（fail-closed）。gate を通ったら Postgres の scope
//! 管理 state を真実源に docs replica sync participant を立ち上げる。
//!
//! safety scan の構築境界は #406 で結線済み: provider が構成されていれば
//! `SafetyScanService`（scan → risk signal / signed event 永続化）を構築・検証する。
//! provider 未構成なら scan service を構成せず ingest を起動しない（unscanned を index しない
//! fail-closed と整合。`CommunityIndex` capability が `Availability::Planned` である現状と一致する）。
//! relay gate 自体はそれとは独立に起動時へ適用する。

use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;
use tracing_subscriber::EnvFilter;

use kukuri_cn_core::PgSafetyArtifactStore;

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

    // safety scan runtime の構築境界（#406）。provider が構成されていれば service を構築・検証する
    // （構成不正 = 未知 provider 名 / emit 有効なのに署名鍵なし、は起動失敗）。未構成なら scan
    // service を構成せず、ingest は起動されない（fail-closed）。
    let safety = kukuri_cn_core::build_safety_scan_service(
        &config.safety,
        Arc::new(PgSafetyArtifactStore::new(pool.clone())),
    )?;
    match safety.as_ref() {
        Some(service) => info!(
            issuer_node_id = %service.issuer_node_id(),
            "safety scan service constructed; ingest loop remains gated on #391 / #404"
        ),
        None => info!("safety providers not configured; ingest stays disabled (fail-closed)"),
    }

    // NOTE: 構築境界（orchestrator / scan service）は #406 で結線済み。実 ingest loop の起動は
    // 本番 provider（#391 / #411）と fail-closed indexing 本体（#404）が揃い `CommunityIndex` が
    // 昇格した段階で行う。participant / ingest pipeline（`crate::participant` / `crate::ingest`）は
    // 上記 `safety` service を注入して起動する。
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
