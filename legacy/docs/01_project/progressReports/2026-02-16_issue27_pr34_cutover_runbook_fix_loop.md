# Issue #27 / PR #34 cutover runbook fix loop

最終更新日: 2026年02月16日

## 概要

PR #34 の review comments（`discussion_r2811782984` / `discussion_r2811782990` / `discussion_r2811782994`）に対応し、検索 PG cutover Runbook の運用手順を実装挙動へ一致させた。
コード変更は行わず、docs-only で staged rollout の安全性を補強した。

## 対応内容

1. 5% カナリア手順の修正（P1）
- 変更: `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- 修正前: 5% 手順で `search_read_backend` / `suggest_read_backend` を `pg` に変更していたため、最初の操作で実質 100% cutover となる記述だった。
- 修正後: 5%/25%/50% は `shadow_sample_rate` の段階拡大として運用し、binary な read backend 切替は 100% 段階でのみ実施する手順に更新。

2. zero-result 監視 SQL の列名修正（P2）
- 変更: `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- 修正前: `cn_search.shadow_read_logs` に存在しない `recorded_at` を参照。
- 修正後: 実際の列定義に合わせて `created_at` を参照。

3. Index lag consumer ラベル修正（P2）
- 変更:
  - `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
  - `docs/03_implementation/community_nodes/ops_runbook.md`
- 修正前: `consumer="index-search-v1"`
- 修正後: 実装定数（`cn-index`）に一致する `consumer="index-v1"`

## タスク管理更新

- `docs/01_project/activeContext/tasks/status/in_progress.md` に PR #34 fix loop 完了メモを追記。
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md` に PR #34 fix loop の完了内容と検証ログを追記。

## 検証

- `git diff --check`（pass）
- `rg -n "recorded_at|index-search-v1" docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md docs/03_implementation/community_nodes/ops_runbook.md`（pass: 対象ファイル一致なし）

## 変更ファイル

- `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_pr34_cutover_runbook_fix_loop.md`
