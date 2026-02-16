# Issue #27 follow-up: user_api suggest/runtime flag docs sync

最終更新日: 2026年02月16日

## 概要

Issue #27 最終再監査の residual task #2 に対応し、`user_api.md` の公開 API 一覧と search/suggest runtime flag の運用メモを実装挙動へ同期した。コード変更はなく docs-only。

## 対応内容

1. `/v1/communities/suggest` を User API 一覧へ反映
- 変更: `docs/03_implementation/community_nodes/user_api.md`
- 追記: `GET /v1/communities/suggest?q=...&limit=...`
- 実装同期ポイント:
  - `q` 正規化後が空文字なら `items=[]`
  - `limit` は `1..50` clamp（既定 20）
  - `suggest_read_backend=pg` かつ Stage-A 候補 0 件時は `legacy_fallback`

2. search/suggest runtime flag 運用メモを追加
- 変更: `docs/03_implementation/community_nodes/user_api.md`
- 追記対象フラグ:
  - `search_read_backend`
  - `search_write_mode`
  - `suggest_read_backend`
  - `suggest_rerank_mode`
  - `suggest_relation_weights`
  - `shadow_sample_rate`
- 運用同期ポイント:
  - read backend は二値フラグであり、5/25/50% カナリアは `shadow_sample_rate` で運用
  - 切替は `INSERT ... ON CONFLICT (flag_name) DO UPDATE` で更新

3. タスク管理更新
- `docs/01_project/activeContext/tasks/status/in_progress.md` に residual task #2 完了メモを追記し、残タスクを 2 件へ更新。
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md` に本フォローアップ完了ログを追記。

## 検証

- `git diff --check`（pass）
- `rg -n "/v1/communities/suggest|search_read_backend|suggest_read_backend|suggest_rerank_mode|suggest_relation_weights|shadow_sample_rate" docs/03_implementation/community_nodes/user_api.md`（pass）

## 変更ファイル

- `docs/03_implementation/community_nodes/user_api.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_followup_user_api_suggest_runtime_flags.md`
