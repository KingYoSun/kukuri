# Issue #46 ダークモード対応の改善

最終更新日: 2026年02月17日

## 概要

Issue #46（`[ui] ダークモード対応の改善`）に対して、ダークモード時の視認性低下を引き起こしていたライト固定色を監査し、主要画面でテーマ切替しても破綻しない最小修正を適用した。

根本原因は、トークン化済みテーマ設計の中に一部コンポーネントだけ `bg-white` / `text-gray-*` など固定色が残っていたこと。
特に常時表示に近いステータスUIで明度差が大きく、ダークテーマ時に読みにくさが発生していた。

## 実施内容

1. `OfflineIndicator` のオンラインピルをテーマトークン（`bg-card` / `text-card-foreground` / `border-border`）へ置換し、オフラインピルにも `dark:` を追加。
2. `RelayStatus` の接続状態バッジ（connected/connecting/disconnected/error）へダーク配色クラスを追加。
3. `P2PStatus` の未接続アイコン色を `text-gray-500` から `text-muted-foreground` に変更。
4. `SearchBar` の warning 表示へ `dark:text-amber-400` を追加。
5. `trending` 画面のスコア増減色へ `dark:text-emerald-400` / `dark:text-red-400` を追加。
6. 回帰防止として `OfflineIndicator` と `RelayStatus` のユニットテストを強化。

## 変更ファイル

- `kukuri-tauri/src/components/OfflineIndicator.tsx`
- `kukuri-tauri/src/components/RelayStatus.tsx`
- `kukuri-tauri/src/components/P2PStatus.tsx`
- `kukuri-tauri/src/components/search/SearchBar.tsx`
- `kukuri-tauri/src/routes/trending.tsx`
- `kukuri-tauri/src/tests/unit/components/OfflineIndicator.test.tsx`
- `kukuri-tauri/src/tests/unit/components/RelayStatus.test.tsx`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/progressReports/2026-02-17_issue46_dark_mode_ui_improvements.md`

## 検証

- `cd kukuri-tauri && pnpm format:check`（pass）
- `cd kukuri-tauri && pnpm lint`（pass）
- `cd kukuri-tauri && pnpm type-check`（pass）
- `cd kukuri-tauri && pnpm exec vitest run src/tests/unit/components/OfflineIndicator.test.tsx src/tests/unit/components/RelayStatus.test.tsx src/tests/unit/components/P2PStatus.test.tsx`（pass）
- `cd kukuri-tauri && pnpm exec vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/components/search/UserSearchResults.test.tsx src/tests/unit/components/search/PostSearchResults.test.tsx`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

ログ:

- `tmp/logs/gh-act-format-check-issue46.log`
- `tmp/logs/gh-act-native-test-linux-issue46.log`
- `tmp/logs/gh-act-community-node-tests-issue46.log`
