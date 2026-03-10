# PR #160 review discussion r2844317596 対応レポート

作成日: 2026年02月24日

## 概要

- 対象:
  - `kukuri-tauri/src/hooks/usePrivacySettingsAutoSync.ts`
  - `kukuri-tauri/src/App.tsx`
  - `kukuri-tauri/src/routes/settings.tsx`
  - `kukuri-tauri/src/tests/unit/hooks/usePrivacySettingsAutoSync.test.tsx`
  - `kukuri-tauri/src/tests/unit/app/App.test.tsx`
  - `kukuri-tauri/src/tests/unit/routes/settings.test.tsx`
- PR #160 の review discussion `r2844317596`（P2）に対応し、online 復帰時の pending 同期トリガーを `SettingsPage` からアプリ全体へ移設した。
- 設定画面にいない状態でネットワーク復帰しても、未同期プライバシー設定が自動送信されるようにした。

## 実装詳細

- `usePrivacySettingsAutoSync`（新規）
  - `online` イベントで pending 同期を実行。
  - マウント時に pending 同期を試行。
  - `currentUser` の復元（`npub` 変化）時に online かつ pending なら同期を実行。
  - `inFlightRef` により多重同期を防止。
  - 同期成功時は `markSyncSuccess` と `updateUser` を実行。
  - 同期失敗時は `markSyncFailure` と `errorHandler.log` を実行。

- `App.tsx`
  - アプリ起動時に `usePrivacySettingsAutoSync` を常駐初期化。

- `settings.tsx`
  - `online` リスナーと pending 同期処理を削除。
  - オフライン時ローカル保存・オンライン時即時同期という UI 層責務に限定。

- テスト
  - `usePrivacySettingsAutoSync.test.tsx` を追加し、online 復帰・ユーザー復元の両経路を検証。
  - `settings.test.tsx` は「オフライン時は未同期状態維持」へ期待を更新。
  - `App.test.tsx` は自動同期フック初期化を検証。

## 実行コマンド

- `docker compose -f docker-compose.test.yml run --rm --build ts-test pnpm vitest run src/tests/unit/routes/settings.test.tsx src/tests/unit/app/App.test.tsx src/tests/unit/hooks/usePrivacySettingsAutoSync.test.tsx`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
