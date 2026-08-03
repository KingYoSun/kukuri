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
//!
//! media scan 用の一時 fetch（#609）と docs replica sync（#613 T1）は、同じ persistent iroh node
//! （`data_dir` 配下）を共有する。provider が構成されている場合のみ node を立ち上げ、
//! `IrohBlobService` + `BlobMediaFetcher`（media fetch）と `IrohDocsSync`（docs participant）の
//! 双方へ渡す。シードピア（`COMMUNITY_NODE_INDEXER_SEED_PEERS`）は docs 同期へ適用する。

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::postgres::PgPool;
use tracing::info;

use kukuri_blob_service::IrohBlobService;
use kukuri_cn_core::{ChannelSecretCipher, PgIndexEntryStore, PgSafetyArtifactStore};
use kukuri_cn_safety::provider::MediaFetcher;
use kukuri_cn_safety_runtime::SafetyScanService;
use kukuri_docs_sync::{DocsSync, IrohDocsSync};
use kukuri_iroh_node::IrohDocsNode;
use kukuri_transport::{DhtDiscoveryOptions, TransportNetworkConfig, TransportRelayConfig};

use crate::arcadedb::ArcadeDbProjection;
use crate::config::IndexerConfig;
use crate::ingest::IngestPipeline;
use crate::media_fetcher::BlobMediaFetcher;
use crate::participant::IndexerParticipant;

/// 環境変数から設定を読み、relay validation 起動 gate を適用して cn-indexer を起動する。
///
/// 現段階では relay gate の適用、scope state の準備確認、ingest 経路の本番依存の組み立て
/// （共有 iroh node / docs 同期 / ArcadeDB 投影 / 真実源 / 取り込みパイプライン）までを行う。
/// 常駐 ingest loop の起動は #613 T2 で行う。
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
    let cipher =
        kukuri_cn_core::ChannelSecretCipher::from_key_material(config.channel_secret_key.as_str())
            .context("invalid COMMUNITY_NODE_CHANNEL_SECRET_KEY")?;

    info!(
        data_dir = %config.data_dir.display(),
        arcadedb_url = %config.arcadedb.base_url,
        arcadedb_database = %config.arcadedb.database,
        "cn-indexer configuration validated"
    );

    // 共有 iroh node（#613 T1）。provider が構成されている場合のみ persistent node を 1 つ立ち上げ、
    // media scan の一時 fetch（#609）と docs replica sync の双方で共有する。provider 未構成なら
    // scan service 自体を構成しない（fail-closed）ため node も立てない。
    let node: Option<Arc<IrohDocsNode>> = if config.safety.providers.is_empty() {
        None
    } else {
        Some(
            IrohDocsNode::persistent_with_discovery_config(
                &config.data_dir,
                TransportNetworkConfig::from_env()?,
                DhtDiscoveryOptions::disabled(),
                TransportRelayConfig {
                    iroh_relay_urls: config.relay.external_relay_urls.clone(),
                }
                .normalized(),
            )
            .await
            .context("failed to start the cn-indexer iroh node")?,
        )
    };

    // media scan 用の一時 fetch（#609）。共有 node の blob 経路から組み、provider 解決へ注入する。
    let media_fetcher: Option<Arc<dyn MediaFetcher>> = node.as_ref().map(|node| {
        let blob_service = Arc::new(IrohBlobService::new(Arc::clone(node)));
        info!(
            max_bytes = config.media_fetch.max_bytes,
            timeout_secs = config.media_fetch.timeout.as_secs(),
            "media fetcher constructed (ephemeral blob fetch; no permanent blob storage)"
        );
        Arc::new(BlobMediaFetcher::new(
            blob_service,
            config.media_fetch.clone(),
        )) as Arc<dyn MediaFetcher>
    });

    // safety scan runtime の構築境界（#406）。provider が構成されていれば service を構築・検証する
    // （構成不正 = 未知 provider 名 / emit 有効なのに署名鍵なし、は起動失敗）。未構成なら scan
    // service を構成せず、ingest は起動されない（fail-closed）。
    let safety_providers =
        kukuri_cn_core::resolve_safety_providers(&config.safety.providers, media_fetcher)?;
    let safety = kukuri_cn_safety_runtime::build_safety_scan_service(
        &config.safety,
        safety_providers,
        Arc::new(PgSafetyArtifactStore::new(pool.clone())),
    )?;
    match (safety, node) {
        (Some(service), Some(node)) => {
            info!(
                issuer_node_id = %service.issuer_node_id(),
                "safety scan service constructed"
            );
            let _participant = compose_ingest_stack(&config, pool, cipher, service, node).await?;
            // NOTE: 常駐 ingest loop の起動は #613 T2 で行う。ここでは本番依存の組み立てと
            // 起動時検証（ArcadeDB 接続 / seed 適用）までを行い、終了する。
            info!("ingest stack composed; resident ingest loop lands with #613 T2");
        }
        _ => info!("safety providers not configured; ingest stays disabled (fail-closed)"),
    }
    Ok(())
}

/// ingest 経路の本番依存を組み立てる（#613 T1）。
///
/// - 共有 iroh node から `IrohDocsSync` を作り、シードピアを適用する。
/// - ArcadeDB 投影は起動時に schema 準備（`ensure_schema`）を行い、接続できなければ起動失敗に
///   する（fail-closed。投影へ書けない構成で取り込みを始めない）。
/// - 索引の真実源は Postgres（`PgIndexEntryStore`）。
async fn compose_ingest_stack(
    config: &IndexerConfig,
    pool: PgPool,
    cipher: ChannelSecretCipher,
    safety: SafetyScanService,
    node: Arc<IrohDocsNode>,
) -> Result<IndexerParticipant> {
    let docs_sync = Arc::new(IrohDocsSync::new(node));
    if !config.seed_peers.is_empty() {
        docs_sync
            .set_seed_peers(config.seed_peers.clone())
            .await
            .context("failed to apply COMMUNITY_NODE_INDEXER_SEED_PEERS to docs sync")?;
        info!(
            seed_peers = config.seed_peers.len(),
            "applied seed peers to docs sync"
        );
    }

    let entries = Arc::new(PgIndexEntryStore::new(pool.clone()));
    let projection = ArcadeDbProjection::new(config.arcadedb.clone())
        .context("failed to build the ArcadeDB projection client")?;
    projection.ensure_schema().await.context(
        "cn-indexer cannot reach ArcadeDB for the index projection \
         (fail-closed startup gate)",
    )?;
    let projection = Arc::new(projection);

    let pipeline = IngestPipeline::new(
        docs_sync.clone(),
        Arc::new(safety),
        entries.clone(),
        projection.clone(),
    );
    Ok(IndexerParticipant::new(
        pool, docs_sync, entries, projection, pipeline, cipher,
    ))
}

fn init_tracing() {
    kukuri_cn_runtime_support::init_tracing("info,kukuri_cn_indexer=debug");
}
