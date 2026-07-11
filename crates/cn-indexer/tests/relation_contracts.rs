//! relation graph / 解析 worker の contract テスト（ADR 0026 §6.1, #415）。
//!
//! - 解析 worker（in-memory、常時実行）: co-participation 集計 → feature 点数化 → graph 反映と
//!   cluster 帰属（dominant shared topic）を固定する。private channel 由来が relation に
//!   混ざらないこと・index 真実源を改変しないことも固定する。
//! - ArcadeDB（`KUKURI_CN_RUN_ARCADEDB_TESTS=1`、要 live ArcadeDB）: in-memory と同一の
//!   共有 contract スイート（`relation_testing`）を実行し、backend 間の drift を防ぐ。

use std::sync::Arc;

use anyhow::Result;

use kukuri_cn_core::{IndexEntryStore, IndexScopeKind, MemoryIndexEntryStore, NewIndexEntry};
use kukuri_cn_indexer::{ArcadeDbConfig, ArcadeDbRelationGraph, analyze_relations, topic_cluster};
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{ReasonCode, SafetyAction, SafetyVerdict};
use kukuri_cn_safety_runtime::{MemorySafetyArtifactStore, SafetyArtifactStore};
use kukuri_cn_trust::relation_testing::assert_relation_store_contracts;
use kukuri_cn_trust::{
    FEATURE_CO_PARTICIPATION_EVENTS, FEATURE_SHARED_TOPICS, MemoryRelationStore, RelationStore,
};

fn allow_verdict() -> SafetyVerdict {
    SafetyVerdict {
        action: SafetyAction::Allow,
        labels: Vec::new(),
        critical: false,
        reason_code: ReasonCode::NoKnownMatch,
        confidence: None,
        provider: Some("mock-known-csam".to_string()),
        provider_capability: None,
        policy_version: "policy-v1-test".to_string(),
        scanned_at: "2026-07-02T09:00:00Z".to_string(),
    }
}

/// cn-core の co_participation テストと同じシード:
/// topic-a = A×2 + B×1、topic-b = A×1 + B×1、private channel chan-x = A×1 + C×1。
async fn seeded_entries() -> Result<MemoryIndexEntryStore> {
    let verdicts = Arc::new(MemorySafetyArtifactStore::new());
    let store = MemoryIndexEntryStore::new(verdicts.clone());
    let seed = [
        (IndexScopeKind::PublicTopic, "topic-a", "post-1", "author-a"),
        (IndexScopeKind::PublicTopic, "topic-a", "post-2", "author-a"),
        (IndexScopeKind::PublicTopic, "topic-a", "post-3", "author-b"),
        (IndexScopeKind::PublicTopic, "topic-b", "post-4", "author-a"),
        (IndexScopeKind::PublicTopic, "topic-b", "post-5", "author-b"),
        (
            IndexScopeKind::PrivateChannel,
            "chan-x",
            "post-6",
            "author-a",
        ),
        (
            IndexScopeKind::PrivateChannel,
            "chan-x",
            "post-7",
            "author-c",
        ),
    ];
    for (i, (scope_kind, scope_id, object_id, author)) in seed.iter().enumerate() {
        let verdict_id = verdicts
            .persist_verdict(SubjectKind::Post, &format!("subject-{i}"), &allow_verdict())
            .await?;
        store
            .upsert_entry(&NewIndexEntry {
                scope_kind: *scope_kind,
                scope_id: scope_id.to_string(),
                object_id: object_id.to_string(),
                author_pubkey: author.to_string(),
                created_at: 1_700_000_000,
                source_replica_id: format!("replica::{scope_id}"),
                verdict_id,
                verdict_action: "allow".to_string(),
                critical: false,
            })
            .await?;
    }
    Ok(store)
}

#[tokio::test]
async fn relation_worker_builds_pairwise_proximity_from_co_participation() -> Result<()> {
    let entries = seeded_entries().await?;
    let graph = MemoryRelationStore::new();

    let report = analyze_relations(&entries, &graph, 1000).await?;
    assert_eq!(report.edges_upserted, 1, "(A, B) の 1 ペアのみ");
    assert_eq!(report.clusters_assigned, 2, "public 参加のある A / B のみ");

    // 同一 cluster で共起の濃い A, B は proximity が根拠つきで返る。
    let proximity = graph
        .pairwise_proximity("author-a", "author-b")
        .await?
        .expect("co-participating pair has proximity");
    assert!(proximity.score > 0.0);
    let shared = proximity
        .basis
        .iter()
        .find(|e| e.feature == FEATURE_SHARED_TOPICS)
        .expect("shared_topics feature in basis");
    assert_eq!(shared.value, 2.0);
    let events = proximity
        .basis
        .iter()
        .find(|e| e.feature == FEATURE_CO_PARTICIPATION_EVENTS)
        .expect("co_participation_events feature in basis");
    assert_eq!(events.value, 2.0);

    // private channel でしか共起しない (A, C) は relation に出ない
    // （relation_private_channel_signal_stays_local_and_scoped の graph 側）。
    assert!(
        graph
            .pairwise_proximity("author-a", "author-c")
            .await?
            .is_none()
    );

    // cluster 帰属 = dominant shared topic（初期実装）。
    assert_eq!(
        graph.cluster_of("author-a").await?,
        Some(topic_cluster("topic-a"))
    );
    // public 参加の無い C は帰属しない。
    assert!(graph.cluster_of("author-c").await?.is_none());
    Ok(())
}

#[tokio::test]
async fn relation_worker_is_idempotent_and_does_not_mutate_index_source() -> Result<()> {
    let entries = seeded_entries().await?;
    let graph = MemoryRelationStore::new();

    let before = entries.entries_snapshot().len();
    let first = analyze_relations(&entries, &graph, 1000).await?;
    let proximity_first = graph.pairwise_proximity("author-a", "author-b").await?;
    let second = analyze_relations(&entries, &graph, 1000).await?;
    let proximity_second = graph.pairwise_proximity("author-a", "author-b").await?;

    // 冪等: 再実行しても graph は同じ状態に収束する。
    assert_eq!(first, second);
    assert_eq!(proximity_first, proximity_second);

    // worker は観測元（index 真実源）を読むだけで改変しない（canonical 非改変の overlay 境界。
    // social graph canonical への書き込み口はそもそも存在しない）。
    assert_eq!(entries.entries_snapshot().len(), before);
    Ok(())
}

/// ArcadeDB backend への共有 contract スイート実行。
///
/// `KUKURI_CN_RUN_ARCADEDB_TESTS=1` かつ live ArcadeDB（`COMMUNITY_NODE_ARCADEDB_*`）が
/// あるときのみ実行する（他の integration テストと同じ env-gate 流儀）。
#[tokio::test]
async fn arcadedb_relation_store_satisfies_shared_contracts() -> Result<()> {
    if !kukuri_test_support::env_flag_enabled("KUKURI_CN_RUN_ARCADEDB_TESTS") {
        eprintln!("skipping ArcadeDB relation test; set KUKURI_CN_RUN_ARCADEDB_TESTS=1");
        return Ok(());
    }
    let graph = ArcadeDbRelationGraph::new(ArcadeDbConfig::from_env())?;
    graph.ensure_schema().await?;
    // 走行間の残留データと衝突しない一意 prefix（in-memory と同一スイート）。
    let prefix = format!(
        "cntest-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    assert_relation_store_contracts(&graph, &prefix).await?;
    Ok(())
}
