//! relation 解析 worker（batch・非常駐, ADR 0026 §6.1, #415）。
//!
//! index 真実源の co-participation 集計（`CoParticipationSource`、public topic のみ =
//! private channel 由来は観測に入らない）を feature に点数化し、relation graph
//! （`RelationStore`）へ upsert する。あわせて cluster 帰属の初期実装
//! （dominant shared topic = 最多 entry の topic への帰属。プラン Assumption 6）を書き込む。
//!
//! - **非常駐**: `CommunityLocalTrust` capability が `Availability::Planned` の間は
//!   `cn-cli relation analyze` からの手動 batch 実行のみ（capability 昇格判断に紐づけて
//!   常駐化を判断する。#404 の ingest loop と同じ扱い）。
//! - **canonical 非改変**: 書き込み先は relation graph（node-local derived overlay）のみ。
//!   social graph / user identity / index 真実源への書き込み口を持たない
//!   （`relation_does_not_mutate_social_graph_canonical`）。
//! - clustering の高度化（community detection / 重み実測）は ADR 0026 §4 の後続 Issue。

use anyhow::Result;

use kukuri_cn_core::CoParticipationSource;
use kukuri_cn_trust::{
    ClusterRef, EdgeFeatures, FEATURE_CO_PARTICIPATION_EVENTS, FEATURE_SHARED_TOPICS, RelationStore,
};

/// 1 回の解析で読む集計行の上限（有界化）。
pub const DEFAULT_ANALYSIS_LIMIT: usize = 10_000;

/// dominant topic 由来の cluster 名。
pub fn topic_cluster(topic_id: &str) -> ClusterRef {
    ClusterRef(format!("topic:{topic_id}"))
}

/// 解析結果の要約（CLI 出力・テスト検証用）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationAnalysisReport {
    /// upsert した pairwise edge 数。
    pub edges_upserted: usize,
    /// 割り当てた cluster 帰属数。
    pub clusters_assigned: usize,
}

/// co-participation 集計 → relation graph の batch 解析。
///
/// 冪等: 同じ観測に対して何度実行しても同じ graph 状態に収束する（edge / cluster とも upsert）。
pub async fn analyze_relations(
    source: &dyn CoParticipationSource,
    store: &dyn RelationStore,
    limit: usize,
) -> Result<RelationAnalysisReport> {
    let mut report = RelationAnalysisReport::default();

    for pair in source.list_co_participation_pairs(limit).await? {
        let features = EdgeFeatures::new()
            .with(FEATURE_SHARED_TOPICS, pair.shared_topics as f64)
            .with(
                FEATURE_CO_PARTICIPATION_EVENTS,
                pair.co_participation_events as f64,
            );
        store
            .upsert_edge(&pair.author_a, &pair.author_b, &features)
            .await?;
        report.edges_upserted += 1;
    }

    for dominant in source.list_author_dominant_topics(limit).await? {
        store
            .set_cluster(&dominant.author_pubkey, &topic_cluster(&dominant.topic_id))
            .await?;
        report.clusters_assigned += 1;
    }

    Ok(report)
}
