# PR-07: 切替・監視・運用 Runbook
最終更新: 2026年02月16日

## 目的
- 本番 read を PostgreSQL 検索へ切替え、安定運用へ移行する。
- 切替後の監視・障害対応・段階撤去手順を運用手順として固定化する。

## 変更内容
- `search_read_backend=pg`、`suggest_read_backend=pg` へ段階切替。
- ダッシュボードを PG 検索中心のメトリクスへ更新。
- 既存 reindex 運用を PG ドキュメント再構築手順へ更新。
- 安定期間後に Meili 依存（compose/test/env）を段階撤去。

## DDL/インデックス
- 原則 DDL 追加なし。
- 運用コマンドとして以下を Runbook 化する。

```sql
REINDEX INDEX CONCURRENTLY post_search_text_pgroonga_idx;
VACUUM (ANALYZE) cn_search.post_search_documents;
```

## 切替手順
1. カナリア 5% で `search_read_backend=pg`。
2. 24h 監視し SLO/品質合格なら 25% へ拡大。
3. 50%、100% と同様に段階適用。
4. 100% 後も 7-14 日は Meili を standby で保持。
5. 問題なしを確認後に `meilisearch` サービスと関連 env を削除する。

## ロールバック
- 5 分以内に `search_read_backend=meili`、`suggest_read_backend=legacy` へ戻す。
- write は `dual` を維持しデータ欠落を防ぐ。
- rollback 実施後は原因分類（性能・品質・運用）を記録する。

## テスト/計測
- E2E: 検索、サジェスト、ブロック/ミュート、権限ケース。
- 負荷: ピーク想定の 1.5 倍トラフィックでレイテンシ計測。
- 長時間: index サイズ増加、autovacuum、reindex 実行時間を観測。

## 運用監視
- 検索: `p50/p95/p99 latency`, `error rate`, `zero-result rate`。
- 更新反映: `index lag`, `outbox backlog`。
- サジェスト: stage 別レイテンシ、候補数、filter drop。
- ストレージ: index/table サイズ増加率、vacuum 実行遅延。

## Runbook 追記項目
- 障害時一次切分けフロー（DB負荷、拡張障害、グラフ遅延）。
- 再インデックス手順（全量/topic 単位）。
- 監査ログ確認手順（フラグ変更者、変更時刻）。
- 既知障害テンプレート（事象、暫定回避、恒久対策）。

