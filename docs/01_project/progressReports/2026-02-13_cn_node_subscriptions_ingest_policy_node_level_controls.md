# Community Nodes 進捗レポート: `node_subscriptions.ingest_policy` node-level 制御
作業日: 2026年02月13日

## 背景
`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目:

- `cn-admin-api` + `cn-relay` + Admin Console: `node_subscriptions.ingest_policy`（保持期間/容量上限/バックフィル可否）を編集・保存・反映できるように実装し、`topic_subscription_design.md` / `ingested_record_persistence_policy.md` の node-level 制御要件に合わせる（契約テスト + 統合テスト）。

を完了させる。

## 実施内容
1. `cn-admin-api` に `ingest_policy` 編集/保存を実装
- `NodeSubscription` / `NodeSubscriptionUpdate` に `ingest_policy` を追加。
- `list_node_subscriptions` レスポンスと `update_node_subscription` 更新処理で `ingest_policy` を扱うよう変更。
- `ingest_policy` バリデーション（`retention_days>=0`、`max_events>=1`、`max_bytes>=1`）を追加し、違反時は `400 INVALID_INGEST_POLICY` を返す契約に統一。
- OpenAPI と契約テストを更新し、`ingest_policy` の保存・未指定時保持・レスポンス互換を固定。

2. `cn-relay` に node-level `ingest_policy` 反映を実装
- topic別 `ingest_policy` ローダー（`policy.rs`）を追加。
- WS REQ 初期バックフィルで `allow_backfill=false` topic を除外。
- ingest 時に `max_events` / `max_bytes` 容量上限を適用し、超過は reject。
- retention cleanup で topic override `retention_days` を反映。
- 統合テストを追加し、バックフィル抑止・容量超過拒否・retention override を固定。

3. Admin Console で `ingest_policy` 編集 UI を実装
- `SubscriptionsPage` に `retention_days` / `max_events` / `max_bytes` / `allow_backfill` 編集フォームと保存処理を追加。
- 行単位バリデーションとエラー表示を追加。
- API クライアントを `updateNodeSubscription(topicId, { enabled, ingest_policy })` に更新。
- OpenAPI 生成物（`admin-api.json` / `generated/admin-api.ts`）と UI テストを更新。

4. タスク管理更新
- `community_nodes_roadmap.md` の該当項目を `[x]` に更新。

## 変更ファイル
- `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs`
- `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/crates/cn-relay/src/policy.rs`
- `kukuri-community-node/crates/cn-relay/src/lib.rs`
- `kukuri-community-node/crates/cn-relay/src/ws.rs`
- `kukuri-community-node/crates/cn-relay/src/ingest.rs`
- `kukuri-community-node/crates/cn-relay/src/retention.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `kukuri-community-node/apps/admin-console/src/lib/api.ts`
- `kukuri-community-node/apps/admin-console/src/pages/SubscriptionsPage.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/SubscriptionsPage.test.tsx`
- `kukuri-community-node/apps/admin-console/openapi/admin-api.json`
- `kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-13.md`
- `docs/01_project/progressReports/2026-02-13_cn_node_subscriptions_ingest_policy_node_level_controls.md`

## 検証
- `./scripts/test-docker.ps1 build`（成功）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres-age`（成功）
- `docker compose -f docker-compose.test.yml run --rm --workdir /app/kukuri-community-node -v ./kukuri-community-node:/app/kukuri-community-node rust-test cargo fmt --all -- --check`（成功）
- `docker compose -f docker-compose.test.yml run --rm --workdir /app/kukuri-community-node -v ./kukuri-community-node:/app/kukuri-community-node -e DATABASE_URL=postgres://cn:cn_password@127.0.0.1:15432/cn rust-test cargo test --workspace --all-features`（成功）
- `docker compose -f docker-compose.test.yml run --rm --workdir /app/kukuri-community-node -v ./kukuri-community-node:/app/kukuri-community-node rust-test cargo build --release -p cn-cli`（成功）
- `docker compose -f docker-compose.test.yml run --rm --workdir /app/kukuri-community-node/apps/admin-console -v ./kukuri-community-node:/app/kukuri-community-node node-test bash -lc "corepack pnpm test"`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-ingest-policy-20260213-184904.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知 warning: `useRouter`）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-ingest-policy-20260213-185102.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（初回失敗: 既存 `kukuri-postgres-age` コンテナ名競合）
  - 失敗ログ: `tmp/logs/gh-act-community-node-tests-cn-ingest-policy-20260213-185754.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（再実行成功）
  - 成功ログ: `tmp/logs/gh-act-community-node-tests-cn-ingest-policy-rerun-20260213-190046.log`
