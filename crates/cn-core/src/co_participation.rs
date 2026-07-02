//! relation feature の観測元 = index 真実源からの co-participation 集計（ADR 0026 §2.4, #415）。
//!
//! `cn_index.index_entries`（#404 の fail-closed 真実源）から author 間の共起を集計する。
//! 真実源には `allow` verdict の entry しか存在しない（DB 制約）ため、集計結果も自動的に
//! fail-closed 済みの観測になる。
//!
//! **private channel 由来は集計しない**（ADR 0026 §6.3 / プラン Assumption 5, fail-closed）:
//! `scope_kind = public_topic` のみを対象にし、private channel の co-participation が
//! public relation に混ざらないことを構造的に保証する
//! （`relation_private_channel_signal_stays_local_and_scoped` の入力側）。
//! channel メンバー可視の relation は後続 Issue。
//!
//! 集計セマンティクス（Pg / in-memory で同一）:
//! - `shared_topics`: 両 author が entry を持つ supported topic の数。
//! - `co_participation_events`: 共有 topic ごとの `min(entries_a, entries_b)` の総和
//!   （どちらか一方が大量投稿しても共起強度が過大評価されない有界化）。
//! - dominant topic: author が最も多く entry を持つ topic（cluster_of の初期実装の入力。
//!   同数は scope_id 辞書順で決定論化）。

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::Row;

use crate::index_entries::MemoryIndexEntryStore;
use crate::index_scope::IndexScopeKind;

/// author ペアの co-participation 集計 1 件（`author_a < author_b` の正規化済みペア）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoParticipationPair {
    pub author_a: String,
    pub author_b: String,
    /// 両者が entry を持つ supported topic の数。
    pub shared_topics: i64,
    /// 共有 topic ごとの min(entries_a, entries_b) の総和。
    pub co_participation_events: i64,
}

/// author の dominant topic（最多 entry の topic。cluster 帰属の初期入力）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorDominantTopic {
    pub author_pubkey: String,
    pub topic_id: String,
    pub entries: i64,
}

/// co-participation 集計の read 抽象。
///
/// 本番は Postgres（index 真実源への集計クエリ）、contract test は in-memory
/// （`MemoryIndexEntryStore` のスナップショットへ同一セマンティクスを適用）。
/// relation 解析 worker（`cn-indexer`）はこの trait 越しに観測を読む。
#[async_trait]
pub trait CoParticipationSource: Send + Sync {
    /// author ペアの共起集計（shared_topics 降順 → events 降順 → pubkey 辞書順、最大 limit 件）。
    async fn list_co_participation_pairs(&self, limit: usize) -> Result<Vec<CoParticipationPair>>;

    /// author ごとの dominant topic（author_pubkey 辞書順、最大 limit 件）。
    async fn list_author_dominant_topics(&self, limit: usize) -> Result<Vec<AuthorDominantTopic>>;
}

/// Postgres 実装。
#[derive(Clone, Debug)]
pub struct PgCoParticipationSource {
    pool: PgPool,
}

impl PgCoParticipationSource {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CoParticipationSource for PgCoParticipationSource {
    async fn list_co_participation_pairs(&self, limit: usize) -> Result<Vec<CoParticipationPair>> {
        let rows = sqlx::query(
            "WITH participation AS (
                 SELECT scope_id, author_pubkey, COUNT(*) AS entries
                 FROM cn_index.index_entries
                 WHERE scope_kind = 'public_topic'
                 GROUP BY scope_id, author_pubkey
             )
             SELECT p1.author_pubkey AS author_a,
                    p2.author_pubkey AS author_b,
                    COUNT(*)::BIGINT AS shared_topics,
                    SUM(LEAST(p1.entries, p2.entries))::BIGINT AS co_participation_events
             FROM participation p1
             JOIN participation p2
               ON p1.scope_id = p2.scope_id
              AND p1.author_pubkey < p2.author_pubkey
             GROUP BY p1.author_pubkey, p2.author_pubkey
             ORDER BY shared_topics DESC, co_participation_events DESC, author_a, author_b
             LIMIT $1",
        )
        .bind(i64::try_from(limit)?)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(CoParticipationPair {
                    author_a: row.try_get("author_a")?,
                    author_b: row.try_get("author_b")?,
                    shared_topics: row.try_get("shared_topics")?,
                    co_participation_events: row.try_get("co_participation_events")?,
                })
            })
            .collect()
    }

    async fn list_author_dominant_topics(&self, limit: usize) -> Result<Vec<AuthorDominantTopic>> {
        let rows = sqlx::query(
            "SELECT DISTINCT ON (author_pubkey)
                    author_pubkey,
                    scope_id AS topic_id,
                    COUNT(*)::BIGINT AS entries
             FROM cn_index.index_entries
             WHERE scope_kind = 'public_topic'
             GROUP BY author_pubkey, scope_id
             ORDER BY author_pubkey, COUNT(*) DESC, scope_id
             LIMIT $1",
        )
        .bind(i64::try_from(limit)?)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(AuthorDominantTopic {
                    author_pubkey: row.try_get("author_pubkey")?,
                    topic_id: row.try_get("topic_id")?,
                    entries: row.try_get("entries")?,
                })
            })
            .collect()
    }
}

/// (scope_id, author) → entry 数の集計（public_topic のみ）。
fn participation_counts(store: &MemoryIndexEntryStore) -> HashMap<(String, String), i64> {
    let mut counts: HashMap<(String, String), i64> = HashMap::new();
    for entry in store.entries_snapshot() {
        // private channel 由来は relation の観測に混ぜない（fail-closed。Pg 実装の WHERE と同一）。
        if entry.scope_kind != IndexScopeKind::PublicTopic {
            continue;
        }
        *counts
            .entry((entry.scope_id.clone(), entry.author_pubkey.clone()))
            .or_default() += 1;
    }
    counts
}

#[async_trait]
impl CoParticipationSource for MemoryIndexEntryStore {
    async fn list_co_participation_pairs(&self, limit: usize) -> Result<Vec<CoParticipationPair>> {
        // scope ごとの author → entries に組み替える。
        let mut by_scope: HashMap<String, Vec<(String, i64)>> = HashMap::new();
        for ((scope_id, author), entries) in participation_counts(self) {
            by_scope
                .entry(scope_id)
                .or_default()
                .push((author, entries));
        }

        let mut pairs: HashMap<(String, String), (i64, i64)> = HashMap::new();
        for authors in by_scope.values_mut() {
            authors.sort();
            for i in 0..authors.len() {
                for j in (i + 1)..authors.len() {
                    let (a, ea) = &authors[i];
                    let (b, eb) = &authors[j];
                    let slot = pairs.entry((a.clone(), b.clone())).or_default();
                    slot.0 += 1; // shared_topics
                    slot.1 += (*ea).min(*eb); // co_participation_events
                }
            }
        }

        let mut result: Vec<CoParticipationPair> = pairs
            .into_iter()
            .map(
                |((author_a, author_b), (shared_topics, events))| CoParticipationPair {
                    author_a,
                    author_b,
                    shared_topics,
                    co_participation_events: events,
                },
            )
            .collect();
        result.sort_by(|x, y| {
            y.shared_topics
                .cmp(&x.shared_topics)
                .then(y.co_participation_events.cmp(&x.co_participation_events))
                .then(x.author_a.cmp(&y.author_a))
                .then(x.author_b.cmp(&y.author_b))
        });
        result.truncate(limit);
        Ok(result)
    }

    async fn list_author_dominant_topics(&self, limit: usize) -> Result<Vec<AuthorDominantTopic>> {
        let mut by_author: HashMap<String, Vec<(String, i64)>> = HashMap::new();
        for ((scope_id, author), entries) in participation_counts(self) {
            by_author
                .entry(author)
                .or_default()
                .push((scope_id, entries));
        }
        let mut result: Vec<AuthorDominantTopic> = by_author
            .into_iter()
            .map(|(author_pubkey, mut topics)| {
                // 最多 entry、同数は scope_id 辞書順（Pg 実装の DISTINCT ON と同じ決定論）。
                topics.sort_by(|(sa, ea), (sb, eb)| eb.cmp(ea).then(sa.cmp(sb)));
                let (topic_id, entries) = topics
                    .into_iter()
                    .next()
                    .expect("non-empty by construction");
                AuthorDominantTopic {
                    author_pubkey,
                    topic_id,
                    entries,
                }
            })
            .collect();
        result.sort_by(|a, b| a.author_pubkey.cmp(&b.author_pubkey));
        result.truncate(limit);
        Ok(result)
    }
}
