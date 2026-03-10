# `cn-admin-api` 更新系の commit 失敗伝播 + ロールバック保証

作成日: 2026年02月12日

## 概要

`cn-admin-api` の管理更新系で残っていた `tx.commit().await.ok()` を廃止し、commit 失敗を `500 (DB_ERROR)` として返すように統一した。あわせて、更新と監査ログ挿入を同一トランザクションにまとめ、監査ログ書き込み失敗時にも副作用が残らないことを契約テストで固定した。

対象更新アクション:

- `service_config.update`
- `policy.make_current`
- `subscription_request.approve`
- `plan.create`
- `plan.update`

## 変更内容

1. トランザクション内監査ログヘルパー追加
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `log_admin_audit_tx(...) -> ApiResult<()>` を追加し、監査ログ INSERT を `tx` 内で実行可能にした。
- 失敗時は `AUDIT_LOG_ERROR` で返却。

2. 管理更新系ハンドラのトランザクション整理
- `kukuri-community-node/crates/cn-admin-api/src/services.rs`
  - `update_service_config`: 監査ログを `tx` 内に移動し、`commit` と `pg_notify` 失敗を `DB_ERROR` として返却。
- `kukuri-community-node/crates/cn-admin-api/src/policies.rs`
  - `make_current_policy`: 監査ログを `tx` 内に移動し、`commit` 失敗を返却。
- `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs`
  - `approve_subscription_request` / `create_plan` / `update_plan`: 監査ログを `tx` 内に移動し、`commit` 失敗を返却。

3. 契約テスト追加
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - `transactional_admin_mutations_rollback_when_audit_log_write_fails`
    - 監査ログ失敗時に `500 + AUDIT_LOG_ERROR` とロールバック（副作用なし）を検証。
  - `transactional_admin_mutations_rollback_when_commit_fails`
    - commit 失敗時に `500 + DB_ERROR` とロールバック（副作用なし）を検証。
  - commit 失敗再現用に `cn_admin.test_commit_failures` + deferred constraint trigger を追加。

## 検証結果

- `cargo fmt --all`（`kukuri-community-node`）: 成功
- `./scripts/test-docker.ps1 rust`: 成功
- `gh act --workflows .github/workflows/test.yml --job format-check`: 成功
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: 成功（既知の `useRouter` 警告のみ）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: 成功

