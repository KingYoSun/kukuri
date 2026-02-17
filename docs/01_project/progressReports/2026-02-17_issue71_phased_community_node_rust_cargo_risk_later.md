# Issue #71 Phase D: `kukuri-community-node` Rust Cargo risk-later 依存更新

最終更新日: 2026年02月17日

## 概要

Issue #71 の Phase D として、`kukuri-community-node` スコープの Rust Cargo 依存を対象に、risk-later 候補から `jsonwebtoken` ファミリー更新のみを最小差分で適用した。

## 対象マニフェスト（網羅）

- `kukuri-community-node/Cargo.lock`
- `kukuri-community-node/Cargo.toml`
- `kukuri-community-node/crates/cn-admin-api/Cargo.toml`
- `kukuri-community-node/crates/cn-bootstrap/Cargo.toml`
- `kukuri-community-node/crates/cn-cli/Cargo.toml`
- `kukuri-community-node/crates/cn-core/Cargo.toml`
- `kukuri-community-node/crates/cn-index/Cargo.toml`
- `kukuri-community-node/crates/cn-kip-types/Cargo.toml`
- `kukuri-community-node/crates/cn-moderation/Cargo.toml`
- `kukuri-community-node/crates/cn-relay/Cargo.toml`
- `kukuri-community-node/crates/cn-trust/Cargo.toml`
- `kukuri-community-node/crates/cn-user-api/Cargo.toml`

## 更新内容

- `kukuri-community-node/Cargo.toml`:
  - `jsonwebtoken`: `10.2.0 -> 10.3.0`
- `kukuri-community-node/Cargo.lock`:
  - `jsonwebtoken`: `10.2.0 -> 10.3.0`

変更方針: `cargo update --dry-run` で確認される広範囲更新（75 package）を避け、認証ライブラリの単一ファミリー更新に限定して差分と回帰範囲を最小化。

## 検証

- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（初回 fail: `cn-admin-api` の `trust_contract_success_and_shape`）
- `git worktree add /tmp/kukuri-main-issue71-phased origin/main`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v /tmp/kukuri-main-issue71-phased:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features"`（pass: ベースライン）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（再実行 pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue71-phased.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue71-phased.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue71-phased.log`（pass）

## 変更ファイル

- `kukuri-community-node/Cargo.toml`
- `kukuri-community-node/Cargo.lock`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue71_phased_community_node_rust_cargo_risk_later.md`

## ログ

- `tmp/logs/issue71-phased-cargo-update-jsonwebtoken.log`
- `tmp/logs/issue71-phased-community-node-compose-up.log`
- `tmp/logs/issue71-phased-community-node-build-test-runner.log`
- `tmp/logs/issue71-phased-community-node-cargo-test-build.log`
- `tmp/logs/issue71-phased-baseline-main-community-node-cargo-test.log`
- `tmp/logs/issue71-phased-community-node-cargo-test-build-retry.log`
- `tmp/logs/gh-act-format-check-issue71-phased.log`
- `tmp/logs/gh-act-native-test-linux-issue71-phased.log`
- `tmp/logs/gh-act-community-node-tests-issue71-phased.log`

## Carry-over

- Phase E（Issue #71 計画）で、未適用の広範囲更新候補（`clap` / `futures` / `uuid` など）を整理し、follow-up Issue の要否を確定する。
