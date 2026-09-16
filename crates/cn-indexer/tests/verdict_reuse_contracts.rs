//! #1050: 保存済み verdict の再利用と、変更 key に絞った取り込みの contract テスト。
//!
//! - 内容と scan 構成が不変なら 2 回目以降の pass で provider を呼ばず、artifact も増えない。
//! - policy / provider 構成の変化、state の変化、hold は再 scan の契機になる。
//! - 撤回・tombstone・送信防止は再利用より優先して de-index する。
//! - 変更通知の key に対応する object だけを取り込み、特定できない key は scope 全体へ倒す。

use std::sync::Arc;

use anyhow::Result;
use kukuri_cn_core::IndexScopeKind;
use kukuri_cn_indexer::ingest::IngestPipeline;
use kukuri_cn_indexer::projection::{IndexProjection, MemoryIndexProjection};
use kukuri_cn_safety::{MockSafetyProvider, ModerationEventSigner, SafetyPolicy};
use kukuri_cn_safety_runtime::clock::SystemScanClock;
use kukuri_cn_safety_runtime::id::UuidEventIdGenerator;
use kukuri_cn_safety_runtime::{
    MemorySafetyArtifactStore, SafetyOrchestrator, SafetyScanService,
    Secp256k1ModerationEventSigner,
};
use kukuri_core::TopicId;
use kukuri_docs_sync::{DocOp, DocsSync, MemoryDocsSync, stable_key, topic_replica_id};

mod ingest_support;
use ingest_support::*;

/// provider 呼び出しを記録する known-CSAM provider を使う service（store を共有できる）。
fn recording_service(
    store: Arc<MemorySafetyArtifactStore>,
) -> (Arc<SafetyScanService>, Arc<RecordingProvider>) {
    let provider = RecordingProvider::new();
    let service = service_with_store(
        store,
        SafetyPolicy::public_node_default(),
        vec![provider.clone()],
    );
    (service, provider)
}

#[test]
fn changed_keys_classify_objects_withdrawals_and_fallback() {
    use kukuri_cn_indexer::ingest::{ChangedKeys, classify_changed_keys};
    assert_eq!(
        classify_changed_keys([
            "objects/a/state",
            "objects/a/envelope",
            "withdrawals/b/state"
        ]),
        ChangedKeys::Objects(vec!["a".to_string(), "b".to_string()])
    );
    // #1065: 索引 key は無視し、同じ batch の object だけを残す。
    assert_eq!(
        classify_changed_keys([
            "indexes/thread/a/s/a",
            "indexes/timeline/s/a",
            "objects/a/envelope",
            "objects/a/state",
        ]),
        ChangedKeys::Objects(vec!["a".to_string()])
    );
    assert_eq!(
        classify_changed_keys([
            "indexes/timeline/s/a",
            "reactions/a/r/state",
            "envelopes/e",
            "sessions/live/s/state",
            "channels/metadata",
            "metaverse/dome-hosting/i/x",
        ]),
        ChangedKeys::Ignored
    );
    assert_eq!(
        classify_changed_keys(["objects/a/state", "manifests/media/m1/envelope"]),
        ChangedKeys::WholeScope {
            reason: "manifests/media".to_string()
        }
    );
    assert_eq!(
        classify_changed_keys(["objects/a/state", "unknown/secret-id/state"]),
        ChangedKeys::WholeScope {
            reason: "unregistered:unknown".to_string()
        }
    );
    assert_eq!(
        classify_changed_keys(["objects//state"]),
        ChangedKeys::WholeScope {
            reason: "malformed:objects".to_string()
        }
    );
    assert_eq!(
        classify_changed_keys(Vec::<&str>::new()),
        ChangedKeys::WholeScope {
            reason: "empty".to_string()
        }
    );
}

/// #1065 INVAR-3: 無視する種別は indexer が読む種別と交わらず、読む種別はすべて取り込みの契機になる。
#[test]
fn ignored_key_families_never_include_what_the_indexer_reads() {
    use kukuri_cn_indexer::ingest::{INDEXER_READ_FAMILIES, KeyDisposition, key_disposition};
    use kukuri_docs_sync::SharedReplicaKeyFamily;
    for family in SharedReplicaKeyFamily::ALL {
        if key_disposition(family) == KeyDisposition::Ignore {
            assert!(
                !INDEXER_READ_FAMILIES.contains(&family),
                "{family:?} is read by the indexer and must not be ignored"
            );
        }
    }
    for family in INDEXER_READ_FAMILIES {
        assert_ne!(
            key_disposition(family),
            KeyDisposition::Ignore,
            "{family:?}"
        );
    }
}

#[tokio::test]
async fn second_pass_with_unchanged_content_performs_no_provider_calls() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "stable body").await;

    let store = Arc::new(MemorySafetyArtifactStore::new());
    let (service, provider) = recording_service(store.clone());
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (service, store.clone()));

    let first = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(
        (first.indexed, first.scans_fresh, first.scans_reused),
        (1, 1, 0)
    );
    assert_eq!(provider.subjects(), vec![object_id.clone()]);

    let second = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(
        (second.indexed, second.scans_fresh, second.scans_reused),
        (1, 0, 1),
        "unchanged content is reused without a provider call"
    );
    assert_eq!(provider.subjects().len(), 1);
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    assert!(
        projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    assert!(store.events().is_empty());
    assert!(store.signals().is_empty());
    Ok(())
}

#[tokio::test]
async fn reingest_deindexes_when_verdict_flips_after_policy_change() -> Result<()> {
    // 同じ store を共有したまま scan 構成（policy_version）が変わると再 scan され、非 allow への
    // 変化が観測されて de-index される。構成が同じままなら provider 結果が変わっても再 scan しない
    // （再利用の契約。運用上の再評価は policy / provider 構成の更新で起こす）。
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "flips after policy").await;

    let (allow, store) = allow_service();
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (allow, store.clone()));
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    let flip_provider =
        MockSafetyProvider::known_csam("mock-known-csam").with_known_hash_match(object_id.as_str());
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).expect("signer");
    let issuer = signer.issuer_node_id().to_string();

    // 構成が同じ（既定 policy）なら保存済み allow を再利用し、flip は観測されない。
    let same_config = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    )
    .provider(Arc::new(flip_provider.clone()))
    .build()
    .expect("orchestrator");
    let same_config = Arc::new(
        SafetyScanService::builder(Arc::new(same_config), store.clone())
            .signer(Arc::new(
                Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).expect("signer"),
            ))
            .build()
            .expect("service"),
    );
    let summary = IngestPipeline::new(
        docs.clone(),
        same_config,
        entries.clone(),
        projection.clone(),
    )
    .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
    .await?;
    assert_eq!(summary.scans_reused, 1);
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    // policy_version が変わると再 scan され、known hash match で de-index される。
    let policy = kukuri_cn_safety::SafetyPolicy {
        policy_version: "2026-09-test-v2".to_string(),
        ..kukuri_cn_safety::SafetyPolicy::public_node_default()
    };
    let new_config = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    )
    .policy(policy)
    .provider(Arc::new(flip_provider))
    .build()
    .expect("orchestrator");
    let new_config = Arc::new(
        SafetyScanService::builder(Arc::new(new_config), store.clone())
            .signer(Arc::new(signer))
            .build()
            .expect("service"),
    );
    let summary = IngestPipeline::new(
        docs.clone(),
        new_config,
        entries.clone(),
        projection.clone(),
    )
    .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
    .await?;
    assert_eq!(summary.scans_fresh, 1);
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn held_verdict_is_retried_and_indexed_when_provider_recovers() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let object_id = persist_post(&docs, &replica, &topic, "held then allowed").await;

    // provider 利用不可 → hold（fail-closed）。
    let (unavailable, store) = provider_unavailable_service();
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (unavailable, store.clone()));
    let first = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!((first.indexed, first.skipped_non_allow), (0, 1));
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    // 同じ構成（provider 名 / capability / policy）で provider が復旧すると、hold は再利用されず
    // 再 scan されて allow になる。
    let signer = Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).expect("signer");
    let issuer = signer.issuer_node_id().to_string();
    let recovered = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock),
        Arc::new(UuidEventIdGenerator),
    )
    .provider(Arc::new(MockSafetyProvider::known_csam("mock-known-csam")))
    .build()
    .expect("orchestrator");
    let recovered = Arc::new(
        SafetyScanService::builder(Arc::new(recovered), store.clone())
            .signer(Arc::new(signer))
            .build()
            .expect("service"),
    );
    let second = IngestPipeline::new(docs.clone(), recovered, entries.clone(), projection.clone())
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(
        (second.indexed, second.scans_fresh, second.scans_reused),
        (1, 1, 0)
    );
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    Ok(())
}

#[tokio::test]
async fn state_content_change_triggers_rescan() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let (object_id, _keys, _envelope, mut state) =
        persist_post_with_source(&docs, &replica, &topic, "editable state").await;

    let store = Arc::new(MemorySafetyArtifactStore::new());
    let (service, provider) = recording_service(store.clone());
    let (pipeline, _entries, _) = pipeline_with(&docs, &projection, (service, store));
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(provider.subjects().len(), 1);

    // state（status）が変わると content hash が変わり、再 scan される。
    state["status"] = serde_json::json!("edited");
    docs.apply_doc_op(
        &replica,
        DocOp::SetJson {
            key: stable_key("objects", &format!("{object_id}/state")),
            value: state,
        },
    )
    .await?;
    let summary = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!((summary.scans_fresh, summary.scans_reused), (1, 0));
    assert_eq!(provider.subjects().len(), 2);
    Ok(())
}

#[tokio::test]
async fn withdrawal_after_reuse_still_deindexes() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let (object_id, keys, envelope, _state) =
        persist_post_with_source(&docs, &replica, &topic, "withdrawn later").await;

    let (allow, store) = allow_service();
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (allow, store));
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    let reused = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(reused.scans_reused, 1);
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    let withdrawal = kukuri_core::build_post_withdrawal_envelope(
        &keys,
        &envelope,
        1,
        None,
        kukuri_core::WithdrawalReasonVisibility::Public,
        Some(kukuri_core::PostWithdrawalReason::AuthorRequest),
    )?;
    docs.apply_doc_op(
        &replica,
        DocOp::SetJson {
            key: stable_key("withdrawals", &format!("{object_id}/state")),
            value: serde_json::to_value(&withdrawal)?,
        },
    )
    .await?;

    // 撤回 key だけの変更通知でも対象 object が de-index される（再利用より撤回が優先）。
    let summary = pipeline
        .ingest_changed_keys(
            IndexScopeKind::PublicTopic,
            "rust",
            &replica,
            &[stable_key("withdrawals", &format!("{object_id}/state"))],
        )
        .await?;
    assert_eq!((summary.scanned, summary.deindexed), (1, 1));
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn ingest_changed_keys_processes_only_the_changed_object() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let first = persist_post(&docs, &replica, &topic, "first").await;
    let second = persist_post(&docs, &replica, &topic, "second").await;

    let store = Arc::new(MemorySafetyArtifactStore::new());
    let (service, provider) = recording_service(store.clone());
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (service, store));
    let full = pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(full.scanned, 2);
    assert_eq!(provider.subjects().len(), 2);

    // 3 件目の変更通知: その object だけを走査・scan する。
    let third = persist_post(&docs, &replica, &topic, "third").await;
    let summary = pipeline
        .ingest_changed_keys(
            IndexScopeKind::PublicTopic,
            "rust",
            &replica,
            &[stable_key("objects", &format!("{third}/state"))],
        )
        .await?;
    assert_eq!(
        (summary.scanned, summary.indexed, summary.scans_fresh),
        (1, 1, 1)
    );
    let subjects = provider.subjects();
    assert_eq!(subjects.len(), 3);
    assert_eq!(subjects[2], third);
    for id in [&first, &second, &third] {
        assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", id));
    }

    // 対象を特定できない key が混ざると scope 全体の見直しへ倒れる（既存分は再利用）。
    let fallback = pipeline
        .ingest_changed_keys(
            IndexScopeKind::PublicTopic,
            "rust",
            &replica,
            &["manifests/media/m1/envelope".to_string()],
        )
        .await?;
    assert_eq!(
        (
            fallback.scanned,
            fallback.scans_fresh,
            fallback.scans_reused
        ),
        (3, 0, 3)
    );
    assert_eq!(provider.subjects().len(), 3);
    Ok(())
}

/// #1065 TR-1: 実クライアントの 1 投稿に伴う key 集合（state / envelope / timeline 索引 / thread 索引）
/// は当該 object だけを取り込み、scope 全体を見直さない。
#[tokio::test]
async fn client_post_change_keys_ingest_only_that_object() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    persist_post(&docs, &replica, &topic, "first").await;
    persist_post(&docs, &replica, &topic, "second").await;

    let store = Arc::new(MemorySafetyArtifactStore::new());
    let (service, provider) = recording_service(store.clone());
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (service, store));
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(provider.subjects().len(), 2);

    let (third, _, _, _, changed) = persist_client_post(&docs, &replica, &topic, "third").await;
    assert_eq!(changed.len(), 4, "state / envelope / timeline / thread");
    let summary = pipeline
        .ingest_changed_keys(IndexScopeKind::PublicTopic, "rust", &replica, &changed)
        .await?;
    assert_eq!(
        (
            summary.scanned,
            summary.indexed,
            summary.scans_fresh,
            summary.scans_reused
        ),
        (1, 1, 1, 0),
        "only the changed object is read; the other two are not revisited"
    );
    assert_eq!(provider.subjects().len(), 3);
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &third));
    Ok(())
}

/// #1065 TR-2: 索引に影響しない key だけの batch は ingest を起こさない。
#[tokio::test]
async fn non_indexing_change_keys_do_not_ingest() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let (object_id, _, _, _, written) =
        persist_client_post(&docs, &replica, &topic, "already indexed").await;

    let store = Arc::new(MemorySafetyArtifactStore::new());
    let (service, provider) = recording_service(store.clone());
    let (pipeline, _, _) = pipeline_with(&docs, &projection, (service, store));
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(provider.subjects().len(), 1);

    let index_only: Vec<String> = written
        .iter()
        .filter(|key| key.starts_with("indexes/"))
        .cloned()
        .collect();
    let reaction_only = vec![
        stable_key("reactions", &format!("{object_id}/r1/state")),
        stable_key("reactions", &format!("{object_id}/r1/envelope")),
        stable_key("envelopes", "reaction-envelope-id"),
    ];
    let others = vec![
        stable_key("sessions/live", "s1/state"),
        stable_key("sessions/game", "g1/state"),
        stable_key("channels", "metadata"),
        stable_key("channels/participants", "p1/envelope"),
        stable_key("metaverse/dome-deletions", "h1/state"),
    ];
    for batch in [index_only, reaction_only, others] {
        let summary = pipeline
            .ingest_changed_keys(IndexScopeKind::PublicTopic, "rust", &replica, &batch)
            .await?;
        assert_eq!(
            summary,
            kukuri_cn_indexer::ingest::IngestSummary::default(),
            "batch {batch:?} must not ingest"
        );
    }
    assert_eq!(provider.subjects().len(), 1);
    Ok(())
}

/// #1065 TR-4: 撤回 key と索引 key が同じ batch に入っても対象 object を de-index する。
#[tokio::test]
async fn withdrawal_with_index_keys_still_deindexes() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    let (object_id, keys, envelope, _, written) =
        persist_client_post(&docs, &replica, &topic, "withdrawn with index keys").await;

    let (allow, store) = allow_service();
    let (pipeline, entries, _) = pipeline_with(&docs, &projection, (allow, store));
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert!(entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));

    let withdrawal = kukuri_core::build_post_withdrawal_envelope(
        &keys,
        &envelope,
        1,
        None,
        kukuri_core::WithdrawalReasonVisibility::Public,
        Some(kukuri_core::PostWithdrawalReason::AuthorRequest),
    )?;
    let withdrawal_key = stable_key("withdrawals", &format!("{object_id}/state"));
    docs.apply_doc_op(
        &replica,
        DocOp::SetJson {
            key: withdrawal_key.clone(),
            value: serde_json::to_value(&withdrawal)?,
        },
    )
    .await?;

    let mut batch: Vec<String> = written
        .iter()
        .filter(|key| key.starts_with("indexes/"))
        .cloned()
        .collect();
    batch.push(withdrawal_key);
    batch.sort();
    let summary = pipeline
        .ingest_changed_keys(IndexScopeKind::PublicTopic, "rust", &replica, &batch)
        .await?;
    assert_eq!((summary.scanned, summary.deindexed), (1, 1));
    assert!(!entries.contains(IndexScopeKind::PublicTopic, "rust", &object_id));
    assert!(
        !projection
            .contains_object(IndexScopeKind::PublicTopic, "rust", &object_id)
            .await?
    );
    Ok(())
}

/// #1065 TR-3: 未登録 key を含む batch は scope 全体へ倒れ、回数と理由（識別子を含まない）を記録する。
#[tokio::test]
async fn unregistered_change_key_falls_back_to_whole_scope_and_is_observed() -> Result<()> {
    let docs = Arc::new(MemoryDocsSync::default());
    let projection = Arc::new(MemoryIndexProjection::new());
    let topic = TopicId::new("rust");
    let replica = topic_replica_id("rust");
    persist_post(&docs, &replica, &topic, "first").await;
    let (second, _, _, _, written) = persist_client_post(&docs, &replica, &topic, "second").await;

    let store = Arc::new(MemorySafetyArtifactStore::new());
    let (service, provider) = recording_service(store.clone());
    let metrics = Arc::new(kukuri_cn_indexer::state::IndexerRuntimeState::default());
    let (pipeline, _, _) = pipeline_with(&docs, &projection, (service, store));
    let pipeline = pipeline.with_metrics(metrics.clone());
    pipeline
        .ingest_scope(IndexScopeKind::PublicTopic, "rust", &replica)
        .await?;
    assert_eq!(provider.subjects().len(), 2);

    // 投稿 1 件分の key だけなら全体見直しは起きず、観測値も増えない。
    pipeline
        .ingest_changed_keys(IndexScopeKind::PublicTopic, "rust", &replica, &written)
        .await?;
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.event_whole_scope_fallbacks, 0);
    assert_eq!(snapshot.last_whole_scope_fallback_reason, None);

    let mut batch = written.clone();
    batch.push(format!("future-feature/{second}/state"));
    let summary = pipeline
        .ingest_changed_keys(IndexScopeKind::PublicTopic, "rust", &replica, &batch)
        .await?;
    assert_eq!(
        (summary.scanned, summary.scans_reused),
        (2, 2),
        "whole scope is revisited"
    );
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.event_whole_scope_fallbacks, 1);
    assert_eq!(
        snapshot.last_whole_scope_fallback_reason.as_deref(),
        Some("unregistered:future-feature")
    );

    // media manifest も全体見直しの契機として記録される。
    pipeline
        .ingest_changed_keys(
            IndexScopeKind::PublicTopic,
            "rust",
            &replica,
            &[stable_key("manifests/media", "m1/envelope")],
        )
        .await?;
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.event_whole_scope_fallbacks, 2);
    assert_eq!(
        snapshot.last_whole_scope_fallback_reason.as_deref(),
        Some("manifests/media")
    );
    assert_eq!(provider.subjects().len(), 2);
    Ok(())
}
