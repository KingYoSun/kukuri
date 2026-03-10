# Issue #164 Community Node Admin UI Bootstrap 専用ページ化レポート

作成日: 2026年02月25日

## 概要

- 対象:
  - `kukuri-community-node/apps/admin-console/src/App.tsx`
  - `kukuri-community-node/apps/admin-console/src/router.tsx`
  - `kukuri-community-node/apps/admin-console/src/pages/BootstrapPage.tsx`
  - `kukuri-community-node/apps/admin-console/src/App.test.tsx`
  - `kukuri-community-node/apps/admin-console/src/pages/BootstrapPage.test.tsx`
- Bootstrap 情報表示をサイドバー内カードから専用ページへ移行した。
- サイドバーから `Bootstrap` メニューで遷移できる導線を追加した。

## 実装詳細

- `App.tsx`
  - 既存の Bootstrap カード表示ロジック（`nodeSubscriptions` / `subscriptions` クエリと集計）を削除。
  - サイドバーナビゲーションへ `Bootstrap` 項目を追加。
- `router.tsx`
  - `BootstrapPage` を import し、`/bootstrap` ルートを追加。
- `pages/BootstrapPage.tsx`
  - `api.nodeSubscriptions()` と `api.subscriptions()` を取得。
  - 接続ノードを `normalizeConnectedNode` で `node_id@host:port` 表記へ正規化し、重複排除して表示。
  - 接続ユーザーは `subscriber_pubkey` の最新行を採用し、`active` のみ集計して一覧と件数を表示。
  - 0件時の空表示、読み込み中表示、取得失敗表示を実装。
- テスト
  - `App.test.tsx` を新導線（Bootstrap メニュー表示）へ更新。
  - `BootstrapPage.test.tsx` を追加し、以下を検証:
    - `node_id@host:port` 表示
    - 接続ユーザー一覧と件数整合
    - 0件時の空表示

## 実行コマンド

- `docker run --rm -v /home/kingyosun/kukuri:/workspace -w /workspace/kukuri-community-node/apps/admin-console node:22-bullseye bash -lc "set -euo pipefail; corepack enable; pnpm --version; pnpm install --frozen-lockfile; pnpm test src/App.test.tsx src/pages/BootstrapPage.test.tsx; pnpm typecheck"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue164.log`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue164.log`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue164.log`

すべて pass。
