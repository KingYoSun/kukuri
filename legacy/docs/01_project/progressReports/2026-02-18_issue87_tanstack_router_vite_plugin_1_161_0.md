# Issue #87: `@tanstack/router-vite-plugin` 1.161.0 追従

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/87

## 実施概要

`kukuri-tauri` の devDependency `@tanstack/router-vite-plugin` を `1.160.2` から `1.161.0` へ更新した。`pnpm-lock.yaml` も同時に更新し、関連する `@tanstack/router-plugin` の解決結果も `1.161.0` に揃えた。

## 変更ファイル

- `kukuri-tauri/package.json`
- `kukuri-tauri/pnpm-lock.yaml`
- `docs/01_project/activeContext/tasks/completed/2026-02-18.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`

## 実行コマンドと結果

- `cd kukuri-tauri && pnpm up @tanstack/router-vite-plugin@1.161.0`
  - pass（`@tanstack/router-vite-plugin 1.160.2 -> 1.161.0`）
- `cd kukuri-tauri && pnpm lint`
  - pass
- `cd kukuri-tauri && pnpm type-check`
  - pass
- `cd kukuri-tauri && pnpm test -- --runInBand`
  - pass（97 files / 843 passed / 6 skipped）
- `cd kukuri-tauri && pnpm outdated --format json`
  - pass（`{}`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue87.log`
  - pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue87.log`
  - pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue87.log`
  - pass

## 備考

- `pnpm test` / `gh act native-test-linux` では既知の `act(...)` 警告が出力されるが、いずれもテスト自体は成功した。
