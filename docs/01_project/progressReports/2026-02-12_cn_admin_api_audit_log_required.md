# `cn-admin-api` 監査ログ必須化（`log_audit(...).await.ok()` 廃止）

作成日: 2026年02月12日

## 概要

`cn-admin-api` の管理操作で監査ログ書き込み失敗を黙殺していた実装（`cn_core::admin::log_audit(...).await.ok()`）を廃止し、失敗時に API 全体を `500 (AUDIT_LOG_ERROR)` で失敗させるよう統一した。

対象カテゴリ:

- `services`
- `policies`
- `subscriptions`
- `moderation`
- `trust`
- `access_control`
- `dsar`
- `reindex`
- `auth`

## 変更内容

1. 共通ヘルパー追加
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs` に `log_admin_audit(...) -> ApiResult<()>` を追加。
- `cn_core::admin::log_audit` の失敗を `AUDIT_LOG_ERROR` にマップ。

2. 管理 API の監査ログ呼び出し統一
- 対象モジュール内の `cn_core::admin::log_audit(...).await.ok()` を `crate::log_admin_audit(...).await?` に置換。
- 対象ファイル:
  - `kukuri-community-node/crates/cn-admin-api/src/services.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/policies.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/moderation.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/trust.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/access_control.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/dsar.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/reindex.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/auth.rs`

3. 契約テスト追加
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - テスト用監査失敗トリガー（`cn_admin.test_audit_failures` + trigger function）を追加。
  - `admin_mutations_fail_when_audit_log_write_fails` を追加し、各カテゴリ更新 API の `500 + AUDIT_LOG_ERROR` を検証。

## 検証結果

- `cargo fmt --all`（`kukuri-community-node`）: 成功
- `./scripts/test-docker.ps1 rust`: 成功
- `docker compose ... cargo test -p cn-admin-api -- --nocapture --test-threads=1`: 成功
  - 初回失敗原因:
    - DB 未起動 (`PoolTimedOut`)
    - `AGE` 拡張なし DB (`extension "age" is not available`)
  - 対応:
    - `kukuri-community-node-postgres` イメージで一時 DB を起動後、再実行成功。
- `gh act --workflows .github/workflows/test.yml --job format-check`: 成功
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: 成功（既知の `useRouter` 警告のみ）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: 成功

