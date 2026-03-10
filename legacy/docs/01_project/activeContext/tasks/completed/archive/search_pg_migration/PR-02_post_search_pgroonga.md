# PR-02: 投稿検索ドキュメント導入（PGroonga）
最終更新: 2026年02月16日

## 目的
- 投稿本文検索を PostgreSQL 内で実現する。
- 多言語混在投稿を前提に、正規化・インデックス・ランキングを実装する。

## 変更内容
- `cn_search.post_search_documents` を追加。
- `post_search_documents` の一意キーを `(post_id, topic_id)` とし、同一 `post_id` の multi-topic 行を保持する。
- 既存 outbox 消費時に検索ドキュメントを upsert/delete する処理を追加。
- 検索語正規化と文書正規化を同一関数で統一する。
- `/v1/search` に PG 実装を追加し、フラグで read 経路を選択可能にする。

## DDL/インデックス
```sql
CREATE TABLE IF NOT EXISTS cn_search.post_search_documents (
    post_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    visibility TEXT NOT NULL,
    body_raw TEXT NOT NULL,
    body_norm TEXT NOT NULL,
    hashtags_norm TEXT[] NOT NULL DEFAULT '{}',
    mentions_norm TEXT[] NOT NULL DEFAULT '{}',
    community_terms_norm TEXT[] NOT NULL DEFAULT '{}',
    search_text TEXT NOT NULL,
    language_hint TEXT NULL,
    popularity_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    normalizer_version SMALLINT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (post_id, topic_id)
);

CREATE INDEX IF NOT EXISTS post_search_text_pgroonga_idx
ON cn_search.post_search_documents
USING pgroonga (search_text);

CREATE INDEX IF NOT EXISTS post_search_topic_created_idx
ON cn_search.post_search_documents (topic_id, created_at DESC);

CREATE INDEX IF NOT EXISTS post_search_visibility_idx
ON cn_search.post_search_documents (visibility, is_deleted);
```

- migration 履歴:
  - `20260216030000_m7_post_search_documents.sql` でテーブルを導入。
  - `20260216040000_m8_post_search_documents_topic_key.sql` で主キーを `(post_id, topic_id)` へ更新。

## 正規化方針（多言語混在）
- NFKC 正規化（全角/半角ゆれ吸収）。
- 小文字化（ケース概念のある言語）。
- 制御文字除去、連続空白圧縮。
- `#` と `@` は保持、その他記号は空白化。
- `normalizer_version` により再正規化を可能にする。

## ランキング指標（合成）
```text
final_score =
  0.55 * text_score
+ 0.25 * freshness_score
+ 0.20 * popularity_score_norm
```

```sql
SELECT d.post_id,
       (
         0.55 * pgroonga_score(tableoid, ctid) +
         0.25 * exp(-((:now - d.created_at) / 3600.0) / 72.0) +
         0.20 * LEAST(1.0, LN(1 + d.popularity_score) / LN(101.0))
       ) AS final_score
FROM cn_search.post_search_documents d
WHERE d.topic_id = :topic_id
  AND d.is_deleted = FALSE
  AND d.search_text &@~ :query_norm
  AND NOT EXISTS (
      SELECT 1
      FROM blocks b
      WHERE b.muter_id = :viewer_id
        AND b.muted_id = d.author_id
  )
ORDER BY final_score DESC, d.created_at DESC
LIMIT :limit OFFSET :offset;
```

## 移行/バックフィル手順
1. high-watermark として `MAX(cn_relay.events_outbox.seq)` を取得。
2. `created_at,event_id` 昇順のチャンクで `cn_relay.events(kind=1)` を取り込み。
3. `ON CONFLICT (post_id, topic_id) DO UPDATE` で冪等 upsert する。
4. high-watermark 以降は outbox dual-write で追従。
5. backlog が 0 になるまで catch-up を繰り返す。

## ロールバック
- `search_read_backend=meili` に戻す。
- 書込は `search_write_mode=dual` を維持し、再切替しやすくする。
- 必要時のみ `search_write_mode=meili_only` に戻す。

## テスト/計測
- 正規化ユニットテスト（混在言語、絵文字、全半角、記号）。
- 検索統合テスト（topic 制約、ブロック反映、ページング）。
- 負荷試験で P50/P95、更新反映遅延、index サイズを計測。

## 運用監視
- `search_query_latency_ms`（P50/P95/P99）。
- `search_index_lag_seconds`（イベント発生から検索反映まで）。
- `search_zero_result_rate`（クエリ長別）。
