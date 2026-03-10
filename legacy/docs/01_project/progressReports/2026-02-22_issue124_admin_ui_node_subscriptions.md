# Issue #124 Admin UI node subscription 改善レポート

作成日: 2026年02月22日

## 概要

- 対象:
  - `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs`
  - `kukuri-community-node/apps/admin-console/src/pages/SubscriptionsPage.tsx`
- Admin UI で Relay 接続情報を `node_id@host:port` で表示し、topic 単位の接続ノード一覧/件数を確認できるようにした。
- 併せて node subscription の CRUD（追加/編集/削除）を Admin UI で操作可能にした。
- 既存の認可（admin session）、状態管理、保存挙動（更新成功時の再取得・エラー表示）は維持。

## 実装詳細

- `cn-admin-api`
  - `NodeSubscription` に `connected_nodes` / `connected_node_count` を追加。
  - `POST /v1/admin/node-subscriptions`（create）を追加。
  - `DELETE /v1/admin/node-subscriptions/{topic_id}`（delete）を追加。
  - `load_connected_nodes_by_topic` を追加し、`cn_bootstrap.events` の:
    - kind `39000`（descriptor）から node endpoint を抽出
    - kind `39001`（topic service）から topic ごとの接続ノードを集約
  - 表示形式を `node_id@host:port` に統一。
  - `ref_count > 0` の topic delete を `409 NODE_SUBSCRIPTION_IN_USE` で拒否。
  - ルーティング・OpenAPI 定義・契約テストを更新。

- `admin-console`
  - `SubscriptionsPage` に以下を追加。
    - 接続ノードの件数/一覧表示（topic 単位）
    - 新規 topic 追加フォーム
    - topic 削除ボタンと行単位エラー表示
  - 既存編集（enabled toggle / ingest policy 保存）を維持。
  - `api.ts` に `createNodeSubscription` / `deleteNodeSubscription` を追加。
  - OpenAPI artifact（`openapi/admin-api.json` / `src/generated/admin-api.ts`）を同期。
  - `SubscriptionsPage.test.tsx` を CRUD + 接続ノード表示に追従。

## 検証

- `cargo run --locked -p cn-cli -- openapi export --service admin-api --output apps/admin-console/openapi/admin-api.json --pretty`
- `docker compose --project-name kukuri -f docker-compose.test.yml up -d --wait community-node-postgres`
- `docker compose --project-name kukuri -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cd /workspace/kukuri-community-node/apps/admin-console; pnpm install --frozen-lockfile; pnpm generate:api; pnpm test -- src/pages/SubscriptionsPage.test.tsx; cd /workspace/kukuri-community-node; cargo test -p cn-admin-api subscription_requests_and_node_subscriptions_contract_success -- --nocapture"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-admin-api -- --nocapture"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `docker run --rm -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-tauri/src-tauri kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
