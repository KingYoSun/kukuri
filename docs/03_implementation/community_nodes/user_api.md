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
- レート制限・利用量計測（IP + pubkey（必要なら token）。詳細: `docs/03_implementation/community_nodes/rate_limit_design.md` / `docs/03_implementation/community_nodes/billing_usage_metering.md`）

### User API が担わない（原則）

- 各サービスの運用設定（それは `Admin API`）
- リレー互換 WS のプロトコル実装（それは `relay`。ただし公開経路は統合可能）

## 外部公開の形（推奨）

外部公開を 1 ドメインにまとめ、逆プロキシでルーティングする。v1 は **Caddy** を推奨する（コンテナ運用可、TLS終端/パスルーティングを最小で揃えられる）。

- `/api/*` → `user-api`
- `/relay` → `relay`（WS）
- `/admin/*` → `admin-console`
- `/admin-api/*` → `admin-api`

補足:
- `PUBLIC_BASE_URL` は reverse proxy 後の公開URLを正とする（例: `https://node.example/api`）。`auth_event_json` の `["relay","..."]` はこれと一致させる。

参考（例: Caddyfile の最小イメージ）:

```caddyfile
node.example {
  handle_path /api/* {
    reverse_proxy user-api:8080
  }
  handle /relay* {
    reverse_proxy relay:8080
  }
  handle_path /admin-api/* {
    reverse_proxy admin-api:8080
  }
  handle_path /admin/* {
    reverse_proxy admin-console:5173
  }
}
```

## 公開範囲（デフォルト）

- **認証なしで到達可能（public）**
  - 規約/プライバシーポリシーの取得（`GET /v1/policies/*`）
  - 認証（challenge/verify）（`POST /v1/auth/*`）
  - bootstrap 情報（初回接続・発見のため。`BOOTSTRAP_AUTH_REQUIRED=false` の間は同意も不要）
- **認証が必要（authenticated）**
  - 同意状態の取得/同意登録（`GET /v1/consents/status`, `POST /v1/consents`）
- **同意が必要（consent_required）**
  - 原則として authenticated のうち「ユーザー操作」（topic購読申請、検索/トレンド、trust、通報等）は ToS/Privacy への同意を前提条件にする

補足:
- relay/boostrap は **デフォルトは認証OFF** とし、管理画面から後から必須化できる（詳細は各サービス設計を参照）。
  - 認証OFFの間は「ユーザー操作の手間を最小化」するため、同意も不要とする（relay は匿名のため同意を強制できない）。
- 認証OFF→ON 切替時の既存接続/猶予期間/互換性は `docs/03_implementation/community_nodes/auth_transition_design.md` を参照。

## 認証（v1確定）

課金・購読を pubkey 単位で扱う前提で、**Nostr鍵（secp256k1）ベース**の認証に寄せる。

### HTTP（User API）

- **署名チャレンジ方式（v1採用）**
  - 目的: “毎リクエスト署名”を避け、クライアント実装コストと運用負荷を下げる
  - フロー:
    1. `POST /v1/auth/challenge`（入力: `pubkey`）→ `challenge` + `expires_at`
    2. `POST /v1/auth/verify`（入力: `auth_event_json`）→ `access_token`（短命）+ `expires_at`
    3. 以後、`Authorization: Bearer <access_token>` で保護 API を呼び出す
  - `auth_event_json` は **NIP-42 と同形式の署名済みイベント（kind=22242）**を推奨する
    - 必須 tags（推奨の最小）:
      - `["relay","<PUBLIC_BASE_URL>"]`（User API の公開 base URL。reverse proxy 後の URL を正とする）
      - `["challenge","<challenge>"]`
    - 任意（推奨）:
      - `["scope","user-api"]`（他用途へのリプレイを避けるため、User API 側で検証してもよい）
  - サーバの検証（v1最小）
    - `kind==22242`
    - `created_at` が許容範囲（例: 10分以内）
    - `relay` が `PUBLIC_BASE_URL` と一致
    - `challenge` が未使用かつ期限内（単回使用）
    - 署名検証 OK（`pubkey` と `sig`）
  - token（v1確定）
    - `access_token` は **JWT（HS256）**とする（短命、例: 15分。refresh token は v1 では持たない）
    - 推奨 claims（最小）: `sub=<pubkey>`, `exp`, `iat`, `jti`, `aud="kukuri-community-node:user-api"`, `iss=<PUBLIC_BASE_URL>`
    - 失効の即時反映: `cn_user.subscriber_accounts` 等に `status=disabled/deleting/deleted` を持ち、保護 API は token の有効期限内でも拒否する
    - 認可（同意/購読/停止）は毎リクエスト DB 状態で判定し、token の状態に閉じ込めない（即時反映）

補足（v1最小: 認証状態のDB）:

- `cn_user.subscriber_accounts`
  - `subscriber_pubkey TEXT PRIMARY KEY`
  - `status TEXT NOT NULL`（`active|disabled|deleting|deleted`）
  - `updated_at TIMESTAMPTZ NOT NULL`

- **NIP-98（HTTP Auth）互換（v2候補）**
  - NIP-98（kind=27235）を `Authorization: Nostr <base64(event)>` で受理する互換モードを v2 で検討する
  - reverse proxy 配下の “絶対URL一致” や “毎リクエスト署名” の実装負荷があるため、v1 は採用しない

### WS（relay）

- NIP-42（AUTH event）での認証を前提にする（購読制限/課金が必要な場合）

## 規約/プライバシーポリシー同意（必須）

- 保護された API（検索/トラスト/購読申請/通報など）は、**current の ToS/Privacy への同意**を前提条件にする
- 未同意の場合、User API は `428 Precondition Required`（例）で `CONSENT_REQUIRED` を返す
- ポリシーの取得と同意登録は、同意がなくても到達できる必要がある（ただし同意登録は pubkey を確定するため認証は必要）

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

### Access Control（invite/keys）

- v1 は P2P-only。User API に `/v1/invite/redeem` `/v1/keys/envelopes` は提供しない。
- クライアントは `access_control_issue_invite` / `access_control_request_join` を利用する。


### 規約/プライバシー

- `GET /v1/policies/current`
- `GET /v1/policies/:type/:version?locale=ja-JP`
- `GET /v1/consents/status`
- `POST /v1/consents`

### bootstrap（デフォルト: 認証OFF）

- `GET /v1/bootstrap/nodes`（node descriptor の一覧/差分）
- `GET /v1/bootstrap/topics/:topic/services`（topic_service の一覧）
- `GET /v1/bootstrap/hints/latest?since=<seq>`（更新ヒントの最新スナップショット。新規がなければ `204`）

配布ポリシー（v1）:
- 39000/39001 は `bootstrap` が署名生成し、DB に保存された **署名済み event JSON** を配布する（User API は生成しない）
- `ETag` / `Last-Modified` による条件付きGETを推奨し、`Cache-Control: max-age=300` 程度の短期キャッシュを許容する
- gossip/DHT/既知URL は発見/更新のヒントであり、最終整合は `GET /v1/bootstrap/*` の取得で行う
  - 詳細: `docs/03_implementation/community_nodes/services_bootstrap.md`

### topic購読

- `POST /v1/topic-subscription-requests`
  - DoS ガード（pending 同時保留数上限）に到達した場合は `429 Too Many Requests` を返す。
  - エラー契約: `code="PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED"` と `details.metric/current/limit/scope`。
- `GET /v1/topic-subscriptions`
- `DELETE /v1/topic-subscriptions/:topic_id`（解約/停止）

詳細: `docs/03_implementation/community_nodes/topic_subscription_design.md`

### 検索/サジェスト/トレンド（index）

- `GET /v1/communities/suggest?q=...&limit=...`
- `GET /v1/search?topic=...&q=...`
- `GET /v1/trending?topic=...`

補足:
- `/v1/communities/suggest` の `q` は `search_normalizer` で正規化し、空文字になった場合は `items=[]` を返す。
- `/v1/communities/suggest` と `/v1/search` の `limit` は `1..50` に clamp（未指定は 20）。
- `/v1/communities/suggest` で `suggest_read_backend=pg` かつ Stage-A 候補が 0 件の場合、`legacy_fallback` へ自動フォールバックする。

### モデレーション（moderation）

- `POST /v1/reports`
- `GET /v1/labels?target=...`

### トラスト（trust）

- `GET /v1/trust/report-based?subject=...`
- `GET /v1/trust/communication-density?subject=...`
  - `subject` は `pubkey:<hex>` / `event:<32-byte-hex>` / `relay:<url>` / `topic:<topic_id>` / `addressable:<kind>:<pubkey>:<d-tag>` を受理
  - score row（`report_scores` / `communication_scores`）が存在し、参照先 `attestation_id` が失効/欠損している場合は `assertion: null` を返す（最新 assertion への自動フォールバックは行わない）
  - score row が存在しない場合は `cn_trust.attestations` の最新 active assertion（subject + claim）を参照して返却できる

### 個人データ（削除/エクスポート）

- `POST /v1/personal-data-export-requests`
- `GET /v1/personal-data-export-requests/:export_request_id`
- `GET /v1/personal-data-export-requests/:export_request_id/download`
- `POST /v1/personal-data-deletion-requests`
- `GET /v1/personal-data-deletion-requests/:deletion_request_id`

## 検索/サジェスト runtime flag 運用メモ

正本は `cn_search.runtime_flags`。`cn-user-api` / `cn-index` はこのテーブルを参照して挙動を切り替える。

| flag_name | 値 | 対象 | 実装時挙動 |
|---|---|---|---|
| `search_read_backend` | `pg` | `/v1/search`（`cn-user-api`） | PostgreSQL 検索を使用（`pgroonga`）。 |
| `search_write_mode` | `pg_only` | 検索索引書込（`cn-index`） | outbox から `cn_search.post_search_documents` へ書込。 |
| `suggest_read_backend` | `legacy` / `pg` | `/v1/communities/suggest`（Stage-A） | 候補生成の backend を切替。未知値・読取失敗時は `legacy`。 |
| `suggest_rerank_mode` | `shadow` / `enabled` | `/v1/communities/suggest`（Stage-B） | `enabled` で rerank 順を応答順に適用。`shadow` は Stage-A 順を維持しつつ `stage_b_rank` と比較メトリクスを記録。 |
| `suggest_relation_weights` | JSON | `/v1/communities/suggest`（Stage-B） | relation score 重み。JSON 不正時は既定値へフォールバック。 |
| `shadow_sample_rate` | `0` - `100` | `/v1/communities/suggest` | sampled shadow 比較率。数値以外は `0`、`100` 超は `100` に丸める。 |

運用メモ:
- read backend（`search_read_backend` / `suggest_read_backend`）は二値フラグであり比率適用しない。5% / 25% / 50% のカナリア段階は `shadow_sample_rate` で実施する。
- 切替 SQL は `INSERT ... ON CONFLICT (flag_name) DO UPDATE` で更新する（詳細手順: `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`）。

## 課金/利用量計測（最小モデル）

決済プロバイダ連携（Stripe 等）は後回しでも、内部データモデルは先に用意する。

- `plans`（提供メニュー: topics上限、検索回数上限、保持期間、優先度等）
- `subscriptions`（pubkey → plan、状態、期間）
- `usage_counters`（日次/週次のAPI利用量、超過判定）

v1 の詳細（課金単位、メータリング、超過時の挙動、無料枠/上限、監査）は `docs/03_implementation/community_nodes/billing_usage_metering.md` を参照。
瞬間レート超過（DoS/濫用）の実装方針（Redis無し/in-mem）は `docs/03_implementation/community_nodes/rate_limit_design.md` を参照。

## リレー（取込）との関係

- User API は **購読 topic の申請/承認**を行い、DB に反映する
- relay は DB の「ノード取込購読（node-level subscription）」を監視し、topic の subscribe/unsubscribe を実施する
- `index/moderation/trust` は relay が保存したレコードを入力として扱う（取込経路の一本化）

詳細は `docs/03_implementation/community_nodes/topic_subscription_design.md` を参照。

Access Control（39020/39021/39022、P2P join、epoch ローテ）は `docs/03_implementation/community_nodes/access_control_design.md` を参照。

## 実装スタック（参照）

- Web フレームワーク/OpenAPI/認証/middleware/logging/metrics の決定: `docs/03_implementation/community_nodes/api_server_stack.md`
