# Issue #125 サイドバー自動幅調整レポート

作成日: 2026年02月22日

## 概要

- 対象:
  - `kukuri-tauri/src/components/layout/MainLayout.tsx`
  - `kukuri-tauri/src/components/layout/Sidebar.tsx`
  - `kukuri-tauri/src/tests/unit/components/layout/Sidebar.test.tsx`
- 固定 `w-64` による幅不足を解消するため、サイドバー内部コンテンツの `scrollWidth` と viewport 制約から幅を動的算出する実装へ置換。
- `MainLayout` 側の二重幅制御を撤去し、`Sidebar` コンポーネント単体で開閉幅を管理。
- 狭い画面ではサイドバー最大幅を制限し、メイン領域のレイアウト破綻を防止。

## 実装詳細

- `Sidebar`
  - `SIDEBAR_MIN_WIDTH_PX` / `SIDEBAR_MAX_WIDTH_PX` と viewport 依存の上限計算を追加。
  - `ResizeObserver` + `window.resize` で再計測し、開状態のみ `style.width` に動的適用。
  - 閉状態では `width: 0px` を適用し、従来どおりトグル開閉を維持。
- `MainLayout`
  - `sidebarOpen` による外側ラッパー幅制御を削除し、`Sidebar` を直接配置。
  - メイン領域に `min-w-0` を付与して横方向の崩れを防止。
- テスト
  - `Sidebar` 閉状態テストを `w-0` クラス判定から `width: 0px` 判定へ更新。
  - 開状態の最小幅 (`256px`) テストを追加して回帰を固定。

## 実行コマンド

- `docker compose --project-name kukuri_tests -f docker-compose.test.yml run --rm ts-test pnpm vitest run src/tests/unit/components/layout/MainLayout.test.tsx src/tests/unit/components/layout/Sidebar.test.tsx`
- `bash scripts/test-docker.sh ts --no-build`
- `bash scripts/test-docker.sh lint --no-build`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
