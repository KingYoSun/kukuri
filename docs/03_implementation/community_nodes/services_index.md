# Index サービス（検索移行運用対応）

**作成日**: 2026年01月22日
**最終更新日**: 2026年02月16日
**役割**: relay outbox から検索派生データを同期し、検索基盤の段階移行（dual-write/backfill/shadow/cutover）を支える

## 責務

- outbox consumer `index-v1` で `cn_relay.events_outbox` を順次処理し、`cn_relay.consumer_offsets` を更新する
- `cn_search.runtime_flags.search_write_mode` は `pg_only` 固定で、検索書込先は `cn_search.post_search_documents` のみ
- `cn_search.backfill_jobs` / `cn_search.backfill_checkpoints` を使い、PG 側検索ドキュメントを checkpoint 再開可能で backfill する
- stale `running` ジョブの lease reclaim（5分）と `started_at` フェンシングで backfill の多重実行を防止する
- outbox 差分を AGE suggest graph へ同期し、`cn_search.user_community_affinity` を定期再計算する
- reindex job（`cn_index.reindex_jobs`）を処理する（`cn_search.post_search_documents` の再構築）

## 外部インタフェース

- Index service endpoint
  - `GET /healthz`
  - `GET /metrics`
- 外部公開 API（User/Admin API 経由）
  - `GET /v1/search?topic=...&q=...`
  - `GET /v1/communities/suggest?q=...&limit=...`
  - `GET /v1/trending?topic=...`
  - `POST /v1/reindex`（Admin API）
- 役割分離
  - shadow-read 比較の実行主体は `cn-user-api`（`cn_search.shadow_read_logs` へ保存）
  - `cn-index` は dual-write/backfill と outbox 同期の責務を持つ

## PG-only 運用モデル（Issue #107 以降）

1. `search_write_mode=pg_only` を維持し、outbox から `cn_search.post_search_documents` へ同期する
2. `cn_search.backfill_jobs` に `target='post_search_documents'` ジョブを登録し、high-watermark 以前を backfill する
3. `cn-user-api` の `shadow_sample_rate` は `/v1/communities/suggest` の品質観測用途として運用する
4. 検索不整合時は `cn_index.reindex_jobs` と `cn_search.backfill_jobs` を実行して再構築する

## 主要テーブル・メトリクス

- テーブル
  - `cn_search.runtime_flags`
  - `cn_search.post_search_documents`
  - `cn_search.backfill_jobs`
  - `cn_search.backfill_checkpoints`
  - `cn_search.shadow_read_logs`（writer: `cn-user-api`）
  - `cn_relay.consumer_offsets`（consumer: `index-v1`）
- 主要メトリクス
  - `backfill_processed_rows`
  - `backfill_eta_seconds`
  - `outbox_backlog{consumer="index-v1"}`

## 運用上の注意

- 現行実装の `cn-index` `GET /healthz` は DB と内部依存（relay health target）をチェックする。
- cutover 手順・監視 SQL・rollback は以下を正本として運用する。
  - `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
  - `docs/03_implementation/community_nodes/ops_runbook.md`
  - `docs/03_implementation/community_nodes/user_api.md`
