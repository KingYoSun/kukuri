# Issue #154 オフラインユーザー設定永続化とオンライン同期レポート

作成日: 2026年02月24日

## 概要

- 対象:
  - `kukuri-tauri/src/stores/privacySettingsStore.ts`
  - `kukuri-tauri/src/routes/settings.tsx`
  - `kukuri-tauri/src/lib/settings/privacySettingsSync.ts`
  - `kukuri-tauri/src/locales/ja.json`
  - `kukuri-tauri/src/locales/en.json`
  - `kukuri-tauri/src/locales/zh-CN.json`
  - `kukuri-tauri/src/tests/unit/stores/privacySettingsStore.test.ts`
  - `kukuri-tauri/src/tests/unit/routes/settings.test.tsx`
- プライバシー設定をローカル永続化し、オフライン時でも即時に設定変更を反映できるようにした。
- オンライン復帰時に未同期設定を自動送信し、サーバー状態と再同期できるようにした。

## 実装詳細

- `privacySettingsStore`
  - `ownerNpub` / `hasPendingSync` / `lastSyncedAt` / `lastSyncError` / `updatedAt` を追加。
  - `applyLocalChange` でオフライン・オンライン問わずローカル変更を保存。
  - `markSyncSuccess` / `markSyncFailure` で同期状態を管理。
  - 同一ユーザーで未同期がある場合は `hydrateFromUser` でローカル値を優先。

- `privacySettingsSync.ts`
  - `syncPrivacySettings` を追加し、`TauriApi.updatePrivacySettings` を実行。
  - Nostr メタデータ更新失敗時は `errorHandler` に記録して設定保存自体は継続。

- `settings.tsx`
  - トグル操作時にまず `applyLocalChange` でローカル保存。
  - オフライン時はトースト表示のみで終了、オンライン時は即時同期。
  - `online` イベントと初回マウント時に未同期設定があれば再同期を実行。
  - 同期中のトグル無効化と未同期状態メッセージ表示を追加。

- ローカライズ
  - `settings.privacy.pendingSync`
  - `settings.toast.privacySavedOffline`
  - 上記キーを `ja/en/zh-CN` へ追加。

- テスト
  - store テストで未同期状態遷移と同一ユーザー時ローカル優先を検証。
  - settings ルートテストでオフライン保存後に `online` イベントで同期されることを検証。

## 実行コマンド

- `docker compose -f docker-compose.test.yml build ts-test`
- `docker compose -f docker-compose.test.yml run --rm ts-test bash -lc "cd /app/kukuri-tauri && pnpm vitest run src/tests/unit/routes/settings.test.tsx src/tests/unit/stores/privacySettingsStore.test.ts"`
- `docker compose -f docker-compose.test.yml run --rm ts-test bash -lc "cd /app/kukuri-tauri && pnpm lint && pnpm format:check"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
