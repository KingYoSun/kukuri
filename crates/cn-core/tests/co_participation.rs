//! co-participation 集計（relation feature の観測元、ADR 0026 §2.4 / #415）の contract テスト。
//!
//! - in-memory（DB 不要、常時実行）: 集計セマンティクスと private channel 除外
//!   （`relation_private_channel_signal_stays_local_and_scoped` の入力側）を固定する。
//! - Postgres（`KUKURI_CN_RUN_INTEGRATION_TESTS=1`）: Pg 実装が in-memory と同一の
//!   セマンティクスを返すことを同一シードで検証する（実装間 drift の防止）。

use std::sync::Arc;

use anyhow::Result;

use kukuri_cn_core::{
    CoParticipationPair, CoParticipationSource, IndexEntryStore, IndexScopeKind,
    MemoryIndexEntryStore, MemorySafetyArtifactStore, NewIndexEntry, PgCoParticipationSource,
    SafetyArtifactStore, TestDatabase, connect_postgres, initialize_database, upsert_index_entry,
    upsert_scan_verdict,
};
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{ReasonCode, SafetyAction, SafetyVerdict};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

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

fn entry(
    scope_kind: IndexScopeKind,
    scope_id: &str,
    object_id: &str,
    author: &str,
    verdict_id: &str,
) -> NewIndexEntry {
    NewIndexEntry {
        scope_kind,
        scope_id: scope_id.to_string(),
        object_id: object_id.to_string(),
        author_pubkey: author.to_string(),
        created_at: 1_700_000_000,
        source_replica_id: format!("replica::{scope_id}"),
        verdict_id: verdict_id.to_string(),
        verdict_action: "allow".to_string(),
        critical: false,
    }
}

/// テストシード: topic-a に A×2 + B×1、topic-b に A×1 + B×1、private channel chan-x に
/// A×1 + C×1（→ private 由来は集計に出ないこと）。
///
/// 期待値:
/// - ペアは (A, B) のみ: shared_topics = 2、co_participation_events = min(2,1) + min(1,1) = 2。
/// - (A, C) は private channel のみの共起なので **存在しない**。
/// - dominant topic: A → topic-a（2 entries）、B → topic-a（1 entry、同数 tie は辞書順）。
///   C は public 参加が無いので出ない。
struct Seed {
    /// (scope_kind, scope_id, object_id, author)
    entries: Vec<(IndexScopeKind, &'static str, &'static str, &'static str)>,
}

fn seed() -> Seed {
    use IndexScopeKind::{PrivateChannel, PublicTopic};
    Seed {
        entries: vec![
            (PublicTopic, "topic-a", "post-1", "author-a"),
            (PublicTopic, "topic-a", "post-2", "author-a"),
            (PublicTopic, "topic-a", "post-3", "author-b"),
            (PublicTopic, "topic-b", "post-4", "author-a"),
            (PublicTopic, "topic-b", "post-5", "author-b"),
            (PrivateChannel, "chan-x", "post-6", "author-a"),
            (PrivateChannel, "chan-x", "post-7", "author-c"),
        ],
    }
}

async fn assert_co_participation_contracts(source: &dyn CoParticipationSource) -> Result<()> {
    let pairs = source.list_co_participation_pairs(100).await?;
    assert_eq!(
        pairs,
        vec![CoParticipationPair {
            author_a: "author-a".to_string(),
            author_b: "author-b".to_string(),
            shared_topics: 2,
            co_participation_events: 2,
        }],
        "public topic の共起のみが集計され、private channel 由来の (A, C) ペアは出ないこと"
    );

    let dominant = source.list_author_dominant_topics(100).await?;
    assert_eq!(dominant.len(), 2, "public 参加の無い author-c は出ない");
    assert_eq!(dominant[0].author_pubkey, "author-a");
    assert_eq!(dominant[0].topic_id, "topic-a");
    assert_eq!(dominant[0].entries, 2);
    assert_eq!(dominant[1].author_pubkey, "author-b");
    // B は topic-a / topic-b とも 1 entry。tie は scope_id 辞書順で決定論化。
    assert_eq!(dominant[1].topic_id, "topic-a");

    // limit の有界化。
    let limited = source.list_co_participation_pairs(0).await?;
    assert!(limited.is_empty());
    Ok(())
}

#[tokio::test]
async fn co_participation_pairs_from_public_topics_only_memory() -> Result<()> {
    let verdicts = Arc::new(MemorySafetyArtifactStore::new());
    let store = MemoryIndexEntryStore::new(verdicts.clone());
    for (i, (scope_kind, scope_id, object_id, author)) in seed().entries.iter().enumerate() {
        let verdict_id = verdicts
            .persist_verdict(SubjectKind::Post, &format!("subject-{i}"), &allow_verdict())
            .await?;
        store
            .upsert_entry(&entry(
                *scope_kind,
                scope_id,
                object_id,
                author,
                &verdict_id,
            ))
            .await?;
    }
    assert_co_participation_contracts(&store).await
}

#[tokio::test]
async fn co_participation_pairs_from_public_topics_only_postgres() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core co_participation test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_co_participation").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        for (i, (scope_kind, scope_id, object_id, author)) in seed().entries.iter().enumerate() {
            let stored = upsert_scan_verdict(
                &pool,
                SubjectKind::Post,
                &format!("subject-{i}"),
                &allow_verdict(),
            )
            .await?;
            upsert_index_entry(
                &pool,
                &entry(*scope_kind, scope_id, object_id, author, &stored.id),
            )
            .await?;
        }
        let source = PgCoParticipationSource::new(pool.clone());
        assert_co_participation_contracts(&source).await
    }
    .await;
    database.cleanup().await?;
    result
}
