# Issue #152 Community Node Admin UI Bootstrap サイドバー追加レポート

作成日: 2026年02月24日

## 概要

- 対象:
  - `kukuri-community-node/apps/admin-console/src/App.tsx`
  - `kukuri-community-node/apps/admin-console/src/App.test.tsx`
  - `kukuri-community-node/apps/admin-console/src/lib/bootstrap.ts`
  - `kukuri-community-node/apps/admin-console/src/pages/SubscriptionsPage.tsx`
- Admin Console サイドバーに `Bootstrap` カードを追加し、接続先 `node_id@host:port`、接続ユーザー一覧、接続ユーザー数を表示できるようにした。
- 既存 API（`/v1/admin/node-subscriptions` と `/v1/admin/subscriptions`）を再利用し、新規 API 追加なしで要件を満たした。

## 実装詳細

- `App.tsx`
  - `useQuery` で `api.nodeSubscriptions()` と `api.subscriptions()` を取得。
  - `nodeSubscriptions.connected_nodes` を `normalizeConnectedNode` で正規化し、重複排除・ソートして `node_id@host:port` 表示。
  - `subscriptions` は `subscriber_pubkey` 単位で最新 `started_at` を採用し、`status === "active"` のみを接続ユーザーとして集計。
  - サイドバー `Bootstrap` カードに以下を表示:
    - 接続ユーザー数
    - 接続先一覧
    - 接続ユーザー一覧
    - 読み込み中・取得失敗時の Notice

- `lib/bootstrap.ts`
  - 接続先表示の正規化関数 `normalizeConnectedNode` を共通化。
  - `SubscriptionsPage` 側の重複実装を削除して同関数を再利用。

- `App.test.tsx`
  - API モックへ `nodeSubscriptions` / `subscriptions` を追加。
  - Bootstrap カードで以下を検証するテストを追加:
    - `Connected users` 件数
    - `node_id@host:port` 形式の接続先表示
    - 接続ユーザー一覧（最新状態が paused のユーザーは除外）

## 実行コマンド

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cd /workspace/kukuri-community-node/apps/admin-console; pnpm install --frozen-lockfile; pnpm test -- src/App.test.tsx src/pages/SubscriptionsPage.test.tsx"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
