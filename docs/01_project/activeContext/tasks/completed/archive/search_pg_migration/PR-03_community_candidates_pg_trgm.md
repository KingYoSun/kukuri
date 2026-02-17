# PR-03: コミュニティ候補生成（pg_trgm + prefix）
最終更新: 2026年02月16日

## 目的
- コミュニティ名/別名サジェストの候補生成を PostgreSQL に移し、短い入力や typo に強くする。

## 変更内容
- `cn_search.community_search_terms` を追加。
- `pg_trgm` と prefix 用 `text_pattern_ops` インデックスを追加。
- 入力長に応じて候補生成ロジックを切り替え。
- 候補生成 API を 2 段階サジェストの Stage-A として独立実装。

## DDL/インデックス
```sql
CREATE TABLE IF NOT EXISTS cn_search.community_search_terms (
    community_id TEXT NOT NULL,
    term_type TEXT NOT NULL CHECK (term_type IN ('name', 'alias')),
    term_raw TEXT NOT NULL,
    term_norm TEXT NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (community_id, term_type, term_norm)
);

CREATE INDEX IF NOT EXISTS community_term_trgm_idx
ON cn_search.community_search_terms
USING gin (term_norm gin_trgm_ops);

CREATE INDEX IF NOT EXISTS community_term_prefix_idx
ON cn_search.community_search_terms (term_norm text_pattern_ops);
```

## 候補生成クエリ（擬似）
```sql
SELECT community_id,
       MAX(CASE WHEN term_norm LIKE :q_norm || '%' THEN 1 ELSE 0 END) AS prefix_hit,
       MAX(similarity(term_norm, :q_norm)) AS trgm_score
FROM cn_search.community_search_terms
WHERE term_norm LIKE :q_norm || '%'
   OR term_norm % :q_norm
GROUP BY community_id
ORDER BY prefix_hit DESC, trgm_score DESC
LIMIT :candidate_n;
```

## 入力長別戦略
- 1-2 文字: prefix を優先し、trgm は補助。
- 3 文字以上: prefix + trgm のハイブリッド。
- 完全一致がある場合は上位固定（安定性向上）。

## 移行/バックフィル手順
1. `communities.name` と `aliases[]` を展開し正規化して投入。
2. `community_id` 範囲でチャンク実行し checkpoint を記録。
3. 以後は communities 更新イベントで差分同期する。
4. 同期失敗時は対象 `community_id` のみ再投入可能にする。

## ロールバック
- `suggest_read_backend=legacy` に戻す。
- Stage-A 新実装は停止し、既存候補生成経路へ戻す。

## テスト/計測
- typo パターン（欠落/置換/転置）で Recall@10 を測定。
- 1-3 文字入力で P95 を計測。
- 多言語別名（日本語、英語、混在）で候補再現率を検証。

## 運用監視
- `suggest_candidate_latency_ms`。
- `suggest_candidate_count`（入力長ごとの平均/最小）。
- `suggest_no_candidate_rate`。

