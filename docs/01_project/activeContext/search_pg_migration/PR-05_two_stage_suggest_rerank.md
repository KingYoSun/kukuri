# PR-05: サジェスト 2 段階クエリ（候補生成 + 関係性再ランキング）
最終更新: 2026年02月16日

## 目的
- コミュニティサジェストを 2 段階化し、入力補完精度と関係性適合率を同時に改善する。
- ブロック/ミュート/権限フィルタを必ず SQL 側で最終適用する。

## 変更内容
- Stage-A: PR-03 の候補生成で topN を取得。
- Stage-B: `user_community_affinity` と必要時 AGE 補完で再ランキング。
- 最終フィルタで block/mute/visibility を適用。
- `/v1/communities/suggest`（または内部呼出）を新パイプラインへ接続。

## 関係性スコア式（初期値）
```text
relation_score =
  1.20 * is_member
+ 0.80 * is_following_community
+ 0.35 * min(1, friends_member_count / 5)
+ 0.25 * min(1, two_hop_follow_count / 10)
+ 0.15 * exp(-hours_since_last_view / 168)
- 1.00 * is_muted_or_blocked
```

```text
final_suggest_score =
  0.40 * name_match_score
+ 0.45 * relation_score
+ 0.10 * global_popularity
+ 0.05 * recency_boost
```

## SQL 適用方針（擬似）
```sql
WITH candidate AS (
  -- PR-03 の候補生成
),
rank_base AS (
  SELECT c.community_id,
         c.name_match_score,
         COALESCE(a.relation_score, 0.0) AS relation_score
  FROM candidate c
  LEFT JOIN cn_search.user_community_affinity a
    ON a.user_id = :viewer_id
   AND a.community_id = c.community_id
),
filtered AS (
  SELECT r.*
  FROM rank_base r
  JOIN communities cm ON cm.id = r.community_id
  WHERE NOT EXISTS (
      SELECT 1
      FROM blocks b
      WHERE b.muter_id = :viewer_id
        AND (b.community_id = cm.id OR b.muted_id = cm.owner_id)
  )
    AND (
      cm.visibility = 'public'
      OR EXISTS (
        SELECT 1
        FROM community_members m
        WHERE m.user_id = :viewer_id
          AND m.community_id = cm.id
      )
    )
)
SELECT *
FROM filtered
ORDER BY (
  0.40 * name_match_score +
  0.45 * relation_score +
  0.10 * :global_popularity +
  0.05 * :recency_boost
) DESC
LIMIT :limit;
```

## オンライン計算コスト対策
- 既定は `user_community_affinity` 参照のみ。
- 欠損候補のみ AGE 補完（候補 ID 限定）。
- 任意で `viewer_id + q_norm` の短TTLキャッシュ（30-60 秒）。

## 移行/バックフィル手順
1. まず Stage-A のみ本番有効化。
2. Stage-B は shadow 計測のみでログ収集。
3. 指標合格後に Stage-B 有効化し重み調整をフラグ化する。

## ロールバック
- `suggest_read_backend=legacy` で即時復旧。
- あるいは `relation_weight=0` として Stage-A 単独運用に戻す。

## テスト/計測
- Golden set で NDCG@10 / MRR / Recall@10 を測定。
- 短文字列、typo、混在言語で順位安定性を検証。
- ブロック/ミュート/権限の回帰テストを必須化。

## 運用監視
- `suggest_stage_a_latency_ms`。
- `suggest_stage_b_latency_ms`。
- `suggest_block_filter_drop_count`。

