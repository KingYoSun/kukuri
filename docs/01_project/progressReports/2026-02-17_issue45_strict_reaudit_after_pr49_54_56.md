# Issue #45 strict re-audit（PR #49 / #54 / #56 マージ後）

作成日: 2026年02月17日
最終更新日: 2026年02月17日

## 監査前提

- 監査対象 Issue: `https://github.com/KingYoSun/kukuri/issues/45`
- 監査対象 HEAD: `15191558133f561ef4090dc6f6a58607bc4211de`（`main`）
- strict checklist: `references/community-nodes-strict-audit-checklist.md` は未存在（`ls references` で確認）。
- 適用ゲート: Issue #5 fallback gate（明示）
  - `https://github.com/KingYoSun/kukuri/issues/5#issuecomment-3900483686`

## Gate 判定（Issue #5 fallback gate + Issue #45 scope）

| Gate | 判定 | 根拠 |
|---|---|---|
| Gate1: codex CLI 実行品質 | PASS | `echo CODEX_COMMAND_OK` を含む実コマンド監査で証跡取得（placeholder 判定なし）。 |
| Gate2: fallback 401/428/429 境界契約テスト存在確認 | PASS | `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:424`, `:440`, `:477` に該当テストが存在。 |
| Gate3: fallback listener 実DB受信経路テスト存在確認 | PASS | `kukuri-community-node/crates/cn-user-api/src/lib.rs:367`, `:520` を確認。 |
| Gate4: fallback Frontend テスト存在確認 | PASS | `kukuri-tauri/src/tests/unit/components/settings/CommunityNodePanel.test.tsx:117`, `kukuri-tauri/src/tests/unit/components/search/PostSearchResults.test.tsx:130` を確認。 |
| Gate5: locale drift 解消（ja/en/zh-CN キー集合一致） | FAIL | `ja:1050 / en:1049 / zh-CN:1048`。`en` に `posts.submit` 欠落、`zh-CN` に `bootstrapConfig.add` / `bootstrapConfig.noNodes` 欠落。 |
| Gate6: PR-1 対象キーの再後退有無 | PASS | `posts.deleteSuccess` 参照は 0 件。`common.adding/conflict/count` は3ロケール定義あり。 |
| Gate7: 時刻表示ロケール統一（環境ロケール依存 API 残件） | FAIL | `toLocaleString` / `Intl.DateTimeFormat(undefined, ...)` が本番コードに14件残存。 |
| Gate8: エビデンステーブル付き報告 | PASS | 本レポートにファイル/コマンド/結果を明示。 |

## エビデンステーブル（Issue #45 scope）

| 監査項目 | 実装ファイル | 確認コマンド | 結果 |
|---|---|---|---|
| locale キー集合一致（drift なし） | `kukuri-tauri/src/locales/ja.json`, `kukuri-tauri/src/locales/en.json`, `kukuri-tauri/src/locales/zh-CN.json` | `jq -r 'paths(scalars)...'` + `comm -23` | FAIL: `ja-en` 差分 `posts.submit`、`ja-zh-CN` 差分 `bootstrapConfig.add`, `bootstrapConfig.noNodes` |
| PR-1 対象キーの再後退（`posts.deleteSuccess`） | `kukuri-tauri/src/hooks/usePosts.ts` | `rg -n "posts\\.deleteSuccess" kukuri-tauri/src --glob '!**/tests/**'` | PASS: 該当なし |
| `common.adding/conflict/count` 参照と定義整合 | `kukuri-tauri/src/components/SyncStatusIndicator.tsx`, locale 3ファイル | `rg -n "common\\.adding|common\\.conflict|common\\.count|bootstrapConfig\\.add|bootstrapConfig\\.noNodes" kukuri-tauri/src --glob '!**/tests/**'` + `jq` | 部分FAIL: `common.*` は解消済み、`bootstrapConfig.*` は zh-CN 欠落 |
| 時刻ロケール統一（環境依存 API 残件） | `kukuri-tauri/src/components/*`（下記） | `rg -n "toLocaleString\\(|Intl\\.DateTimeFormat\\(undefined" kukuri-tauri/src --glob '!**/tests/**'` | FAIL: 14件（`NostrTestPanel.tsx`, `DirectMessageDialog.tsx`, `DirectMessageInbox.tsx`, `PeerConnectionPanel.tsx`, `PostSearchResults.tsx`, `CommunityNodePanel.tsx`, `KeyManagementDialog.tsx`, `summaryTime.ts`, `ConflictResolutionDialog.tsx` など） |
| PR #54 / #56 の Issue #45 影響有無 | PR diff file list | `gh pr view 54 --json files --jq '.files[].path'`, `gh pr view 56 --json files --jq '.files[].path'` | PASS（監査観点）: #54/#56 は i18n locale drift / time locale 統一の未解消点を変更していない |

## 判定

- Issue #45 は **未完**（再現可能な残タスク 2 件）。
- クローズ条件（`ja/en/zh-CN` キー欠落なし、時刻ロケール統一）を満たしていない。

## 残タスク（1タスク = 1PR）

1. PR-2: locale drift 是正
- `en.posts.submit` を追加。
- `zh-CN.bootstrapConfig.add` / `zh-CN.bootstrapConfig.noNodes` を追加。
- 3ロケールキー集合の一致チェック（`jq`/テスト）を追加し、再発防止を導入。

2. PR-3: 時刻表示ロケールの i18n 統一
- `toLocaleString` / `Intl.DateTimeFormat(undefined, ...)` を `i18n.language` ベースの共通フォーマッタへ置換。
- 置換対象（少なくとも）: `NostrTestPanel.tsx`, `DirectMessageDialog.tsx`, `DirectMessageInbox.tsx`, `PeerConnectionPanel.tsx`, `PostSearchResults.tsx`, `CommunityNodePanel.tsx`, `KeyManagementDialog.tsx`, `summaryTime.ts`, `ConflictResolutionDialog.tsx`。
- 関連ユニットテストを i18n 固定ロケールで更新し、表示差分を検証。
