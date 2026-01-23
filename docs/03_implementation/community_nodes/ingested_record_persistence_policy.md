# 取込レコード永続化ポリシー（relay）

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`（主に `relay` / `cn_relay`）

## 目的

- relay が保存する「取込レコード（Nostr event）」の永続化ポリシーを確定し、運用・容量・下流同期を破綻させない
- dedupe/削除/置換/期限切れ/保持期間/容量上限/パーティションを一貫したルールで扱う

## 参照（ローカル）

- `docs/03_implementation/community_nodes/event_treatment_policy.md`（イベント種別と有効性ルール）
- `docs/nips/01.md`（replaceable/ephemeral/addressable、同一timestampの扱い）
- `docs/nips/09.md`（kind=5 deletion request）
- `docs/nips/40.md`（`expiration` tag）

## 方針（v1）

### 1) dedupe（重複排除）

- 冪等キーは **NIP-01 の `event.id`** とする
- DB では「保存テーブル」とは別に、dedupe 専用テーブルを持つ（パーティション構成に依存しないため）
  - 例: `cn_relay.event_dedupe(event_id PRIMARY KEY, first_seen_at, last_seen_at, seen_count)`
- relay の取込フロー（概念）
  1. `event_dedupe` に `event_id` を insert（重複なら last_seen_at 更新のみ）
  2. 新規（初回観測）の場合のみ、取込レコードを events へ保存し、topic 紐付け等を行う
- dedupe の保持期間は「取込レコード本体」より長くする（例: 180日）。短すぎると purge 後に同一イベントを再取り込みして下流が再処理する。

### 2) 削除/編集（置換）イベントの扱い

イベントの有効性（soft delete、replaceable/addressable の effective view 等）は `docs/03_implementation/community_nodes/event_treatment_policy.md` の通り。

- **削除（kind=5, NIP-09）**
  - deletion request 自体は保存する（regular）
  - 対象イベントは **soft delete** を基本とする（監査/再計算のため物理削除しない）
  - 対象が未到着の場合に備えて tombstone を保持する
    - 例: `cn_relay.deletion_tombstones(target_event_id|target_a, deletion_event_id, requested_at, applied_at NULL)`
- **編集（置換: replaceable/addressable, NIP-01）**
  - `(pubkey,kind)` / `(kind,pubkey,d)` で “最新（effective）” を決める
  - 同一 `created_at` の競合は `event.id` の辞書順が小さい方を採用する（NIP-01）
  - 永続化は「最新を引ける構造」を優先し、旧版は短期保持（監査用途）にとどめる
    - 例: `cn_relay.replaceable_current(key..., event_json, updated_at)` / `cn_relay.addressable_current(...)`

### 3) 保持期間（retention）

- retention の基準は `created_at` ではなく **`ingested_at`**（relay が受理した時刻）を正とする
  - 目的: 時計ずれ・バックフィル・再取得時でも運用上の「保存量」を制御しやすくする
- retention はまずグローバル既定を置き、次に topic ごとの上書き（`node-level subscription.ingest_policy`）で制御する
- 初期値（例。運用で調整）
  - regular: 30日
  - replaceable/addressable:
    - current: 長期（例: 180日〜無期限）
    - 履歴: 短期（例: 7日）または保持しない
  - deletion request / tombstone: 180日（削除の整合性を保つため長め）
  - `event_dedupe`: 180日（取込本体より長め）

### 4) 容量上限（capacity）

- 受理時のハード上限（relay で拒否）
  - 最大イベントサイズ（bytes）
  - tags 数/総サイズ（DoS 対策）
  - 期限切れ（`expiration`）は受理しない（`invalid: expired`）
- DB 容量（soft limit）
  - 全体容量と topic 別容量（最大件数/最大bytes）を設定できるようにする（`ingest_policy`）
  - 超過時の段階的な対処（例）
    1. 取込のバックフィル停止（リアルタイムのみ）
    2. 低優先 topic の取込停止（subscribe解除）
    3. 最古データの purge（パーティション drop を優先）

### 5) パーティション（推奨）

v1 では「保持期間で安全に捨てられる」ことを最優先にする。

- `cn_relay.events` は `ingested_at` の RANGE パーティション（例: 月次）を推奨
  - purge は古いパーティションを drop する（DELETE より速く、vacuum 負荷が低い）
- `event.id` の一意性は **`event_dedupe` で担保**し、`events` 本体にはグローバル UNIQUE を要求しない（パーティション制約回避）
- 置換イベントの current（replaceable/addressable）は、purge の影響を受けない専用テーブル（非パーティション）に保持する

## 下流サービスへの反映（前提）

outbox/NOTIFY の配信セマンティクス（at-least-once/offset/リプレイ/バックプレッシャ）は `docs/03_implementation/community_nodes/outbox_notify_semantics.md` に整理した。永続化ポリシーとして最低限以下を前提とする。

- `upsert`: 有効なイベントの新規追加/更新（replaceable/addressable の effective 更新も含む）
- `delete`: 有効だったイベントの無効化（deletion request 適用、expiration 到来 等）
