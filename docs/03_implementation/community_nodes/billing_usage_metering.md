# 課金/利用量計測（Billing & Metering）設計（v1）

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（主に User API、補助で relay/bootstrap）

## ゴール

- 将来の決済連携（Stripe 等）を後付けできるよう、**先にデータモデル/計測/超過時挙動/監査**を確定する
- 課金/権限/レート制限の入口を **User API に集約**し、サービス分離（profiles/別ホスト）しても運用できる
- relay/bootstrap はデフォルト認証OFF（同意不要）を維持しつつ、**認証ON時のみ** pubkey ベースの課金/権限制御を適用できるようにする

## 参照（ローカル）

- `docs/03_implementation/community_nodes/user_api.md`（外部I/F統合・認証/同意）
- `docs/03_implementation/community_nodes/topic_subscription_design.md`（user-level subscription / node-level subscription）
- `docs/03_implementation/community_nodes/policy_consent_management.md`（同意必須化）
- `docs/03_implementation/community_nodes/auth_transition_design.md`（relay/bootstrap 認証OFF→ON 切替）
- `docs/03_implementation/community_nodes/personal_data_handling_policy.md`（保持期間/削除・エクスポート/同意ログ）

## 前提（v1の割り切り）

- 課金主体は **pubkey**（User API の認証単位）
- v1 では「請求書発行/決済」は未実装でもよい（状態管理とメータリングを先に作る）
- relay/bootstrap が認証OFFの間は pubkey を確定できないため、課金というより **IPレート制限**が中心になる

## 課金モデル（v1提案）

### 1) プラン（plan）

プランは「上限（クォータ）と権利（entitlement）」の集合。

- 例: `max_topics`, `search_requests_per_day`, `trust_requests_per_day`, `reports_per_day` 等
- 実装は `plan_limits`（行）または `limits_json`（JSON）どちらでもよいが、v1 は **行モデル**を推奨（集計/比較が楽）

### 2) サブスクリプション（pubkey → plan）

- pubkey ごとに 1 つの有効プラン（`active`）を持つ（v1）
- 将来、期間課金・一時停止・トライアル等の状態を追加できるようにする

### 3) topic購読（user-level subscription）

topic購読は「そのtopicのサービス結果を使う権利」。

- billing 的には「購読できるtopic数の上限（`max_topics`）」が最初の差別化ポイント
- `topic-subscription-requests` の approve 時に `user-level subscription` を `active` にし、プラン上限を超える場合は拒否する

補足:
- Access Control（invite redeem）で購読を同時に `active` にする運用は許容（詳細: `docs/03_implementation/community_nodes/access_control_design.md`）。

## 計測メトリクス（v1提案）

### 基本単位

- v1 は原則 **request count** を計測単位とする（bytes課金は v2）
- 例（推奨のメトリクス名）
  - `index.search_requests`
  - `index.trending_requests`
  - `trust.requests`
  - `moderation.report_submits`
  - `invite.redeem_attempts`（濫用されやすいので “attempts” を推奨）

### relay（認証ON時のみ、v1は任意）

relay を課金対象にする場合、認証（NIP-42 等）で pubkey を特定できることが前提。

- `relay.ws_event_publishes`（`EVENT` の受理成功）
- `relay.ws_backfill_requests`（`REQ` のバックフィル要求）

## メータリング方式（idempotency/二重計上防止）

### 計測点

- **User API の認可レイヤ**で計測する（入口を一つにして一貫性を保つ）
- “成功時のみカウント”が基本。ただし濫用防止が目的のメトリクスは “attempt” を別メトリクスで持つ

### Idempotency（推奨）

リトライ/再送で二重計上しないため、次を推奨する。

- クライアントは `X-Request-Id` を付与（UUID等）
- サーバは `(pubkey, request_id, metric)` を一意に記録し、同一キーは **再加算しない**
  - 可能なら同一レスポンスを返す（idempotent response）

## 超過時の挙動（v1提案）

### 1) クォータ超過（課金/上限）

- HTTP: `402 Payment Required` を推奨
- body（例）:
  - `code: "QUOTA_EXCEEDED"`
  - `metric`, `current`, `limit`, `reset_at`
  - `upgrade_url`（任意）

### 2) 瞬間レート超過（DoS/濫用）

- HTTP: `429 Too Many Requests` + `Retry-After`

### 3) relay WS（認証ON時）

- `NOTICE` で理由（`auth-required` / `consent-required` / `quota-exceeded`）を通知し、必要に応じて `CLOSED`/切断
- 認証OFF→ON 切替時の扱いは `docs/03_implementation/community_nodes/auth_transition_design.md` に従う

## 無料枠/上限（初期値の例）

運用で調整する前提で、v1 の初期案を置く。

- Free
  - `max_topics = 1`
  - `index.search_requests <= 100/day`
  - `trust.requests <= 100/day`
  - `moderation.report_submits <= 20/day`
- Paid（例）
  - `max_topics = N`（プラン別）
  - `index.search_requests`/`trust.requests` を段階的に増やす

## 監査（必須）

### 監査対象

- plan 作成/更新（上限変更）
- pubkey の plan 変更、購読の approve/revoke、手動の例外付与
- 主要APIの利用量イベント（メータリング）

### 方針

- 監査ログは **append-only** を基本（訂正は相殺イベントで行う）
- 保持期間は最低 180日（運用・会計・調査のため）
- アカウント削除要求時は、会計/濫用調査のために “識別子を匿名化した監査イベント” として保持し得る（詳細: `docs/03_implementation/community_nodes/personal_data_handling_policy.md`）

## DB データモデル（提案）

`cn_user` に課金/利用量、`cn_admin` に監査（管理操作）を寄せる。

### `cn_user.plans`

- `plan_id TEXT PRIMARY KEY`
- `name TEXT NOT NULL`
- `is_active BOOL NOT NULL`
- `created_at TIMESTAMPTZ NOT NULL`

### `cn_user.plan_limits`

- `plan_id TEXT NOT NULL`
- `metric TEXT NOT NULL`
- `window TEXT NOT NULL`（`day|week|month` のいずれか。v1 は `day` で開始推奨）
- `limit BIGINT NOT NULL`
- `PRIMARY KEY(plan_id, metric, window)`

### `cn_user.subscriptions`

- `subscription_id TEXT PRIMARY KEY`
- `subscriber_pubkey TEXT NOT NULL`
- `plan_id TEXT NOT NULL`
- `status TEXT NOT NULL`（`active|paused|ended`）
- `started_at TIMESTAMPTZ NOT NULL`
- `ended_at TIMESTAMPTZ NULL`

### `cn_user.usage_counters_daily`（v1推奨）

- `subscriber_pubkey TEXT NOT NULL`
- `metric TEXT NOT NULL`
- `day DATE NOT NULL`
- `count BIGINT NOT NULL`
- `PRIMARY KEY(subscriber_pubkey, metric, day)`

### `cn_user.usage_events`（監査/再計算用、append-only）

- `event_id BIGSERIAL PRIMARY KEY`
- `subscriber_pubkey TEXT NOT NULL`
- `metric TEXT NOT NULL`
- `day DATE NOT NULL`
- `request_id TEXT NULL`（`X-Request-Id`）
- `units BIGINT NOT NULL`（v1は 1）
- `outcome TEXT NOT NULL`（`ok|rejected|error`）
- `created_at TIMESTAMPTZ NOT NULL`
- `UNIQUE(subscriber_pubkey, metric, request_id)`（`request_id` がある場合）

## 実装メモ（v1）

- まずは User API の read API（search/trending/trust）からメータリングを適用する
- report/redeem は “attempts” を別メトリクスにして濫用対策に寄せる
- Admin Console は購読一覧に加えて、pubkey ごとの usage（当日/過去N日）を見られると運用が安定する
