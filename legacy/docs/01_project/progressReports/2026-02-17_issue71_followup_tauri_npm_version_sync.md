# Issue #71 follow-up: Desktop E2E `tauri` version mismatch 修正

最終更新日: 2026年02月17日

## 概要

Issue #71 再オープン後の follow-up として、`main` の失敗 run `22118410647` を再調査し、Desktop E2E のブロッカーだった `tauri` パッケージバージョン不一致を最小差分で修正した。

- Issue: `https://github.com/KingYoSun/kukuri/issues/71`
- Failed run: `https://github.com/KingYoSun/kukuri/actions/runs/22118410647`
- Failed job: `https://github.com/KingYoSun/kukuri/actions/runs/22118410647/job/63932372778`

## 根因

`kukuri-tauri/src-tauri/Cargo.toml` 側は Phase C で `tauri = 2.10.2` に更新済みだった一方、Node 側 lock 解決が `@tauri-apps/api = 2.9.1` のまま残っていた。

その結果、Desktop E2E の `pnpm tauri build --debug --no-bundle` 実行時に以下のチェックで即時失敗していた。

- `tauri (v2.10.2) : @tauri-apps/api (v2.9.1)`

`Push Heavy Checks` はこの `desktop-e2e` 失敗を受けて gate fail になっていた。

## 修正内容

- `kukuri-tauri/package.json`
  - `@tauri-apps/api`: `^2.9.1` -> `^2.10.1`
  - `@tauri-apps/cli`: `^2.9.6` -> `^2.10.0`
- `kukuri-tauri/pnpm-lock.yaml`
  - `@tauri-apps/api@2.10.1` / `@tauri-apps/cli@2.10.0` に同期

## 検証

- 失敗ログ解析:
  - `gh run view 22118410647 --repo KingYoSun/kukuri --job 63932372778 --log-failed`
  - `gh run view 22118410647 --repo KingYoSun/kukuri --job 63934581816 --log-failed`
- 失敗経路の実行:
  - `DOCKER_CONFIG=/tmp/docker-config bash ./scripts/test-docker.sh e2e-community-node`
  - 結果: `tauri` version mismatch は再発せず、E2E 本体まで進行。
  - 補足: `community-node.friend-plus` で `invalid session id`（ページクラッシュ/ハング）を観測し、これは本修正とは別軸の既存不安定として切り分け。
- 根因ポイントの直接確認:
  - `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml run --rm --no-deps ts-test bash -lc 'set -euo pipefail; export PATH=/usr/local/cargo/bin:$PATH; cd /app/kukuri-tauri; pnpm e2e:seed >/tmp/e2e-seed.log; pnpm tauri build --debug --no-bundle'`
  - 結果: pass（mismatch エラー再発なし、`Built application` 到達）
- AGENTS 必須の `gh act` ジョブ:
  - `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue71-followup.log`
  - 結果: pass
  - `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue71-followup.log`
  - 結果: pass
  - `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue71-followup.log`
  - 結果: fail（既存コンテナ名衝突 `/kukuri-community-node-meilisearch`）
  - `docker rm -f kukuri-community-node-meilisearch kukuri-community-node-postgres` 後に同ジョブ再実行:
    - `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue71-followup-rerun.log`
  - 結果: pass

## 変更ファイル

- `kukuri-tauri/package.json`
- `kukuri-tauri/pnpm-lock.yaml`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue71_followup_tauri_npm_version_sync.md`
