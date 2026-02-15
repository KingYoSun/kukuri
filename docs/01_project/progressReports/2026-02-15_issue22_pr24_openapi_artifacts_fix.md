# Issue #22 / PR #24 OpenAPI Artifacts Check 修正

作成日: 2026年02月15日

## 概要

- 対象: PR #24（`feat/issue22-pending-limit-contract-tests`）の CI 失敗 `OpenAPI Artifacts Check`
- 原因: `kukuri-community-node/apps/admin-console/openapi/user-api.json` の末尾改行が、`cn-cli openapi export` の生成結果（末尾改行なし）と不一致
- 対応方針: API 契約本体は変更せず、生成物の改行状態のみを最小修正

## 調査結果

- 失敗ジョブ: `https://github.com/KingYoSun/kukuri/actions/runs/22036921575/job/63671295920`
- `gh` 経由で job ログを取得し、`Verify generated artifacts are up-to-date` の `git diff` 出力を確認。
- 差分は `user-api.json` の最終行のみで、内容変更はなく `\ No newline at end of file` の有無だけが不一致だった。

## 実施内容

- CI と同一コマンドで OpenAPI 生成と Admin Console クライアント生成を再実行し、差分をローカル再現。
- `user-api.json` を生成結果に合わせて末尾改行なしへ統一。
- そのほかの API 生成物（`admin-api.json` / `src/generated/admin-api.ts`）に追加変更がないことを確認。

## 変更ファイル

- `kukuri-community-node/apps/admin-console/openapi/user-api.json`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`

## 検証

- `cd kukuri-community-node/apps/admin-console && CI=true pnpm install --frozen-lockfile`
  - 成功
- `cd kukuri-community-node && cargo run --locked -p cn-cli -- openapi export --service user-api --output apps/admin-console/openapi/user-api.json --pretty && cargo run --locked -p cn-cli -- openapi export --service admin-api --output apps/admin-console/openapi/admin-api.json --pretty`
  - 成功
- `cd kukuri-community-node/apps/admin-console && pnpm generate:api`
  - 成功
- `cd kukuri-community-node && cargo test -p cn-user-api openapi_contract_ -- --nocapture`
  - 成功（1 passed）
- `git diff --exit-code -- kukuri-community-node/apps/admin-console/openapi/*.json kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`
  - 修正コミット後に成功（差分なし）

## 次アクション

- Issue #22 の残タスク（node-level 同時取込 topic 上限実装 / 回帰テスト）を 1タスク=1PR で継続する。
