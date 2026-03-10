# Issue #27 follow-up: services_index dual-write/backfill/shadow/cutover sync

最終更新日: 2026年02月16日

## 概要

Issue #27 最終再監査の residual task #3 に対応し、`services_index.md` を Meili-only 記述から実装準拠の検索移行運用モデルへ更新した。コード変更はなく docs-only。

## 対応内容

1. `services_index.md` を運用実態へ同期
- 変更: `docs/03_implementation/community_nodes/services_index.md`
- 同期した要点:
  - `search_write_mode`（`meili_only` / `dual` / `pg_only`）と outbox dual-write の責務
  - 片系失敗時の再送（outbox replay）と関連メトリクス
  - `cn_search.backfill_jobs` / `cn_search.backfill_checkpoints` による checkpoint 再開可能 backfill
  - `shadow_sample_rate` カナリア（5/25/50%）から 100% read cutover までの段階順序
  - `shadow_read_logs` の保存主体は `cn-user-api` である責務境界
  - `cn-index` `GET /healthz` が Meili readiness を必須とする現状注記

2. タスク管理更新
- `docs/01_project/activeContext/tasks/status/in_progress.md` の residual task #3 完了メモを追記し、残タスクを 1 件へ更新。
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md` に本フォローアップ完了ログを追記。

## 検証

- `git diff --check`（pass）
- `rg -n "search_write_mode|backfill_jobs|shadow_sample_rate|search_read_backend=pg|healthz" docs/03_implementation/community_nodes/services_index.md`（pass）

## 変更ファイル

- `docs/03_implementation/community_nodes/services_index.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_followup_services_index_cutover_sync.md`
