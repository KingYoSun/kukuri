# Community Nodes `cn-user-api` bootstrap 条件付き GET / キャッシュ契約テスト追加

作成日: 2026年02月09日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-user-api` bootstrap 配布の条件付き GET（`If-None-Match` / `If-Modified-Since`）と `ETag` / `Last-Modified` / `Cache-Control` / `next_refresh_at` を検証するテストを追加する

を実装し、完了状態へ更新した。

## 実装内容

- `cn-user-api` 契約テストを拡張（`crates/cn-user-api/src/subscriptions.rs`）
  - `bootstrap_services_conditional_get_and_cache_headers_contract_compatible` を追加
  - `/v1/bootstrap/topics/{topic_id}/services` の 200 応答で以下を検証
    - `Cache-Control: public, max-age=300`
    - `ETag` ヘッダー存在
    - `Last-Modified` ヘッダー存在
    - `next_refresh_at` が最短有効期限（最小 `expires_at`）になること
  - 同一リソースへの条件付き GET を検証
    - `If-None-Match` 指定時に `304 Not Modified`
    - `If-Modified-Since` 指定時に `304 Not Modified`

- `cn-user-api` 実装を最小修正（`crates/cn-user-api/src/bootstrap.rs`）
  - `If-Modified-Since` 判定を秒精度比較に変更し、HTTP-date（秒精度）と DB タイムスタンプ（秒未満含む）の差で 304 判定が不安定化しないようにした

- タスク記録を更新
  - `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当チェックを完了化
  - `docs/01_project/activeContext/tasks/completed/2026-02-09.md` に完了内容と検証ログを追記

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-09.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api --lib bootstrap_services_conditional_get_and_cache_headers_contract_compatible -- --nocapture"`（成功、1 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-20260209-061212.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260209-061326.log`
