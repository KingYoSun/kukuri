# Admin API（control plane）最小設計（v1）

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（Admin API / Admin Console / 全サービス）

## 目的

- `Admin Console` からの管理操作を **1つの入口（Admin API）** に集約する（ブラウザから各サービスを直接叩かない）
- 設定（config）と監査（audit）の **運用上の正（source of truth）を `cn_admin` に置く**
- サービスの稼働状態（health）を集約し、UI を「未起動/未接続でも壊れない」形にする
- 設定変更の反映方式（poll / `LISTEN/NOTIFY`）を v1 方針として固定し、実装ブレを防ぐ

## スコープ（v1）

### Admin API が担う

- 管理者認証（admin-only、RBACは後回し）
- サービス設定の CRUD（正は DB）
- 監査ログ（append-only）
- services health の集約（内部NWで `GET /healthz` を監視し、DBに保存）
- 設定変更通知（`NOTIFY`。最終整合は DB の再取得）

### Admin API が担わない（原則）

- ユーザー向けの公開 API（それは `User API`）
- リレー互換 WS の実装（それは `relay`）
- 秘匿値の DB 平文保存（secrets は env/secret に置く）

## 運用上の正（source of truth）

- **SoT = `cn_admin`**（設定・監査・health の最新状態）
- 画面（Admin Console）は **Admin API のみ**を呼び出し、サービス内部 API をブラウザから直叩きしない

## 認証（v1最小）

- v1 は **admin単一ロール**を前提にする（RBACは v2）
- 推奨: password login + **`httpOnly` セッションCookie**
  - ブラウザ運用での XSS 耐性を優先
- 代替: JWT（`Authorization: Bearer`）でもよいが、失効/ローテの運用を先に決める（v1 は cookie を推奨）

## データモデル（`cn_admin` 提案）

### 1) 管理者

- `admin_users`
  - `admin_user_id TEXT PRIMARY KEY`
  - `username TEXT UNIQUE NOT NULL`
  - `password_hash TEXT NOT NULL`
  - `is_active BOOL NOT NULL`
  - `created_at TIMESTAMPTZ NOT NULL`
- `admin_sessions`（cookie運用の場合）
  - `session_id TEXT PRIMARY KEY`
  - `admin_user_id TEXT NOT NULL`
  - `expires_at TIMESTAMPTZ NOT NULL`
  - `created_at TIMESTAMPTZ NOT NULL`

### 2) サービス設定（正）

単一の設定モデルに寄せ、サービスごとの設定は `config_json` に入れる。

- `service_configs`
  - `service TEXT PRIMARY KEY`（例: `relay`, `bootstrap`, `user-api`, `moderation`）
  - `version BIGINT NOT NULL`（単調増加。更新ごとに `+1`）
  - `config_json JSONB NOT NULL`
  - `updated_at TIMESTAMPTZ NOT NULL`
  - `updated_by TEXT NOT NULL`（admin_user_id）

方針:
- DBに保存してよいのは **非秘匿の設定**（閾値、ON/OFF、URL等）
- `OPENAI_API_KEY` 等の secrets は DB に保存せず、コンテナの env/secret で注入する
- **優先順位（v1）**: `cn_admin.service_configs` を正（SoT）とし、env は初回seed入力と secrets に限定する（seed 後に env 変更で挙動がドリフトしないようにする）

### 3) 監査ログ（必須 / append-only）

- `audit_logs`
  - `audit_id BIGSERIAL PRIMARY KEY`
  - `actor_admin_user_id TEXT NOT NULL`
  - `action TEXT NOT NULL`（例: `service_config.update`, `policy.publish`）
  - `target TEXT NOT NULL`（例: `service:relay`, `policy:privacy:2026-01-23`）
  - `diff_json JSONB NULL`（before/after の差分。秘匿は redaction する）
  - `request_id TEXT NULL`
  - `created_at TIMESTAMPTZ NOT NULL`

### 4) services health 集約（UI安定化のため）

- `service_health`
  - `service TEXT PRIMARY KEY`
  - `status TEXT NOT NULL`（`healthy|degraded|unreachable`）
  - `checked_at TIMESTAMPTZ NOT NULL`
  - `details_json JSONB NULL`（latency/依存関係など。本文/識別子は入れない）

## services health 集約（v1）

- 各サービスは内部NWで `GET /healthz` を提供する
- `Admin API` が定期ポーリング（例: 10s〜60s）で収集し、`cn_admin.service_health` を更新する
- Admin Console は `Admin API` の集約結果を表示する（サービス未起動でも `unreachable` として扱える）

## 設定反映方式（poll vs `LISTEN/NOTIFY`）

v1 は「push + pull のハイブリッド」を推奨し、どちらか片方に依存しない。

- push（起床通知）: `Admin API` が `service_configs` 更新後に `NOTIFY cn_admin_config, '<service>:<version>'`
- pull（最終整合）: 各サービスは DB から `service_configs` を再取得して反映する
  - NOTIFY は取りこぼし得るため、各サービスは `poll_interval_seconds`（例: 30s）でも追従する

## 最小 API（例）

### 認証

- `POST /v1/admin/auth/login`
- `POST /v1/admin/auth/logout`
- `GET /v1/admin/auth/me`

### サービス設定

- `GET /v1/admin/services`（一覧 + health）
- `GET /v1/admin/services/:service/config`
- `PUT /v1/admin/services/:service/config`（更新。`audit_logs` へ記録）

### 監査

- `GET /v1/admin/audit-logs?service=...&action=...&since=...`

## 初期セットアップ（v1推奨）

DB が正（SoT）のため、運用開始時は「migrate + seed」を明示的に行う。

- `cn-cli migrate`
  - Postgres の migrations を適用する（サービスは勝手に migrate しない）
- `cn-cli admin bootstrap`
  - 初回 admin を作成する（既に admin が存在する場合は何もしない）
  - 監査は `system` 相当の actor で記録する（`audit_logs`）
- （復旧）`cn-cli admin reset-password`
  - admin のパスワードを再設定する（運用事故時の手順として用意しておく）
- `cn-cli config seed`
  - `service_configs` が無いサービス分の default `config_json` を投入する（既存は上書きしない）

補足:
- secrets（例: `OPENAI_API_KEY`）は seed せず env/secret で注入する。

## 既存設計との接続点（例）

- relay/bootstrap 認証OFF→ON の切替設定（`auth_mode/enforce_at/grace_seconds/...`）は `service_configs` に格納し、更新を監査する
  - 詳細: `docs/03_implementation/community_nodes/auth_transition_design.md`
- rate limit（DoS/濫用対策）の設定も `service_configs` に格納し、更新を監査する
  - 詳細: `docs/03_implementation/community_nodes/rate_limit_design.md`
- 規約/プライバシーポリシー（policies）の管理 API も Admin API が提供し、`audit_logs` に残す
  - 詳細: `docs/03_implementation/community_nodes/policy_consent_management.md`

## 実装スタック（参照）

- Web フレームワーク/OpenAPI/認証/middleware/logging/metrics の決定: `docs/03_implementation/community_nodes/api_server_stack.md`
