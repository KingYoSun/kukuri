# 2026-02-12 Access Control 再配布結果（success/failed/pending + reason）実装

## 概要

- 対象: `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目
  - `admin_console.md` の Access Control 要件（epoch ローテ時の再配布「失敗/未配布」検知）に合わせ、`cn-core`/`cn-admin-api`/Admin Console で配布結果を記録・参照可能にする
  - Access Control 再配布結果テスト（`cn-core`/`cn-admin-api`/Admin Console）を補完する
- 結果: 上記2項目を実装し、roadmap のチェックを完了（`[x]`）へ更新した。

## 実装内容

### 1. `cn-core`（配布結果の記録）

- `kukuri-community-node/crates/cn-core/src/access_control.rs`
  - 再配布ステータスを `pending` / `success` / `failed` に統一。
  - `DistributionResult { recipient_pubkey, status, reason }` を追加。
  - rotate/revoke の返却構造 `RotateSummary` へ `distribution_results` を追加。
  - epoch ローテ時に対象 recipient を `pending` で先行登録し、再配布処理後に `success|failed` + `reason` へ更新するフローに変更。
  - テスト追加:
    - 不正ステータス正規化の拒否
    - 不正 pubkey を含むケースで `failed` 記録と `reason` が残ることの検証

### 2. DB マイグレーション

- `kukuri-community-node/migrations/20260211000000_access_control_distribution_results.sql`（新規）
  - `cn_user.key_envelope_distribution_results` を追加。
  - 主キー: `(topic_id, scope, epoch, recipient_pubkey)`
  - `status` は `pending|success|failed` の制約を付与。
  - `reason` / `created_at` / `updated_at` と検索用インデックスを追加。

### 3. `cn-admin-api`（参照 API とレスポンス拡張）

- `kukuri-community-node/crates/cn-admin-api/src/access_control.rs`
  - rotate/revoke レスポンスに `distribution_results` を追加。
  - `GET /v1/admin/access-control/distribution-results` を追加（topic/scope/pubkey/epoch/status/limit で検索可能）。
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
  - 新規 API のルーティングを追加。
- `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
  - `distribution-results` の path/schema を OpenAPI に反映。
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - rotate/revoke の `distribution_results` shape を契約テストで検証。
  - `distribution-results` 検索 API の契約テストを追加。

### 4. Admin Console（可視化）

- `kukuri-community-node/apps/admin-console/src/lib/types.ts`
  - `AccessControlDistributionResult` / `AccessControlDistributionResultRow` を追加。
  - rotate/revoke レスポンス型に `distribution_results` を追加。
- `kukuri-community-node/apps/admin-console/src/lib/api.ts`
  - `accessControlDistributionResults(...)` を追加。
- `kukuri-community-node/apps/admin-console/src/pages/AccessControlPage.tsx`
  - rotate/revoke 実行結果に配布サマリ（success/failed/pending 件数）を表示。
  - failed/pending の詳細（reason 含む）を表示。
  - 「Redistribution Results」検索フォーム + テーブルを追加。
- `kukuri-community-node/apps/admin-console/src/pages/AccessControlPage.test.tsx`
  - failed/pending の可視化と検索 API 呼び出しを検証する UI テストへ拡張。

### 5. 補助修正

- `kukuri-community-node/crates/cn-cli/tests/cli_access_control_node_key_integration.rs`
  - テスト cleanup に `key_envelope_distribution_results` の削除を追加（副作用隔離）。

## 検証結果

- `./scripts/test-docker.ps1 rust -NoBuild`（ログ上成功）
  - `tmp/logs/test-docker-rust-access-control-distribution-20260212-035553.log`
- `./scripts/test-docker.ps1 ts -NoBuild`（ログ上成功）
  - `tmp/logs/test-docker-ts-access-control-distribution-20260212-035613.log`
- `docker run --rm -v ${PWD}:/workspace -w /workspace/kukuri-community-node/apps/admin-console -e CI=true node:20-bookworm bash -lc "corepack enable && pnpm install --frozen-lockfile && pnpm test -- src/pages/AccessControlPage.test.tsx"`（成功）
  - `tmp/logs/docker-admin-console-access-control-page-20260212-040903.log`
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - `tmp/logs/gh-act-format-check-access-control-distribution-20260212-035709.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - `tmp/logs/gh-act-native-test-linux-access-control-distribution-20260212-035828.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - `tmp/logs/gh-act-community-node-tests-access-control-distribution-20260212-040450.log`

## タスク管理反映

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 再配布結果実装 + テスト補完の2項目を `[x]` に更新。
- `docs/01_project/activeContext/tasks/completed/2026-02-12.md`
  - 完了タスクと検証ログを追記。
