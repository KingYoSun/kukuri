# イベント種別（Nostr）と保存/配信ポリシー

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`（主に `relay` と下流サービス）

## 目的

- relay/下流サービスで「イベントの有効性（表示・検索・計算対象）」の扱いを統一する
- 削除/置換/期限切れ等の変化を、検索（index）・モデレーション（moderation）・トラスト（trust）へ矛盾なく反映できるようにする
- 可能な限り Nostr の既存仕様に寄せる

## 参照（ローカル）

- `docs/nips/01.md`（replaceable/ephemeral/addressable の分類、REQ/EOSE、同一timestampの扱い）
- `docs/nips/09.md`（kind=5 deletion request）
- `docs/nips/40.md`（`expiration` tag）

## 前提（topic と受理条件）

- v1 では topic は `t` タグ（`["t","<topic_id>"]`）で表現する
  - topic購読（WS REQ）では `#t` を必須とし、topic タグが無いイベントは `invalid: missing topic` として受理しない
  - topic_id の正規形と iroh-gossip TopicId への写像は `docs/03_implementation/community_nodes/topic_subscription_design.md` を参照
- relay は NIP-01 に従い `event.id` と `sig` を検証し、`event.id` を冪等キーとして扱う

## イベント種別（NIP-01）と保存/配信（v1）

NIP-01 の kind レンジに従い、イベントを次の4種に分類する。

### 1) regular

- 対象: `1000 <= kind < 10000 || 4 <= kind < 45 || kind == 1 || kind == 2`（kind=5 deletion request もここ）
- 保存: Postgres に保存（`event.id` で重複排除）
- 配信: WS / iroh-gossip に配信
- 下流反映: outbox（正）+ `LISTEN/NOTIFY`（起床通知）により下流サービスへ通知（基本は `upsert` として扱う）
  - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`

### 2) replaceable

- 対象: `10000 <= kind < 20000 || kind == 0 || kind == 3`
- 置換キー（NIP-01）: `(pubkey, kind)`
  - 同一 `created_at` の競合は `event.id` の辞書順が小さい方を採用（NIP-01）
- 保存: v1 では「全版を保存してよい」が、**有効な最新（effective view）は1つ**とする
- 配信（WS）:
  - REQ に応答する際は、effective view の最新のみを返す（NIP-01）
  - 置換により有効でなくなった旧版は、原則として再配信しない
- 下流反映:
  - effective view が更新された場合は `upsert` で通知する（index 等は上書き）
  - topicごとに別の値が必要な用途は addressable（`d` に topic_id を埋める等）へ寄せる

### 3) addressable

- 対象: `30000 <= kind < 40000`（KIP-0001 の 390xx 系もここ）
- 置換キー（NIP-01）: `(kind, pubkey, d)`
  - `d` タグが無い場合は `invalid: missing d` として受理しない（v1）
  - 同一 `created_at` の競合は `event.id` の辞書順が小さい方を採用（NIP-01）
- 保存/配信/下流反映: replaceable と同様（effective view は1つ、下流へは `upsert`）
  - 補足: `39020/39021/39022`（Access Control）は **P2P-only** を正とし、relay では配布/再配信しない（詳細: `docs/03_implementation/community_nodes/access_control_design.md`）

### 4) ephemeral

- 対象: `20000 <= kind < 30000`
- 保存: しない（バックフィル対象外）
- 配信: WS/iroh-gossip のリアルタイムのみ（購読者がいなければ破棄され得る）
- 下流反映: 原則として下流サービスへ流さない（index/moderation/trust の入力にしない）

## 削除（kind=5, NIP-09）と反映方針（v1）

- deletion request イベント（kind=5）自体は regular として保存/配信する（NIP-09）
- relay が削除を適用する条件（v1）
  - 対象イベントが Postgres に存在し、かつ `target.pubkey == deletion.pubkey` の場合のみ適用する（NIP-09 の推奨に寄せる）
- 対象の指定
  - `["e","<event_id>"]` による単体指定
  - `["a","<kind>:<pubkey>:<d>"]` による addressable/replaceable 指定
  - `["k","<kind>"]` がある場合は補助情報として使う（無くてもよい）
- 反映（v1）
  - 対象イベントは物理削除ではなく **soft delete（非表示化）** を基本とする（監査/再計算のため）
  - 検索/購読/下流サービスは「削除済みイベント」を表示・配信・インデックス対象から除外する
- 下流反映（提案）
  - relay が削除を適用したタイミングで、対象イベントに対する `delete` を outbox で通知する（index は doc を削除）

## 期限切れ（`expiration` tag, NIP-40）と反映方針（v1）

- 受理時点で既に期限切れの場合、relay は受理しない（`invalid: expired`）
- 期限到来後は「保存されていても配信しない」を優先し、必要なら purge（物理削除）を実行する
- 下流反映（提案）
  - 期限到来後に削除扱いにする場合、対象イベントを `delete` として通知する（index から除外）

## 下流サービスへの反映（推奨）

outbox/NOTIFY の配信セマンティクス（outbox を正、`LISTEN/NOTIFY` は起床通知）は `docs/03_implementation/community_nodes/outbox_notify_semantics.md` を参照。ここでは outbox に載せる `op` の “意味” を定義する。

- `upsert`: 有効なイベントが新規に追加・更新された（regular の insert / replaceable,addressable の effective view 更新）
- `delete`: 有効だったイベントが無効化された（deletion request 適用 / expiration 到来 等）

サービスごとの推奨動作（v1）:

- index: `upsert` は `cn_search.post_search_documents` を更新、`delete` はドキュメントを論理削除
- moderation: `upsert` を評価対象にし、`delete` は参照/表示対象から外す（label は `exp` で自然失効させる）
- trust:
  - report-based は削除に影響させない（通報履歴は残す）
  - communication-density は v1 では削除でエッジを消さず（ゲーム耐性）、表示や根拠提示からは除外できるようにする
