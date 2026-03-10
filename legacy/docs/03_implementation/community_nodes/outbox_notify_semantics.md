# outbox/NOTIFY 配信セマンティクス（v1）

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`（relay → index/moderation/trust）

## 目的

- relay から下流サービス（index/moderation/trust）への更新通知を、**at-least-once** 前提で安全に流す
- 冪等性（idempotency）・offset・リプレイ・バックプレッシャの取り扱いを統一する

## 結論（v1）

- **配信の正は Postgres の outbox** とし、`LISTEN/NOTIFY` は「低遅延の起床通知」に限定する（NOTIFY単体での配信保証はしない）
- 下流は **`seq`（単調増加）で追従**し、**at-least-once を冪等処理**で吸収する
- outbox の操作種別は `upsert` / `delete` を最小とする（意味は `docs/03_implementation/community_nodes/event_treatment_policy.md` に準拠）

## 参照

- `docs/03_implementation/community_nodes/event_treatment_policy.md`（`upsert`/`delete` の意味）
- `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`（永続化/retention の前提）

## テーブル（提案）

### 1) `cn_relay.events_outbox`

- `seq BIGSERIAL PRIMARY KEY`（consumer が追う offset）
- `op TEXT NOT NULL`（`upsert` / `delete`）
- `event_id TEXT NOT NULL`（NIP-01 の `event.id`）
- `topic_id TEXT NOT NULL`（正規形）
- `kind INT NOT NULL`
- `created_at INT NOT NULL`（event の `created_at`）
- `ingested_at TIMESTAMPTZ NOT NULL`（relay の受理時刻）
- `effective_key TEXT NULL`（replaceable/addressable のキー。必要なら）
- `reason TEXT NULL`（`delete` の理由: `nip09`/`expiration` 等。必要なら）

方針:

- **payload は参照中心**（基本は `event_id` で `cn_relay.events` を引く）。NOTIFY の payload にイベント本体は載せない
- 下流にとって不要な列は削ってよい（ただし `seq`/`op`/`event_id`/`topic_id` は残す）

### 2) `cn_relay.consumer_offsets`

- `consumer TEXT PRIMARY KEY`（例: `index-v1` / `moderation-v1` / `trust-v1`）
- `last_seq BIGINT NOT NULL`（最後にコミットした `seq`）
- `updated_at TIMESTAMPTZ NOT NULL`

補足:

- v1 は **consumer ごとに単一インスタンス**で動かす前提にする（水平分散は v2 で設計）

## relay 側（生成）のセマンティクス

- relay は取込レコードの保存（および delete/upsert の判定）と **同一トランザクション**で outbox を insert する
  - 目的: 「イベントは保存されたが outbox が無い」/「outbox はあるがイベントが無い」を避ける
- outbox insert 後に `NOTIFY` を発行してよい（通知は commit 時に配送される）
  - channel 例: `cn_relay_outbox`
  - payload は `max_seq`（文字列）程度に抑える（NOTIFY は永続ではなく、payload サイズ制約もある）

## consumer 側（消費）のセマンティクス

### 1) 起動時 / 復帰時

- `consumer_offsets.last_seq` を読み、`seq > last_seq` を **昇順**で取得して処理する
  - 例: `SELECT * FROM events_outbox WHERE seq > $last_seq ORDER BY seq ASC LIMIT $batch_size`

### 2) NOTIFY の扱い

- `LISTEN cn_relay_outbox` は **起床トリガ**として使う
- NOTIFY を取りこぼしてもよい（必ず DB 追従で補完する）
  - 実装: NOTIFY が無い場合も定期ポーリングで追いつけるようにする

### 3) offset コミット

- `seq` は **単調増加**なので、consumer は「処理完了した最後の `seq`」を `consumer_offsets.last_seq` に保存する
- at-least-once 前提なので、コミットタイミングは次を推奨する
  - 下流の副作用（Postgres検索更新/label 発行/AGE 更新）が成功した後に `last_seq` を更新する

## 冪等性（idempotency）指針

前提: outbox は at-least-once であり、同じ `event_id` が重複して観測され得る。

- index: `document_id = event_id` とし、`upsert`/`delete` を冪等にする（同一操作の再実行を許容）
- moderation/trust:
  - “加算（集計）”型の処理は二重計上しやすいので、`(consumer, event_id)` の processed 記録テーブルを持つ、または派生テーブルに `UNIQUE` を置いて `ON CONFLICT` で吸収する

## リプレイ（再処理）

- outbox は `seq` で追えるため、`consumer_offsets.last_seq` を戻せば任意地点からリプレイできる
  - 全再構築が必要なら `last_seq=0` + 下流ストア再生成（index は reindex、trust は再計算）
- outbox 保持期間を超えて遅延した consumer は追いつけないため、v1 は「遅延 = 再構築」を運用手順として持つ

## バックプレッシャ（遅延/詰まり）の扱い

監視指標（例）:

- `backlog = (SELECT max(seq) FROM events_outbox) - consumer_offsets.last_seq`

対処（例）:

1. consumer の処理能力を上げる（batch_size/並列度/スケール）
2. relay 側で backfill を止める（リアルタイムのみ）/ 低優先 topic を停止（`ingest_policy`）
3. それでも詰まる場合は、受理レート制限・topic数制限などの運用制約を強化する

## outbox の保持と削除（v1）

- v1 は単純に **時間ベース（例: 30日）**で outbox を保持し、古い行を削除（またはパーティション drop）する
- consumer の遅延が保持期間を超えた場合は、リプレイではなく再構築で復旧する
