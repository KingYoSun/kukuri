# Issue #27 follow-up: ops_runbook shadow canary sequence sync

最終更新日: 2026年02月16日

## 概要

Issue #27 最終再監査の residual task #4 に対応し、`ops_runbook.md` の PG cutover 監視記述を実装準拠へ更新した。コード変更はなく docs-only。

## 対応内容

1. `ops_runbook.md` の cutover 順序を明確化
- 変更: `docs/03_implementation/community_nodes/ops_runbook.md`
- 同期した要点:
  - `search_read_backend` / `suggest_read_backend` は二値フラグであり、5%/25%/50% の段階運用は比率切替ではないことを明示
  - 5%/25%/50% は `search_read_backend='meili'` / `suggest_read_backend='legacy'` を維持したまま `shadow_sample_rate` を引き上げるカナリアであることを明記
  - 100% 段階でのみ `search_read_backend='pg'` / `suggest_read_backend='pg'` へ一括切替することを明記

2. タスク管理更新
- `docs/01_project/activeContext/tasks/status/in_progress.md` から Issue #27 最終再監査フォローアップ項目を削除（残タスク 0 件）。
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md` に本フォローアップ完了ログを追記。

## 検証

- `git diff --check`（pass）
- `rg -n "shadow_sample_rate|5%/25%/50% canary|100% cutover|search_read_backend='pg'" docs/03_implementation/community_nodes/ops_runbook.md`（pass）

## 変更ファイル

- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_followup_ops_runbook_shadow_canary_sync.md`
