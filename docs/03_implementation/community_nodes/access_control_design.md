# Access Control（KIP-0001: 39020/39021）設計

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`（User API / relay / Admin API/Console）

## ゴール

- topic の公開度（`public`/`friend`/`invite`/`friend_plus`）を扱える
- `invite.capability(kind=39021)` による **join/redeem** を提供できる
- 追放/漏洩時に **epoch ローテーション**で「未来閲覧」を止められる（過去暗号文は回収不能）
- relay（WS/iroh-gossip）が最低限の整合性・濫用対策（rate limit 等）を保てる

## 参照（ローカル）

- `docs/01_project/activeContext/community_node_plan.md`（KIP-0001 Draft の Access Control 節）
- `docs/nips/01.md`（addressable/replaceable の扱い、イベント検証の前提）
- `docs/nips/42.md`（WS relay 認証: AUTH）
- `docs/nips/44.md`（暗号化ペイロード）
- `docs/03_implementation/community_nodes/user_api.md`
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
- join/redeem は **HTTP API（User API）**で開始する（KIP Draft を踏襲）
- `kind=39020/39021` はイベント表現を持つが、**配布経路は relay ではなく User API を正**とする
  - 理由: `p` タグ等のメタデータでメンバーシップが漏れやすく、gossip/relay 配信に不向き

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
- relay/iroh-gossip へは **再配信しない**
- User API が「本人にのみ」返す（`GET /v1/keys/envelopes` 等）

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
- Admin Console で発行し、運用者が QR/コピペ等で共有する（User API 経由の配布でもよいが認可必須）

## join/redeem API（User API）

### `POST /v1/invite/redeem`（v1）

- 入力: `capability_event_json`（39021 の event JSON をそのまま提示）
- 前提: 認証済み + ToS/Privacy 同意済み（未同意は `CONSENT_REQUIRED`）
- 動作（最小）
  1. 39021 を検証（署名/期限/nonce/`max_uses`/replay）
  2. `topic_memberships` を `active` にする（`topic_id, scope=invite, pubkey`）
  3. `current_epoch` の `39020` を発行し、レスポンスに含める（または直後に取得可能にする）
  4. 必要なら user-level subscription（課金/権限）も同時に `active` にする（v1 は実装簡略のため同時更新を推奨）
- 出力（例）: `{ topic_id, scope, epoch, key_envelopes: [key_envelope_event_json...] }`

### `GET /v1/keys/envelopes`（v1）

- 用途: 端末追加/再インストール/epoch ローテ後の再取得
- 認可: `topic_memberships(active)` があること

補足:
- relay/bootstrap は認証OFFの間は同意不要という方針だが、`redeem`/`keys` は「ユーザー操作」なので同意必須とする（`docs/03_implementation/community_nodes/policy_consent_management.md`）。

## Admin 運用（epoch ローテ/追放）

### 追放（revoke）

- 手順（推奨）
  1. 対象 pubkey を `topic_memberships` で `revoked` にする
  2. 直後に `epoch++`（rotate）して残留者へ新しい `39020` を再配布する

### epoch ローテ（rotate）

- `topic_id + scope` 単位で `current_epoch++` し、新しい群鍵を生成する
- 残留メンバー全員へ `39020` を再発行する
- 受理ルール（relay）を更新する（`epoch < current_epoch` の新規投稿は拒否）

## relay での受理/拒否（最低限）

### 投稿（write）

- `scope=public` は従来通り
- `scope!=public` は次を必須化（v1 推奨）
  - `["t","<topic_id>"]`
  - `["scope","friend|invite|friend_plus"]`
  - `["epoch","<int>"]`
- `event.pubkey` が `topic_memberships(active)` に存在しない場合は拒否（署名検証済み pubkey による判定）
- `epoch` が `topic_scope_state.current_epoch` 未満なら拒否（古い鍵での投稿を抑止）

### 購読（read/backfill）

- iroh-gossip は購読者を識別しにくいため、**暗号化が第一の防御**になる（メタデータ漏洩は残る）
- WS で private scope を「誰に返すか」を制御するには、pubkey を特定する必要がある
  - v1 推奨: private scope の WS 購読/バックフィルは **NIP-42（AUTH）必須**とし、未認証は拒否
  - 認証済み pubkey に対して `topic_memberships(active)` を確認し、未加入は拒否

## DB データモデル（提案）

`cn_user`/`cn_admin` のスキーマ分割は `docs/03_implementation/community_nodes/postgres_age_design.md` に従う。
保持期間/削除要求時の匿名化（member/invite/key_envelope を含む個人データの扱い）は `docs/03_implementation/community_nodes/personal_data_handling_policy.md` を参照。

- `cn_admin.topic_scope_state(topic_id, scope, current_epoch, updated_at)`（PK: topic_id+scope）
- `cn_admin.topic_scope_keys(topic_id, scope, epoch, key_ciphertext, created_at)`（群鍵。平文保存は禁止）
- `cn_user.topic_memberships(topic_id, scope, pubkey, status, joined_at, revoked_at, revoked_reason)`（PK: topic_id+scope+pubkey）
- `cn_user.invite_capabilities(topic_id, issuer_pubkey, nonce, expires_at, max_uses, used_count, revoked_at, capability_event_json, created_at)`（nonce は UNIQUE）
- `cn_user.key_envelopes(topic_id, scope, epoch, recipient_pubkey, key_envelope_event_json, created_at)`（監査/再配布用。v1 では必須ではない）

## 未決定（v2 以降の検討）

- `friend_plus` の対象集合の決め方（trust 連携/クライアント計算/プライバシー）
- private scope のイベントを node の `index/moderation/trust` が扱うか（復号の是非、ポリシー/同意/ログ方針）
- 群鍵の保管方式（KMS/OS keychain/HSM、ローテ手順、バックアップ/復旧）
