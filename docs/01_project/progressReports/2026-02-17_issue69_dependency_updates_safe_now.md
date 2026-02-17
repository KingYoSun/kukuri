# Issue #69 依存関係アップデート（safe-now）

最終更新日: 2026年02月17日

## 概要

Issue #69（`[chore] 依存関係アップデート（Rust/Node/pnpm/GitHub Actions）`）として、リポジトリ全体の更新候補を監査し、互換性影響が小さい `safe-now` だけを最小差分で反映した。

## 監査結果（safe-now / risky-later）

### safe-now（今回適用）

- Node（`kukuri-tauri`）:
  - `@types/lodash` `4.17.20 -> 4.17.23`
  - `lodash` `4.17.21 -> 4.17.23`
  - `remark-gfm` `4.0.0 -> 4.0.1`
- Rust（`kukuri-tauri/src-tauri`）:
  - `anyhow` `1.0.100 -> 1.0.101`
  - `bytes` `1.11.0 -> 1.11.1`
  - `futures` 系 `0.3.31 -> 0.3.32`
  - `tempfile` `3.24.0 -> 3.25.0`
- GitHub Actions（`test.yml`）:
  - `corepack prepare pnpm@9 --activate` を `pnpm@10.16.1` に統一（3箇所）

### risky-later（今回は除外）

- GitHub Actions のメジャー更新:
  - `actions/checkout@v4 -> v6`
  - `actions/setup-node@v4 -> v6`
  - `actions/cache@v4 -> v5`
  - `actions/upload-artifact@v4 -> v6`
  - `actions/github-script@v7 -> v8`
- `kukuri-tauri` の広範囲な minor/major 更新（例: `@tanstack/react-router`, `@tauri-apps/api`, `eslint@10` など）
- `admin-console` の major 差分（`react 18 -> 19`, `vite 5 -> 7`, `vitest 1 -> 4` 等）
- pnpm 自体の最新（`10.16.1 -> 10.30.0`）への引き上げ

除外理由: メジャーまたは影響範囲が大きく、回帰検証負荷が増えるため。別PRで段階適用が妥当。

## 実施内容

1. `pnpm outdated` / `cargo update --dry-run` / GitHub API（`gh api`）で更新候補を収集。
2. `safe-now` のみ更新:
   - `pnpm add lodash@4.17.23 @types/lodash@4.17.23 remark-gfm@4.0.1`
   - `cargo update -p anyhow -p bytes -p futures -p tempfile`
   - `test.yml` の pnpm バージョン統一
3. 変更面に応じた検証を実施（TS/Rust/Community Node/Docker/gh act）。

## 変更ファイル

- `.github/workflows/test.yml`
- `kukuri-tauri/package.json`
- `kukuri-tauri/pnpm-lock.yaml`
- `kukuri-tauri/src-tauri/Cargo.lock`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue69_dependency_updates_safe_now.md`

## 検証

- `cd kukuri-tauri && pnpm install --frozen-lockfile`（pass）
- `cd kukuri-tauri && pnpm lint`（pass）
- `cd kukuri-tauri && pnpm type-check`（pass）
- `cd kukuri-tauri && pnpm test:unit`（pass）
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue69.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue69.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue69.log`（pass）

ログ:

- `tmp/logs/issue69-pnpm-install.log`
- `tmp/logs/issue69-pnpm-lint.log`
- `tmp/logs/issue69-pnpm-typecheck.log`
- `tmp/logs/issue69-pnpm-test-unit.log`
- `tmp/logs/issue69-tauri-cargo-test.log`
- `tmp/logs/issue69-community-node-up.log`
- `tmp/logs/issue69-community-node-build-runner.log`
- `tmp/logs/issue69-community-node-cargo-test-build.log`
- `tmp/logs/gh-act-format-check-issue69.log`
- `tmp/logs/gh-act-native-test-linux-issue69.log`
- `tmp/logs/gh-act-community-node-tests-issue69.log`
