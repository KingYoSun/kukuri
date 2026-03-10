# Issue #27 初期監査: search_pg_migration と現行実装の差分

作成日: 2026年02月16日

## 監査対象

- `docs/01_project/activeContext/search_pg_migration/README.md`
- `docs/01_project/activeContext/search_pg_migration/PR-01_extensions_and_flags.md`
- `docs/01_project/activeContext/search_pg_migration/PR-02_post_search_pgroonga.md`
- `docs/01_project/activeContext/search_pg_migration/PR-03_community_candidates_pg_trgm.md`
- `docs/01_project/activeContext/search_pg_migration/PR-04_age_graph_sync.md`
- `docs/01_project/activeContext/search_pg_migration/PR-05_two_stage_suggest_rerank.md`
- `docs/01_project/activeContext/search_pg_migration/PR-06_dual_write_backfill_shadow.md`
- `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- `docs/01_project/activeContext/search_pg_migration/appendix_risks_open_points_poc.md`

## PR別監査サマリ（implemented / missing / risky）

### PR-01 拡張導入と移行フラグ基盤

- implemented
  - `cn_admin.service_configs` + `pg_notify('cn_admin_config', ...)` によるランタイム設定配布基盤は既存実装あり。
- missing
  - `pgroonga` / `pg_trgm` の導入、`cn_search` スキーマ、`runtime_flags` テーブルは未実装。
  - `search_read_backend` / `search_write_mode` / `suggest_read_backend` の読取実装は未実装。
- risky
  - 設定正本（`cn_admin.service_configs` か `cn_search.runtime_flags`）が未確定のまま実装に入ると、フラグ反映経路が二重化する。
- evidence
  - `kukuri-community-node/crates/cn-core/src/service_config.rs:85`
  - `kukuri-community-node/crates/cn-admin-api/src/services.rs:379`
  - `kukuri-community-node/docker/postgres-age/Dockerfile:12`
  - `kukuri-community-node/migrations/20260127000000_m5_trust.sql:1`
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs:196`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:77`

### PR-02 投稿検索ドキュメント（PGroonga）

- implemented
  - 既存の outbox consumer で投稿イベントを検索インデックスへ反映する経路は存在。
- missing
  - `cn_search.post_search_documents`、PGroonga索引、正規化バージョン管理、SQLランキング合成式は未実装。
  - `/v1/search` は PostgreSQL 実装ではなく Meilisearch 直呼び出し。
- risky
  - 現行の検索レスポンス契約を維持したまま SQL 実装へ切替える変換層が未設計。
- evidence
  - `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:296`
  - `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:325`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:479`

### PR-03 コミュニティ候補生成（pg_trgm + prefix）

- implemented
  - 実装なし。
- missing
  - `cn_search.community_search_terms`、`pg_trgm` index、prefix index、候補生成 API が未実装。
  - リポジトリ内で suggest 関連コードが未検出。
- risky
  - 現行モデルは `topic_id` 中心で、PR案の `community_id` モデルに直接対応するテーブルが無く、データモデル変換が先行課題。
- evidence
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs:316`
  - `kukuri-community-node/migrations/20260124000000_m2.sql:153`
  - コマンド結果: `rg -n "pgroonga|pg_trgm|cn_search\.|search_read_backend|suggest_read_backend|search_write_mode|shadow_read_logs|backfill_jobs|post_search_documents|community_search_terms|user_community_affinity|graph_sync_offsets" kukuri-community-node` -> `NO_MATCH`

### PR-04 AGE グラフ拡張 + 同期

- implemented
  - AGE 拡張と trust 用 Userグラフ（`REPORTED` / `INTERACTED`）は稼働済み。
- missing
  - `Community` ノード、user-community エッジ、`cn_search.graph_sync_offsets`、`cn_search.user_community_affinity` は未実装。
- risky
  - 既存 trust グラフと suggest 用グラフを同一 graph 名で混在させると、責務と運用監視が不明瞭になる。
- evidence
  - `kukuri-community-node/migrations/20260127000000_m5_trust.sql:1`
  - `kukuri-community-node/crates/cn-trust/src/lib.rs:1032`
  - `kukuri-community-node/crates/cn-trust/src/lib.rs:1041`

### PR-05 2段階サジェスト（候補生成 + 再ランキング）

- implemented
  - 実装なし。
- missing
  - Stage-A / Stage-B パイプライン、`/v1/communities/suggest`、block/mute/visibility 最終SQLフィルタは未実装。
- risky
  - 現行 user-api は `search` のみ公開しており、suggest APIの認証/同意/課金境界を新規に定義する必要がある。
- evidence
  - `kukuri-community-node/crates/cn-user-api/src/openapi.rs:209`
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs:316`
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs:317`

### PR-06 dual-write + backfill + shadow-read

- implemented
  - outbox offset 管理、`reindex_jobs`、cutoff の概念は既存実装あり（Meili 向け）。
- missing
  - `cn_search.backfill_jobs` / `backfill_checkpoints` / `shadow_read_logs`、dual-write モード制御、shadow-read 比較保存は未実装。
- risky
  - 現行 reindex は Meili 全削除前提であり、PG移行時の dual-write フェーズへそのまま流用するとドリフト検証が不足する。
- evidence
  - `kukuri-community-node/migrations/20260125000000_m3_index.sql:3`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:392`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:633`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:708`

### PR-07 切替・監視・運用 Runbook

- implemented
  - Meilisearch 前提の運用/テスト経路は既に存在。
- missing
  - `search_read_backend=pg` / `suggest_read_backend=pg` の段階切替手順、PG検索監視Runbook、Meili段階撤去手順は未実装。
- risky
  - `cn-user-api` / `cn-index` の `/healthz` が Meili ready を必須としており、切替後のヘルス定義見直しが必要。
- evidence
  - `kukuri-community-node/docker-compose.yml:101`
  - `kukuri-community-node/docker-compose.yml:140`
  - `docker-compose.test.yml:40`
  - `docker-compose.test.yml:66`
  - `kukuri-community-node/crates/cn-index/src/lib.rs:135`
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs:432`

## 総括

- 現状は **検索基盤が Meilisearch に強く固定** されており、PR-01/02/03/05/07 は実装着手前。
- PR-04/06 は **部分的な再利用基盤（AGE利用、outbox/reindex運用）** があるため、ここを土台に段階移行するのが現実的。
- 最初の実装ブロッカーは **設定正本の確定**（`service_configs` vs `runtime_flags`）と **topicモデルとcommunityモデルの対応方針確定**。
