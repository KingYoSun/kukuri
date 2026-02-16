# Issue #45 PR-1 i18nキー不整合修正

最終更新日: 2026年02月16日

## 概要

Issue #45 の PR-1 スコープとして、未定義/不整合の i18n キー参照を最小差分で修正した。
本PRでは PR-2（locale drift）および PR-3（時刻ロケール統一）には着手していない。

## 実施内容

1. `posts.deleteSuccess` の不整合を解消
- ファイル: `kukuri-tauri/src/hooks/usePosts.ts`
- 変更: `i18n.t('posts.deleteSuccess')` を `i18n.t('posts.deleted')` へ統一。
- 理由: `posts.deleted` は `ja/en/zh-CN` すべてで既存定義済み。

2. `common.adding` / `common.conflict` / `common.count` を3ロケールに追加
- ファイル: `kukuri-tauri/src/locales/ja.json`
- ファイル: `kukuri-tauri/src/locales/en.json`
- ファイル: `kukuri-tauri/src/locales/zh-CN.json`
- 変更: `common` 配下へ `adding` / `conflict` / `count` を追加。
- 理由: `SyncStatusIndicator` が参照するキーの未定義を解消し、キー露出を防止。

## スコープ外（明示）

- PR-2: `en.posts.submit` / `zh-CN.bootstrapConfig.add` / `zh-CN.bootstrapConfig.noNodes` の locale drift 修正。
- PR-3: `toLocaleString` / `Intl.DateTimeFormat(undefined, ...)` の i18n 言語統一。

## 検証コマンド

- `cd kukuri-tauri && CI=true pnpm install --frozen-lockfile`
- `cd kukuri-tauri && pnpm vitest run src/tests/unit/hooks/usePosts.test.tsx`
- `cd kukuri-tauri && pnpm vitest run src/tests/unit/hooks/usePosts.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx`
- `cd kukuri-tauri && pnpm lint`
- `cd kukuri-tauri && pnpm eslint src/hooks/usePosts.ts`
- `cd kukuri-tauri && pnpm prettier --check src/hooks/usePosts.ts src/locales/ja.json src/locales/en.json src/locales/zh-CN.json`

## 検証結果

- `usePosts` 単体テスト: pass。
- `usePosts + SyncStatusIndicator` 併走テスト: fail（`SyncStatusIndicator` の 26 ケースが英語文言前提で失敗。今回の修正対象外の既存不整合）。
- `pnpm lint`: fail（`src/stores/authStore.ts` の既存未使用変数 `storageError` / `loadError`）。
- `pnpm eslint src/hooks/usePosts.ts`: pass。
- `prettier --check`（変更ファイル）: pass。
- `gh act --job format-check`: fail（既存40ファイルのPrettier未整形。ログ: `tmp/logs/gh-act-format-check-issue45-pr1.log`）。
- `gh act --job native-test-linux`: fail（既存テストが英語文言/未初期化i18n前提で多数失敗。ログ: `tmp/logs/gh-act-native-test-linux-issue45-pr1.log`）。
- `gh act --job community-node-tests`: pass（ログ: `tmp/logs/gh-act-community-node-tests-issue45-pr1.log`）。

## 影響範囲

- 投稿削除成功トーストと同期ステータス表示の i18n キー解決性に限定。
- API・ドメインロジック・時刻表示ロジックには影響なし。
