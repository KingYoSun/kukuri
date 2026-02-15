# Issue #22 Task3: node-level 同時取込 topic 数上限の実装

作成日: 2026年02月15日

## 概要

- 対象: `cn-admin-api` / `cn-user-api` / `cn-relay` の購読承認フロー境界
- 目的: `topic_subscription_design.md` の DoS 要件にある node-level 同時取込 topic 数上限を承認時に強制し、超過時に明示契約で拒否する。
- スコープ外: node-level 上限に関する専用の回帰テスト拡張（`cn-relay` 側統合シナリオ拡張を含む）は次PRで実施。

## 実施内容

- `cn-admin-api`:
  - `approve_subscription_request` に node-level 上限チェックを追加。
  - 上限判定は transaction 内で advisory lock（2キー）を取得して実行し、同時承認時の競合超過を防止。
  - 上限超過時は `429 Too Many Requests` + `code=NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED` + `details(metric/scope/current/limit)` を返す。
  - OpenAPI に `POST /v1/admin/subscription-requests/{request_id}/approve` の `429` レスポンスを追加。
  - 契約テスト `subscription_request_approve_rejects_when_node_topic_limit_reached` を追加し、拒否契約と pending 維持を固定。
- `cn-core`:
  - `service_config` に `node_subscription.max_concurrent_topics` パーサを追加し、既定値 `100` を定義。
- `cn-relay`:
  - runtime config に `node_subscription.max_concurrent_topics` を追加。
  - relay の enabled topic 同期/health 判定で `max_concurrent_topics` を参照し、`ORDER BY updated_at DESC LIMIT` で購読対象を上限内に制約。

## 契約として固定した仕様

- 承認で新規 topic 有効化が必要な場合、`enabled=true` 件数が上限以上だと承認を拒否する。
- 拒否レスポンス:
  - `status`: `429`
  - `code`: `NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED`
  - `details.metric`: `node_subscriptions.enabled_topics`
  - `details.scope`: `node`
  - `details.current` / `details.limit`: 判定時の値
- 超過拒否時は `topic_subscription_requests.status` は `pending` のまま残り、`topic_subscriptions` / `node_subscriptions` の副作用は発生しない。

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs`
- `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/crates/cn-core/src/service_config.rs`
- `kukuri-community-node/crates/cn-relay/src/config.rs`
- `kukuri-community-node/crates/cn-relay/src/gossip.rs`
- `kukuri-community-node/crates/cn-relay/src/lib.rs`
- `kukuri-community-node/apps/admin-console/openapi/admin-api.json`
- `kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`

## 検証

- Community Node（Docker 経路）
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
  - `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
  - `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -e RUST_TEST_THREADS=1 -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
  - 結果: `success`
- OpenAPI 生成物
  - `cd kukuri-community-node && cargo run --locked -p cn-cli -- openapi export --service user-api --output apps/admin-console/openapi/user-api.json --pretty && cargo run --locked -p cn-cli -- openapi export --service admin-api --output apps/admin-console/openapi/admin-api.json --pretty`
  - `cd kukuri-community-node/apps/admin-console && pnpm install --frozen-lockfile && pnpm generate:api`
  - 結果: `success`
- AGENTS 必須 `gh act` ジョブ
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - 結果: `success`（ログ: `tmp/logs/gh-act-format-check-issue22-node-level-topic-limit.log`, `tmp/logs/gh-act-native-test-linux-issue22-node-level-topic-limit.log`, `tmp/logs/gh-act-community-node-tests-issue22-node-level-topic-limit.log`）

## 次アクション

- Issue #22 の残タスクとして、node-level 上限の専用回帰テスト（`cn-admin-api` 契約 + `cn-relay` 統合）を次PRで実施する。
