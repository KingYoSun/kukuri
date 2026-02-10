# Admin Console Privacy/Data DSAR 運用ビュー追加

作成日: 2026年02月10日
対象: `kukuri-community-node`

## 概要
Admin Console `Privacy / Data` に DSAR 運用ビューを追加し、削除/エクスポート要求ジョブの状態監視（`queued|running|completed|failed`）、再実行・中止操作、監査ログ連携を実装した。合わせて Admin API 契約テストとフロント UI テストを追加した。

## 実装内容
- Admin API (`cn-admin-api`)
  - 新規: `GET /v1/admin/personal-data-jobs`
  - 新規: `POST /v1/admin/personal-data-jobs/{job_type}/{job_id}/retry`
  - 新規: `POST /v1/admin/personal-data-jobs/{job_type}/{job_id}/cancel`
  - 監査ログ追加: `dsar.job.retry` / `dsar.job.cancel`
  - OpenAPI へ DSAR path/schema (`DsarJobRow`) を追加
- Admin Console (`apps/admin-console`)
  - `PrivacyDataPage` に `DSAR Operations` セクションを追加
  - ジョブ一覧（Type / Request ID / Requester / Status / Created / Completed / Error / Actions）を表示
  - 行単位アクション: `Retry` / `Cancel`
  - `Recent Privacy/Data Audits` へ DSAR 操作ログを統合
- テスト
  - `cn-admin-api`: `dsar_jobs_contract_list_retry_cancel_and_audit_success` を追加
  - `cn-admin-api`: `openapi_contract_contains_admin_paths` に DSAR path/schema 検証を追加
  - `admin-console`: `PrivacyDataPage.test.tsx` に DSAR UI 操作検証を追加

## 検証結果
- pass: `cargo test -p cn-admin-api --tests -- --nocapture`（25 passed）
- pass: `cargo fmt --all --check`
- pass: `pnpm vitest run src/pages/PrivacyDataPage.test.tsx`（1 passed）
- pass: `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-format-check-dsar-20260210-110131.log`
- pass: `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-native-test-linux-dsar-20260210-110255.log`
- pass: `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-community-node-tests-dsar-20260210-111112.log`
- 既存失敗（本対応外）: `pnpm typecheck`
  - `AuditPage.test.tsx` / `PoliciesPage.test.tsx` / `SubscriptionsPage.test.tsx` / `TrustPage.test.tsx` の既知型不整合
