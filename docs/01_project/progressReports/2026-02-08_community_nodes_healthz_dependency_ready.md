# Community Nodes `/healthz` 依存関係 ready 判定拡張

作成日: 2026年02月08日

## 概要

Runbook 要件（`GET /healthz` は依存関係込みの ready 判定）に合わせて、以下 6 サービスの health 判定を DB 単体から拡張した。

- `cn-user-api`
- `cn-admin-api`
- `cn-index`
- `cn-moderation`
- `cn-trust`
- `cn-bootstrap`

## 実装内容

- `cn-core` に共通ヘルス補助を追加
  - `parse_health_targets`
  - `ensure_health_target_ready`
  - `ensure_health_targets_ready`
  - `ensure_endpoint_reachable`
- `MeiliClient` に `check_ready` を追加（`/health` を使用）
- 各 `/healthz` を以下判定に変更
  - `cn-user-api`: DB + Meilisearch
  - `cn-admin-api`: DB + 内部依存サービス（`user-api`/`relay`/`bootstrap`/`index`/`moderation`/`trust`）
  - `cn-index`: DB + Meilisearch + 内部依存（`relay`）
  - `cn-moderation`: DB + 内部依存（`relay`）+ LLM 依存（`openai`/`local`）
  - `cn-trust`: DB + 内部依存（`relay`/`moderation`）
  - `cn-bootstrap`: DB + 内部依存（`relay`/`user-api`）
- `cn-index` / `cn-trust` / `cn-bootstrap` に `reqwest` 依存を追加

## 変更ファイル

- `kukuri-community-node/crates/cn-core/src/health.rs`
- `kukuri-community-node/crates/cn-core/src/lib.rs`
- `kukuri-community-node/crates/cn-core/src/meili.rs`
- `kukuri-community-node/crates/cn-user-api/src/lib.rs`
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-moderation/src/lib.rs`
- `kukuri-community-node/crates/cn-trust/src/lib.rs`
- `kukuri-community-node/crates/cn-bootstrap/src/lib.rs`
- `kukuri-community-node/crates/cn-index/Cargo.toml`
- `kukuri-community-node/crates/cn-trust/Cargo.toml`
- `kukuri-community-node/crates/cn-bootstrap/Cargo.toml`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api -p cn-admin-api -p cn-index -p cn-moderation -p cn-trust -p cn-bootstrap --tests -- --nocapture"`
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - ログ: `tmp/logs/gh-act-format-check-20260208-214815.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - ログ: `tmp/logs/gh-act-native-test-linux-20260208-214815.log`

## 補足

- `gh act` 実行時に標準エラーで `some refs were not updated` が出力されるが、両ジョブとも成功。
