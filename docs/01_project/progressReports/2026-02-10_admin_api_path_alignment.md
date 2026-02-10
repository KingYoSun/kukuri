# Community Nodes Admin API パス設計の不整合解消

作成日: 2026年02月10日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- Admin API パス設計の不整合を解消する（`services_trust.md` の `POST /v1/attestations`、`services_moderation.md` の `POST /v1/labels`、`policy_consent_management.md` の `/v1/policies*` を実装系の `/v1/admin/*` と統一。必要なら互換エイリアス実装 + 契約テスト）

を実装し、完了状態へ更新した。

## 実装内容

- `cn-admin-api` ルーターに後方互換エイリアスを追加
  - `POST /v1/attestations` -> `trust::create_job`（正規: `/v1/admin/trust/jobs`）
  - `POST /v1/labels` -> `moderation::create_label`（正規: `/v1/admin/moderation/labels`）
  - `/v1/policies*` -> `policies::*`（正規: `/v1/admin/policies*`）
- 契約テスト `legacy_admin_path_aliases_contract_success` を `cn-admin-api` に追加
  - 旧パスで `policies` の create/update/publish/make-current が動作
  - 旧 `POST /v1/labels` で label 発行でき、正規 list API から参照できる
  - 旧 `POST /v1/attestations` で trust job を作成でき、正規 list API から参照できる
- 設計ドキュメントを `/v1/admin/*` 基準へ更新
  - `services_trust.md`
  - `services_moderation.md`
  - `policy_consent_management.md`
  - いずれも非推奨の互換エイリアスを明記
- `community_nodes_roadmap.md` の該当チェックを完了化

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `docs/03_implementation/community_nodes/services_trust.md`
- `docs/03_implementation/community_nodes/services_moderation.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-10.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api --tests -- --nocapture"`（成功: 22 passed）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api --lib legacy_admin_path_aliases_contract_success -- --nocapture"`（成功: 1 passed）
- `docker run --rm -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && rustup component add rustfmt && cargo fmt --all --check"`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-format-check-admin-path-alias-20260210-090859.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-admin-path-alias-20260210-091015.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（再実行で成功）
  - 初回失敗ログ（flaky）: `tmp/logs/gh-act-community-node-tests-admin-path-alias-20260210-091824.log`
  - 再実行成功ログ: `tmp/logs/gh-act-community-node-tests-admin-path-alias-retry-20260210-092217.log`

## 補足

- `community-node-tests` 初回失敗は `cn-relay` の既知 flaky（`integration_tests::ingest_outbox_ws_gossip_integration` gossip timeout）で、再実行で解消した。
