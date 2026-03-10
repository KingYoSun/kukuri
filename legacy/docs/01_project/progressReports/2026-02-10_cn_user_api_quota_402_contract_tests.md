# 2026-02-10 `cn-user-api` Billing/quota 402 契約テスト追加

## 概要

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未完了項目
  - `cn-user-api`: Billing/quota の 402 契約テスト追加（search/trending/report/topic-subscription、`QUOTA_EXCEEDED` details/reset_at、同一 `request_id` 冪等）
  - を実装完了した。

## 実装内容

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - `api_contract_tests` に以下の 402 契約テストを追加:
    - `search_quota_contract_payment_required_with_request_id_idempotent`
    - `trending_quota_contract_payment_required_with_request_id_idempotent`
    - `submit_report_quota_contract_payment_required_with_request_id_idempotent`
    - `topic_subscription_quota_contract_payment_required`
  - `x-request-id` 付き再送時の冪等性を DB（`cn_user.usage_events`）件数で検証。
  - 402 エラーペイロードの `code=QUOTA_EXCEEDED` と `details` を共通アサーションで検証。

- `kukuri-community-node/crates/cn-user-api/src/billing.rs`
  - `consume_quota` の `request_id` 再送（既存 outcome = `rejected`）分岐で、402 応答に `details` と `reset_at` を含めるよう補完。
  - 既存の通常 quota 超過分岐と同一形式になるよう `quota_exceeded_error` を共通化。

- タスク管理
  - `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当項目を `[x]` へ更新。

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker run --rm --network host -e DATABASE_URL=postgres://cn:cn_password@localhost:15432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "/usr/local/cargo/bin/cargo test --package cn-user-api -- --nocapture"`（成功: 32 passed）
- `docker run --rm -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "/usr/local/cargo/bin/cargo fmt --all -- --check"`（成功）
- `docker run --rm -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "/usr/local/cargo/bin/cargo clippy --package cn-user-api --all-features -- -D warnings"`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - `tmp/logs/gh-act-format-check-cn-user-api-quota-20260211-013431.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - `tmp/logs/gh-act-native-test-linux-cn-user-api-quota-20260211-013545.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - `tmp/logs/gh-act-community-node-tests-cn-user-api-quota-20260211-014209.log`
