# Issue #22 Task4: node-level 上限回帰テスト追加

作成日: 2026年02月15日

## 概要

- 対象: `cn-admin-api` / `cn-relay`
- 目的: node-level 同時取込 topic 数上限の拒否・上限制御を回帰テストで固定する。
- スコープ: テスト追加のみ（実装変更は行わない）。

## 実施内容

- `cn-admin-api` (`contract_tests.rs`)
  - 既存の「上限到達（current == limit）」ケースに加えて、「既に超過（current > limit）」でも承認拒否される契約テストを追加。
  - 期待契約:
    - `status`: `429`
    - `code`: `NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED`
    - `details.metric`: `node_subscriptions.enabled_topics`
    - `details.scope`: `node`
    - `details.current > details.limit`
  - 拒否後に request が `pending` のまま、`topic_subscriptions` / `node_subscriptions` へ副作用がないことを検証。

- `cn-relay` (`integration_tests.rs`)
  - `load_enabled_topics(..., 1)` を使い、enabled topic を増やしても選択件数が 1 を超えて増えないことを統合テストで検証。
  - 上限超過時は件数を固定したまま、最新 topic への入替のみ発生し得ることを確認。

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`
- `docs/01_project/progressReports/2026-02-15_issue22_node_level_limit_regression_tests.md`

## 検証

- Community Node（Docker 経路）
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
  - `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
  - `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -e RUST_TEST_THREADS=1 -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-admin-api subscription_request_approve_rejects_when_node_topic_limit_ -- --nocapture --test-threads=1; cargo test -p cn-relay node_subscription_limit_prevents_desired_topic_growth_when_over_limit -- --nocapture --test-threads=1"`
  - 結果: `success`（`cn-admin-api`: 2 tests passed, `cn-relay`: 1 test passed）

- AGENTS 必須 `gh act` ジョブ
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check`
    - 結果: `success`
    - ログ: `tmp/logs/gh-act-format-check-issue22-node-level-limit-regression-tests.log`
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux`
    - 結果: `success`
    - ログ: `tmp/logs/gh-act-native-test-linux-issue22-node-level-limit-regression-tests.log`
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests`
    - 結果: `failed`（既知不安定）
    - 1回目失敗: `transactional_admin_mutations_rollback_when_audit_log_write_fails` の assertion mismatch（`left: 200, right: 500`）
    - 2回目失敗: `trigger "test_audit_failures_trigger" already exists`
    - ログ: `tmp/logs/gh-act-community-node-tests-issue22-node-level-limit-regression-tests.log`, `tmp/logs/gh-act-community-node-tests-issue22-node-level-limit-regression-tests-rerun.log`

## 備考

- `community-node-tests` の失敗は今回変更箇所ではなく、`cn-admin-api` の監査失敗系契約テストで再現する既知の不安定事象。
