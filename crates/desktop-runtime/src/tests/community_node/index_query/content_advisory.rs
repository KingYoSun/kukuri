use super::*;

// #1055: index 応答の content advisory（ADR 0028 §8.6）を client が採用する経路。
// 合成の有効化、issuer 照合、取得ゲートへの登録を固定する。

// #1055 / ADR 0046 §6.2 / AC-3: 合成が有効なとき、index を返した設定済み node が発行した
// blob 対象の advisory は `blob_media_payload` の取得ゲートへ登録され、応答の
// `content_advisories` はそのまま client へ渡る(第 2 のラベル源として保持する)。
#[tokio::test]
async fn community_node_index_registers_advisory_media_hashes_from_configured_issuer() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    runtime.set_content_advisory_synthesis_enabled(true);
    *state.response_advisories.lock().await =
        vec![blob_advisory(INDEX_NODE_ID, ADVISORY_BLOB_HASH)];

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");

    assert_eq!(search.entries[0].content_advisories.len(), 1);
    assert_eq!(
        search.entries[0].content_advisories[0].subject_id,
        ADVISORY_BLOB_HASH
    );
    assert!(
        runtime
            .app_service
            .is_advisory_media_hash(ADVISORY_BLOB_HASH)
            .await
    );
    assert_eq!(state.manifest_hits.load(Ordering::SeqCst), 1);

    server.abort();
}

// #1055 / AC-4 / TR-5: entry の `issuer_node_id` が index を返した node の manifest `node_id` と
// 一致しない advisory は採用せず、取得ゲートにも登録しない。
#[tokio::test]
async fn community_node_index_drops_advisories_from_mismatched_issuer() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    runtime.set_content_advisory_synthesis_enabled(true);
    let other_issuer = "3333333333333333333333333333333333333333333333333333333333333333";
    *state.response_advisories.lock().await = vec![blob_advisory(other_issuer, ADVISORY_BLOB_HASH)];

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");

    assert!(search.entries[0].content_advisories.is_empty());
    assert!(
        !runtime
            .app_service
            .is_advisory_media_hash(ADVISORY_BLOB_HASH)
            .await
    );

    server.abort();
}

// #1055 / AC-4 / TR-8: manifest を取得できない node の advisory は issuer を確認できないため
// 採用しない(fail-closed)。ゲート登録も行わない。
#[tokio::test]
async fn community_node_index_drops_advisories_when_manifest_is_unavailable() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    runtime.set_content_advisory_synthesis_enabled(true);
    *state.manifest_node_id.lock().await = None;
    *state.response_advisories.lock().await =
        vec![blob_advisory(INDEX_NODE_ID, ADVISORY_BLOB_HASH)];

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");

    assert!(search.entries[0].content_advisories.is_empty());
    assert!(
        !runtime
            .app_service
            .is_advisory_media_hash(ADVISORY_BLOB_HASH)
            .await
    );

    server.abort();
}

// #1055 / ADR 0046 §6.4 / TR-6: 合成が無効なら advisory を応答から落とし、manifest も引かず、
// 取得ゲートにも登録しない(#1056 で既定は有効になった。無効化経路の回帰を固定する)。
#[tokio::test]
async fn community_node_index_strips_advisories_when_synthesis_disabled() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    runtime.set_content_advisory_synthesis_enabled(false);
    *state.response_advisories.lock().await =
        vec![blob_advisory(INDEX_NODE_ID, ADVISORY_BLOB_HASH)];

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");

    assert!(search.entries[0].content_advisories.is_empty());
    assert!(
        !runtime
            .app_service
            .is_advisory_media_hash(ADVISORY_BLOB_HASH)
            .await
    );
    assert_eq!(state.manifest_hits.load(Ordering::SeqCst), 0);

    server.abort();
}

// #1056 / TR-7 / INV-2c: 利用者が採用を OFF にした node の advisory は「見つける」でも採用しない。
#[tokio::test]
async fn community_node_index_drops_advisories_from_node_with_adoption_disabled() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    runtime.community_node_config.lock().await.nodes[0].content_advisory_enabled = false;
    *state.response_advisories.lock().await =
        vec![blob_advisory(INDEX_NODE_ID, ADVISORY_BLOB_HASH)];

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");

    assert!(search.entries[0].content_advisories.is_empty());
    assert!(
        !runtime
            .app_service
            .is_advisory_media_hash(ADVISORY_BLOB_HASH)
            .await
    );
    assert_eq!(state.manifest_hits.load(Ordering::SeqCst), 0);

    server.abort();
}

// #1056: 合成は既定で有効(利用規約改訂・再同意と同時に有効化した)。
#[tokio::test]
async fn community_node_index_synthesizes_advisories_by_default() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    *state.response_advisories.lock().await =
        vec![blob_advisory(INDEX_NODE_ID, ADVISORY_BLOB_HASH)];

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");

    assert_eq!(search.entries[0].content_advisories.len(), 1);
    assert!(
        runtime
            .app_service
            .is_advisory_media_hash(ADVISORY_BLOB_HASH)
            .await
    );

    server.abort();
}
