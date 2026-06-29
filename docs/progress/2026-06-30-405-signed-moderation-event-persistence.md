# #405: signed moderation event の永続化・実鍵署名・配布

最終更新日: 2026-06-30

## 位置づけ

Issue #405（#353 safety foundation の続き）として、signed moderation event の secp256k1 実鍵署名・
DB 永続化、risk signal の永続化と visibility 配布境界を実装した。#353 段階で mock signer のみ・
未署名 artifact 生成までだった部分を、本番署名 + 永続化 + 配布境界まで進めた。

設計の真実源:
- `docs/safety/community-node-critical-safety.md`（§9 signed events / risk signals, §12 prerequisites）
- `docs/architecture/moderation-event-trust-semantics.md`（advisory ≠ command, visibility 3段階）

## 実装範囲

### canonical serialization の安定化（`cn-safety`）

- `ModerationEventBody::canonical_json()` を、object キーを再帰的に辞書順ソートしてから
  シリアライズする実装に変更（`canonicalize_json_value`）。struct のフィールド宣言順や
  `serde_json` の `preserve_order` feature 有無に依存しないクロス実装・クロスバージョン安定 canonical。
- golden byte vector とフィールド順序非依存テストで固定（`tests/domain_model.rs`）。

### 実鍵署名 / 検証（`cn-safety-runtime`）

- `Secp256k1ModerationEventSigner`（本番 `ModerationEventSigner` 実装）。
  - 署名対象は `sha256(body.canonical_bytes())`。`kukuri-core` の secp256k1 schnorr を再利用。
  - `issuer_node_id()` = 署名鍵の x-only 公開鍵 hex。`Debug` で秘密鍵を出さない。
  - `from_secret`（hex / `nsec`）/ `from_env`（`COMMUNITY_NODE_SAFETY_SIGNING_KEY`）。
    空 / placeholder（`change-me`）/ 不正鍵は `SignerKeyError` で拒否。
- `verify_signed_event` … `body.issuer_node_id` を公開鍵として canonical digest の schnorr 署名を検証。
  body 改竄 / 別鍵署名 / issuer 詐称 / 不正 encoding をそれぞれ別エラーで検出。

### 永続化 / 配布境界（`cn-core`）

- migration `202606300001_safety_events.sql`:
  - `cn_safety` schema 新設。
  - `cn_safety.signed_moderation_events`（body 各列 + `labels` JSONB + `signature` + 原文
    `event_created_at` TEXT）。`event_created_at` は署名対象なので timestamptz へ正規化せず原文保持し、
    ロード後も署名検証が通る。
  - `cn_safety.risk_signals`（`expires_at` は RFC3339 TEXT、配布時に `::timestamptz` で比較）。
  - index: target、および配布クエリ（filter=visibility → sort=persisted_at DESC）と新着順一覧の
    双方をカバーする複合 index `(visibility, persisted_at DESC)`。
- `ensure_database_ready` に両テーブルを登録。
- `safety_events.rs` storage:
  - `persist_signed_moderation_event`（保存前に `verify_signed_event` で署名検証 = trust boundary、
    event id 冪等、空/空白 target_id 拒否）/ `get_signed_moderation_event` /
    `list_signed_moderation_events`。
  - `persist_risk_signal`（空/空白 target_id 拒否）/ `get_risk_signal` /
    `list_risk_signals_for_target`。
  - 配布境界クエリ: `list_distributable_moderation_events` /
    `list_distributable_risk_signals`。`DistributionAudience`（`SubscribedNodes` / `Public`）で
    `local` を必ず除外し、risk signal は `expires_at` 失効分を除外する。
  - enum 列は `cn-safety` の serde（snake_case）を経由して文字列化・復元し、列値と canonical の
    drift を防ぐ。

### デプロイ順序（重要）

`ensure_database_ready`（`DatabaseInitMode::RequireReady`）が新テーブル
`cn_safety.signed_moderation_events` / `cn_safety.risk_signals` の存在を要求する。既に ready 状態の
本番 DB は、本 migration を適用するまで RequireReady 起動が失敗する。ロールアウト時は **新バイナリの
RequireReady 起動より前に migration（Prepare / migrate 手順）を適用**すること。既存の readiness gate
と同じ挙動（fail-closed）であり、意図的。

### 署名鍵プロビジョニング（`cn-operator`）

- `safety.events.signing_key_secret_id`（Secret Manager secret ID。値ではない）を config に追加・検証。
- readiness check `signing_key_secret_configured` を追加（`READINESS_CHECK_IDS` を 11→12）。
- `generate-tfvars` に `safety_signing_key_secret_id` を出力（値は出さず ID のみ）。
- SAMPLE_CONFIG / 既存テストを更新。

## 維持した境界（本 PR に含まない）

- 本番 ingest / index 経路への orchestrator 配線（scan→sign→persist の end-to-end 自動実行）。
- P2P network への実配布（subscribed_nodes / public の実送出）。配布境界は storage クエリまで。
- terraform `.tf` モジュール本体の改修（tfvars 出力のみ）。
- `cn-safety` は DB / network / credential 非依存を維持（実鍵署名は `cn-safety-runtime`、
  永続化は `cn-core`）。`Moderation` / `CommunityIndex` / `CommunityLocalTrust` は引き続き
  `Availability::Planned`。

## 受け入れ条件（#405）との対応

- [x] signed moderation event を生成・保存できる（保存部分）… 実鍵署名 + `persist_signed_moderation_event`
  （保存前に署名検証）+ 冪等。
- [x] trustness / relation に risk signal を反映できる（永続化・配布部分）… `persist_risk_signal` +
  visibility / expiry 配布境界クエリ。

## 検証

- `cargo test -p kukuri-cn-safety`（domain_model 19 / policy_router 21）: pass。
- `cargo test -p kukuri-cn-safety-runtime`（signer 8 含む）: pass。
- `cargo test -p kukuri-cn-operator`（safety 18 含む）: pass。
- `cargo check -p kukuri-cn-safety -p kukuri-cn-safety-runtime --no-default-features`（mock 無し production build）: pass。
- `cargo clippy -p kukuri-core -p kukuri-cn-safety -p kukuri-cn-safety-runtime -p kukuri-cn-core -p kukuri-cn-operator --all-targets --all-features -- -D warnings`: clean。
- `cargo fmt`（5 crate）--check: clean。
- `cargo xtask cn-check`: pass。
- `cargo xtask cn-test`（Postgres harness）: pass。`tests/safety_events.rs` の 4 integration test
  （署名・永続化・冪等・ロード後検証・改竄 event の保存拒否 / moderation event 配布境界 / risk signal 配布境界 + 失効 /
  空 target_id 拒否）が実 DB で pass。
