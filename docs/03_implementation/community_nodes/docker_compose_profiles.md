# Docker Compose / profiles 設計

**作成日**: 2026年01月22日  
**対象**: `kukuri-community-node/docker-compose.yml`

## 目的

- 1コマンドで「必要なサービスだけ」起動できる
- 運用者が「全部入り」も「役割別」も選べる（ノード市場の前提）
- DB は 1 つの Postgres に集約し、サービス横断で参照できる

## profiles（提案）

- **コア（profiles無しで常時起動を推奨）**: `postgres` / `relay` / `user-api`
- `admin`: `admin-api` / `admin-console`
- `bootstrap`: bootstrap サービス
- `index`: index サービス + Meilisearch
- `moderation`: moderation サービス（必要に応じて `llm-*` と併用）
- `trust`: trust サービス（Apache AGE を利用）
- `llm-openai`: OpenAI Moderation API を利用（コンテナ追加は不要、設定のみ）
- `llm-local`: self-host LLM（例: `ollama` / `vllm` を別サービスとして起動）
- `observability`（任意）: Prometheus/Grafana/OTel Collector 等

運用要件（監視/バックアップ/マイグレーション/違法・通報対応）の詳細は `docs/03_implementation/community_nodes/ops_runbook.md` を参照。

## 起動コマンド例

```bash
# 管理画面と bootstrap だけ（core: postgres/relay/user-api は常時起動想定）
docker compose --profile admin --profile bootstrap up -d

# 全部入り（admin + 4サービス。relay/user-api は core）
docker compose --profile admin --profile bootstrap --profile index --profile moderation --profile trust up -d

# moderation + self-host LLM
docker compose --profile moderation --profile llm-local up -d
```

## サービス公開ポリシー（推奨）

- 外部公開は原則 `user-api`（HTTP）+ `relay`（WS）に集約し、他サービスは内部ネットワークに閉じる
- `admin-api` / `admin-console` は VPN/社内NW などで保護し、インターネット公開しない運用を推奨
- 外部公開時は reverse proxy（Caddy/Traefik 等）で TLS 終端 + 追加防御（WAF/Basic/Auth 等）を推奨

## 環境変数（例）

`.env` は `kukuri-community-node/.env` に配置する想定。

- `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB`
- `DATABASE_URL`（サービス側。例: `postgres://...`）
- `MEILI_URL` / `MEILI_MASTER_KEY`
- `ADMIN_JWT_SECRET`（または `ADMIN_SESSION_SECRET`）
- `USER_JWT_SECRET`（または `USER_SESSION_SECRET`）
- `OPENAI_API_KEY`（`llm-openai` で使用）
- `LLM_PROVIDER`（`openai` / `local` / `disabled`）
- `LLM_LOCAL_ENDPOINT`（`llm-local` 用。例: `http://ollama:11434`）
- `RELAY_AUTH_REQUIRED`（`true|false`、デフォルト: `false`）
- `BOOTSTRAP_AUTH_REQUIRED`（`true|false`、デフォルト: `false`）

補足:
- これらは「初期デフォルト」の想定で、運用中は Admin Console から DB 上の設定を更新して切り替えられるようにする。
  - 認証OFF（`false`）の間は同意（ToS/Privacy）も不要として扱い、認証ON（`true`）に切り替えた場合に同意チェックを有効化できるようにする。

## Postgres（Apache AGE）について

- trust 計算は Apache AGE を使うため、Postgres は `age` 拡張が導入されたイメージを前提とする
- 例: `kukuri-community-node/docker/postgres-age/Dockerfile` で `postgres:<version>` を拡張してビルドする
