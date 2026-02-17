# Issue #45 PR-3 時刻表示ロケール統一

最終更新日: 2026年02月17日

## 概要

Issue #45 の残スコープ（PR-3）として、アプリ本体の `toLocaleString` /
`Intl.DateTimeFormat(undefined, ...)` を `i18n.language` ベースの共通フォーマッタへ統一した。
これにより、環境ロケール依存の時刻表示を解消し、表示ロケールを i18n 設定に一致させた。

## 実施内容

1. i18n言語ベースの共通フォーマッタを追加
- ファイル: `kukuri-tauri/src/lib/utils/localeFormat.ts`
- 変更:
  - `formatDateTimeByI18n(value, options?)` を追加（`getCurrentLocale()` + `Intl.DateTimeFormat`）。
  - `formatNumberByI18n(value, options?)` を追加（`getCurrentLocale()` + `Intl.NumberFormat`）。
  - 不正日時/非有限数値は空文字を返して表示崩れを抑止。

2. PR-3対象9領域の時刻表示を置換
- `kukuri-tauri/src/components/NostrTestPanel.tsx`
- `kukuri-tauri/src/components/directMessages/DirectMessageDialog.tsx`
- `kukuri-tauri/src/components/directMessages/DirectMessageInbox.tsx`
- `kukuri-tauri/src/components/p2p/PeerConnectionPanel.tsx`
- `kukuri-tauri/src/components/search/PostSearchResults.tsx`
- `kukuri-tauri/src/components/settings/CommunityNodePanel.tsx`
- `kukuri-tauri/src/components/settings/KeyManagementDialog.tsx`
- `kukuri-tauri/src/components/summary/summaryTime.ts`
- `kukuri-tauri/src/components/sync/ConflictResolutionDialog.tsx`

3. ロケール固定挙動のユニットテストを追加
- ファイル: `kukuri-tauri/src/tests/unit/lib/localeFormat.test.ts`
- 変更:
  - `ja` / `en` / `zh-CN` 切替時に `formatDateTimeByI18n` が対象ロケールの結果と一致することを検証。
  - `formatNumberByI18n` のロケール依存フォーマットと異常値フォールバックを検証。

4. 再監査
- `rg -n "toLocaleString\\(|Intl\\.DateTimeFormat\\(undefined" kukuri-tauri/src --glob '!**/tests/**'`
- 結果: ヒット 0 件（対象パターンの本番コード残件なし）。

## 検証コマンド

- `cd kukuri-tauri && pnpm vitest run src/tests/unit/lib/localeFormat.test.ts src/tests/unit/components/NostrTestPanel.test.tsx src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx src/tests/unit/components/directMessages/DirectMessageInbox.test.tsx src/tests/unit/components/p2p/PeerConnectionPanel.test.tsx src/tests/unit/components/search/PostSearchResults.test.tsx src/tests/unit/components/settings/CommunityNodePanel.test.tsx src/tests/unit/components/settings/KeyManagementDialog.test.tsx src/tests/unit/components/sync/ConflictResolutionDialog.test.tsx src/tests/unit/components/trending/TrendingSummaryPanel.test.tsx src/tests/unit/components/following/FollowingSummaryPanel.test.tsx`
- `cd kukuri-tauri && pnpm type-check`
- `cd kukuri-tauri && pnpm lint`
- `cd kukuri-tauri && pnpm format:check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue45-pr3.log`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue45-pr3.log`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue45-pr3.log`

## 検証結果

- 対象ユニットテスト: pass（`11 files, 54 passed, 4 skipped`）
- `pnpm type-check`: pass
- `pnpm lint`: pass
- `pnpm format:check`: pass
- `gh act --job format-check`: pass（`tmp/logs/gh-act-format-check-issue45-pr3.log`）
- `gh act --job native-test-linux`: pass（`tmp/logs/gh-act-native-test-linux-issue45-pr3.log`）
- `gh act --job community-node-tests`: pass（`tmp/logs/gh-act-community-node-tests-issue45-pr3.log`）

## 残タスク

- Issue #45 の PR-3 スコープは完了。残タスクなし（0件）。
