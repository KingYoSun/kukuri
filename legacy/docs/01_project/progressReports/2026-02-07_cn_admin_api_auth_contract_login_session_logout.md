# Community Nodes / cn-admin-api 契約テスト: login-session-me-logout 互換

最終更新日: 2026年02月07日

## 概要

`cn-admin-api` の認証契約テストに、`login -> session cookie -> /v1/admin/auth/me -> logout` の成功系フローを追加した。既存クライアントが依存する cookie ベース認証契約の後方互換をテストで担保する。

## 実施内容

- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - `auth_contract_login_me_logout_success` を追加。
  - テスト内で admin user を作成し、`POST /v1/admin/auth/login` の 200 とレスポンス shape（`admin_user_id` / `username` / `expires_at`）を検証。
  - `Set-Cookie` から `cn_admin_session` を抽出し、`GET /v1/admin/auth/me` の 200 とユーザー情報一致を検証。
  - `POST /v1/admin/auth/logout` の 200 と `status=ok` を検証。
  - `logout` 後、同一 cookie で `GET /v1/admin/auth/me` が 401 になることを検証。
  - 併せて `insert_admin_user` ヘルパーを追加し、既存 `insert_admin_session` から再利用するように整理。

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 未完了項目
    - `` `cn-admin-api` 契約テスト: `login -> session cookie -> /v1/admin/auth/me -> logout` ... ``
    を `[x]` に更新。

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -c "source /usr/local/cargo/env && cargo test -p cn-admin-api --tests -- --nocapture"`（成功: 9 passed）
- `./scripts/test-docker.ps1 rust -NoBuild`（成功）
- `docker run --rm -v C:\Users\kgm11\kukuri\kukuri-community-node:/app -w /app rust:1.88-bookworm bash -c "source /usr/local/cargo/env && cargo test -- --nocapture"`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功。ログ: `tmp/logs/gh-act-format-check-20260207-120025.log`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。ログ: `tmp/logs/gh-act-native-test-linux-20260207-115219.log`）
