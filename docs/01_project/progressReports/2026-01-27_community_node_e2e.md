# Community Node E2E 組み込み
日付: 2026年01月27日

## 概要
- Community Node の設定/認証/同意取得を E2E で検証できるように、モック API とシナリオを追加。
- UI に data-testid を追加して E2E 操作の安定性を確保。

## 対応内容
- WDIO 準備時に Community Node モックサーバーを起動し、`E2E_COMMUNITY_NODE_URL` を配布。
- Community Node 設定 UI に data-testid を追加。
- Community Node 設定/認証/同意取得の E2E スペックを追加。

## 検証
- `./scripts/test-docker.ps1 e2e`
- `gh act --workflows .github/workflows/test.yml --job format-check`（警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（警告: `git clone` の `some refs were not updated`、`useRouter must be used inside a <RouterProvider>`）

## 補足
- `native-test-linux` の `useRouter must be used inside a <RouterProvider>` は既存警告。

## 追記（2026年01月27日）
- docker-compose.test.yml に community node (Postgres/Meilisearch/User API) を追加。
- scripts/test-docker.* に e2e-community-node を追加し、SCENARIO=community-node-e2e と E2E_COMMUNITY_NODE_URL を配布。
- WDIO/テストで mock/実ノード切替に対応。
