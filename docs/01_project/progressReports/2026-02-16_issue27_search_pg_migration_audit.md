# Issue #27 検索PG移行計画 初期監査レポート

作成日: 2026年02月16日

## 目的

- `docs/01_project/activeContext/search_pg_migration/` の PR-01..PR-07 計画を、現行実装（community-node）と突合して、実装前ギャップを確定する。

## 監査結果サマリ

| PR | 判定 | implemented | missing | risky |
|---|---|---|---|---|
| PR-01 | PARTIAL | `service_configs` + `pg_notify` による設定配布基盤あり | `pgroonga`/`pg_trgm`/`cn_search.runtime_flags`/検索フラグ読取なし | 設定正本未確定で二重管理化リスク |
| PR-02 | MISSING | outbox→検索反映パターンは既存 | `post_search_documents`/PGroonga検索/API切替なし | 既存 `/v1/search` 契約を維持した切替層が未設計 |
| PR-03 | MISSING | なし | `community_search_terms`/候補生成API/`pg_trgm`なし | topicモデルとcommunityモデルの差分が未解決 |
| PR-04 | PARTIAL | AGE + User trust graph は既存 | Communityノード/affinity同期基盤なし | trustグラフとsuggestグラフの責務混在リスク |
| PR-05 | MISSING | なし | 2段階suggest実装・API・最終SQLフィルタなし | suggest の認証/同意/課金境界が未定義 |
| PR-06 | PARTIAL | outbox offset/reindex job/cutoff は既存 | dual-write/backfill checkpoint/shadow-readなし | 既存reindex手順をそのまま流用するとドリフト検証不足 |
| PR-07 | MISSING | Meili前提の運用経路は既存 | PG切替runbook/canary/撤去手順なし | healthz が Meili ready 前提で切替後に不整合 |

## 主要エビデンス

- Meili固定経路:
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs:196`
  - `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:325`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:77`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:479`
- 拡張不足（AGEのみ）:
  - `kukuri-community-node/docker/postgres-age/Dockerfile:12`
  - `kukuri-community-node/migrations/20260127000000_m5_trust.sql:1`
- 設定配布基盤（再利用候補）:
  - `kukuri-community-node/crates/cn-core/src/service_config.rs:85`
  - `kukuri-community-node/crates/cn-admin-api/src/services.rs:379`
- 段階移行の再利用候補（offset/reindex）:
  - `kukuri-community-node/crates/cn-index/src/lib.rs:392`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:633`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:708`
- compose/test の Meili依存:
  - `kukuri-community-node/docker-compose.yml:101`
  - `kukuri-community-node/docker-compose.yml:140`
  - `docker-compose.test.yml:40`
  - `docker-compose.test.yml:66`
- 追加確認コマンド:
  - `rg -n "pgroonga|pg_trgm|cn_search\.|search_read_backend|suggest_read_backend|search_write_mode|shadow_read_logs|backfill_jobs|post_search_documents|community_search_terms|user_community_affinity|graph_sync_offsets" kukuri-community-node`
  - 結果: `NO_MATCH`

## 今回のドキュメント更新

- 監査結果: `docs/01_project/activeContext/search_pg_migration/issue27_initial_audit_2026-02-16.md`
- 未着手タスク起票: `docs/01_project/activeContext/tasks/priority/search_pg_migration_roadmap.md`
- タスク管理更新:
  - `docs/01_project/activeContext/tasks/status/in_progress.md`
  - `docs/01_project/activeContext/tasks/completed/2026-02-16.md`

## 結論

- 実装着手の第一優先は、PR-01 の「拡張導入 + フラグ正本確定」。
- PR-04/06 の既存資産（AGE運用、outbox/reindex）を再利用し、PR-02/03/05 を段階実装する構成が最小リスク。
