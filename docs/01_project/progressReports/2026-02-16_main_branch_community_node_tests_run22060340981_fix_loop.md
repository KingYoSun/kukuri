# Main branch Community Node Tests fix loop（Run `22060340981`）

最終更新日: 2026年02月16日

## 概要

GitHub Actions Run `22060340981` の `Community Node Tests`（Job `63738742754`）失敗を triage し、`cn-user-api` 契約テストの不安定失敗（`428` vs `402`）を最小修正で安定化した。

## 原因

- 失敗テスト: `subscriptions::api_contract_tests::auth_consent_quota_metrics_regression_counters_increment`
- 失敗箇所: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`（CIログでは `assert_eq!(status, StatusCode::PAYMENT_REQUIRED)`）
- CIログの実値: `left: 428` / `right: 402`
- テスト並列実行中に `current policy` が増減するタイミング競合で、2回目の購読リクエストが一時的に `CONSENT_REQUIRED(428)` へ戻ることがある。

## 実装内容

1. 契約テストの2回目リクエストを consent retry 付きに変更
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `post_json(...)` を `post_json_with_consent_retry(...)` に差し替え、競合時に `ensure_consents` 後リトライするようにした。
- プロダクトコードは変更せず、テスト安定化のみを実施。

## 検証

- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml down --remove-orphans --volumes`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache`（pass）

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_main_branch_community_node_tests_run22060340981_fix_loop.md`
