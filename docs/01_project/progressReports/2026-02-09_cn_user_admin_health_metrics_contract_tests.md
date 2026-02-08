# Community Nodes `cn-user-api` / `cn-admin-api` `/healthz` `/metrics` 契約テスト追加

作成日: 2026年02月09日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-user-api` / `cn-admin-api` の `/healthz` `/metrics` 契約テストを追加し、status code とレスポンス shape（`status`、Prometheus content-type）を固定する

を実装し、完了状態へ更新した。

## 実装内容

- `cn-admin-api` 契約テストを拡張
  - `healthz_contract_success_shape_compatible`
    - `200 OK` と `{"status":"ok"}` を検証
  - `healthz_contract_dependency_unavailable_shape_compatible`
    - 依存先不達時の `503 Service Unavailable` と `{"status":"unavailable"}` を検証
  - `metrics_contract_prometheus_content_type_shape_compatible`
    - `200 OK`、`Content-Type: text/plain; version=0.0.4`、`cn_up{service="cn-admin-api"} 1` を検証
  - テキストレスポンス検証用ヘルパー `get_text` を追加

- `cn-user-api` 契約テストを拡張
  - `healthz_contract_success_shape_compatible`
    - Meilisearch `/health` mock が `200` を返す場合に `200 OK` と `{"status":"ok"}` を検証
  - `healthz_contract_unavailable_shape_compatible`
    - Meilisearch `/health` mock が `503` を返す場合に `503 Service Unavailable` と `{"status":"unavailable"}` を検証
  - `metrics_contract_prometheus_content_type_shape_compatible`
    - `200 OK`、`Content-Type: text/plain; version=0.0.4`、`cn_up{service="cn-user-api"} 1` を検証
  - テキストレスポンス検証ヘルパー `get_text_public` と Meilisearch health mock `spawn_mock_meili_health` を追加

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-09.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml down -v`（成功、検証前に DB/Meilisearch ボリュームを初期化）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api -p cn-admin-api --tests -- --nocapture"`（成功、`cn-admin-api` 21 tests / `cn-user-api` 27 tests）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api --lib healthz_contract_ -- --nocapture && cargo test -p cn-admin-api --lib metrics_contract_prometheus_content_type_shape_compatible -- --nocapture"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api --lib healthz_contract_ -- --nocapture && cargo test -p cn-user-api --lib metrics_contract_prometheus_content_type_shape_compatible -- --nocapture"`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-20260209-014704.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260209-014817.log`

## 補足

- 初回検証時に DB 状態依存で `cn-user-api` 契約テストが不安定化したため、検証環境を初期化（`docker compose ... down -v`）して再実行し、`cn-user-api` / `cn-admin-api` ともに全件成功を確認した。
