# Issue #45 初期監査（ja/en/zh-CN i18n）

作成日: 2026年02月16日
Issue: https://github.com/KingYoSun/kukuri/issues/45
対象PR: https://github.com/KingYoSun/kukuri/pull/41
Issueコメント: https://github.com/KingYoSun/kukuri/issues/45#issuecomment-3909126334

## 監査サマリー

- `main` ブランチには i18n 導入（`react-i18next` / `src/locales/*.json`）が未マージ。
- 監査は PR #41 head（`pr-41-audit`）を参照して実施。
- 主要リスク 3 点（未定義キー、locale drift、時刻ロケール一貫性）すべてで要修正項目を確認。

## 監査結果（mismatch list）

### 1) 未定義/不整合キー参照

1. `posts.deleteSuccess` が未定義
- 参照: `kukuri-tauri/src/hooks/usePosts.ts:148`（PR #41）
- 実在キー: `posts.deleted`（`ja/en/zh-CN` すべて）
- 影響: 投稿削除成功時に key 露出/フォールバックが発生。

2. `common.adding` が未定義
- 参照: `kukuri-tauri/src/components/SyncStatusIndicator.tsx:923`（PR #41）
- 影響: stale cache の同期キュー投入ボタン文言が key 露出。

3. `common.conflict` が未定義
- 参照: `kukuri-tauri/src/components/SyncStatusIndicator.tsx:552`（PR #41）
- 影響: 競合件数表示で key 露出。

4. `common.count` が未定義
- 参照: `kukuri-tauri/src/components/SyncStatusIndicator.tsx:552`, `kukuri-tauri/src/components/SyncStatusIndicator.tsx:563`（PR #41）
- 影響: 件数サフィックスで key 露出。

### 2) locale ファイル key drift

1. `en` に `posts.submit` が欠落
- 差分: `ja` / `zh-CN` には存在、`en` のみ欠落。
- 影響: 投稿 submit 文言が `en` で欠落。

2. `zh-CN` に `bootstrapConfig.add` / `bootstrapConfig.noNodes` が欠落
- 参照: `kukuri-tauri/src/components/p2p/BootstrapConfigPanel.tsx:183`, `kukuri-tauri/src/components/p2p/BootstrapConfigPanel.tsx:188`（PR #41）
- 影響: Bootstrap 設定パネルで key 露出。

### 3) 時刻表示ロケールの i18n 一貫性不足

- `formatDistanceToNow` 系は `getDateFnsLocale()` を利用しており概ね統一済み。
- 一方で以下は `toLocaleString()` / `Intl.DateTimeFormat(undefined, ...)` を使用しており、i18n 言語ではなく環境依存ロケールで表示される。

対象（PR #41）:
- `kukuri-tauri/src/components/NostrTestPanel.tsx`
- `kukuri-tauri/src/components/directMessages/DirectMessageDialog.tsx`
- `kukuri-tauri/src/components/directMessages/DirectMessageInbox.tsx`
- `kukuri-tauri/src/components/p2p/PeerConnectionPanel.tsx`
- `kukuri-tauri/src/components/search/PostSearchResults.tsx`
- `kukuri-tauri/src/components/settings/CommunityNodePanel.tsx`
- `kukuri-tauri/src/components/settings/KeyManagementDialog.tsx`
- `kukuri-tauri/src/components/summary/summaryTime.ts`
- `kukuri-tauri/src/components/sync/ConflictResolutionDialog.tsx`

## 最小実行計画（1タスク=1PR）

### PR-1: 未定義/不整合キー修正
- スコープ:
  - `posts.deleteSuccess` -> 既存キー `posts.deleted` へ統一、または locale 側に `deleteSuccess` を追加して参照統一。
  - `common.adding` / `common.conflict` / `common.count` を `common` へ追加、もしくは既存キーへ置換。
- Done:
  - `t(...)` 参照キーが ja/en/zh-CN で 100% 解決。
  - 該当 UI で key 露出なし。

### PR-2: locale drift 是正
- スコープ:
  - `en` に `posts.submit` を追加。
  - `zh-CN` に `bootstrapConfig.add` / `bootstrapConfig.noNodes` を追加。
  - 3 locale の key セット差分を CI で検証（キー一覧比較スクリプト導入）。
- Done:
  - `ja/en/zh-CN` のキー集合が一致。

### PR-3: 時刻ロケール統一
- スコープ:
  - `toLocaleString` / `Intl.DateTimeFormat(undefined, ...)` を `i18n.language` ベースの共通ヘルパーへ置換。
  - `date`/`time`/`dateTime` の表示方針を `src/i18n.ts` 近傍へ集約。
- Done:
  - 言語切替（ja/en/zh-CN）時、絶対時刻/相対時刻の表示ロケールが i18n 設定と一致。

## 実施コマンド（監査）

- `git fetch origin pull/41/head:pr-41-audit`
- `git show pr-41-audit:kukuri-tauri/src/i18n.ts`
- `git show pr-41-audit:kukuri-tauri/src/locales/{ja,en,zh-CN}.json`
- `jq -r 'paths(scalars) | map(tostring) | join(".")' ... | sort -u`
- `comm -23` で locale 間キー差分を比較
- `git grep -n -F "posts.deleteSuccess" pr-41-audit -- 'kukuri-tauri/src/**/*.ts' 'kukuri-tauri/src/**/*.tsx'`
- `git grep -n -F "toLocaleString(" pr-41-audit -- 'kukuri-tauri/src/**/*.ts' 'kukuri-tauri/src/**/*.tsx' ':(exclude)kukuri-tauri/src/tests/**'`

## テスト

- docs-only 監査のため、テスト/ビルド/`gh act` は未実施（`AGENTS.md` の docs-only 例外に従う）。
