# Issue #128 設定永続化（ダークモード/言語）レポート

作成日: 2026年02月22日

## 概要

- 対象:
  - `kukuri-tauri/src/stores/uiStore.ts`
  - `kukuri-tauri/src/stores/config/persist.ts`
  - `kukuri-tauri/src/i18n.ts`
  - `kukuri-tauri/src/routes/settings.tsx`
  - `kukuri-tauri/src/testing/registerE2EBridge.ts`
  - `kukuri-tauri/src/tests/unit/stores/uiStore.test.ts`
  - `kukuri-tauri/src/tests/unit/i18n/localePersistence.test.ts`
- `theme` が再起動で失われる問題を `useUIStore` 永続化で解消し、起動時に保存値を復元する実装へ更新。
- 言語設定は `kukuri-locale` を正規化して起動時復元し、旧キー（`i18nextLng`）からの移行保存を追加。
- 既存データ互換として、旧テーマキー（`kukuri-theme` / `theme`）と旧ロケールキーを読めるようにした。

## 実装詳細

- `uiStore`
  - `persistKeys.ui = 'ui-storage'` を追加し、`theme` を `withPersist` で永続化。
  - 起動時 `resolveThemeFromStorage` で `ui-storage` → 旧キーの順に復元。
  - 旧形式（boolean / JSON オブジェクト）を許容する正規化を実装。
- `i18n`
  - `LOCALE_STORAGE_KEY` と互換キー群を定義。
  - 起動時に `resolveStoredLocale` で復元し、旧キー検出時は `kukuri-locale` へ移行保存。
  - 設定画面の言語変更は `persistLocale` を経由して保存。
- `E2E bridge`
  - テスト初期化時に `persistKeys.ui` / `kukuri-locale` / `i18nextLng` をクリア対象へ追加。

## テスト

- `bash ./scripts/test-docker.sh ts`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
