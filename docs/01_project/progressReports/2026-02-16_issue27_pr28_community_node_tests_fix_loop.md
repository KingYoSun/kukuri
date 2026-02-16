# Issue #27 / PR #28 Community Node Tests fix loop（cn-admin-api 契約テスト競合解消）

最終更新日: 2026年02月16日

## 概要

PR #28 の Community Node Tests 失敗（Run `22048263032` / Job `63700968967`）を調査し、`cn-admin-api` 契約テストの並列競合を最小差分で解消した。
Issue #27 PR-01 の基盤変更に影響を広げず、テスト側の直列化のみを実施した。

## 失敗原因

1. 失敗テストは `contract_tests::subscription_request_approve_rejects_when_node_topic_limit_reached`。
2. `service='relay'` の `cn_admin.service_configs` と `cn_admin.node_subscriptions` を共有する複数契約テストが並列実行され、期待値と実測値が競合した。
3. 具体的には以下が同時に発生しうる状態だった。
- `max_concurrent_topics` が別テストで上書きされ、`429` 期待が `200` になる。
- `transactional_admin_mutations_rollback_when_audit_log_write_fails` などで `500` 期待が `429` になる。

## 実施内容

1. `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs` に `relay_subscription_approval_test_lock`（`OnceLock<tokio::sync::Mutex<()>>`）を追加。
2. 共有状態に依存する以下 5 テストへ同一ロックを導入して直列化。
- `transactional_admin_mutations_rollback_when_audit_log_write_fails`
- `transactional_admin_mutations_rollback_when_commit_fails`
- `subscription_requests_and_node_subscriptions_contract_success`
- `subscription_request_approve_rejects_when_node_topic_limit_reached`
- `subscription_request_approve_rejects_when_node_topic_limit_already_exceeded`

## 検証コマンド

- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
- `export BUILDX_CONFIG=/tmp/buildx; docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc 'set -euo pipefail; source /usr/local/cargo/env; for i in $(seq 1 8); do cargo test -p cn-admin-api --lib -- --test-threads=8; done'`
- `cd kukuri-tauri/src-tauri && cargo test`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc 'set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli'`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

## 検証結果

1. `cn-admin-api` 反復実行（8連続）で対象失敗の再発なし。
2. `kukuri-tauri/src-tauri` の `cargo test` は pass。
3. `gh act` の `format-check` / `native-test-linux` / `community-node-tests` はすべて pass。
4. `community-node-tests` では `cn-admin-api` 43件（対象の承認系契約テスト含む）を含めて全クレート通過し、PR #28 の失敗条件は解消。

## 影響範囲

- 変更は `cn-admin-api` 契約テストに限定。
- アプリ実装・DB スキーマ・API 契約の本体挙動は変更なし。
