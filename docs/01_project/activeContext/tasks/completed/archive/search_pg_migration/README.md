# kukuri 検索基盤移行計画（PGroonga + Apache AGE + pg_trgm）
最終更新: 2026年02月16日

## 目的
- 検索基盤を Meilisearch 依存から PostgreSQL 拡張中心へ移行し、運用を簡素化する。
- 投稿検索は PGroonga、コミュニティサジェスト候補生成は pg_trgm、関係性再ランキングは Apache AGE で実装する。
- 低ダウンタイムのため `dual-write + backfill + shadow-read + cutover` で段階移行する。

## 前提マッピング
- 既存の投稿実体は主に `cn_relay.events(kind=1)`。
- トピック紐付けは `cn_relay.event_topics`。
- 既存の `cn-index`（outbox 消費・再インデックス）と `cn-trust`（AGE グラフ）を拡張して活用する。

## ファイル構成
- `PR-01_extensions_and_flags.md`
- `PR-02_post_search_pgroonga.md`
- `PR-03_community_candidates_pg_trgm.md`
- `PR-04_age_graph_sync.md`
- `PR-05_two_stage_suggest_rerank.md`
- `PR-06_dual_write_backfill_shadow.md`
- `PR-07_cutover_runbook.md`
- `appendix_risks_open_points_poc.md`

## 実装順
1. PR-01: 拡張導入とフラグ基盤
2. PR-02: 投稿検索ドキュメントと PGroonga
3. PR-03: コミュニティ候補生成（pg_trgm）
4. PR-04: AGE グラフ同期と事前集計
5. PR-05: 2段階サジェスト実装
6. PR-06: dual-write/backfill/shadow-read
7. PR-07: 切替・運用 Runbook・段階撤去

## 運用判断ゲート（共通）
- 機能ゲート: ブロック/ミュート/可視性が既存挙動と一致。
- 品質ゲート: shadow-read 比較で overlap/NDCG が基準を超える。
- 性能ゲート: P95 レイテンシ、反映遅延、エラー率が SLO 内。

