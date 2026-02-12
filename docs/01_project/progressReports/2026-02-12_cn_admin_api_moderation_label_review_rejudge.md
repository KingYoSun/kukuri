# `cn-admin-api` + Admin Console: Moderation ラベル human review / 再判定 / 無効化

作成日: 2026年02月12日

## 概要

`services_moderation.md` の運用要件（human review / 再判定 / 無効化）を満たすため、ラベルのレビュー状態管理を DB・Admin API・User API・Admin Console まで一連で実装した。  
併せて review/rejudge 操作の監査ログを append-only かつ必須（失敗時 API 失敗）で統一した。

## 変更内容

1. DB / migration
- `kukuri-community-node/migrations/20260212010000_m4_label_review_workflow.sql` を追加。
- `cn_moderation.labels` に `review_status` / `review_reason` / `reviewed_by` / `reviewed_at` を追加。
- `review_status` を `active|disabled` 制約で管理。
- `review_status='active'` のみ一意制約を持つ index へ置換。

2. `cn-admin-api`
- `kukuri-community-node/crates/cn-admin-api/src/moderation.rs`
  - `list_labels` に `review_status` フィルタを追加。
  - `create_label` の手動作成時に review 情報を初期化。
  - `POST /v1/admin/moderation/labels/{label_id}/review` を追加。
  - `POST /v1/admin/moderation/labels/{label_id}/rejudge` を追加。
  - review/rejudge を `log_admin_audit_tx` で監査ログ必須化。
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
  - moderation review/rejudge ルートを追加。
- `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
  - review/rejudge path と schema、`review_status` query を追加。
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - review/rejudge 成功系と監査失敗系の契約テストを追加。

3. `cn-moderation` / `cn-user-api`
- `kukuri-community-node/crates/cn-moderation/src/lib.rs`
  - ラベル重複判定/存在判定を `review_status='active'` 前提へ更新。
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - labels 一覧を `review_status='active'` のみに制限。
  - disabled ラベル非表示の契約テスト補助を追加。

4. Admin Console
- `kukuri-community-node/apps/admin-console/src/pages/ModerationPage.tsx`
  - review status フィルタ、review 情報表示、operator note 入力、Enable/Disable/Rejudge 操作を追加。
- `kukuri-community-node/apps/admin-console/src/lib/api.ts`
  - `reviewModerationLabel` / `rejudgeModerationLabel` を追加。
- `kukuri-community-node/apps/admin-console/src/lib/types.ts`
  - `ModerationLabel` に review 系フィールドを追加。
- `kukuri-community-node/apps/admin-console/src/pages/ModerationPage.test.tsx`
  - 無効化と再判定トリガの UI 回帰テストを追加。

## 検証結果

- `./scripts/test-docker.ps1 rust -NoBuild` : 成功
- `docker compose -f docker-compose.test.yml run --rm test-runner bash -lc "source /usr/local/cargo/env && cd /app/kukuri-community-node && cargo test --workspace --all-features && cargo build --release -p cn-cli"` : 成功
- `docker run --rm -v ${PWD}:/workspace -w /workspace/kukuri-community-node/apps/admin-console -e CI=true node:20-bookworm bash -lc "corepack enable && pnpm install --frozen-lockfile && pnpm vitest run src/pages/ModerationPage.test.tsx"` : 成功
- `gh act --workflows .github/workflows/test.yml --job format-check` : 成功
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` : 成功（既知の `useRouter` warning のみ）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests` : 最終成功
  - 1回目: `15432` ポート競合で失敗
  - 2回目: `cn-admin-api` 契約テストの `E0382` で失敗
  - 3回目: `contract_tests.rs` の `app.clone()` 修正後に成功
