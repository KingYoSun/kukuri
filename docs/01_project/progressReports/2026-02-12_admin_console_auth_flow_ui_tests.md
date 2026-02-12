# Community Nodes / Admin Console 認証導線 UI テスト追加（`LoginPage` / `App`）

最終更新日: 2026年02月12日

## 概要

- Admin Console の認証導線回帰を防ぐため、`LoginPage` と `App` の UI テストを追加した。
- 対象シナリオは以下:
  - セッションブートストラップ（`/v1/admin/auth/me`）
  - ログイン成功/失敗
  - ログアウト後のログイン画面遷移
- タスク管理上は `community_nodes_roadmap.md` の該当未完了項目を `[x]` に更新した。

## 変更ファイル

- `kukuri-community-node/apps/admin-console/src/App.test.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/LoginPage.test.tsx`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 実施内容

- `App.test.tsx` を追加:
  - 未認証セッション時に `api.me` が呼ばれ、ログイン画面（`Admin Login`）へ遷移することを検証
  - 有効セッション時に `Sign out` 後、ログイン画面へ戻ることを検証
- `LoginPage.test.tsx` を追加:
  - ログイン成功時に `api.login` 呼び出しと `password` 入力クリアを検証
  - ログイン失敗時にエラーメッセージ表示を検証

## 検証

- `./scripts/test-docker.ps1 ts`（成功）
- `docker compose -f docker-compose.test.yml run --rm ts-test bash -lc "cd /app/kukuri-community-node/apps/admin-console && corepack pnpm install --frozen-lockfile && pnpm test"`（成功）
  - `tmp/logs/docker-admin-console-auth-flow-ui-tests-20260212-210707.log`
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - `tmp/logs/gh-act-format-check-admin-console-auth-ui-tests-20260212-205443.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - `tmp/logs/gh-act-native-test-linux-admin-console-auth-ui-tests-20260212-205604.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - `tmp/logs/gh-act-community-node-tests-admin-console-auth-ui-tests-20260212-210302.log`
