# Issue #71 Phase A: `kukuri-tauri` Node/pnpm risk-later 依存更新

最終更新日: 2026年02月17日

## 概要

Issue #71（`[chore] risk-later deps更新`）の Phase A として、`kukuri-tauri` スコープの Node/pnpm 依存を最小差分で更新した。safe-now で見送っていた risk-later 候補のうち、影響範囲を限定できる `@tanstack/react-router` 系 2 パッケージのみを適用した。

## 対象マニフェスト（網羅）

- `kukuri-tauri/package.json`
- `kukuri-tauri/pnpm-lock.yaml`

## 更新内容

- `@tanstack/react-router`: `1.141.6 -> 1.160.2`
- `@tanstack/router-vite-plugin`: `1.141.7 -> 1.160.2`

変更方針: risk-later 候補を全量更新せず、ルーティング基盤の同一ファミリー更新に限定して回帰リスクと差分を最小化。

## Carry-over（次フェーズ以降）

- Phase B（Issue #71 計画）: `kukuri-community-node` の Node risk-later 更新を実施。
- Phase E までの残課題候補:
  - `kukuri-tauri` の未適用 risk-later（例: `@tauri-apps/api` / `@tauri-apps/cli` / `eslint@10` / `i18next@25` / `pnpm@10.30.0`）
  - 上記は互換性影響が大きいため、別フェーズで段階適用。

## 変更ファイル

- `kukuri-tauri/package.json`
- `kukuri-tauri/pnpm-lock.yaml`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue71_phasea_tauri_node_pnpm_risk_later.md`

## 検証

- `cd kukuri-tauri && pnpm up @tanstack/react-router@1.160.2 @tanstack/router-vite-plugin@1.160.2`（pass）
- `cd kukuri-tauri && pnpm install --frozen-lockfile`（pass）
- `cd kukuri-tauri && pnpm lint`（pass）
- `cd kukuri-tauri && pnpm type-check`（pass）
- `cd kukuri-tauri && pnpm test:unit`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue71-phasea.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue71-phasea.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue71-phasea.log`（pass）

ログ:

- `tmp/logs/issue71-phasea-pnpm-up-router.log`
- `tmp/logs/issue71-phasea-pnpm-install-frozen.log`
- `tmp/logs/issue71-phasea-pnpm-lint.log`
- `tmp/logs/issue71-phasea-pnpm-typecheck.log`
- `tmp/logs/issue71-phasea-pnpm-test-unit.log`
- `tmp/logs/gh-act-format-check-issue71-phasea.log`
- `tmp/logs/gh-act-native-test-linux-issue71-phasea.log`
- `tmp/logs/gh-act-community-node-tests-issue71-phasea.log`
