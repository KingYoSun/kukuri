# Issue #27 / PR #29 fix loop（post_search_documents multi-topic key）

最終更新日: 2026年02月16日

## 背景

PR #29 の review（`discussion_r2810577528`）で、`cn_search.post_search_documents` が `post_id` 単独キーのため、同一 event が複数 topic に属する場合に PG 側ドキュメントが上書きされる問題が指摘された。

## 原因

- schema が `post_id PRIMARY KEY` だったため、topic 別行を保持できなかった。
- `cn-index` の upsert が `ON CONFLICT (post_id)` で実装されており、後続 topic の書き込みで前の topic 行を更新していた。
- `/v1/search` は `topic_id` でフィルタするため、PG backend では最後に上書きされた topic 以外で欠落が起きうる状態だった。

## 実施内容

1. schema 修正
- 追加: `kukuri-community-node/migrations/20260216040000_m8_post_search_documents_topic_key.sql`
- `post_search_documents` の主キーを `(post_id, topic_id)` に変更。

2. write path 修正（`cn-index`）
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- upsert を `ON CONFLICT (post_id, topic_id)` に変更。
- delete/expiry/stale-mark を `topic_id` 条件付き更新に変更し、topic 単位の状態更新へ統一。

3. 回帰テスト追加
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
  - `outbox_dual_write_preserves_post_search_documents_per_topic` を追加。
  - 同一 event を 2 topic に upsert したとき PG 側に 2 行残ることを検証。
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - `search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id` を追加。
  - 同一 `post_id` を topic A/B に投入し、`/v1/search?topic=...` が topic ごとに正しい1件を返すことを検証。

## 検証

- `cd kukuri-community-node && cargo fmt`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml down -v`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network ... kukuri-test-runner ... "cargo clean -p cn-core; cargo test -p cn-index outbox_dual_write_updates_meili_and_post_search_documents -- --nocapture; cargo test -p cn-index outbox_dual_write_preserves_post_search_documents_per_topic -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_switch_normalization_and_version_filter -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id -- --nocapture"`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network ... kukuri-test-runner ... "cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `cd kukuri-tauri/src-tauri && cargo test`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr29-fix-loop.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr29-fix-loop.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr29-fix-loop.log`（pass）

## 補足

- 初回のターゲットテストで `there is no unique or exclusion constraint matching the ON CONFLICT specification` が発生したため、`cargo clean -p cn-core` 後に再実行し解消。`sqlx::migrate!` の埋め込み更新を確実に反映させた。
