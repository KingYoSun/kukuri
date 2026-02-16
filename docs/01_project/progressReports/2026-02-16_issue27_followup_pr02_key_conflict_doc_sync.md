# Issue #27 follow-up: PR-02 key/conflict docs sync

最終更新日: 2026年02月16日

## 概要

Issue #27 最終再監査の residual task #1 に対応し、`PR-02_post_search_pgroonga.md` の主キー/競合ポリシー記述を実装・migration 履歴に一致させた。コード変更はなく docs-only。

## 対応内容

1. PR-02 DDL の主キー記述を修正
- 変更: `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`
- 修正前: `post_id TEXT PRIMARY KEY`
- 修正後: `PRIMARY KEY (post_id, topic_id)`
- 追記: `m7`（初期導入）→ `m8`（複合主キー化）の migration 履歴注記

2. backfill upsert の conflict target を実装値へ同期
- 変更: `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`
- 修正前: `ON CONFLICT (post_id) DO UPDATE`
- 修正後: `ON CONFLICT (post_id, topic_id) DO UPDATE`

3. タスク管理更新
- `docs/01_project/activeContext/tasks/status/in_progress.md` に residual task #1 完了メモを追記し、残タスクを 3件へ更新。
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md` に本フォローアップ完了ログを追記。

## 検証

- `git diff --check`（pass）
- `rg -n "post_id TEXT PRIMARY KEY|ON CONFLICT \\(post_id\\)" docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`（pass: 一致なし）
- `rg -n "PRIMARY KEY \\(post_id, topic_id\\)|ON CONFLICT \\(post_id, topic_id\\)|20260216040000_m8_post_search_documents_topic_key.sql" docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`（pass）

## 変更ファイル

- `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_followup_pr02_key_conflict_doc_sync.md`
