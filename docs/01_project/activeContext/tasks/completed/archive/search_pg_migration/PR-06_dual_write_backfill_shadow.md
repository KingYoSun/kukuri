# PR-06: dual-write + backfill + シャドーリード
最終更新: 2026年02月16日

## 目的
- 低ダウンタイムで移行し、品質劣化なしを実測で確認する。
- 失敗時は即時で旧検索へ戻せる状態を維持する。

## 変更内容
- 書込モードを `meili_only -> dual -> pg_only` で段階切替。
- backfill ジョブ基盤を追加し、再実行・進捗管理を可能にする。
- shadow-read で同一クエリの結果と遅延を比較保存する。

## DDL/インデックス
```sql
CREATE TABLE IF NOT EXISTS cn_search.backfill_jobs (
    job_id TEXT PRIMARY KEY,
    target TEXT NOT NULL,
    status TEXT NOT NULL, -- pending/running/succeeded/failed
    high_watermark_seq BIGINT NULL,
    processed_rows BIGINT NOT NULL DEFAULT 0,
    error_message TEXT NULL,
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_search.backfill_checkpoints (
    job_id TEXT NOT NULL,
    shard_key TEXT NOT NULL,
    last_cursor TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (job_id, shard_key)
);

CREATE TABLE IF NOT EXISTS cn_search.shadow_read_logs (
    id BIGSERIAL PRIMARY KEY,
    endpoint TEXT NOT NULL,
    user_id TEXT NOT NULL,
    query_norm TEXT NOT NULL,
    meili_ids TEXT[] NOT NULL,
    pg_ids TEXT[] NOT NULL,
    overlap_at_10 DOUBLE PRECISION NOT NULL,
    latency_meili_ms INT NOT NULL,
    latency_pg_ms INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## 移行/バックフィル手順
1. `search_write_mode=dual`、`search_read_backend=meili` に設定。
2. `high_watermark_seq = MAX(cn_relay.events_outbox.seq)` を採取。
3. high-watermark 以前をチャンク分割で backfill。
4. high-watermark 以後は dual-write 差分で追随。
5. drift が 0 近傍になるまで catch-up を繰り返す。
6. shadow-read を 5% -> 25% -> 50% と拡大。
7. overlap/NDCG/latency 基準を満たしたら cutover 判定へ進む。

## バッチ設計（再実行性）
- チャンクキー: `topic_id + created_at + event_id`。
- 書込方式: `INSERT ... ON CONFLICT DO UPDATE`。
- checkpoint 保存頻度: 1 チャンクごと。
- 失敗時: 失敗チャンクのみ再実行。

## ロールバック
- 即時: `search_read_backend=meili`。
- 安全: write は `dual` を維持し、復旧後の再評価を容易にする。
- 重障害: `search_write_mode=meili_only` へ戻す。

## テスト/計測
- backfill の停止再開テスト。
- shadow-read 比較一致性テスト。
- dual-write 片系失敗時の再送・整合回復テスト。

## 運用監視
- `backfill_processed_rows`、`backfill_eta_seconds`。
- `shadow_overlap_at_10`。
- `shadow_latency_delta_ms`。
- dual-write エラー率と再試行回数。

