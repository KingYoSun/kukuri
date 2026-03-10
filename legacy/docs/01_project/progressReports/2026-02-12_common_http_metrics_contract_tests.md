# Community Nodes Runbook 共通 HTTP メトリクス契約固定

作成日: 2026年02月12日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- Runbook の共通必須メトリクス（`http_requests_total` / `http_request_duration_seconds_bucket`）を各サービスの `/metrics` 契約テストで固定し、`service,route,method,status` ラベル互換を保証する

を実装し、完了状態へ更新した。

## 実装内容

- 対象サービス:
  - `cn-bootstrap`
  - `cn-user-api`
  - `cn-admin-api`
  - `cn-index`
  - `cn-moderation`
  - `cn-trust`
  - `cn-relay`
- 各サービスの `/metrics` 契約テストで、共通 HTTP メトリクスを検証
  - `http_requests_total`
  - `http_request_duration_seconds_bucket`
- 各メトリクスで `service,route,method,status` ラベルを固定検証
  - テスト内で `record_http_request(..., "GET", "/metrics-contract", 200, ...)` を記録し、出力メトリクス本文からラベル一致をアサート
- タスク更新
  - `community_nodes_roadmap.md` の該当項目を `[x]` に更新
  - `tasks/completed/2026-02-12.md` に完了記録を追記

## 変更ファイル

- `kukuri-community-node/crates/cn-bootstrap/src/lib.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-moderation/src/lib.rs`
- `kukuri-community-node/crates/cn-trust/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-12.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - 既知の `useRouter must be used inside a <RouterProvider>` 警告は出力されるが、ジョブは成功
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
