# Index サービス（検索移行運用対応）

**作成日**: 2026年01月22日
**最終更新日**: 2026年02月16日
**役割**: relay outbox から検索派生データを同期し、検索基盤の段階移行（dual-write/backfill/shadow/cutover）を支える

## 責務

- outbox consumer `index-v1` で `cn_relay.events_outbox` を順次処理し、`cn_relay.consumer_offsets` を更新する
- `cn_search.runtime_flags.search_write_mode` に応じて検索書込先を切り替える
  - `meili_only`: Meilisearch のみ
  - `dual`: Meili + `cn_search.post_search_documents` 両系
  - `pg_only`: `cn_search.post_search_documents` のみ
- dual-write 中の片系失敗を検知し、outbox 再処理で整合回復する（error/retry メトリクスを記録）
- `cn_search.backfill_jobs` / `cn_search.backfill_checkpoints` を使い、PG 側検索ドキュメントを checkpoint 再開可能で backfill する
- stale `running` ジョブの lease reclaim（5分）と `started_at` フェンシングで backfill の多重実行を防止する
- outbox 差分を AGE suggest graph へ同期し、`cn_search.user_community_affinity` を定期再計算する
- reindex job（`cn_index.reindex_jobs`）を処理する（現行は Meili インデックス再構築が中心）

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

## 段階移行モデル（Issue #27 実装後）

1. `search_write_mode=meili_only` で稼働し、PG 側 schema/migration を準備する
2. `search_write_mode=dual` に変更し、Meili + PG の並行書込を開始する
3. `cn_search.backfill_jobs` に `target='post_search_documents'` ジョブを登録し、high-watermark 以前を backfill する
4. `cn-user-api` の `shadow_sample_rate` を `5% -> 25% -> 50%` へ段階拡大し、shadow 比較を収集する
5. 品質/性能ゲート達成後に `search_read_backend=pg` / `suggest_read_backend=pg` で 100% read cutover する
6. 安定観測後に `search_write_mode=pg_only` と Meili 段階撤去へ進む（障害時は read/write flag を rollback）

## 主要テーブル・メトリクス

- テーブル
  - `cn_search.runtime_flags`
  - `cn_search.post_search_documents`
  - `cn_search.backfill_jobs`
  - `cn_search.backfill_checkpoints`
  - `cn_search.shadow_read_logs`（writer: `cn-user-api`）
  - `cn_relay.consumer_offsets`（consumer: `index-v1`）
- 主要メトリクス
  - `search_dual_write_errors_total`
  - `search_dual_write_retries_total`
  - `backfill_processed_rows`
  - `backfill_eta_seconds`
  - `outbox_backlog{consumer="index-v1"}`

## 運用上の注意

- 現行実装の `cn-index` `GET /healthz` は Meili readiness を必須チェックする。`pg_only` 運用中も Meili 接続設定を残す前提で運用する。
- cutover 手順・監視 SQL・rollback は以下を正本として運用する。
  - `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
  - `docs/03_implementation/community_nodes/ops_runbook.md`
  - `docs/03_implementation/community_nodes/user_api.md`
