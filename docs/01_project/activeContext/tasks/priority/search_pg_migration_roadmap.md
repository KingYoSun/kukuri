# Search PG Migration ロードマップ（Issue #27）

最終更新日: 2026年02月16日

目的: `docs/01_project/activeContext/search_pg_migration/` の PR-01..PR-07 を、現行実装との差分を埋める単位に分解して管理する。

## PR-01 拡張導入と移行フラグ基盤

- [x] `kukuri-community-node/docker/postgres-age/Dockerfile` に PGroonga 導入手順を追加し、`docker compose` build で再現性を確認する。
- [x] migration を追加し、`pg_trgm` / `pgroonga` / `age` の extension と `cn_search.runtime_flags` を作成する。
- [x] 検索フラグの正本を `cn_search.runtime_flags` に確定し、`cn-user-api` / `cn-index` の読取実装を `cn-core::search_runtime_flags` へ統一する。

## PR-02 投稿検索ドキュメント（PGroonga）

- [x] `cn_search.post_search_documents` と PGroonga index を追加する migration を作成する。
- [x] outbox 消費で `post_search_documents` へ upsert/delete する dual-write（Meili + PG）を実装する。
- [x] `/v1/search` に PG検索実装を追加し、`search_read_backend` で切替可能にする。
- [x] 正規化関数（NFKC/記号処理/normalizer_version）と回帰テストを追加する。

## PR-03 コミュニティ候補生成（pg_trgm + prefix）

- [x] `cn_search.community_search_terms` と `gin_trgm_ops` / `text_pattern_ops` index を作成する migration を追加する。
- [x] 現行 `topic_id` モデルから `community_id` 候補へ変換するデータソース定義を確定する。
- [x] Stage-A 候補生成 API（prefix + trgm）を実装し、入力長別ロジック（1-2文字/3文字以上）をテストで固定する。

## PR-04 AGE グラフ拡張 + 同期

- [x] suggest 用グラフ設計（graph名、Node/Edge、trust 既存グラフとの分離方針）を確定する。
- [x] `cn_search.graph_sync_offsets` / `cn_search.user_community_affinity` を追加し、同期ワーカーを実装する。
- [x] outbox 差分同期 + affinity 再計算ジョブ（checkpoint 再開可能）を追加する。

## PR-05 2段階サジェスト（候補生成 + 再ランキング）

- [x] `/v1/communities/suggest`（または同等内部API）を追加し、Stage-A/Stage-B パイプラインを接続する。
- [x] block/mute/visibility 最終フィルタを SQL 側で適用し、既存仕様との回帰テストを追加する。
- [x] relation score 重みをランタイム設定化し、shadow計測モードを実装する。

## PR-06 dual-write + backfill + shadow-read

- [x] `cn_search.backfill_jobs` / `cn_search.backfill_checkpoints` / `cn_search.shadow_read_logs` を追加する migration を作成する。
- [x] 書込モード `meili_only -> dual -> pg_only` を実装し、片系失敗時の再送と監査ログを追加する。
- [x] shadow-read 比較（overlap@10/latency delta）を保存し、段階的サンプル率制御を実装する。

## PR-07 cutover / runbook

- [ ] `search_read_backend` / `suggest_read_backend` のカナリア切替手順を Runbook 化する。
- [ ] PG検索向け監視項目（latency/index lag/zero-result/filter drop）をダッシュボードへ追加する。
- [ ] Meili 依存の compose/test/env を段階撤去する条件と手順を Runbook に明文化する。

## 共通ゲート（着手前に固定）

- [ ] 品質ゲートの閾値（overlap@10, NDCG@10, P95 latency）を文書で固定する。
- [ ] rollback 手順（5分以内復旧）を運用コマンド単位で検証し、証跡を残す。
