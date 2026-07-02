//! #404 query 境界（search / discovery / recommendation）+ fail-closed query gate の contract test。
//!
//! in-memory な `DocsSync` + `MemoryIndexProjection` + `MemoryIndexEntryStore` + mock safety
//! provider で、ユーザー向け読み口の不変条件を検証する:
//! - `allow` verdict の entry のみが search / discovery / recommendation に出る
//!   （`search_discovery_recommendation_excludes_non_allow`）。
//! - 投影に残留した hit（真実源に無い）は返さない（fail-closed gate）。
//! - index 後に verdict が非 allow / critical へ変わった entry は de-index 前でも返さない。
//! - topic 内検索は scope に閉じ、横断検索は supported set 全体（= 投影全体）を対象にする。

use std::sync::Arc;

use anyhow::Result;
use kukuri_cn_core::{
    IndexScopeKind, MemoryIndexEntryStore, MemorySafetyArtifactStore, SafetyArtifactStore,
    SafetyScanService,
};
use kukuri_cn_indexer::ingest::IngestPipeline;
use kukuri_cn_indexer::projection::{IndexProjection, IndexedEntry, MemoryIndexProjection};
use kukuri_cn_indexer::query::{FailClosedIndexQuery, IndexQuery, MAX_QUERY_LIMIT};
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{
    MockSafetyProvider, ModerationEventSigner, ReasonCode, SafetyAction, SafetyVerdict,
};
use kukuri_cn_safety_runtime::clock::SystemScanClock;
use kukuri_cn_safety_runtime::id::UuidEventIdGenerator;
use kukuri_cn_safety_runtime::{SafetyOrchestrator, Secp256k1ModerationEventSigner};
use kukuri_core::{KukuriKeys, ReplicaId, TopicId, build_post_envelope};
use kukuri_docs_sync::{DocOp, DocsSync, MemoryDocsSync, stable_key, topic_replica_id};

const TEST_SECRET: &str = "0000000000000000000000000000000000000000000000000000000000000001";

/// mock provider（known CSAM = NoKnownMatch → allow）で scan service を組む。
fn allow_service() -> (Arc<SafetyScanService>, Arc<MemorySafetyArtifactStore>) {
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).expect("signer");
    let issuer = signer.issuer_node_id().to_string();
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let orchestrator = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    )
    .provider(Arc::new(MockSafetyProvider::known_csam("mock-known-csam")))
    .build()
    .expect("orchestrator");
    let service = SafetyScanService::builder(Arc::new(orchestrator), store.clone())
        .signer(Arc::new(signer))
        .build()
        .expect("service");
    (Arc::new(service), store)
}

/// 本文 text の post envelope を共有 replica に実在させ、object_id を返す。
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

/// ingest 済みの環境一式（docs / 投影 / 真実源 / artifact store / gated query）を組む。
struct QueryFixture {
    docs: Arc<MemoryDocsSync>,
    projection: Arc<MemoryIndexProjection>,
    entries: Arc<MemoryIndexEntryStore>,
    store: Arc<MemorySafetyArtifactStore>,
    pipeline: IngestPipeline,
    query: FailClosedIndexQuery,
}

fn fixture() -> QueryFixture {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let (service, store) = allow_service();
    let entries = Arc::new(MemoryIndexEntryStore::new(store.clone()));
    let pipeline = IngestPipeline::new(docs.clone(), service, entries.clone(), projection.clone());
    let query = FailClosedIndexQuery::new(projection.clone(), entries.clone());
    QueryFixture {
        docs,
        projection,
        entries,
        store,
        pipeline,
        query,
    }
}

/// 非 allow / critical の verdict（後から verdict が変わるケースの模擬用）。
fn exclude_critical_verdict() -> SafetyVerdict {
    SafetyVerdict {
        action: SafetyAction::Exclude,
        labels: Vec::new(),
        critical: true,
        reason_code: ReasonCode::CsamConfirmed,
        confidence: None,
        provider: Some("mock-known-csam".to_string()),
        provider_capability: None,
        policy_version: "policy-v1-test".to_string(),
        scanned_at: "2026-07-02T10:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn topic_scoped_search_returns_allow_entries_in_scope_only() -> Result<()> {
    let f = fixture();
    let rust_topic = TopicId::new("rust");
    let rust_replica = topic_replica_id("rust");
    let go_topic = TopicId::new("golang");
    let go_replica = topic_replica_id("golang");
    let rust_post = persist_post(&f.docs, &rust_replica, &rust_topic, "tokio async runtime").await;
    let go_post = persist_post(&f.docs, &go_replica, &go_topic, "goroutine async runtime").await;

    f.pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &rust_replica)
        .await?;
    f.pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "golang", &go_replica)
        .await?;

    // topic 内検索は scope に閉じる（ADR 0025 §2.7: トピック毎の検索窓が基本 UX）。
    let hits = f
        .query
        .search_scope(IndexScopeKind::PublicTopic, "rust", "async", 10)
        .await?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].object_id, rust_post);

    // 横断検索は supported set 全体（= 投影全体）に当たる。
    let hits = f.query.search_all("async", 10).await?;
    let ids: Vec<&str> = hits.iter().map(|hit| hit.object_id.as_str()).collect();
    assert!(ids.contains(&rust_post.as_str()));
    assert!(ids.contains(&go_post.as_str()));
    Ok(())
}

#[tokio::test]
async fn search_discovery_recommendation_excludes_non_allow() -> Result<()> {
    let f = fixture();
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let kept = persist_post(&f.docs, &replica, &topic, "clean searchable post").await;
    let flipped = persist_post(&f.docs, &replica, &topic, "flips to excluded post").await;

    f.pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;

    // index 後に verdict が exclude / critical へ変わる（de-index はまだ走っていない）。
    // 真実源の verdict record は対象ごとに upsert されるため、gate の join が最新値を見る。
    f.store
        .persist_verdict(
            SubjectKind::Post,
            flipped.as_str(),
            &exclude_critical_verdict(),
        )
        .await?;

    // search: 非 allow は返らない。
    let hits = f
        .query
        .search_scope(IndexScopeKind::PublicTopic, "rust", "post", 10)
        .await?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].object_id, kept);

    // discovery / recommendation（新着列挙）: critical / 非 allow は入らない。
    let recent = f
        .query
        .list_recent(Some((IndexScopeKind::PublicTopic, "rust")), 10)
        .await?;
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].object_id, kept);

    let recent_all = f.query.list_recent(None, 10).await?;
    assert_eq!(recent_all.len(), 1);
    assert_eq!(recent_all[0].object_id, kept);
    Ok(())
}

#[tokio::test]
async fn projection_residue_without_authoritative_entry_is_not_surfaced() -> Result<()> {
    // 投影に「残留」した hit（真実源に entry が無い = verdict を伴わない）は返さない。
    // unscanned content が投影へ紛れ込む障害を模擬する（DB 制約 + gate の防御の重ね）。
    let f = fixture();
    f.projection
        .upsert_entry(&IndexedEntry {
            scope_kind: IndexScopeKind::PublicTopic,
            scope_id: "rust".to_string(),
            object_id: "ghost-post".to_string(),
            author_pubkey: "author".to_string(),
            text: "ghost searchable text".to_string(),
            created_at: 1,
            source_replica_id: "topic::rust".to_string(),
        })
        .await?;

    assert!(
        f.query
            .search_scope(IndexScopeKind::PublicTopic, "rust", "ghost", 10)
            .await?
            .is_empty()
    );
    assert!(
        f.query
            .list_recent(Some((IndexScopeKind::PublicTopic, "rust")), 10)
            .await?
            .is_empty()
    );
    assert!(f.query.search_all("ghost", 10).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn deindexed_scope_disappears_from_all_read_surfaces() -> Result<()> {
    // scope の de-index（supported 除去相当）後は search / 新着列挙のどこにも出ない。
    let f = fixture();
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    persist_post(&f.docs, &replica, &topic, "soon to be unsupported").await;
    f.pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(f.query.list_recent(None, 10).await?.len(), 1);

    use kukuri_cn_core::IndexEntryStore;
    f.entries
        .remove_scope(IndexScopeKind::PublicTopic, "rust")
        .await?;
    f.projection
        .remove_scope(IndexScopeKind::PublicTopic, "rust")
        .await?;

    assert!(f.query.search_all("unsupported", 10).await?.is_empty());
    assert!(f.query.list_recent(None, 10).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn query_limit_is_clamped() -> Result<()> {
    let f = fixture();
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    persist_post(&f.docs, &replica, &topic, "clamp me").await;
    f.pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;

    // limit 0 でも 1 に丸められてクエリ自体は成立する（呼び出し側の誤用で全量を引かない）。
    let hits = f.query.list_recent(None, 0).await?;
    assert_eq!(hits.len(), 1);
    // 上限超えの limit は MAX_QUERY_LIMIT に丸められる（有界であること自体を確認）。
    let hits = f.query.list_recent(None, MAX_QUERY_LIMIT * 100).await?;
    assert_eq!(hits.len(), 1);
    Ok(())
}
