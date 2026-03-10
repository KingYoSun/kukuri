# 2026年02月13日 `cn-user-api` `delete_topic_subscription` commit 失敗伝播修正

## 概要

- `cn-user-api` の `delete_topic_subscription` が `tx.commit().await.ok()` で commit 失敗を握りつぶしていたため、`topic_subscription_design.md` の user-level subscription 停止フロー要件に沿って `5xx(DB_ERROR)` を返す実装へ修正した。
- commit 失敗時の回帰を防ぐため、契約テストを追加して `status=ended` を誤返却しないことと、`topic_subscriptions` / `node_subscriptions` の副作用ロールバックを固定した。

## 実装内容

1. `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `delete_topic_subscription` の commit 処理を `map_err(...)?` に変更し、commit 失敗時に `ApiError(INTERNAL_SERVER_ERROR, DB_ERROR, ...)` を返すよう統一。

2. `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`（契約テスト）
- commit 失敗を再現するため、`cn_user.topic_subscriptions` に対する deferrable constraint trigger のインストール/削除ヘルパーを追加。
- `topic_subscription_delete_commit_failure_returns_db_error_and_rolls_back` を追加し、以下を検証:
  - レスポンスが `500` かつ `code=DB_ERROR`
  - レスポンスに `status` が含まれない（`status=ended` を返さない）
  - `cn_user.topic_subscriptions` が `active` / `ended_at IS NULL` のまま
  - `cn_admin.node_subscriptions` が `ref_count=1` / `enabled=true` のまま

## タスク更新

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - `delete_topic_subscription` の commit 失敗伝播タスクを `[x]` に更新
  - 異常系回帰テスト（`usage_events` / `usage_counters_daily` / `topic_subscriptions` / `node_subscriptions`）補完タスクを `[x]` に更新

## 検証

- `./scripts/test-docker.ps1 rust`（ログ上成功、終了コード `-1` は既知）
  - ログ: `tmp/logs/test-docker-rust-cn-user-api-delete-20260213-122925.log`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace kukuri-test-runner bash -lc "source /usr/local/cargo/env && cd /workspace/kukuri-community-node && cargo test -p cn-user-api topic_subscription_delete_commit_failure_returns_db_error_and_rolls_back -- --nocapture --test-threads=1"`（成功）
  - ログ: `tmp/logs/cn-user-api-topic-subscription-commit-failure-20260213-123537.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-user-api-delete-20260213-123729.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知の `useRouter` 警告のみ）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-user-api-delete-20260213-123853.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-user-api-delete-20260213-124520.log`
