# PR #133 review対応: `AuthStore.initialize` の auth/session guard 追加

作成日: 2026年02月22日

## 概要

- PR #133 のレビュー指摘（discussion `r2838338102`）に対応し、`AuthStore.initialize` の遅延起動タスクに auth/session guard を追加した。
- 自動ログイン後の `initializeNostr` 完了時点で、`isAuthenticated` と `currentUser.npub` が開始時セッションと一致しない場合は後続処理を中断する。
- 後続の `updateRelayStatus` / `bootstrapTopics` / `fetchAndApplyAvatar` についても、各タスク直前で同一セッション確認を行い、ログアウト・アカウント切替直後の実行を防止した。

## 変更ファイル

- `kukuri-tauri/src/stores/authStore.ts`
  - `expectedNpub` を基準に `hasActiveSession` / `runIfSessionActive` を導入。
  - `initializeNostr` 後にセッション不一致なら遅延タスク全体をスキップ。
  - 後続3タスクは `runIfSessionActive` 経由でガード付き実行に変更。
- `kukuri-tauri/src/tests/unit/stores/authStore.accounts.test.ts`
  - 自動ログイン中に `logout` された場合、遅延タスクが実行されないことを検証するテストを追加。
  - 自動ログイン中に `currentUser.npub` が別アカウントへ切り替わった場合、遅延タスクが実行されないことを検証するテストを追加。

## 検証

- `docker compose --project-name kukuri_tests -f docker-compose.test.yml run --rm ts-test pnpm vitest run src/tests/unit/stores/authStore.accounts.test.ts src/tests/unit/stores/authStore.test.ts`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）
