# Issue #79 Phase 2: `kukuri-community-node` Node dependencies latest

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/79

## 実施概要

Issue #79 の Phase 2 として、`kukuri-community-node` 側の Node 依存（`apps/admin-console`）を `latest` へ全面更新した。

## Manifest inventory（Phase 2 scope）

- `kukuri-community-node/apps/admin-console/package.json`
- `kukuri-community-node/apps/admin-console/pnpm-lock.yaml`

## 依存更新結果

`cd kukuri-community-node/apps/admin-console && pnpm up --latest` を実行し、以下を含む依存を最新化した。

- `@tanstack/react-query`: `5.90.20` → `5.90.21`
- `@tanstack/react-router`: `1.158.1` → `1.160.2`
- `openapi-fetch`: `0.15.0` → `0.17.0`
- `zod`: `3.25.76` → `4.3.6`
- `zustand`: `4.5.7` → `5.0.11`
- `@vitejs/plugin-react`: `4.7.0` → `5.1.4`
- `jsdom`: `24.1.3` → `28.1.0`
- `openapi-typescript`: `7.10.1` → `7.13.0`
- `vite`: `5.4.21` → `7.3.1`
- `vitest`: `1.6.1` → `4.0.18`

## 依存更新に伴う調整

メジャー更新で型厳格化が発生したため、最小差分で次を調整した。

- `StatusResponse`/`PolicyResponse`/`TrustJobRow` に合わせてテストモックを更新
- `ModerationLabel` 型と `api.moderationLabels` 返却型の整合を修正
- `ModerationPage` の JSON パース経路に型注釈を追加し `noImplicitAny` を解消

重大かつ修正困難な breaking blocker は発生せず、Manager エスカレーションは不要。

## 検証

### 直接検証

- `cd kukuri-community-node/apps/admin-console && pnpm run typecheck`（pass）
- `cd kukuri-community-node/apps/admin-console && pnpm run test`（pass, 12 files / 20 tests）
- `cd kukuri-community-node/apps/admin-console && pnpm run build`（pass）
- `cd kukuri-community-node/apps/admin-console && pnpm outdated --long`（出力なし = 未更新残なし）

### セッション必須 gh act

- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue79-phase2.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue79-phase2.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue79-phase2.log`（pass）

## carry-over

- Phase 3 以降として、Issue #79 の Rust 依存 latest 化（`kukuri-tauri` → `kukuri-community-node`）を継続する。
