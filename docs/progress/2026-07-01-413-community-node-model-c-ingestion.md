# 2026-07-01 #413 community node ingestion (Model C) + supported-topic scope / indexing request

参照: `docs/adr/0025-community-node-indexing-foundation.md`（§2.2 / §2.5 / §6）

## 実装した範囲

Model C ingestion（docs replica sync participant）と supported-topic scope / indexing request /
relay validation を実装した。seam は「ingest → 投影 + 投影レベル read」まで。ユーザー向け
search / discovery / recommendation 本体と fail-closed query gate は #404。

- **`cn-core`（scope 管理 state）**
  - `cn_index` schema（`supported_topics` / `indexing_requests` / `channel_secrets`）を migration
    `202607010001_index_scope.sql` で追加（すべて additive。既存テーブル非改変）。
  - `index_scope.rs`: supported set / request / channel capability の CRUD。
  - channel capability（namespace secret）は XChaCha20Poly1305 で **at-rest 暗号化**。AAD に
    `channel_id` を束縛し、DB 行の取り違え / コピーによる別 channel への化けを防ぐ。復号鍵は runtime
    供給（`COMMUNITY_NODE_CHANNEL_SECRET_KEY`）で DB に置かない。
  - user 経路の登録は `register_channel_secret`（first-writer-wins）: 同一 secret 再提示は冪等、別
    secret での上書き（capability 乗っ取り）は `ChannelSecretConflict` で拒否。operator 経路は従来の
    `upsert_channel_secret`（無条件 upsert）。
  - env パースヘルパ（`parse_bool_env` / `parse_csv_env`）を `cn-core` に集約し community-node サービス
    間の drift を防止。
- **`cn-cli`**: `supported-topic add/remove/list`、`indexing-request list/approve/reject`。approve で
  対象 scope が supported set に入る。
- **`cn-user-api`**: `POST /v1/indexing/requests`（認証 + consent）。private channel は channel secret
  提示必須（提示できること自体が権限の証明。ADR 0025 §6.3）。secret 無し=400、暗号鍵未設定 node=404、
  別 secret での乗っ取り=409。
- **`cn-indexer`（新 crate/binary）**: docs replica sync participant + relay validation 起動 gate +
  ingest pipeline + ArcadeDB 投影 adapter。
  - relay gate（config 検査）: 自前 relay（`iroh_relay` capability）も外部 relay URL も未設定なら
    indexing を起動しない（fail-closed。ADR 0025 §6.4）。
  - ingest: 共有 replica の実在 post entry のみ scan→`allow` verdict のみ ArcadeDB 投影へ書く。
    unscanned / scan_failed / 非 allow は投影しない。tombstone / deleted / supported 除去 / channel
    secret 失効は de-index。blob は scan 用一時 fetch のみ（no permanent blob storage）。
  - index 投影は ArcadeDB（Lucene 全文のみ。canonical ではない写像。ADR 0026 §6.1 の relation backend に
    相乗り）。ベクトル検索は延期（ADR 0025 §4 画像類似除外と整合）。

## デプロイ順序（重要）

`ensure_database_ready`（`DatabaseInitMode::RequireReady`）が新テーブル `cn_index.supported_topics` /
`cn_index.indexing_requests` / `cn_index.channel_secrets` の存在を要求する。既に ready 状態の本番 DB は
本 migration を適用するまで RequireReady 起動が失敗する（`cn-user-api` / `cn-indexer` 双方）。
ロールアウト時は **新バイナリの RequireReady 起動より前に migration（Prepare / migrate 手順）を適用**する
こと。既存 readiness gate と同じ fail-closed 挙動であり意図的（#405 と同じ運用）。

`cn-indexer` は起動時に relay validation gate も通す。自前 relay / 外部 relay のどちらかを設定して
から起動する。`COMMUNITY_NODE_CHANNEL_SECRET_KEY` は `cn-user-api` と `cn-indexer` で同一値を渡す。

## 維持した境界（本 PR に含まない）

- ユーザー向け search / discovery / recommendation 本体 + fail-closed query gate（#404）。
- 実 ingest loop の常駐化。safety provider（#391 / #411）が実装され `CommunityIndex` が昇格するまで
  `cn-indexer` runtime は relay gate + scope state 準備確認までに留める（`Availability::Planned` と整合）。
- ArcadeDB relation graph 本体 / co-participation 反映（#415）。ベクトル / 画像類似検索、VLM タグ生成（#411）。
- `CommunityIndex` capability は引き続き `Availability::Planned`。
