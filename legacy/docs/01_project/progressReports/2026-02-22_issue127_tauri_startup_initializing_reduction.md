# Issue #127 Tauri 起動時「初期化中...」表示短縮レポート

作成日: 2026年02月22日

## 概要

- 対象:
  - `kukuri-tauri/src/stores/authStore.ts`
  - `kukuri-tauri/src/routes/__root.tsx`
  - `kukuri-tauri/src/tests/unit/stores/authStore.test.ts`
  - `kukuri-tauri/src/tests/unit/stores/authStore.accounts.test.ts`
  - `kukuri-tauri/src/tests/unit/routes/__root.test.tsx`
- 起動直後の `初期化中...` 表示が長引く主要因を `AuthStore.initialize` の直列待機に特定し、非クリティカル処理を遅延実行へ分離した。
- ルート初期化は `try/catch/finally` で保護し、初期化エラー時でも表示が解除されるようにした。
- 既存 `errorHandler` 経路を維持しつつ、初期化ゲート時間を ms 出力して計測可能にした。

## ボトルネック

- 変更前の `AuthStore.initialize` は以下を直列 `await` しており、初期化ゲート解除をブロックしていた。
  - `initializeNostr()`
  - `updateRelayStatus()`
  - `bootstrapTopics()`
  - `fetchAndApplyAvatar()`
  - `loadAccounts()`

## 実装詳細

- `AuthStore.initialize`
  - 認証状態復元（`getCurrentAccount` -> `set`）のみをクリティカルパスに残し、重い後続処理は `runDeferredInitializeTask` へ移動。
  - 遅延タスク失敗時は `errorHandler.log(..., { showToast: false })` で記録。
  - `AuthStore.initialize` のクリティカル完了時間を `AuthStore.initialize` コンテキストで info ログ出力。
- `RootRoute`
  - 初期化を `try/catch/finally` 化し、`initialize` が失敗しても `setIsInitializing(false)` を保証。
  - `RootRoute.initialize` でゲート解除時間（ms）を info ログ出力。

## 計測可能な検証

- `authStore` 単体テストに、非クリティカル処理を意図的に停止させたケースを追加。
  - `initializeNostr` と `listAccounts` を未解決 Promise に固定。
  - `initialize()` を `Promise.race(... timeout=500ms)` で評価し、`timeout` ではなく `resolved` になることを確認。
  - これにより「非クリティカル処理に待機しない」ことを時間境界付きで検証。
- `RootRoute` 単体テストに、初期化失敗時でも初期化画面で停止しないことを追加検証。

## 実行コマンド

- `bash scripts/test-docker.sh ts`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
