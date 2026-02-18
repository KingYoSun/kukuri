# Issue #79 Phase 1: Manifest inventory + `kukuri-tauri` Node dependencies latest

作成日: 2026年02月17日
Issue: https://github.com/KingYoSun/kukuri/issues/79

## 実施概要

Issue #79 の今回スコープ（Manifest inventory + Phase 1）として、リポジトリ内の全 `package.json` / 全 `Cargo.toml` を棚卸しし、`kukuri-tauri` の Node 依存を `latest` へ全面更新した。

## Manifest inventory（repo scope）

### package.json（2件）

- `kukuri-community-node/apps/admin-console/package.json`
- `kukuri-tauri/package.json`

### Cargo.toml（12件）

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
- `kukuri-tauri/src-tauri/Cargo.toml`

## Phase 1 結果（`kukuri-tauri` Node latest 化）

- `cd kukuri-tauri && pnpm up --latest` を実行し、`dependencies` / `devDependencies` を全面更新。
- 追加調整として、deprecated な `@types/uuid` を削除（`uuid` 本体の型定義を利用）。
- `pnpm outdated --format json` の結果は `{}`（未更新残なし）。

### 依存更新に伴う差分対応

メジャー更新（特に `eslint@10`）により lint 指摘が増加したため、最小差分で以下を調整。

- 未使用代入（`no-useless-assignment`）の除去
- catch 再throw時の `cause` 付与（`preserve-caught-error`）
- `eslint-env` コメント廃止対応（flat config）
- Route ファイルの `react-refresh/only-export-components` 警告抑制
- Prettier 3.8 系で差分が発生した `src/lib/utils/tauriEnvironment.ts` の整形

重大なブロッカーは発生せず、Manager エスカレーションは不要。

## 検証

### 直接検証

- `cd kukuri-tauri && pnpm type-check`（pass）
- `cd kukuri-tauri && pnpm lint`（pass）
- `cd kukuri-tauri && pnpm test -- --runInBand`（pass）
- `cd kukuri-tauri && pnpm outdated --format json`（`{}`）

### セッション必須 gh act

- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue79-phase1.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue79-phase1.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue79-phase1.log`（pass）

## 次フェーズへの carry-over

- Phase 2: `kukuri-community-node` 側 Node 依存 latest 化
- 以降、Issue #79 の staged plan（Rust 側 latest 化、残課題整理）を継続
