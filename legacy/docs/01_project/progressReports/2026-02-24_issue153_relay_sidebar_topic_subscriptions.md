# Issue #153 Relay サイドバー追加とトピック購読管理移設レポート

作成日: 2026年02月24日

## 概要

- 対象:
  - `kukuri-community-node/apps/admin-console/src/App.tsx`
  - `kukuri-community-node/apps/admin-console/src/router.tsx`
  - `kukuri-community-node/apps/admin-console/src/pages/RelayPage.tsx`
  - `kukuri-community-node/apps/admin-console/src/pages/SubscriptionsPage.tsx`
  - `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs`
- Admin Console のサイドバーに `Relay` を追加し、トピック購読管理（CRUD）を `Subscriptions` から `Relay` へ移設した。
- Topic ごとの接続ユーザー一覧と接続ユーザー数を API レスポンスに追加し、Relay 画面で可視化した。

## 実装詳細

- `App.tsx` / `router.tsx`
  - サイドバーに `Relay` 項目を追加。
  - 新規ルート `/relay` を追加し、`RelayPage` を表示可能にした。

- `pages/RelayPage.tsx`
  - node-subscription の一覧、作成、更新、削除を1画面に集約。
  - ingest policy の編集 UI を維持しつつ、トピックごとの接続ノード一覧/件数を表示。
  - API 追加フィールドを利用し、トピックごとの接続ユーザー一覧/件数を表示。

- `pages/SubscriptionsPage.tsx`
  - topic subscription 管理 UI を削除し、購読申請・プラン・ユーザー購読・利用状況表示に責務を限定。

- `cn-admin-api/src/subscriptions.rs`
  - `NodeSubscription` に `connected_users` と `connected_user_count` を追加。
  - `cn_user.topic_subscriptions` から `status='active'` かつ `ended_at IS NULL` のユーザーを topic 単位で集約する処理を追加。
  - list/create/update のレスポンスで接続ユーザー情報を返すようにした。

- テスト
  - `RelayPage.test.tsx` を新規追加し、CRUD 操作と接続ユーザー表示を検証。
  - `SubscriptionsPage.test.tsx` を更新し、移設後の責務に沿った検証へ更新。
  - `contract_tests.rs` を更新し、承認後の接続ユーザー数と一覧の契約を検証。

## 実行コマンド

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo run --locked -p cn-cli -- openapi export --service user-api --output apps/admin-console/openapi/user-api.json --pretty; cargo run --locked -p cn-cli -- openapi export --service admin-api --output apps/admin-console/openapi/admin-api.json --pretty"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node/apps/admin-console kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; pnpm install --frozen-lockfile; pnpm generate:api; pnpm typecheck; pnpm test -- src/App.test.tsx src/pages/RelayPage.test.tsx src/pages/SubscriptionsPage.test.tsx"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-admin-api subscription_requests_and_node_subscriptions_contract_success -- --nocapture; cargo test --workspace --all-features; cargo build --release -p cn-cli; cargo fmt --all --check"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
