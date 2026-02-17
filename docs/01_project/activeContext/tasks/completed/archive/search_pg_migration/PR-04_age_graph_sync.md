# PR-04: AGE グラフスキーマ拡張 + 同期
最終更新: 2026年02月16日

## 目的
- コミュニティサジェスト再ランキングに必要な関係グラフを整備する。
- オンラインの Cypher 計算コストを抑えるため、事前集計テーブルを導入する。

## 変更内容
- AGE グラフに `Community` ノードと user-community 関係エッジを追加。
- 既存 outbox 消費に membership/follow/view/friend シグナル同期を追加。
- `cn_search.user_community_affinity` を導入し事前集計を定期更新。

## ノード/エッジ設計
- `(:User {id})`
- `(:Community {id})`
- `(:User)-[:MEMBER_OF {updated_at, weight: 1.0}]->(:Community)`
- `(:User)-[:FOLLOWS_COMMUNITY {updated_at, weight: 0.7}]->(:Community)`
- `(:User)-[:VIEWED_COMMUNITY {last_seen_at, weight: 0.2}]->(:Community)`
- `(:User)-[:FOLLOWS_USER {updated_at, weight: 0.5}]->(:User)`

## DDL/インデックス
```sql
CREATE TABLE IF NOT EXISTS cn_search.graph_sync_offsets (
    consumer TEXT PRIMARY KEY,
    last_seq BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_search.user_community_affinity (
    user_id TEXT NOT NULL,
    community_id TEXT NOT NULL,
    relation_score DOUBLE PRECISION NOT NULL,
    signals_json JSONB NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, community_id)
);

CREATE INDEX IF NOT EXISTS user_community_affinity_score_idx
ON cn_search.user_community_affinity (user_id, relation_score DESC, computed_at DESC);
```

## Cypher 例（候補限定）
```sql
SELECT *
FROM cypher(
  'kukuri_cn',
  $$
    MATCH (u:User {id: $viewer})-[r]->(c:Community)
    WHERE c.id IN $candidate_ids
    RETURN c.id AS community_id, sum(r.weight) AS raw_score
  $$
) AS (community_id agtype, raw_score agtype);
```

## 移行/バックフィル手順
1. 全 membership/follow/friend/view 履歴を読み込んで初期グラフ構築。
2. `MAX(seq)` を high-watermark に取り、以後は outbox 差分で追随。
3. `user_community_affinity` を 1-5 分周期で再計算する。
4. 更新対象を「直近アクティブユーザー + 影響コミュニティ」に限定する。

## ロールバック
- Stage-B（関係性再ランキング）を無効化し Stage-A のみで返す。
- グラフ同期ワーカー停止後も候補生成機能は継続できる。
- 再開時は `graph_sync_offsets` から再同期する。

## テスト/計測
- エッジ upsert/delete の冪等性テスト。
- 再計算ジョブの再実行テスト。
- 大規模候補時の Cypher 呼び出し時間を計測。

## 運用監視
- `graph_sync_lag_seq`。
- `affinity_recompute_duration_ms`。
- `affinity_freshness_seconds`。

