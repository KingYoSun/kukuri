# User API（外部I/F統合）設計

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`

## 目的

- 外部公開する HTTP インターフェイスを **User API に集約**し、`index/moderation/trust/bootstrap/relay` の内部APIを直接公開しない
- 将来の **ユーザー別課金**・レート制限・利用量計測・監査を実装しやすい土台を作る
- サービス分離（profiles/別ホスト）しても、クライアントの接続先は原則変えずに済む構造にする
- 規約/プライバシーポリシーへの同意を必須化し、ノード利用の前提条件として統一的に扱う

## スコープ（User API が担うもの / 担わないもの）

### User API が担う

- ユーザー認証・認可（topic購読/プラン/クォータに基づく）
- 規約/プライバシーポリシーの提示と同意管理（必須化）
- topic購読の申請/承認/停止（後述の購読設計に従う）
- read API（検索/トレンド/ラベル/トラスト等）の統合
- write API（通報/購読申請など）の統合
- レート制限・利用量計測（IP + pubkey + token）

### User API が担わない（原則）

- 各サービスの運用設定（それは `Admin API`）
- リレー互換 WS のプロトコル実装（それは `relay`。ただし公開経路は統合可能）

## 外部公開の形（推奨）

外部公開を 1 ドメインにまとめ、逆プロキシでルーティングする（実装は `Caddy/Traefik/nginx` のどれでもよい）。

- `/api/*` → `user-api`
- `/relay` → `relay`（WS）
- `/admin/*` → `admin-console`
- `/admin-api/*` → `admin-api`

## 公開範囲（デフォルト）

- **認証なしで到達可能（public）**
  - 規約/プライバシーポリシーの取得と同意登録（同意がないと同意できないため）
  - bootstrap 情報（初回接続・発見のため。`BOOTSTRAP_AUTH_REQUIRED=false` の間は同意も不要）
- **認証が必要（authenticated）**
  - topic購読申請、検索/トレンド、trust、通報などの「ユーザー操作」
- **同意が必要（consent_required）**
  - 原則として authenticated のうち「ユーザー操作」は ToS/Privacy への同意を前提条件にする

補足:
- relay/boostrap は **デフォルトは認証OFF** とし、管理画面から後から必須化できる（詳細は各サービス設計を参照）。
  - 認証OFFの間は「ユーザー操作の手間を最小化」するため、同意も不要とする（relay は匿名のため同意を強制できない）。
- 認証OFF→ON 切替時の既存接続/猶予期間/互換性は `docs/03_implementation/community_nodes/auth_transition_design.md` を参照。

## 認証（案）

課金・購読を pubkey 単位で扱う前提で、**Nostr鍵（secp256k1）ベース**の認証に寄せる。

### HTTP（User API）

- 推奨: **署名チャレンジ方式**（最小実装）
  - `POST /v1/auth/challenge`（pubkeyを送る）→ nonce + exp
  - `POST /v1/auth/verify`（nonce署名を送る）→ access token（JWT など）
- 代替: NIP-98（HTTP Auth）互換に寄せる（既存クライアント実装との整合を確認して選定）

### WS（relay）

- NIP-42（AUTH event）での認証を前提にする（購読制限/課金が必要な場合）

## 規約/プライバシーポリシー同意（必須）

- 保護された API（検索/トラスト/購読申請/通報など）は、**current の ToS/Privacy への同意**を前提条件にする
- 未同意の場合、User API は `428 Precondition Required`（例）で `CONSENT_REQUIRED` を返す
- ポリシーの取得と同意登録は、同意がなくても到達できる必要がある

補足:
- bootstrap（認証OFF）と relay（認証OFF）は **同意不要**の運用とし、後から認証必須化した場合に同意チェックを有効化できるようにする。
  - LLM moderation で外部送信（例: OpenAI）を有効化する場合は、Privacy に送信範囲/保持/目的/停止条件を明記する（詳細: `docs/03_implementation/community_nodes/llm_moderation_policy.md`）。

詳細は `docs/03_implementation/community_nodes/policy_consent_management.md` を参照。
個人データの保持/削除/エクスポート方針は `docs/03_implementation/community_nodes/personal_data_handling_policy.md` を参照。

## 認可（エンタイトルメント）

User API は「ユーザーが何をできるか」を DB の状態で決める。

- topic 単位の購読（read/search/trust/moderation 結果取得など）
- 書き込み（通報、購読申請）
- 追加制限（同時接続数、日次リクエスト数、topics上限）

## API（叩き口）案

### 規約/プライバシー

- `GET /v1/policies/current`
- `GET /v1/policies/:type/:version?locale=ja-JP`
- `GET /v1/consents/status`
- `POST /v1/consents`

### bootstrap（デフォルト: 認証OFF）

- `GET /v1/bootstrap/nodes`（node descriptor の一覧/差分）
- `GET /v1/bootstrap/topics/:topic/services`（topic_service の一覧）

### topic購読

- `POST /v1/topic-subscription-requests`
- `GET /v1/topic-subscriptions`
- `DELETE /v1/topic-subscriptions/:topic_id`（解約/停止）

### Access Control（invite/key envelope）

- `POST /v1/invite/redeem`（capability 提示で join。成功時に key.envelope を返す/取得可能にする）
- `GET /v1/keys/envelopes?topic_id=...&scope=...&after_epoch=...`（鍵封筒の再取得）

詳細: `docs/03_implementation/community_nodes/access_control_design.md`

### 検索/トレンド（index）

- `GET /v1/search?topic=...&q=...`
- `GET /v1/trending?topic=...`

### モデレーション（moderation）

- `POST /v1/reports`
- `GET /v1/labels?target=...`

### トラスト（trust）

- `GET /v1/trust/report-based?subject=pubkey:...`
- `GET /v1/trust/communication-density?subject=pubkey:...`

### 個人データ（削除/エクスポート）

- `POST /v1/personal-data-export-requests`
- `GET /v1/personal-data-export-requests/:export_request_id`
- `GET /v1/personal-data-export-requests/:export_request_id/download`
- `POST /v1/personal-data-deletion-requests`
- `GET /v1/personal-data-deletion-requests/:deletion_request_id`

## 課金/利用量計測（最小モデル）

決済プロバイダ連携（Stripe 等）は後回しでも、内部データモデルは先に用意する。

- `plans`（提供メニュー: topics上限、検索回数上限、保持期間、優先度等）
- `subscriptions`（pubkey → plan、状態、期間）
- `usage_counters`（日次/週次のAPI利用量、超過判定）

v1 の詳細（課金単位、メータリング、超過時の挙動、無料枠/上限、監査）は `docs/03_implementation/community_nodes/billing_usage_metering.md` を参照。

## リレー（取込）との関係

- User API は **購読 topic の申請/承認**を行い、DB に反映する
- relay は DB の「ノード取込購読（node-level subscription）」を監視し、topic の subscribe/unsubscribe を実施する
- `index/moderation/trust` は relay が保存したレコードを入力として扱う（取込経路の一本化）

詳細は `docs/03_implementation/community_nodes/topic_subscription_design.md` を参照。

Access Control（39020/39021、join/redeem、epoch ローテ）は `docs/03_implementation/community_nodes/access_control_design.md` を参照。
