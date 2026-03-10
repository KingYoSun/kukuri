# Issue #27 最終再監査（PR-01..PR-07 マージ後）

作成日: 2026年02月16日

## 監査前提

- strict checklist: `references/community-nodes-strict-audit-checklist.md` はリポジトリ内に存在しないため、Issue #5 fallback gate を適用。
  - fallback: https://github.com/KingYoSun/kukuri/issues/5#issuecomment-3900483686
- 監査対象:
  - `docs/01_project/activeContext/search_pg_migration/PR-01..PR-07`
  - `kukuri-community-node/migrations/2026021602*_m6~m12*.sql`
  - `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
  - `kukuri-community-node/crates/cn-index/src/lib.rs`
  - `kukuri-community-node/crates/cn-user-api/src/{lib.rs,subscriptions.rs,bootstrap.rs}`
  - `docs/03_implementation/community_nodes/{ops_runbook.md,user_api.md,services_index.md}`

## Gate 判定（PASS/FAIL）

| Gate | 判定 | 根拠 |
|---|---|---|
| Gate1: codex CLI 実行品質 | PASS | 実コマンドで再監査（`find` / `rg` / `nl -ba` / `sed` / `gh api`）を実施し、placeholder 判定なし。 |
| Gate2: `GET /v1/bootstrap/hints/latest` 401/428/429 境界契約 | PASS | `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:424`, `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:440`, `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:477` を確認。 |
| Gate3: `spawn_bootstrap_hint_listener` 実DB受信経路 | PASS | 実装 `kukuri-community-node/crates/cn-user-api/src/lib.rs:367`、実DB通知テスト `kukuri-community-node/crates/cn-user-api/src/lib.rs:520` を確認。 |
| Gate4: `CommunityNodePanel.tsx` / `PostSearchResults.tsx` 対応テスト | PASS | `kukuri-tauri/src/tests/unit/components/settings/CommunityNodePanel.test.tsx:117`、`kukuri-tauri/src/tests/unit/components/search/PostSearchResults.test.tsx:130` を確認。 |
| Gate5: エビデンステーブル付き報告 | PASS | 本レポートの「Issue #27 整合性監査」セクションで、ファイル/コマンド/結果を明示。 |
| Gate6: Issue #27 要件整合（runtime flags/cutover、PG search/suggest/backfill、migration/runbook） | FAIL | 実装は概ね整合だが、運用・設計ドキュメントに4件の残差分あり（下表）。 |

## Issue #27 整合性監査

| 監査項目 | 実装/ドキュメント | 確認コマンド | 結果 |
|---|---|---|---|
| runtime flags 定義と実装 | `kukuri-community-node/migrations/20260216020000_m6_search_runtime_flags.sql:14`、`kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs:8`、`kukuri-community-node/crates/cn-user-api/src/lib.rs:257`、`kukuri-community-node/crates/cn-index/src/lib.rs:183` | `nl -ba ...`, `rg -n "search_read_backend|search_write_mode|suggest_read_backend|shadow_sample_rate"` | **整合（PASS）**: フラグ seed / ローダー / watcher は一致。 |
| cutover 手順（5/25/50/100） | `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md:18`、`docs/03_implementation/community_nodes/ops_runbook.md:106` | `nl -ba ...`, `rg -n "shadow_sample_rate|search_read_backend"` | **不整合（FAIL）**: PR-07 は 5/25/50 を `shadow_sample_rate` で運用、100% で read 切替だが、`ops_runbook.md` は `search_read_backend=pg` の段階適用中と読める表現が残存。 |
| 投稿検索ドキュメント DDL と実migration | `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md:17`、`kukuri-community-node/migrations/20260216040000_m8_post_search_documents_topic_key.sql:1`、`kukuri-community-node/crates/cn-index/src/lib.rs:1694` | `nl -ba ...`, `rg -n "ON CONFLICT \\(post_id, topic_id\\)"` | **不整合（FAIL）**: 実装は `(post_id, topic_id)` 主キーだが、PR-02 文書は `post_id` 単独主キー/`ON CONFLICT(post_id)` 記載のまま。 |
| suggest path の公開仕様 | `kukuri-community-node/crates/cn-user-api/src/lib.rs:326`、`kukuri-community-node/apps/admin-console/openapi/user-api.json:268`、`docs/03_implementation/community_nodes/user_api.md:179` | `rg -n "/v1/communities/suggest|/v1/search"` | **不整合（FAIL）**: 実装と OpenAPI には `/v1/communities/suggest` があるが、`user_api.md` の API 一覧に未記載。 |
| backfill/shadow 実装と Runbook | `kukuri-community-node/migrations/20260216080000_m12_search_backfill_shadow.sql:3`、`kukuri-community-node/crates/cn-index/src/lib.rs:2149`、`kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:796`、`docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md:101` | `rg -n "backfill_jobs|shadow_read_logs|overlap_at_10|latency_delta"` | **整合（PASS）**: テーブル定義、worker、shadow 保存、運用SQL（`created_at`/`/v1/search`）は一致。 |
| index サービス運用ドキュメント | `docs/03_implementation/community_nodes/services_index.md:1`、`kukuri-community-node/crates/cn-index/src/lib.rs:98`, `kukuri-community-node/crates/cn-index/src/lib.rs:2149` | `nl -ba ...`, `rg -n "Meilisearch|SearchWriteMode|backfill"` | **不整合（FAIL）**: `services_index.md` が Meili-only 前提の記述で、Issue #27 後の dual-write/backfill/shadow 運用を反映できていない。 |

## 実行コマンド（抜粋）

- `find . -name 'community-nodes-strict-audit-checklist.md'`
- `gh api repos/KingYoSun/kukuri/issues/comments/3900483686 | jq '{html_url, created_at, user: .user.login, body}'`
- `rg -n "search_read_backend|suggest_read_backend|search_write_mode|shadow_sample_rate" ...`
- `nl -ba docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md | sed -n '1,360p'`
- `nl -ba kukuri-community-node/crates/cn-user-api/src/subscriptions.rs | sed -n '400,1740p'`
- `nl -ba kukuri-community-node/crates/cn-index/src/lib.rs | sed -n '1420,2620p'`
- `rg -n "/v1/communities/suggest|/v1/search" kukuri-community-node/crates/cn-user-api/src/lib.rs kukuri-community-node/apps/admin-console/openapi/user-api.json docs/03_implementation/community_nodes/user_api.md`

## 残タスク / リスク

1. `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md` を `(post_id, topic_id)` 主キー前提へ更新する。
2. `docs/03_implementation/community_nodes/user_api.md` に `/v1/communities/suggest` と search/suggest runtime flag 切替運用を追記する。
3. `docs/03_implementation/community_nodes/services_index.md` を Meili-only から dual-write/backfill/shadow/cutover 前提へ更新する。
4. `docs/03_implementation/community_nodes/ops_runbook.md` の cutover 記述を `shadow_sample_rate` 段階カナリア順序に合わせて明確化する。

## 結論

- 最終判定: **NEEDS_FOLLOWUP**
- 理由: 実装系（runtime flags、suggest、backfill/shadow、migration）は成立しているが、Issue close 判定に必要なドキュメント整合が未完了。
