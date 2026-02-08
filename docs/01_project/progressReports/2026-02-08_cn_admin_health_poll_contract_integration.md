# Community Nodes `cn-admin-api` health 集約ポーリング契約/統合テスト追加

作成日: 2026年02月08日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-admin-api` の health 集約ポーリング（`services::poll_health_once`）を契約/統合テストで検証し、`cn_admin.service_health` の `healthy|degraded|unreachable` と `details_json` 更新の後方互換を担保

を実装し、完了状態へ更新した。

## 実装内容

- `cn-admin-api` の `poll_health_once` を `pub(crate)` 化し、契約テストから直接呼び出せるように調整
  - `kukuri-community-node/crates/cn-admin-api/src/services.rs`
- `contract_tests` を拡張し、以下を追加
  - `services_health_poll_contract_status_matrix_backward_compatible`
    - healthy/degraded/unreachable の 3 状態を `poll_health_once` 実行で生成
    - `/v1/admin/services` 応答で `health.status` と `health.details` 互換（`details.status` / `details.error`）を検証
  - `services_health_poll_updates_details_json_on_status_change`
    - 同一 service の health が `healthy -> degraded -> unreachable` へ遷移した際に `details_json` が上書き更新されることを検証
    - unreachable 更新後に `details.status` が残留しないことを確認
- 追加ヘルパー
  - health mock server 起動（可変 status code）
  - `service_configs` upsert
  - `service_health` 行取得
  - health target 付き `AppState` 生成

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/services.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api --tests -- --nocapture"`（成功、18 tests passed）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-20260209-004003.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260209-004204.log`

## 補足

- `cargo test -p cn-admin-api` 初回実行時に `sqlx::Row` import 漏れで失敗したため修正し、再実行で成功を確認。
- `gh act` 実行時の `some refs were not updated` は既知の非致命ログで、ジョブ自体は成功。
