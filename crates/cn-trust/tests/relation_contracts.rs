//! relation（pairwise cluster proximity）の contract テスト（ADR 0026 §6.1, Issue #415）。
//!
//! 契約本体は共有スイート（`relation_testing`）にあり、in-memory 実装へ課す。
//! ArcadeDB 実装（`cn-indexer`）も同じスイートを実行する（backend 間の drift 防止）。

use kukuri_cn_trust::relation_testing::{
    assert_cluster_roundtrip, assert_neighbors_ranked, assert_pairwise_cluster_proximity,
    assert_proximity_is_explainable, assert_symmetric_lookup,
};
use kukuri_cn_trust::{
    EdgeFeatures, FEATURE_FOLLOW_PROJECTION, FEATURE_SHARED_TOPICS, MemoryRelationStore,
    proximity_from_features,
};

#[tokio::test]
async fn relation_is_pairwise_cluster_proximity() {
    let store = MemoryRelationStore::new();
    assert_pairwise_cluster_proximity(&store, "prox")
        .await
        .unwrap();
}

#[tokio::test]
async fn relation_read_is_explainable() {
    let store = MemoryRelationStore::new();
    assert_proximity_is_explainable(&store, "explain")
        .await
        .unwrap();
}

#[tokio::test]
async fn relation_lookup_is_symmetric() {
    let store = MemoryRelationStore::new();
    assert_symmetric_lookup(&store, "sym").await.unwrap();
}

#[tokio::test]
async fn relation_neighbors_are_ranked_and_limited() {
    let store = MemoryRelationStore::new();
    assert_neighbors_ranked(&store, "nb").await.unwrap();
}

#[tokio::test]
async fn relation_cluster_assignment_round_trips() {
    let store = MemoryRelationStore::new();
    assert_cluster_roundtrip(&store, "cl").await.unwrap();
}

#[test]
fn follow_projection_feature_is_a_seam_that_composes() {
    // follow projection の producer は未実装（プラン Assumption 4）だが、feature key は
    // 予約済みで、値が入り次第 proximity に自然合流する。
    let without = proximity_from_features(&EdgeFeatures::new().with(FEATURE_SHARED_TOPICS, 2.0));
    let with_follow = proximity_from_features(
        &EdgeFeatures::new()
            .with(FEATURE_SHARED_TOPICS, 2.0)
            .with(FEATURE_FOLLOW_PROJECTION, 1.0),
    );
    assert_ne!(without.score, with_follow.score);
    assert!(
        with_follow
            .basis
            .iter()
            .any(|e| e.feature == FEATURE_FOLLOW_PROJECTION)
    );
}
