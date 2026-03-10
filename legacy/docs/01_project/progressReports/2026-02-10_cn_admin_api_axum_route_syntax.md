# Community Nodes `cn-admin-api` Axum 0.8 ルーティング形式統一

作成日: 2026年02月10日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-admin-api`: Axum 0.8 のルーティング規則に合わせ、`/v1/admin/*/:param` と `/v1/policies/:policy_id*` を `{param}` 形式へ統一する
- `cn-admin-api`: ルータ初期化のスモークテストを追加する

を実装し、完了状態へ更新した。

## 実装内容

- `cn-admin-api` の全 path parameter を `:param` から `{param}` に置換
  - `/v1/admin/services/{service}/config`
  - `/v1/admin/policies/{policy_id}`
  - `/v1/admin/moderation/rules/{rule_id}`
  - `/v1/admin/subscription-requests/{request_id}/{action}`
  - `/v1/admin/node-subscriptions/{topic_id}`
  - `/v1/admin/plans/{plan_id}`
  - `/v1/admin/subscriptions/{subscriber_pubkey}`
  - `/v1/admin/personal-data-jobs/{job_type}/{job_id}/{action}`
  - `/v1/admin/trust/schedules/{job_type}`
  - 互換 alias の `/v1/policies/{policy_id}*` も同様に置換
- ルータ構築処理を `build_router(state)` に切り出し
- ルータ初期化スモークテスト `router_initializes_with_axum_08_paths` を追加
  - Axum 0.8 非互換 path が混入するとルータ構築時に panic して CI で検知できる
- `community_nodes_roadmap.md` の該当2項目を `[x]` に更新

## 変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-10.md`

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-admin-api --tests -- --nocapture"`（成功: 26 passed）
- `docker run --rm -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && rustup component add rustfmt >/dev/null 2>&1 || true && cargo fmt --all --check"`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-admin-axum-20260210-134616.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-admin-axum-20260210-134737.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-admin-axum-20260210-135547.log`
