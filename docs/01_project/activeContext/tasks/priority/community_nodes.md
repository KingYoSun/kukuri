# Community Nodes 実装タスク（M1）

最終更新日: 2026年01月23日

目的: `docs/03_implementation/community_nodes/*` の設計に基づき、次セッションから実装に着手できるように具体タスクを整理する。

参照:
- `docs/03_implementation/community_nodes/summary.md`
- `docs/03_implementation/community_nodes/repository_structure.md`
- `docs/03_implementation/community_nodes/docker_compose_profiles.md`
- `docs/03_implementation/community_nodes/api_server_stack.md`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/03_implementation/community_nodes/admin_api.md`
- `docs/03_implementation/community_nodes/user_api.md`

ロードマップ:
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## M1: リポジトリ雛形 + Compose（実装着手ブロッカー解消）

### 1) リポジトリ/ワークスペース雛形

- [ ] `./kukuri-community-node/` を作成し、`repository_structure.md` に沿って最小の構造を作る
  - `apps/admin-console/`（空でもよい）
  - `crates/`（この時点では空の箱でもよい）
    - `cn-cli`, `cn-core`, `cn-kip-types`
    - `cn-user-api`, `cn-admin-api`
    - `cn-relay`, `cn-bootstrap`
    - `cn-index`, `cn-moderation`, `cn-trust`
  - `migrations/`（sqlx）
  - `docker/postgres-age/`（Apache AGE 入り Postgres イメージ）
- [ ] Cargo workspace を作成し、`cn-cli` からサブコマンドでサービスを起動できる形にする（単一バイナリ方式）

### 2) Compose/公開経路（v1確定: Caddy）

- [ ] `docker-compose.yml`（core: `postgres`/`relay`/`user-api`）を作成する
- [ ] Postgres は AGE 拡張入りイメージを前提とし、`docker/postgres-age/Dockerfile` を用意して compose で build する（詳細: `docker_compose_profiles.md`）
- [ ] `docker-compose.yml` に profile `admin`（`admin-api`/`admin-console`）を追加する
- [ ] reverse proxy は Caddy を採用し、外部公開を `https://<host>/api/*`（User API）+ `wss://<host>/relay`（relay）に集約する
  - `PUBLIC_BASE_URL=https://<host>/api` を正にする
- [ ] `.env.example` を作成し、最低限の env を揃える（`docker_compose_profiles.md` に準拠）
- [ ] profile `observability` を用意し、Prometheus/Grafana/OTel Collector を起動できるようにする（詳細: `ops_runbook.md` / `docker_compose_profiles.md`）

### 3) DB/migrations（sqlx）

- [ ] `cn-admin` と `cn-user` の最小スキーマを migrations で作る
  - `cn_admin`: `service_configs`, `audit_logs`, `admin_users`, `admin_sessions`, `service_health`
  - `cn_user`: `subscriber_accounts`（JWTでも即時失効できる状態）
- [ ] `cn-cli migrate`（one-shot）で migrations を適用できるようにする（サービスが勝手に migrate しない）
- [ ] `cn-cli config seed` で `cn_admin.service_configs` のデフォルトを投入できるようにする（既存は上書きしない）
- [ ] `cn-cli admin bootstrap` / `cn-cli admin reset-password` を実装し、初期 admin 作成/復旧手順を用意する

### 4) API サービス最小起動（axum + utoipa）

- [ ] `cn-user-api`（axum）を起動できるようにする
  - `GET /healthz`
  - `GET /metrics`（最小でOK）
  - `GET /v1/openapi.json`（utoipa 生成）
- [ ] `cn-user-api` に `api_server_stack.md` の基本 middleware（request id/timeout/body size limit/trace）と JSON logging を導入する（本文/トークンは出さない。詳細: `ops_runbook.md`）
- [ ] `cn-admin-api`（axum）を起動できるようにする
  - `GET /healthz`
  - `GET /metrics`
  - `GET /v1/openapi.json`
  - 認証方式は session cookie 前提で雛形だけ用意（詳細実装は後続タスクで段階的に）
- [ ] `cn-admin-api` にも同様に共通 middleware と JSON logging を導入する（詳細: `api_server_stack.md`）

### 5) relay/bootstrap の箱（先に起動できる状態へ）

- [ ] `cn-relay` / `cn-bootstrap` の daemon を「空でもよい」ので起動可能にする（`GET /healthz` を返す）
  - 本実装（iroh-gossip/WS/39000/39001）は M2 以降で段階的に投入

次: `docs/01_project/activeContext/tasks/priority/community_nodes_m2.md`
