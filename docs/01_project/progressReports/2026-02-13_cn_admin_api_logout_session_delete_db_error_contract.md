# 2026年02月13日 `cn-admin-api` `auth::logout` セッション削除失敗の 5xx 契約固定

## 概要

- `cn-admin-api` の `auth::logout` で `DELETE cn_admin.admin_sessions ... .await.ok()` により DB 失敗が握り潰され、`200` が返ってしまう経路を修正した。
- セッション削除失敗時のレスポンス契約を `500 (DB_ERROR)` として OpenAPI/契約テストで明文化し、後方互換を固定した。

## 実装内容

1. `kukuri-community-node/crates/cn-admin-api/src/auth.rs`
- `logout` のセッション削除で `.await.ok()` を廃止し、`map_err` で `ApiError(INTERNAL_SERVER_ERROR, "DB_ERROR", ...)` を返すように修正。
- 削除成功時のみ cookie を除去するようにし、削除失敗時に成功レスポンスへ進まないようにした。

2. `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
- `POST /v1/admin/auth/logout` の `responses` に `500 + ErrorResponse` を追加。

3. `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- logout 削除失敗を再現するため、`cn_admin.test_logout_failures` と `BEFORE DELETE` トリガを用意するテストヘルパーを追加。
- `auth_contract_logout_returns_500_when_session_delete_fails` を追加し、以下を検証:
  - `status=500`
  - `code=DB_ERROR`
  - `cn_admin.admin_sessions` の対象行が残存すること
- `openapi_contract_contains_admin_paths` に `.../auth/logout/post/responses/500` の存在検証を追加。

4. OpenAPI 生成物更新
- `kukuri-community-node/apps/admin-console/openapi/admin-api.json`
- `kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`

5. タスク更新
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の対象項目を `[x]` に更新。

## 検証

- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api auth_contract_ -- --nocapture --test-threads=1"`（成功）
  - ログ: `tmp/logs/cn-admin-api-auth-logout-contract-20260213-140818.log`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api openapi_contract_contains_admin_paths -- --nocapture --test-threads=1"`（成功）
  - ログ: `tmp/logs/cn-admin-api-openapi-logout-contract-20260213-140836.log`
- `./scripts/test-docker.ps1 rust -NoBuild`（成功）
  - ログ: `tmp/logs/test-docker-rust-cn-admin-logout-20260213-140743.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-admin-logout-20260213-135317.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知の `useRouter` 警告のみ）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-admin-logout-20260213-135441.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-admin-logout-20260213-140350.log`
