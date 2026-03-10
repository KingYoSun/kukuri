# Community Nodes / cn-admin-api 契約テスト拡充（services〜trust）

最終更新日: 2026年02月07日

## 概要

`cn-admin-api` の契約テストを拡充し、`services` / `policies` / `moderation` / `subscription-requests` / `node-subscriptions` / `plans` / `subscriptions` / `usage` / `audit-logs` / `trust` の主要エンドポイント成功系とレスポンス shape 互換を検証できるようにした。

## 実施内容

- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - 追加テスト:
    - `services_contract_success_and_shape`
    - `policies_contract_lifecycle_success`
    - `moderation_contract_success_and_shape`
    - `subscription_requests_and_node_subscriptions_contract_success`
    - `plans_subscriptions_usage_contract_success`
    - `audit_logs_contract_success_and_shape`
    - `trust_contract_success_and_shape`
  - 上記で対象10カテゴリの主要エンドポイントを成功系で呼び出し、主要フィールドの型/存在をアサートする契約テストを追加。
  - 契約テスト向けに `put_json` と DB seed ヘルパー（`insert_service_health` / `insert_report` / `insert_subscription_request` / `insert_usage_counter` / `insert_audit_log`）を追加。

- `kukuri-community-node/crates/cn-admin-api/src/services.rs`
  - `update_service_config` 内の通知クエリを `NOTIFY cn_admin_config, $1` から `SELECT pg_notify('cn_admin_config', $1)` に修正。
  - 理由: 前者は失敗時にトランザクションが abort され、サービス設定更新の成功系契約テストが 500 になるため。

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 未実装/不足事項（2026年02月07日 監査追記）の
    - `` `cn-admin-api` 契約テストを拡充し、... ``
    を `[x]` に更新。

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "${PWD}/kukuri-community-node:/app" -w /app rust:1.88-bookworm bash -c "cargo test -p cn-admin-api --tests -- --nocapture"`（成功: 16 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功。ログ: `tmp/logs/gh-act-format-check-20260207-222337.log`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。ログ: `tmp/logs/gh-act-native-test-linux-20260207-222451.log`）
