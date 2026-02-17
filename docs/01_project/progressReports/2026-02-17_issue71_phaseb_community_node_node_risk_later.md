# Issue #71 Phase B: `kukuri-community-node` Node risk-later 依存更新

最終更新日: 2026年02月17日

## 概要

Issue #71 の Phase B として、`kukuri-community-node` 配下の Node マニフェストを網羅確認したうえで、risk-later 候補から React 系メジャー更新のみを最小差分で適用した。

## 対象マニフェスト（網羅）

- `kukuri-community-node/apps/admin-console/package.json`
- `kukuri-community-node/apps/admin-console/pnpm-lock.yaml`

## 更新内容

- `react`: `18.3.1 -> 19.2.4`
- `react-dom`: `18.3.1 -> 19.2.4`
- `@types/react`: `18.3.28 -> 19.2.14`
- `@types/react-dom`: `18.3.7 -> 19.2.3`

変更方針: risk-later 候補のうち、ビルドツール（`vite` / `vitest`）まで同時更新せず、React ランタイム系に限定して回帰範囲を抑制。

## 検証

- `cd kukuri-community-node/apps/admin-console && pnpm outdated --format json`（更新候補の取得）
- `cd kukuri-community-node/apps/admin-console && pnpm up react@19.2.4 react-dom@19.2.4 @types/react@19.2.14 @types/react-dom@19.2.3`（pass）
- `cd kukuri-community-node/apps/admin-console && pnpm install --frozen-lockfile`（pass）
- `cd kukuri-community-node/apps/admin-console && pnpm test`（pass）
- `cd kukuri-community-node/apps/admin-console && pnpm build`（pass）
- `cd kukuri-community-node/apps/admin-console && pnpm typecheck`（fail: 既存の型不整合）
- `git worktree add /tmp/kukuri-main-issue71 origin/main && cd /tmp/kukuri-main-issue71/kukuri-community-node/apps/admin-console && pnpm install --frozen-lockfile && pnpm typecheck`（fail: 同一エラーでベースライン再現）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

ログ:

- `tmp/logs/gh-act-format-check-issue71-phaseb.log`
- `tmp/logs/gh-act-native-test-linux-issue71-phaseb.log`
- `tmp/logs/gh-act-community-node-tests-issue71-phaseb.log`

## Carry-over

- Phase C（Issue #71 計画）: `kukuri-tauri` Rust Cargo risk-later 依存更新へ移行。
- `admin-console` の未適用候補（`vite@7` / `vitest@4` / `@vitejs/plugin-react@5` / `zod@4` / `zustand@5` など）は影響範囲が大きく、追加フェーズで段階適用。

## 変更ファイル

- `kukuri-community-node/apps/admin-console/package.json`
- `kukuri-community-node/apps/admin-console/pnpm-lock.yaml`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue71_phaseb_community_node_node_risk_later.md`
