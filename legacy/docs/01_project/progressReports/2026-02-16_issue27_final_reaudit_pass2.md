# Issue #27 最終再監査（post-followups / pass2）

作成日: 2026年02月16日
最終更新日: 2026年02月16日

## 監査前提

- strict checklist: `references/community-nodes-strict-audit-checklist.md` は未存在（`find . -name 'community-nodes-strict-audit-checklist.md'` の結果なし）。
- 適用ゲート: Issue #5 fallback gate
  - https://github.com/KingYoSun/kukuri/issues/5#issuecomment-3900483686
- 監査対象:
  - `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`
  - `docs/01_project/activeContext/search_pg_migration/PR-03_community_candidates_pg_trgm.md`
  - `docs/01_project/activeContext/search_pg_migration/PR-04_age_graph_sync.md`
  - `docs/01_project/activeContext/search_pg_migration/PR-05_two_stage_suggest_rerank.md`
  - `docs/01_project/activeContext/search_pg_migration/PR-06_dual_write_backfill_shadow.md`
  - `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
  - `docs/03_implementation/community_nodes/user_api.md`
  - `docs/03_implementation/community_nodes/services_index.md`
  - `docs/03_implementation/community_nodes/ops_runbook.md`
  - `kukuri-community-node/migrations/20260216020000_m6_search_runtime_flags.sql`
  - `kukuri-community-node/migrations/20260216030000_m7_post_search_documents.sql`
  - `kukuri-community-node/migrations/20260216040000_m8_post_search_documents_topic_key.sql`
  - `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql`
  - `kukuri-community-node/migrations/20260216060000_m10_age_graph_sync.sql`
  - `kukuri-community-node/migrations/20260216070000_m11_suggest_rerank_runtime_flags.sql`
  - `kukuri-community-node/migrations/20260216080000_m12_search_backfill_shadow.sql`
  - `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
  - `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs`
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs`
  - `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - `kukuri-community-node/crates/cn-index/src/lib.rs`
  - `kukuri-community-node/apps/admin-console/openapi/user-api.json`

## Gate 判定（Issue #5 fallback gate + Issue #27 要件）

| Gate | 判定 | 根拠 |
|---|---|---|
| Gate1: codex CLI 実行品質 | PASS | 監査コマンドを実行し、出力に基づく証跡を取得（placeholder 判定なし）。 |
| Gate2: `GET /v1/bootstrap/hints/latest` 401/428/429 境界契約テスト | PASS | `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:424`, `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:440`, `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:477`。 |
| Gate3: `spawn_bootstrap_hint_listener` 実DB受信経路テスト | PASS | 実装 `kukuri-community-node/crates/cn-user-api/src/lib.rs:367`、実DB通知テスト `kukuri-community-node/crates/cn-user-api/src/lib.rs:520`。 |
| Gate4: `CommunityNodePanel.tsx` / `PostSearchResults.tsx` 対応テスト有無 | PASS | `kukuri-tauri/src/tests/unit/components/settings/CommunityNodePanel.test.tsx:117`, `kukuri-tauri/src/tests/unit/components/search/PostSearchResults.test.tsx:130`。 |
| Gate5: エビデンステーブル付き報告 | PASS | 本レポートの「Issue #27 整合性監査」で、ファイル/行番号/コマンド/結果を明示。 |
| Gate6: Issue #27 の最終整合（post-followups） | PASS | PR-02..PR-07・運用3文書・runtime flag/cutover の不整合を再確認し、残差分なし。 |

## Issue #27 整合性監査（post-followups）

| 監査項目 | 実装/文書エビデンス | 確認コマンド | 判定 |
|---|---|---|---|
| PR-02（投稿検索 DDL/競合ポリシー） | `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md:10`, `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md:34`, `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md:50`, `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md:91`, `kukuri-community-node/migrations/20260216040000_m8_post_search_documents_topic_key.sql:5`, `kukuri-community-node/crates/cn-index/src/lib.rs:1694` | `nl -ba ...`, `rg -n "PRIMARY KEY \(post_id, topic_id\)|ON CONFLICT \(post_id, topic_id\)" ...` | PASS |
| PR-03（候補生成 pg_trgm/prefix + fallback） | `docs/01_project/activeContext/search_pg_migration/PR-03_community_candidates_pg_trgm.md:8`, `docs/01_project/activeContext/search_pg_migration/PR-03_community_candidates_pg_trgm.md:58`, `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql:1`, `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql:85`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:873`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:878`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:1512` | `nl -ba ...`, `rg -n "community_search_terms|legacy_fallback|prefix_hit|trgm" ...` | PASS |
| PR-04（AGE 同期 + affinity） | `docs/01_project/activeContext/search_pg_migration/PR-04_age_graph_sync.md:23`, `docs/01_project/activeContext/search_pg_migration/PR-04_age_graph_sync.md:29`, `kukuri-community-node/migrations/20260216060000_m10_age_graph_sync.sql:3`, `kukuri-community-node/crates/cn-index/src/lib.rs:207`, `kukuri-community-node/crates/cn-index/src/lib.rs:211`, `kukuri-community-node/crates/cn-index/src/lib.rs:700`, `kukuri-community-node/crates/cn-index/src/lib.rs:713`, `kukuri-community-node/crates/cn-index/src/lib.rs:1161` | `nl -ba ...`, `rg -n "graph_sync_offsets|user_community_affinity|spawn_affinity_recompute_worker" ...` | PASS |
| PR-05（2段階サジェスト + rerank flags） | `docs/01_project/activeContext/search_pg_migration/PR-05_two_stage_suggest_rerank.md:9`, `docs/01_project/activeContext/search_pg_migration/PR-05_two_stage_suggest_rerank.md:10`, `docs/01_project/activeContext/search_pg_migration/PR-05_two_stage_suggest_rerank.md:84`, `kukuri-community-node/migrations/20260216070000_m11_suggest_rerank_runtime_flags.sql:4`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:1036`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:1065`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:1442` | `nl -ba ...`, `rg -n "suggest_rerank_mode|suggest_relation_weights|Stage-A|Stage-B|enabled" ...` | PASS |
| PR-06（dual-write/backfill/shadow-read） | `docs/01_project/activeContext/search_pg_migration/PR-06_dual_write_backfill_shadow.md:9`, `docs/01_project/activeContext/search_pg_migration/PR-06_dual_write_backfill_shadow.md:15`, `docs/01_project/activeContext/search_pg_migration/PR-06_dual_write_backfill_shadow.md:35`, `kukuri-community-node/migrations/20260216080000_m12_search_backfill_shadow.sql:3`, `kukuri-community-node/migrations/20260216080000_m12_search_backfill_shadow.sql:28`, `kukuri-community-node/crates/cn-index/src/lib.rs:2149`, `kukuri-community-node/crates/cn-index/src/lib.rs:2180`, `kukuri-community-node/crates/cn-index/src/lib.rs:2480`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:796` | `nl -ba ...`, `rg -n "backfill_jobs|backfill_checkpoints|shadow_read_logs|lease_started_at|claim_backfill_job" ...` | PASS |
| PR-07（cutover/canary順序） | `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md:18`, `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md:67`, `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md:72`, `docs/03_implementation/community_nodes/ops_runbook.md:106`, `docs/03_implementation/community_nodes/ops_runbook.md:107`, `docs/03_implementation/community_nodes/ops_runbook.md:108`, `docs/03_implementation/community_nodes/services_index.md:39`, `docs/03_implementation/community_nodes/services_index.md:40`, `docs/03_implementation/community_nodes/user_api.md:222` | `nl -ba ...`, `rg -n "shadow_sample_rate|5%/25%/50%|100% cutover|search_read_backend|suggest_read_backend" ...` | PASS |
| runtime flag 実装整合（定義・読取・fallback） | `kukuri-community-node/migrations/20260216020000_m6_search_runtime_flags.sql:16`, `kukuri-community-node/migrations/20260216020000_m6_search_runtime_flags.sql:19`, `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs:8`, `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs:41`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:550`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:553`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:560`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:1053`, `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:1056`, `kukuri-community-node/crates/cn-index/src/lib.rs:1638` | `nl -ba ...`, `rg -n "search_read_backend|search_write_mode|suggest_read_backend|shadow_sample_rate" ...` | PASS |
| `user_api/services_index/ops_runbook` の運用整合 | `docs/03_implementation/community_nodes/user_api.md:181`, `docs/03_implementation/community_nodes/user_api.md:214`, `docs/03_implementation/community_nodes/services_index.md:10`, `docs/03_implementation/community_nodes/services_index.md:39`, `docs/03_implementation/community_nodes/services_index.md:61`, `docs/03_implementation/community_nodes/ops_runbook.md:105`, `docs/03_implementation/community_nodes/ops_runbook.md:107`, `docs/03_implementation/community_nodes/ops_runbook.md:115`, `kukuri-community-node/crates/cn-user-api/src/lib.rs:326`, `kukuri-community-node/apps/admin-console/openapi/user-api.json:268`, `kukuri-community-node/apps/admin-console/openapi/user-api.json:705`, `kukuri-community-node/crates/cn-index/src/lib.rs:225` | `nl -ba ...`, `rg -n "/v1/communities/suggest|/v1/search|shadow_sample_rate|index-v1|healthz" ...` | PASS |

## 実行コマンド（抜粋）

- `find . -name 'community-nodes-strict-audit-checklist.md'`
- `gh api repos/KingYoSun/kukuri/issues/comments/3900483686 | jq '{html_url, created_at, updated_at, user: .user.login, body}'`
- `rg -n "search_read_backend|search_write_mode|suggest_read_backend|shadow_sample_rate|suggest_rerank_mode|suggest_relation_weights" ...`
- `rg -n "PRIMARY KEY \(post_id, topic_id\)|ON CONFLICT \(post_id, topic_id\)" ...`
- `rg -n "community_search_terms|graph_sync_offsets|user_community_affinity|backfill_jobs|shadow_read_logs" ...`
- `rg -n "shadow_sample_rate|5%/25%/50%|100%|search_read_backend|suggest_read_backend" ...`
- `nl -ba <対象ファイル> | sed -n '<行範囲>p'`

## 結論

- 判定: **PASS**
- 追加タスク: **なし**
- Issue #27 はクローズ可能（post-followups 再監査で残差分なし）。
