//! #413 ingestion 系 contract test（ADR 0025 §2.2 / §2.5 / §6）。
//!
//! in-memory な `DocsSync` + `MemoryIndexProjection` + mock safety provider で ingest pipeline と
//! relay 起動 gate を駆動し、ADR 0025 の ingestion 系 contract を検証する。DB を要さない範囲を対象に
//! するため、supported set の scope ゲート自体（Postgres 依存）は cn-core 側の DB テストに委ね、ここでは
//! pipeline の不変条件（共有 replica 実在のみ / fail-closed / de-index）と relay gate を固定する。

use std::sync::Arc;

use anyhow::Result;
use kukuri_cn_core::IndexScopeKind;
use kukuri_cn_indexer::config::RelayConfig;
use kukuri_cn_indexer::ingest::IngestPipeline;
use kukuri_cn_indexer::participant::ScopeReplica;
use kukuri_cn_indexer::projection::{IndexProjection, MemoryIndexProjection};
use kukuri_cn_safety::MockSafetyProvider;
use kukuri_cn_safety_runtime::SafetyOrchestrator;
use kukuri_cn_safety_runtime::clock::SystemScanClock;
use kukuri_cn_safety_runtime::id::UuidEventIdGenerator;
use kukuri_core::{KukuriKeys, ReplicaId, TopicId, build_post_envelope};
use kukuri_docs_sync::{DocOp, DocQuery, DocsSync, MemoryDocsSync, stable_key, topic_replica_id};

/// mock provider で allow を返す orchestrator（known CSAM = NoKnownMatch、脅威スコア無し）。
///
/// `public_node_default` policy は known CSAM provider を必須とするため、allow を得るには
/// `KnownCsamHashMatch` provider が `NoKnownMatch` を返す必要がある。
fn allow_orchestrator() -> Arc<SafetyOrchestrator> {
    let provider = Arc::new(MockSafetyProvider::known_csam("mock-known-csam"));
    Arc::new(
        SafetyOrchestrator::builder(
            "node-issuer",
            Arc::new(SystemScanClock),
            Arc::new(UuidEventIdGenerator),
        )
        .provider(provider)
        .build()
        .expect("orchestrator"),
    )
}

/// mock provider が scan 失敗を返す orchestrator（fail-closed のテスト用）。
fn scan_failed_orchestrator() -> Arc<SafetyOrchestrator> {
    let provider = Arc::new(MockSafetyProvider::known_csam("mock-known-csam").default_failed());
    Arc::new(
        SafetyOrchestrator::builder(
            "node-issuer",
            Arc::new(SystemScanClock),
            Arc::new(UuidEventIdGenerator),
        )
        .provider(provider)
        .build()
        .expect("orchestrator"),
    )
}

/// known CSAM hash match を返す orchestrator（exclude のテスト用）。
fn known_csam_orchestrator(post_id: &str) -> Arc<SafetyOrchestrator> {
    let provider =
        Arc::new(MockSafetyProvider::known_csam("mock-known-csam").with_known_hash_match(post_id));
    Arc::new(
        SafetyOrchestrator::builder(
            "node-issuer",
            Arc::new(SystemScanClock),
            Arc::new(UuidEventIdGenerator),
        )
        .provider(provider)
        .build()
        .expect("orchestrator"),
    )
}

/// 本文 text の post envelope を共有 replica に実在させる（app-api の persist と同じ key 形状）。
///
/// 返り値は object_id。
async fn persist_post(
    docs: &MemoryDocsSync,
    replica: &ReplicaId,
    topic: &TopicId,
    body: &str,
) -> String {
    let keys = KukuriKeys::generate();
    let envelope = build_post_envelope(&keys, topic, body, None).expect("envelope");
    let object = envelope
        .to_post_object()
        .expect("post object")
        .expect("post object present");
    let object_id = object.object_id.as_str().to_string();
    docs.open_replica(replica).await.expect("open");
    docs.apply_doc_op(
        replica,
        DocOp::SetJson {
            key: stable_key("objects", &format!("{object_id}/state")),
            value: serde_json::to_value(&object).expect("state json"),
        },
    )
    .await
    .expect("state op");
    docs.apply_doc_op(
        replica,
        DocOp::SetJson {
            key: stable_key("objects", &format!("{object_id}/envelope")),
            value: serde_json::to_value(&envelope).expect("envelope json"),
        },
    )
    .await
    .expect("envelope op");
    object_id
}

#[tokio::test]
async fn index_only_indexes_shared_replica_entries() -> Result<()> {
    // 共有 replica に実在する entry のみ index する（ghost 注入を作らない）。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "hello shared replica").await;

    let pipeline = IngestPipeline::new(docs.clone(), allow_orchestrator(), projection.clone());
    let scope = ScopeReplica::from_scope(IndexScopeKind::PublicTopic, "rust");
    let summary = pipeline
        .ingest_scope(scope.kind, &scope.id, &scope.replica_id)
        .await?;

    assert_eq!(summary.scanned, 1);
    assert_eq!(summary.indexed, 1);
    assert!(
        projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn content_not_in_shared_replica_is_not_indexed() -> Result<()> {
    // CN へ直接渡されただけ（= replica に entry が無い）の content は index されない。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    // replica を open するが post entry は入れない（共有 replica に実在しない）。
    let replica = topic_replica_id("empty");
    docs.open_replica(&replica).await?;

    let pipeline = IngestPipeline::new(docs.clone(), allow_orchestrator(), projection.clone());
    let summary = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "empty", &replica)
        .await?;

    assert_eq!(summary.scanned, 0);
    assert_eq!(summary.indexed, 0);
    assert_eq!(
        projection
            .count_scope(IndexScopeKind::PublicTopic, "empty")
            .await?,
        0
    );
    Ok(())
}

#[tokio::test]
async fn index_excludes_unscanned_and_scan_failed() -> Result<()> {
    // scan 失敗（fail-closed）の content は投影に入らない。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "scan will fail").await;

    let pipeline =
        IngestPipeline::new(docs.clone(), scan_failed_orchestrator(), projection.clone());
    let summary = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;

    assert_eq!(summary.scanned, 1);
    assert_eq!(summary.indexed, 0);
    assert_eq!(summary.skipped_non_allow, 1);
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn index_excludes_non_allow_verdict_content() -> Result<()> {
    // known CSAM hash match（exclude verdict）の content は投影に入らない。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "bad content").await;

    let pipeline = IngestPipeline::new(
        docs.clone(),
        known_csam_orchestrator(&object_id),
        projection.clone(),
    );
    let summary = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;

    assert_eq!(summary.indexed, 0);
    assert_eq!(summary.skipped_non_allow, 1);
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn reingest_deindexes_when_verdict_flips_to_non_allow() -> Result<()> {
    // 初回 allow で投影されたあと、後続 scan で非 allow になった entry は de-index される。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "flips later").await;

    IngestPipeline::new(docs.clone(), allow_orchestrator(), projection.clone())
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert!(
        projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );

    IngestPipeline::new(
        docs.clone(),
        known_csam_orchestrator(&object_id),
        projection.clone(),
    )
    .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
    .await?;
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn indexing_startup_requires_validated_relay() -> Result<()> {
    // 自前 relay も外部 relay も無ければ indexing 起動に失敗する。
    assert!(
        RelayConfig::new(false, vec![])
            .validate_for_startup()
            .is_err()
    );
    // 自前 relay 有り、または外部 relay 有りで起動できる。
    assert!(
        RelayConfig::new(true, vec![])
            .validate_for_startup()
            .is_ok()
    );
    assert!(
        RelayConfig::new(false, vec!["https://relay.example.net".to_string()])
            .validate_for_startup()
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn deleted_objects_are_deindexed() -> Result<()> {
    // replica 上で deleted / tombstoned になった object は de-index する。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "to be deleted").await;

    IngestPipeline::new(docs.clone(), allow_orchestrator(), projection.clone())
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert!(
        projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );

    // object state を deleted に更新する。
    let mut object: serde_json::Value = {
        let records = docs
            .query_replica(
                &replica,
                DocQuery::Exact(stable_key("objects", &format!("{object_id}/state"))),
            )
            .await?;
        serde_json::from_slice(&records[0].value)?
    };
    object["status"] = serde_json::json!("deleted");
    docs.apply_doc_op(
        &replica,
        DocOp::SetJson {
            key: stable_key("objects", &format!("{object_id}/state")),
            value: object,
        },
    )
    .await?;

    let summary = IngestPipeline::new(docs.clone(), allow_orchestrator(), projection.clone())
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(summary.deindexed, 1);
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}
