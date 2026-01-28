# Access Control（KIP-0001: 39020/39021/39022）設計

**作成日**: 2026年01月22日  
**対象**: クライアント（P2P）/ `./kukuri-community-node`（任意の補助ノード）

## ゴール

- topic の公開度（`public`/`friend`/`invite`/`friend_plus`）を扱える
- `invite.capability(kind=39021)` と `join.request(kind=39022)` による **P2P join** を提供できる
- 追放/漏洩時に **epoch ローテーション**で「未来閲覧」を止められる（過去暗号文は回収不能）
- **ノード不要**で成立する最小フローを v1 とする（ノードは任意の補助）

## 参照（ローカル）

- `docs/01_project/activeContext/community_node_plan.md`（KIP-0001 Draft の Access Control 節）
- `docs/nips/01.md`（addressable/replaceable の扱い、イベント検証の前提）
- `docs/nips/42.md`（WS relay 認証: AUTH）
- `docs/nips/44.md`（暗号化ペイロード）
- `docs/03_implementation/community_nodes/services_relay.md`
- `docs/03_implementation/community_nodes/topic_subscription_design.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/03_implementation/community_nodes/personal_data_handling_policy.md`

## 前提と用語

- `topic_id`: `docs/03_implementation/community_nodes/topic_subscription_design.md` の正規形
- `scope`: `public | friend | invite | friend_plus`
- `epoch`: `topic_id + scope` 単位の単調増加カウンタ（追放/漏洩時に `epoch++`）
- `K(topic_id, scope, epoch)`: `scope!=public` の暗号化に使う共有鍵（群鍵）
- **重要**: この Access Control は「暗号化 + 鍵配布 + epoch ローテ」により未来閲覧を止める。過去暗号文は原理的に回収できない。

## v1 スコープ（提案）

- 実装優先度: `invite` と `friend` を先に実装し、`friend_plus` は v2（集合計算/鍵配布負荷が大きい）
- join/鍵配布/ローテは **P2P-only**（クライアントが正）とする
- `kind=39020/39021/39022` はイベント表現を持つが、**配布経路は direct P2P / out-of-band を正**とする
  - 理由: `p` タグ等のメタデータでメンバーシップが漏れやすく、gossip/relay 配信に不向き
- コミュニティノードは **Access Control に関与しない**（P2P-only を正とする）

## イベント設計（提案）

### 39020 `kukuri.key.envelope`（鍵封筒）

目的: 受信者 pubkey ごとに `K(topic_id, scope, epoch)` を渡す。

- tags（必須）
  - `["p","<recipient_pubkey_hex>"]`
  - `["t","<topic_id>"]`（relay の topic フィルタ体系に合わせる）
  - `["scope","friend|invite|friend_plus"]`
  - `["epoch","<int>"]`
  - `["ver","1"]`
- `d` タグ（必須）
  - `kind=39020` は NIP-01 上は addressable なので `["d","..."]` を必須化する
  - v1 推奨: `d = "keyenv:<topic_id>:<scope>:<epoch>:<recipient_pubkey_hex>"`
    - 目的: addressable の置換キー `(kind,pubkey,d)` が受信者間で衝突しないようにする
- content
  - KIP Draft の JSON（`key_b64` 等）を **NIP-44 で recipient 宛に暗号化**
  - 例（暗号化前）: `{ schema, topic, scope, epoch, key_b64, issued_at, expires }`

配布経路（v1）:
- **direct P2P**（招待者/既存メンバーから対象者へ送付）を正とする
- relay/iroh-gossip への再配信は **原則しない**（必要な場合も暗号化/最小配布に限定）

### 39021 `kukuri.invite.capability`（招待capability）

目的: join のための capability（短命/回数制限/リプレイ耐性）を配布する。

- tags（必須）
  - `["t","<topic_id>"]`
  - `["scope","invite"]`
  - `["ver","1"]`
  - `["d","invite:<nonce>"]`（1招待=1nonce で append-only 的に運用）
- content（暗号化前の JSON 例）
  - `{ schema, topic, scope, expires, max_uses, nonce, issuer }`

配布経路（v1）:
- 発行者（ユーザー鍵）が作成し、**out-of-band/直接共有**（P2P DM/QR/コピペ等）する

### 39022 `kukuri.join.request`（join申請）

目的: invite/friend の参加希望を通知する。

- tags（必須）
  - `["t","<topic_id>"]`
  - `["scope","invite|friend"]`
  - `["ver","1"]`
  - `["d","join:<topic_id>:<nonce>:<requester_pubkey_hex>"]`
- tags（任意）
  - `["e","<invite_event_id>"]`（invite 利用時）
  - `["p","<issuer_pubkey_hex>"]`（招待発行者/鍵配布先の目安）
- content（暗号化前の JSON 例）
  - `{ schema, topic, scope, invite_event_json, requester, requested_at }`

配布経路（v1）:
- 招待者/既存メンバーへの **direct P2P** が基本（プライバシーを優先）
- 必要に応じて topic への **ブロードキャスト**も許可（メタデータ露出に注意）

## P2P join フロー（v1）

1. **招待発行**: 既存メンバーが `invite.capability` を生成し、対象者へ共有
2. **参加要求**: 参加希望者が `join.request` を送信
3. **検証**: 受信側が invite の署名/期限/スコープを検証（`max_uses` は best-effort）
4. **鍵配布**: 承認したメンバーが `key.envelope` を送付（recipient 宛 NIP-44）
5. **ローカル反映**: 受信側はローカルに membership/鍵を保存し、以後の復号に利用

補足:
- `friend` スコープは **招待不要**で join.request を送ることを許容（承認は各メンバーの裁量）。
- `friend_plus` は v2 に回す（対象集合のプライバシー/計算負荷が大きい）。

## 鍵運用（epoch ローテ/追放）

### 追放（revoke）

- 手順（推奨）
  1. 対象を **ローカルのメンバーリスト**から除外する
  2. 直後に `epoch++`（rotate）して残留者へ新しい `39020` を再配布する

### epoch ローテ（rotate）

- `topic_id + scope` 単位で `current_epoch++` し、新しい群鍵を生成する
- 残留メンバー全員へ `39020` を再発行する
- 受理ルールは **クライアント側**で行う（`epoch < current_epoch` の投稿は無視）

## relay での受理/拒否（最低限）

### 投稿（write）

- `scope=public` は従来通り
- `scope!=public` は次を必須化（v1 推奨）
  - `["t","<topic_id>"]`
  - `["scope","friend|invite|friend_plus"]`
  - `["epoch","<int>"]`
- P2P-only の v1 では **relay が membership を持たない**ため、厳密な拒否は行わない
- private scope を relay で扱う場合は、**認証 + allowlist** を別途導入する（運用オプション）

### 購読（read/backfill）

- iroh-gossip は購読者を識別しにくいため、**暗号化が第一の防御**になる（メタデータ漏洩は残る）
- WS で private scope を「誰に返すか」を制御するには、pubkey を特定する必要がある
  - v1 は **relay で private scope を扱わない**（配信/バックフィルはオフ）ことを推奨
  - 例外として扱う場合は、**NIP-42（AUTH）必須 + allowlist** を導入する

## DB データモデル（v1）

v1 の Access Control は **P2P-only** のため、コミュニティノード側に専用DBを持たない。
（後日、運用方針を変える場合は別途設計する。）

## 未決定（v2 以降の検討）

- `friend_plus` の対象集合の決め方（trust 連携/クライアント計算/プライバシー）
- private scope のイベントを node の `index/moderation/trust` が扱うか（復号の是非、ポリシー/同意/ログ方針）
- 群鍵の保管方式（KMS/OS keychain/HSM、ローテ手順、バックアップ/復旧）
