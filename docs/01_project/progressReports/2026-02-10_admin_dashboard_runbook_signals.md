# Community Nodes Admin Console Dashboard Runbook 指標対応

作成日: 2026年02月10日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- Admin Console `Dashboard` を Runbook 要件に追従させる（`outbox backlog` / `reject` 急増 / DB 逼迫の主要指標表示を追加）+ UI テスト追加

を実装し、完了状態へ更新した。

## 実装内容

- `cn-admin-api` に `GET /v1/admin/dashboard` を追加
  - `outbox backlog`: `cn_relay.events_outbox` / `cn_relay.consumer_offsets` から backlog を集計
  - `reject surge`: relay `/metrics` の `ingest_rejected_total` を集計し、前回サンプルとの差分と `/min` を算出
  - `db pressure`: DB サイズ・接続率・lock waiters を集計し、しきい値で alert 判定
- `cn-admin-api` の OpenAPI に `/v1/admin/dashboard` と関連 schema を追加
- `cn-admin-api` 契約テストを追加
  - `dashboard_contract_runbook_signals_shape_compatible`
  - OpenAPI 契約テストに dashboard path / schema の存在検証を追加
- Admin Console の Dashboard UI を拡張
  - Runbook 指標カード（Outbox backlog / Reject surge / DB pressure）を表示
  - Alert 条件時に `Notice(tone="error")` で runbook alert を表示
  - `Refresh` で `services` と `dashboard` を同時再取得
- Admin Console UI テストを追加・更新
  - `DashboardPage.test.tsx` で runbook 指標表示・alert 表示・Refresh 再取得を検証

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/dashboard.rs`
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/crates/cn-admin-api/src/auth.rs`
- `kukuri-community-node/apps/admin-console/src/lib/api.ts`
- `kukuri-community-node/apps/admin-console/src/lib/types.ts`
- `kukuri-community-node/apps/admin-console/src/pages/DashboardPage.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/DashboardPage.test.tsx`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-10.md`

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api --tests -- --nocapture"`（成功: 24 passed）
- `docker run --rm -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && rustup component add rustfmt && cargo fmt --all --check"`（成功）
- `docker run --rm -e CI=true -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node/apps/admin-console node:20-bookworm bash -lc "set -euo pipefail; corepack enable; pnpm install --frozen-lockfile; pnpm vitest run src/pages/DashboardPage.test.tsx"`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-format-check-admin-dashboard-20260210-100840.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-admin-dashboard-20260210-100956.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（初回失敗、再実行で成功）
  - 初回失敗ログ: `tmp/logs/gh-act-community-node-tests-admin-dashboard-20260210-101805.log`
  - 再実行成功ログ: `tmp/logs/gh-act-community-node-tests-admin-dashboard-retry-20260210-102155.log`

## 補足

- `community-node-tests` 初回失敗は既知の flaky（`cn-relay` の `integration_tests::ingest_outbox_ws_gossip_integration` gossip timeout）で、再実行で解消。
