# Issue #40 PR-1（M-01）Community Node workspace license を MIT へ整合

作成日: 2026年02月16日
最終更新日: 2026年02月16日

## 目的

- Issue #40 監査で抽出した remediation M-01 を最小差分で実施し、`kukuri-community-node` の Rust workspace ライセンス宣言をリポジトリ方針（MIT）へ一致させる。

## 実施内容

- 対象ファイル: `kukuri-community-node/Cargo.toml`
- 変更内容: `[workspace.package].license` を `Apache-2.0` から `MIT` へ変更。
- スコープ制御: M-02（`admin-console/package.json`）と M-03（`kukuri-tauri/package.json`）は未変更のまま次PRへ分離。

## 検証

- `cd /home/kingyosun/kukuri/kukuri-community-node && cargo fmt --all --check`（pass）
- `cd /home/kingyosun/kukuri && git diff --check`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue40-pr1.log`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue40-pr1.log`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue40-pr1.log`（pass）

## 残タスク（Issue #40）

1. M-02: `kukuri-community-node/apps/admin-console/package.json` に `"license": "MIT"` を追加。
2. M-03: `kukuri-tauri/package.json` に `"license": "MIT"` を追加（任意の横断整備）。
