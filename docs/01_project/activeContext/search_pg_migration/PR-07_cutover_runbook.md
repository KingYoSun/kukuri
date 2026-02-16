# PR-07: 切替・監視・運用 Runbook
最終更新: 2026年02月16日

## 目的
- `search_read_backend=pg` / `suggest_read_backend=pg` への段階切替を、運用手順として固定化する。
- cutover 後の監視・一次切り分け・ロールバック・段階撤去を同一 Runbook で扱う。

## スコープ
- カナリア切替（5% -> 25% -> 50% -> 100%）
- 監視ダッシュボード項目（search/suggest/index lag/zero-result/filter drop）
- 5分以内のロールバック手順
- Meilisearch 依存の段階撤去条件と手順

## 前提
- 書込は `search_write_mode=dual` で運用し、read 切替中も片系欠損を防ぐ。
- shadow-read 指標（`shadow_overlap_at_10` / `shadow_latency_delta_ms`）が収集できる状態で開始する。
- フラグ正本は `cn_search.runtime_flags` とし、変更者は `updated_by` に運用IDを残す。

## 品質/性能ゲート（PR-07で固定）

### 判定閾値
- 品質: `overlap@10 >= 0.70`（`shadow_overlap_at_10` の24h P50）
- 品質: `NDCG@10 >= 0.90`（golden set 日次計測）
- 性能: 検索 P95 `<= 180ms`（`/v1/search`）
- 性能: サジェスト P95 `<= 80ms`（`suggest_stage_a_latency_ms` + `suggest_stage_b_latency_ms`）
- 安定性: API 5xx 率 `< 1%`
- 反映遅延: `outbox_backlog{consumer="index-search-v1"} < 1,000` を維持

### 判定期間
- 各カナリア段階で最低 24h 観測。
- 100% 切替後は 7-14 日の安定観測を行い、Meili 撤去判定へ進む。

## カナリア切替手順

### 0. 事前確認
1. 現在値確認:
```sql
SELECT flag_name, flag_value, updated_at, updated_by
FROM cn_search.runtime_flags
WHERE flag_name IN (
  'search_read_backend',
  'suggest_read_backend',
  'search_write_mode',
  'shadow_sample_rate'
)
ORDER BY flag_name;
```
2. write モード確認: `search_write_mode='dual'`
3. shadow サンプル率確認: `shadow_sample_rate >= 5`

### 1. 5% カナリア
1. 以下を実行:
```sql
BEGIN;
INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by)
VALUES
  ('search_read_backend', 'pg', 'ops-cutover-5pct'),
  ('suggest_read_backend', 'pg', 'ops-cutover-5pct')
ON CONFLICT (flag_name)
DO UPDATE SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by;
COMMIT;
```
2. 24h 監視し、品質/性能ゲートを判定する。

### 2. 25% / 50% / 100% 拡大
- 5% と同じ手順で `updated_by` を段階ごとに変える（例: `ops-cutover-25pct`）。
- 各段階で 24h 観測し、閾値未達の場合はロールバックする。

### 3. 100% 後の運用
- 7-14 日は Meili を standby 維持。
- 期間中は `search_write_mode=dual` を維持し、再切戻し余地を残す。

## 監視ダッシュボード（PG cutover 用）

### 必須パネル
- Search latency: `http_request_duration_seconds{service="cn-user-api",route="/v1/search"}` の p50/p95/p99
- Search error rate: `http_requests_total{service="cn-user-api",route="/v1/search",status=~"5.."}` / 全リクエスト
- Suggest latency: `suggest_stage_a_latency_ms{service="cn-user-api"}` / `suggest_stage_b_latency_ms{service="cn-user-api"}`
- Suggest filter drop: `suggest_block_filter_drop_count{service="cn-user-api",backend="pg"}`
- Shadow quality: `shadow_overlap_at_10{service="cn-user-api",endpoint="/v1/search"}` / `shadow_latency_delta_ms{service="cn-user-api",endpoint="/v1/search"}`
- Index lag: `outbox_backlog{service="cn-index",consumer="index-search-v1"}`
- Outbox health: `outbox_consumer_batches_total{service="cn-index",consumer="index-search-v1",result="error"}`

### Zero-result 監視（運用SQL）
> 備考: 現行は専用メトリクス未実装のため、shadow ログを代理指標として使用する。

```sql
SELECT
  date_trunc('hour', recorded_at) AS hour,
  COUNT(*) AS sampled_total,
  SUM(CASE WHEN cardinality(pg_ids) = 0 THEN 1 ELSE 0 END) AS zero_results,
  ROUND(
    SUM(CASE WHEN cardinality(pg_ids) = 0 THEN 1 ELSE 0 END)::numeric / NULLIF(COUNT(*), 0),
    4
  ) AS zero_result_rate
FROM cn_search.shadow_read_logs
WHERE endpoint = '/v1/search'
  AND recorded_at >= NOW() - INTERVAL '24 hours'
GROUP BY 1
ORDER BY 1 DESC;
```

## 運用検証手順（各段階共通）
1. `/healthz` が `200` であることを `cn-user-api` / `cn-index` で確認。
2. `/metrics` で以下の値を採取し、運用ログへ記録:
   - `shadow_overlap_at_10`
   - `shadow_latency_delta_ms`
   - `suggest_block_filter_drop_count`
   - `outbox_backlog{consumer="index-search-v1"}`
3. E2E で以下を実施し結果を保存:
   - 検索（hit / miss / multi-topic）
   - サジェスト（prefix / trgm / block-mute）
   - 権限ケース（非公開topic）
4. Golden set で NDCG@10 を再計測し、前段階との差分を確認。

## ロールバック（5分以内復旧）

### 実行手順
1. read を旧経路に戻す:
```sql
BEGIN;
INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by)
VALUES
  ('search_read_backend', 'meili', 'ops-rollback'),
  ('suggest_read_backend', 'legacy', 'ops-rollback')
ON CONFLICT (flag_name)
DO UPDATE SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by;
COMMIT;
```
2. `search_write_mode='dual'` を維持（切戻し後の再評価に必要）。
3. `/healthz` とスモーク検索を実行して復旧完了を確認。

### ロールバック証跡（必須）
- 実施開始/完了時刻（5分以内を確認）
- 実施者（`updated_by` と一致）
- 原因分類（性能/品質/運用）
- 暫定回避・恒久対策案

## 障害時一次切り分けフロー
1. DB負荷確認: `pg_stat_activity` / lock wait / CPU / IO
2. 拡張障害確認: `pgroonga` / `pg_trgm` / `age` のエラーログ
3. グラフ遅延確認: `index-age-suggest-v1` backlog と affinity 更新遅延
4. 切分け不能時は即ロールバックし、影響範囲を縮小

## 再インデックス手順

### 全量再構築（post search）
```sql
REINDEX INDEX CONCURRENTLY post_search_text_pgroonga_idx;
VACUUM (ANALYZE) cn_search.post_search_documents;
```

### topic 単位再構築（backfill ジョブ）
1. `cn_search.backfill_jobs` に `target='post_search_documents'` の `pending` ジョブを追加。
2. `cn_search.backfill_checkpoints` で進捗確認。
3. 完了後に `processed_rows` と `error_message` を監査ログへ転記。

## 監査ログ確認手順（フラグ変更）
```sql
SELECT flag_name, flag_value, updated_at, updated_by
FROM cn_search.runtime_flags
WHERE flag_name IN ('search_read_backend', 'suggest_read_backend', 'search_write_mode')
ORDER BY updated_at DESC;
```
- `updated_by` が Runbook の作業ID（`ops-cutover-*` / `ops-rollback`）と一致すること。

## 既知障害テンプレート
- 事象:
- 影響範囲:
- 暫定回避:
- 恒久対策:
- 再発防止チェック:

## Meili 依存の段階撤去

### 撤去開始条件
- `search_read_backend=pg` / `suggest_read_backend=pg` を 7-14 日維持
- 品質/性能ゲートを連続達成
- ロールバック訓練（上記手順）が 5 分以内で完了済み

### 撤去ステップ
1. `search_write_mode` を `pg_only` へ変更
2. `shadow_sample_rate` を `0` へ変更（Meili 比較停止）
3. compose/test から `community-node-meilisearch` 依存を削除
4. CI 手順から Meili 起動前提を削除し、PG-only 手順へ更新
5. `.env` / デプロイ設定から Meili 関連値を段階削除

### 撤去後ロールバック
- 重大障害時は Meili サービスを再起動し、`search_read_backend=meili` / `suggest_read_backend=legacy` を再適用。
- write は `dual` に戻してドリフト確認後に再判定する。
