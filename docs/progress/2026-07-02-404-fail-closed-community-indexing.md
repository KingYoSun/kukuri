# 2026-07-02 #404 fail-closed community indexing 本体 + search/discovery/recommendation 除外

参照: `docs/adr/0025-community-node-indexing-foundation.md`（§2.5 fail-closed / §2.6 二重ゲート /
§2.7 検索 UX）、`docs/adr/0027-deterministic-moderation-critical-safety.md`（§2.4 fail-closed
invariants）、`docs/progress/2026-07-01-413-community-node-model-c-ingestion.md`（#413 seam）

## 実装した範囲

#413 の seam（ingest → 投影まで）の続きとして、ユーザー向け search / discovery / recommendation の
query 境界と、fail-closed 不変条件の **DB 制約**による保証を実装した。

- **`cn-core`（verdict state + index 真実源）** — migration `202607020001_scan_verdicts_index_entries.sql`
  - `cn_safety.scan_verdicts`: scan 対象ごとの**最新** verdict（`allow` を含む）。対象ごとに 1 行を
    upsert し、id は初回採番のまま据え置く。`SafetyScanService::scan_and_record`（#406）が毎 scan で
    upsert する（`SafetyArtifactStore::persist_verdict`）。
  - `cn_index.index_entries`: index された entry の**真実源**。fail-closed 不変条件を DB 制約で固定:
    - `verdict_id` NOT NULL + FK → `scan_verdicts`（**verdict 無しの index entry を作らない**。
      unscanned content は verdict 行が無いため構造的に index 不能）
    - `CHECK (verdict_action = 'allow')`（hold / quarantine / exclude / scan_failed /
      provider_unavailable は書き込み自体が拒否される）
    - `CHECK (NOT critical)`（critical verdict は search / discovery / recommendation のどこにも入らない）
  - `filter_surfaceable_objects`: query 境界の突合。真実源に存在し、`verdict_id` join で見た**現在の**
    verdict が `allow` かつ非 critical の hit のみ通す。verdict 行は upsert（同一 id）のため、この join は
    常に最新 verdict を見る（de-index の遅延に依存しない）。
- **`cn-indexer`（二段書き込み + query 境界）**
  - `IngestPipeline` を「① 真実源 upsert（Postgres、DB 制約が backstop）→ ② 投影 upsert（ArcadeDB）」の
    二段書き込みに変更。① 失敗時は投影しない。de-index（tombstone / verdict 変化 / scope 除去）は
    真実源 → 投影の順で両方から消す。
  - `query.rs`: `IndexQuery` trait（`search_scope` = topic 内検索 / `search_all` = supported set 横断検索 /
    `list_recent` = discovery・recommendation 用の新着列挙）。ArcadeDB 実装は Lucene 全文
    （`SEARCH_INDEX('IndexedEntry[text]', :query)`）、テストは in-memory 実装。
  - `FailClosedIndexQuery`: **唯一のユーザー向け読み口**。投影 hit を `filter_surfaceable` で突合し、
    真実源に無い / 非 allow / critical の hit を結果から落とす（エラーにせず返さない側へ倒す）。
    limit は `MAX_QUERY_LIMIT`（100）に丸め、突合コストを有界にする。
- **`cn-user-api`（read エンドポイント）**
  - `GET /v1/index/search`（`q` 必須。`scope_kind`+`scope_id` で topic 内、無指定で横断。ADR 0025 §2.7）、
    `GET /v1/index/discovery`（scope 指定可の新着列挙）、`GET /v1/index/recommendations`（横断新着）。
  - すべて認証（bearer）+ consent + 既存 rate limit を通る。
  - **既定無効**: `COMMUNITY_NODE_INDEX_QUERY_ENABLED`（既定 false）で gate し、無効時は 404
    （`INDEX_QUERY_NOT_CONFIGURED`）。`CommunityIndex` capability が `Availability::Planned` の
    現状と整合する。有効時は ArcadeDB（`COMMUNITY_NODE_ARCADEDB_*`、cn-indexer と同じ env）へ接続する。

## 受け入れ条件（issue #404）との対応

- 「scan 前 media を index しない」— media manifest / attachment を持つ post は media safety scan が
  実装されるまで pipeline 層で de-index し、真実源・投影へ書かない（`scan_before_media_is_not_indexed`）。
  index entry は verdict 行への NOT NULL FK を必ず持つため、scan されていない content は DB 層でも
  index 不能（`index_entries` FK 制約テスト）。unscanned / scan_failed / provider_unavailable は
  `SafetyVerdict::is_indexable()`（単一判定点）でも投影されない（`index_excludes_unscanned_and_scan_failed` /
  `provider_unavailable_is_never_allowed_and_not_indexed`）。
- 「`allow` 以外の verdict が search / discovery / recommendation に入らない」— 三層で保証:
  ① DB CHECK（非 allow / critical は insert 不能）② ingest の verdict gate（非 allow は投影せず
  de-index）③ query gate（`search_discovery_recommendation_excludes_non_allow`。投影残留や
  verdict 変化後の未 de-index も突合で落ちる）。

## デプロイ順序（重要）

`ensure_database_ready`（`RequireReady`）が新テーブル `cn_safety.scan_verdicts` /
`cn_index.index_entries` の存在を要求する。**新バイナリの RequireReady 起動より前に migration
（Prepare / migrate 手順）を適用**すること（#405 / #413 と同じ fail-closed 運用）。

新 env: `COMMUNITY_NODE_INDEX_QUERY_ENABLED`（cn-user-api、既定 false）。有効化する場合は
`COMMUNITY_NODE_ARCADEDB_*` も cn-user-api に渡す（cn-indexer と同一値）。

## 維持した境界（本 PR に含まない）

- `CommunityIndex` / `Moderation` の `Availability::Planned` → 昇格（issue 記載どおり別途判断）。
  それに伴う ingest loop の常駐化・`/v1/index/*` の既定有効化も同判断に紐づけて据え置き。
- ranking / recommendation アルゴリズム・関連度スコアリングの具体（ADR 0025 §4 でスコープ外。
  discovery / recommendation は created_at 降順の新着列挙が最小 surface）。
- media 派生タグの生成と media タグ検索（#411 / #420 の VLM 実装に依存。投影の `text` にタグを載せる
  前提は維持）。ベクトル / 画像類似検索（ADR 0025 §4）。
