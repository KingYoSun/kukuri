# Topic 購読（Subscription）設計

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`

## 目的

- 「topic をどのノードが扱うか」を運用可能な形で定義する
- **relay を必須**の取込入口として、取込済みレコードを `index/moderation/trust` が共通利用できるようにする
- 将来のユーザー課金（topic単位/プラン単位）へ自然に接続できる設計にする

## 用語

- **ユーザー購読（user-level subscription）**: pubkey（ユーザー）に対する購読権限
- **ノード取込購読（node-level subscription）**: relay が実際にネットワークから取込む対象 topic の集合
- **購読申請（subscription request）**: ユーザーが「この topic をこのノードで扱ってほしい」と要求する操作

## 基本方針

1. **外部からの申請/参照は User API に集約**する（サービス個別APIは公開しない）
2. relay は **node-level** の購読だけを見て動く（ユーザー単位の細かい制御は User API が集約）
3. `index/moderation/trust` は **relay が保存した取込レコード**を入力として処理する（取込の一本化）

補足（relay 認証OFFの扱い）:
- relay が認証OFF（anonymous）の場合、user-level subscription（課金/権限）を relay 側で厳密に強制できない。
  - この場合は「public topic の配信口」として運用し、課金/権限制御が必要になったタイミングで relay の認証必須化を ON にする。
  - 併せて、認証OFFの間は同意（ToS/Privacy）も不要として扱い、後から認証必須化した場合に同意チェックを有効化できるようにする。

## relay の取込プロトコル（確定）

- relay の取込（P2P）は **iroh-gossip**（現 `kukuri-cli` 準拠）とする
  - node-level subscription = iroh-gossip の subscribe 対象 topic の集合
- relay の配信（P2P）も iroh-gossip へ broadcast する
  - WS 等で受け付けたイベントを、対応する topic へ再配信（橋渡し）する
  - iroh-gossip 由来のイベントはアプリケーションが同一 topic に再注入しない（重複は `event.id` で冪等処理する）
- 購読フィルタ/整合性（WS）については `docs/nips/01.md` の購読フィルタ（REQ）と EOSE の考え方が参考になる

## topic/購読フィルタ/バックフィルの写像（提案）

### 1) topic_id の正規形と iroh-gossip TopicId への写像

- **topic_name（人間が入力する名前）** と **topic_id（正規形）** を分ける
  - topic_name は表示・入力用（例: `Bitcoin`）
  - topic_id は通信/保存/購読のキー（例: `kukuri:<64hex>`）
- **topic_id（v1）**: `kukuri:` 名前空間の **正規形文字列**を正とする
  - 推奨: 一般の topic は `kukuri:<64hex>`（`hex(blake3(base))`）のハッシュ形式に正規化する（base は正規化した topic_name 等）
  - 例外: 予約済みの明示ID（例: `kukuri:global` / `kukuri:user:<pubkey>`）も許可する（互換/運用のため）
  - 既に `kukuri:<64hex>` 形式の場合はそのまま採用（再ハッシュしない）
- **iroh-gossip TopicId（32bytes）**:
  - `topic_id` が `kukuri:<64hex>` の場合は `<64hex>` をデコードした 32bytes
  - それ以外は `blake3(topic_id)` の 32bytes
- **環境分離**: 同名 topic が dev/prod 等で衝突しないよう、base に `NETWORK_ID`（例: `kukuri:main:` / `kukuri:dev:`）を含める（運用で固定）
- **バージョニング**: 写像のアルゴリズムを変える場合は v2 として明示し、移行期間は v1/v2 の両方を購読できる運用にする（後方互換）

### 2) event と topic の紐付け（event→topic_id）

- relay が扱う「topic」は、Nostrイベントのタグで表現する（NIP-01の `#<tag>` フィルタに寄せる）
- **v1 方針**: topic_id は `t` タグで付与する
  - 例: `["t", "<topic_id>"]`
  - 複数 topic に属する場合は `t` タグを複数付ける（relay は `event.id` は1件として扱い、topicとの紐付けを別で持つ）
- v1 デフォルト: topic タグが無いイベントは受理しない（`invalid: missing topic`）
  - 互換/移行のために「topicなしを `kukuri:global` に変換」するモードも可能だが、混在運用は避ける

### 3) topic→購読フィルタ（WS: NIP-01）写像

- WS の購読（REQ）では **`#t` を topic フィルタとして必須**にする（DoS抑止 + 整合性のため）
  - 例: `["REQ","sub",{"#t":["<topic_id>"],"kinds":[...],"since":...,"limit":...}]`
- `#t` 以外のフィルタ（`authors`/`kinds`/`ids`/`since/until` 等）は NIP-01 の意味に従う
- `#t` が無い REQ は v1 では拒否し、運用ポリシーとして明示する（グローバルを購読したい場合も `#t=["kukuri:global"]` を明示する）

### 4) バックフィル/再接続の整合性

- iroh-gossip は履歴の保証が弱いため、**バックフィルの正は relay のDB**に置く（WSの初期取得・再接続時の整合性は DB が担保）
- WS 購読のフロー（NIP-01 に寄せる）
  - 初期取得: DB検索で保存済みイベントを `EVENT` で送る → `EOSE`
  - 以降: 新着をリアルタイムで `EVENT`（重複は `event.id` で冪等）
- 再接続の取りこぼし対策
  - `since = last_seen_created_at - margin`（時計ずれ/同一秒競合の吸収）+ `event.id` 冪等を基本とする
  - より堅牢にする場合は v2 で「DBの ingest 順序カーソル（例: `ingested_seq`）」を追加し、created_at 依存を減らす

## データモデル（提案）

### 1) subscription request（ユーザー → ノード）

- `request_id`
- `requester_pubkey`
- `topic_id`
- `requested_services`（例: `index|moderation|trust`）
- `status`（`pending|approved|rejected|active|cancelled`）
- `review_note`（任意）
- `created_at` / `reviewed_at`

### 2) user-level subscription（pubkey の権限）

- `topic_id`
- `subscriber_pubkey`
- `plan_id` / `subscription_id`
- `status`（`active|paused|ended`）
- `started_at` / `ended_at`

### 3) node-level subscription（relay の取込設定）

- `topic_id`
- `enabled`（bool）
- `ref_count`（アクティブ購読者数の参照カウント、または計算ビュー）
- `ingest_policy`（保持期間、最大容量、遅延許容、バックフィル有無など）
  - 詳細: `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`
- `created_at` / `updated_at`

## 申請〜取込開始のフロー（提案）

1. ユーザーが `User API` に購読申請（`POST /v1/topic-subscription-requests`）
2. `User API` が検証
   - 認証済み（pubkey）
   - current の ToS/Privacy に同意済み（同意必須化）
   - プラン/クォータ範囲内（topics上限、API利用量など）
     - 詳細: `docs/03_implementation/community_nodes/billing_usage_metering.md`
   - topic_id の妥当性（形式、ブラックリスト、禁止topic等）
3. 承認方式
   - v1: **手動承認**（Admin Console で approve/reject）
   - v2: 条件付き自動承認（支払い済み、上限内等）
4. 承認されたら
   - user-level subscription を `active` にする
   - node-level subscription の `ref_count` を増やす（または集計で増える）
5. relay が node-level subscription の変化を検知して subscribe 開始
   - 実装案: DBポーリング / `LISTEN/NOTIFY` / 内部HTTP呼び出し
6. relay が取込レコードを Postgres に保存し、outbox（正）+ `LISTEN/NOTIFY`（起床通知）で下流へ通知
   - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
7. index/moderation/trust が取込レコードを処理し、結果（検索インデックス/label/attestation）を生成

## relay 必須化に伴う「取込レコード」設計（最小）

- relay はイベント（Nostr互換フィールド）を **正規化して保存**する
  - 例: `event_id`, `pubkey`, `kind`, `created_at`, `tags`, `content`, `sig`, `topic_id`（抽出可能なら）
- 下流サービスは「新着イベント」を追従できる必要がある
  - 例: relay が `cn_relay.events_outbox` を追加し、各サービスが `consumer_offsets` で `seq` を追従する
    - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`

## ユーザーからの購読制御（やりたいことの整理）

- 「このノードで topic を扱ってほしい」= **購読申請**
- 「自分はこの topic の検索/トラスト/ラベル結果を使いたい」= **購読（権限）**
- 「購読をやめたい」= user-level subscription の停止（ref_count が 0 なら node-level 停止も可能）

## Access Control（招待/鍵配布）との関係（v1）

- Access Control は「購読（権限）」とは別に、private scope（`invite/friend` 等）の **鍵配布と epoch ローテ**を扱う
- v1 は **P2P-only** とし、`invite.capability(kind=39021)` + `join.request(kind=39022)` でメンバーが鍵配布を行う
- user-level subscription（ノードの検索/トラスト/ラベル結果の利用）とは **独立**。必要なら別途申請する
- relay の node-level 取込購読は **ユーザー購読の集計のみ**を根拠にする（private scope の membership はノード側で保持しない）

詳細: `docs/03_implementation/community_nodes/access_control_design.md`

## DoS/濫用対策（v1で必須）

- 購読申請の rate limit（IP + pubkey）
- 申請の同時保留数上限（per pubkey）
- node-level の同時取込 topic 数上限（ノード運用者の資源制約）
- 自動承認する場合は、支払い/クレジット/招待capability等の担保が必要
