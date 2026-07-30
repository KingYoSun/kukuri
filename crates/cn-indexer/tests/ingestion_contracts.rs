//! #413 ingestion 系 contract test（ADR 0025 §2.2 / §2.5 / §6）。
//!
//! in-memory な `DocsSync` + `MemoryIndexProjection` + mock safety provider で ingest pipeline と
//! relay 起動 gate を駆動し、ADR 0025 の ingestion 系 contract を検証する。DB を要さない範囲を対象に
//! するため、supported set の scope ゲート自体（Postgres 依存）は cn-core 側の DB テストに委ね、ここでは
//! pipeline の不変条件（共有 replica 実在のみ / fail-closed / de-index）と relay gate を固定する。

use std::sync::Arc;

use anyhow::Result;
use kukuri_cn_core::{IndexScopeKind, MemoryIndexEntryStore};
use kukuri_cn_indexer::config::RelayConfig;
use kukuri_cn_indexer::ingest::IngestPipeline;
use kukuri_cn_indexer::participant::ScopeReplica;
use kukuri_cn_indexer::projection::{IndexProjection, MemoryIndexProjection};
use kukuri_cn_safety::provider::{ScanError, SubjectKind};
use kukuri_cn_safety::{
    MockSafetyProvider, ModerationEventSigner, RiskSignalTarget, SafetyCategory,
};
use kukuri_cn_safety_runtime::clock::SystemScanClock;
use kukuri_cn_safety_runtime::id::UuidEventIdGenerator;
use kukuri_cn_safety_runtime::{MemorySafetyArtifactStore, SafetyScanService};
use kukuri_cn_safety_runtime::{
    SafetyOrchestrator, Secp256k1ModerationEventSigner, verify_signed_event,
};
use kukuri_core::{
    KukuriKeys, ObjectVisibility, PayloadRef, ReplicaId, TopicId, build_post_envelope,
    build_post_envelope_with_payload,
};
use kukuri_docs_sync::{DocOp, DocQuery, DocsSync, MemoryDocsSync, stable_key, topic_replica_id};

const TEST_SECRET: &str = "0000000000000000000000000000000000000000000000000000000000000001";

/// mock provider から scan service（#406）を組む。本番構成と同型（実鍵 signer +
/// SystemScanClock + UuidEventIdGenerator）で、store のみ in-memory。
///
/// 返り値の store から、pipeline 経由で永続化された moderation artifact を検証できる。
fn service_with(
    provider: MockSafetyProvider,
) -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).expect("signer");
    let issuer = signer.issuer_node_id().to_string();
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let orchestrator = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    )
    .provider(Arc::new(provider))
    .build()
    .expect("orchestrator");
    let service = SafetyScanService::builder(Arc::new(orchestrator), store.clone())
        .signer(Arc::new(signer))
        .build()
        .expect("service");
    (Arc::new(service), store)
}

/// mock provider で allow を返す service（known CSAM = NoKnownMatch、脅威スコア無し）。
///
/// `public_node_default` policy は known CSAM provider を必須とするため、allow を得るには
/// `KnownCsamHashMatch` provider が `NoKnownMatch` を返す必要がある。
fn allow_service() -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    service_with(MockSafetyProvider::known_csam("mock-known-csam"))
}

/// mock provider が scan 失敗を返す service（fail-closed のテスト用）。
fn scan_failed_service() -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    service_with(MockSafetyProvider::known_csam("mock-known-csam").default_failed())
}

/// mock provider が unavailable を返す service（provider unavailable の fail-closed テスト用）。
fn provider_unavailable_service() -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    service_with(
        MockSafetyProvider::known_csam("mock-known-csam")
            .default_error(ScanError::Unavailable("mock provider down".to_string())),
    )
}

/// known CSAM hash match を返す service（exclude のテスト用）。
fn known_csam_service(post_id: &str) -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    service_with(MockSafetyProvider::known_csam("mock-known-csam").with_known_hash_match(post_id))
}

/// service + 真実源（`MemoryIndexEntryStore`）+ 投影から二段書き込みの pipeline を組む（#404）。
///
/// 真実源は artifact store の verdict record を参照するため、service と同じ store を渡す。
/// 返り値の store から moderation artifact を、entries から真実源の entry を検証できる。
fn pipeline_with(
    docs: &Arc<MemoryDocsSync>,
    projection: &Arc<MemoryIndexProjection>,
    (service, store): (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>),
) -> (
    IngestPipeline,
    Arc<MemoryIndexEntryStore>,
    Arc<MemorySafetyArtifactStore>,
) {
    let entries = Arc::new(MemoryIndexEntryStore::new(store.clone()));
    let pipeline = IngestPipeline::new(docs.clone(), service, entries.clone(), projection.clone());
    (pipeline, entries, store)
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

async fn persist_media_post(
    docs: &MemoryDocsSync,
    replica: &ReplicaId,
    topic: &TopicId,
    body: &str,
) -> String {
    let keys = KukuriKeys::generate();
    let envelope = build_post_envelope_with_payload(
        &keys,
        topic,
        PayloadRef::InlineText {
            text: body.to_string(),
        },
        Vec::new(),
        vec!["media-manifest-test".to_string()],
        None,
        ObjectVisibility::Public,
    )
    .expect("envelope");
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

    let (pipeline, entries, _) = pipeline_with(&docs, &projection, allow_service());
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
    // 真実源（index_entries）にも記録される（投影とペア。#404）。
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
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

    let (pipeline, _, _) = pipeline_with(&docs, &projection, allow_service());
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

/// known-CSAM mock + 一般 moderation（VLM 相当）mock の 2 provider で scan service を組む。
///
/// media scan の verdict / derived タグは VLM 相当の general provider が担い、known-CSAM は
/// `NoKnownMatch` を返す（public_node_default の require_known_csam を満たすため）。
fn service_with_providers(
    providers: Vec<MockSafetyProvider>,
) -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).expect("signer");
    let issuer = signer.issuer_node_id().to_string();
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let mut orchestrator = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    );
    for provider in providers {
        orchestrator = orchestrator.provider(Arc::new(provider));
    }
    let orchestrator = orchestrator.build().expect("orchestrator");
    let service = SafetyScanService::builder(Arc::new(orchestrator), store.clone())
        .signer(Arc::new(signer))
        .build()
        .expect("service");
    (Arc::new(service), store)
}

const MEDIA_HINT: &str = "media-manifest-test";

#[tokio::test]
async fn media_scan_unavailable_fails_closed_and_post_is_not_indexed() -> Result<()> {
    // media 参照 post は media 参照ごとに scan する（#420）。media scan が実行不能
    // （VLM provider / MediaFetcher 未構成 = Unavailable）なら post 全体を index しない
    // （worst-case 合成の fail-closed。従来の「media 参照 post は index しない」挙動を
    // 標準経路経由で保存する）。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_media_post(&docs, &replica, &topic, "media caption").await;

    // post text scan は allow（NoKnownMatch）を返すが、media hint（未設定 subject）は
    // Unavailable エラーになる mock。
    let provider = MockSafetyProvider::known_csam("mock-known-csam")
        .with_no_known_match(&object_id)
        .default_error(ScanError::Unavailable("no media fetcher".to_string()));
    let (pipeline, entries, store) = pipeline_with(&docs, &projection, service_with(provider));
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
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    // post text の verdict 自体は記録される（allow）が、media scan の fail-closed で index
    // はされない。blob 側の verdict も fail-closed として記録される。
    assert!(store.verdict_for(SubjectKind::Post, &object_id).is_some());
    let blob_verdict = store
        .verdict_for(SubjectKind::Blob, MEDIA_HINT)
        .expect("media verdict recorded");
    assert!(!blob_verdict.1.is_indexable());
    Ok(())
}

#[tokio::test]
async fn allow_media_post_is_indexed_and_searchable_via_derived_tags() -> Result<()> {
    // ADR 0028 contract: derived_tags_only_for_allow_media（indexer 面）+
    // ADR 0025 §2.3: allow media は同一 scan が生成した descriptive タグで検索できる。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_media_post(&docs, &replica, &topic, "media caption").await;

    let known = MockSafetyProvider::known_csam("mock-known-csam");
    let vlm = MockSafetyProvider::with_capabilities(
        "mock-vlm",
        vec![kukuri_cn_safety::SafetyProviderCapability::GeneralMediaModeration],
    )
    .with_derived_tags(MEDIA_HINT, vec!["sunset".to_string(), "beach".to_string()]);
    let (pipeline, entries, _store) =
        pipeline_with(&docs, &projection, service_with_providers(vec![known, vlm]));
    let summary = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;

    assert_eq!(summary.indexed, 1);
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    // 投影 text は本文 + 派生タグの相乗り。タグ経由の検索が hit する。
    let stored = projection
        .entries_in_scope(IndexScopeKind::PublicTopic, "rust")
        .await;
    assert_eq!(stored.len(), 1);
    assert!(stored[0].text.contains("media caption"));
    assert!(stored[0].text.contains("sunset"));
    let hits = kukuri_cn_indexer::query::IndexQuery::search_scope(
        projection.as_ref(),
        IndexScopeKind::PublicTopic,
        "rust",
        "sunset",
        10,
    )
    .await?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].object_id, object_id);
    Ok(())
}

#[tokio::test]
async fn flagged_media_post_is_not_indexed_and_tags_do_not_leak() -> Result<()> {
    // media scan が非 allow（general suspected → exclude）なら post 全体を index せず、
    // その scan のタグも index に流れない（derived_tags_only_for_allow_media の否定側）。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_media_post(&docs, &replica, &topic, "media caption").await;

    let known = MockSafetyProvider::known_csam("mock-known-csam");
    let vlm = MockSafetyProvider::with_capabilities(
        "mock-vlm",
        vec![kukuri_cn_safety::SafetyProviderCapability::GeneralMediaModeration],
    )
    .with_score(
        MEDIA_HINT,
        kukuri_cn_safety::SafetyProviderCapability::GeneralMediaModeration,
        SafetyCategory::Nsfw,
        95,
    )
    .with_derived_tags(MEDIA_HINT, vec!["leaked-tag".to_string()]);
    let (pipeline, entries, _store) =
        pipeline_with(&docs, &projection, service_with_providers(vec![known, vlm]));
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
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    assert!(
        projection
            .entries_in_scope(IndexScopeKind::PublicTopic, "rust")
            .await
            .is_empty()
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

    let (pipeline, entries, _) = pipeline_with(&docs, &projection, scan_failed_service());
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
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    Ok(())
}

#[tokio::test]
async fn provider_unavailable_is_never_allowed_and_not_indexed() -> Result<()> {
    // provider unavailable は allow に倒れず、真実源にも投影にも入らない（issue #404 受け入れ条件）。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "provider is down").await;

    let (pipeline, entries, _) = pipeline_with(&docs, &projection, provider_unavailable_service());
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
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
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

    let (pipeline, entries, _) = pipeline_with(&docs, &projection, known_csam_service(&object_id));
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
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
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

    let (allow_pipeline, entries, _) = pipeline_with(&docs, &projection, allow_service());
    allow_pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert!(
        projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    // 後続 scan（別 service）で非 allow に変わる。真実源 store は共有し、de-index が両方へ届くこと
    // を確認する。
    let (flip_service, _flip_store) = known_csam_service(&object_id);
    IngestPipeline::new(
        docs.clone(),
        flip_service,
        entries.clone(),
        projection.clone(),
    )
    .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
    .await?;
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
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

    let (pipeline, entries, _) = pipeline_with(&docs, &projection, allow_service());
    pipeline
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

    // 真実源 store を共有した再 ingest で de-index が両方へ届く。
    let summary = IngestPipeline::new(
        docs.clone(),
        allow_service().0,
        entries.clone(),
        projection.clone(),
    )
    .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
    .await?;
    assert_eq!(summary.deindexed, 1);
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    Ok(())
}

// --- runtime（pipeline）経由の moderation artifact 記録（#406） ---

#[tokio::test]
async fn ingest_known_csam_records_risk_signal_and_does_not_index() -> Result<()> {
    // known CSAM hash match の post は投影に入らず、risk signal + 署名 event が store に入る。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "bad content").await;

    let (pipeline, entries, store) =
        pipeline_with(&docs, &projection, known_csam_service(&object_id));
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
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    // risk signal が trust/relation reads の入力として永続化される（根拠つき、断定ラベルなし）。
    let signals = store.signals();
    assert_eq!(signals.len(), 1);
    let (_, signal) = &signals[0];
    assert_eq!(signal.target, RiskSignalTarget::PostId);
    assert_eq!(signal.target_id, object_id);
    assert_eq!(signal.category, SafetyCategory::Csam);

    // moderation event は実鍵署名済みで検証に通る。
    let events = store.events();
    assert_eq!(events.len(), 1);
    verify_signed_event(&events[0]).expect("event verifies");
    assert_eq!(events[0].body.target_id, object_id);
    Ok(())
}

#[tokio::test]
async fn ingest_scan_failure_is_fail_closed_and_records_no_risk_signal() -> Result<()> {
    // provider failure は runtime（pipeline）経由でも fail-closed: 投影されず、
    // content の safety category を示さないため risk signal も生成されない。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "scan will fail").await;

    let (pipeline, _, store) = pipeline_with(&docs, &projection, scan_failed_service());
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
    assert!(store.signals().is_empty(), "no false risk labels");
    Ok(())
}

#[tokio::test]
async fn ingest_allow_records_no_artifacts() -> Result<()> {
    // allow verdict は投影のみで moderation artifact を作らない。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "clean content").await;

    let (pipeline, entries, store) = pipeline_with(&docs, &projection, allow_service());
    let summary = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;

    assert_eq!(summary.indexed, 1);
    assert!(
        projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    // allow の verdict state は記録される（artifact とは別。index entry の FK 参照先）。
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    assert!(store.events().is_empty());
    assert!(store.signals().is_empty());
    Ok(())
}
