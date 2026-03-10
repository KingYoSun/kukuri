# Issue #55 Heavy lane stale prebuilt image fix

最終更新日: 2026年02月17日

## 概要

main の Heavy lane（Run `22090400126`）で `Desktop E2E (Community Node, Docker)` が再失敗したため調査した。
失敗ログと workflow/script の分岐を突合した結果、`desktop-e2e` が `KUKURI_TEST_RUNNER_IMAGE=ghcr.io/kingyosun/kukuri-test-runner:latest` を渡し、checkout より古い prebuilt image 内の stale E2E spec を実行していたことが原因だった。

## 実施内容

1. `gh run view 22090400126 --log-failed` で失敗 spec と実行 selector を抽出。
2. `.github/workflows/test.yml` の `desktop-e2e` で `KUKURI_TEST_RUNNER_IMAGE` を空文字に変更。
3. `scripts/test-docker.ps1` の prebuilt 優先分岐に入らないようにし、常に checkout から build する経路へ統一。
4. tracking issue #55 を作成し、#52 は別根因として reopen せずコメントで切り分けを記録。

## 変更ファイル

- `.github/workflows/test.yml`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/progressReports/2026-02-17_issue55_heavy_lane_stale_prebuilt_fix.md`

## 検証

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）
- `DOCKER_CONFIG=/tmp/docker-config bash scripts/test-docker.sh e2e-community-node`（pass, `Spec Files: 17 passed, 17 total`）

ログ:

- `tmp/logs/gh-act-format-check-20260217-084135.log`
- `tmp/logs/gh-act-native-test-linux-20260217-084202.log`
- `tmp/logs/gh-act-community-node-tests-20260217-084503.log`
- `tmp/logs/test-docker-e2e-community-node-20260217-084758.log`

## 参照

- Issue #55: `https://github.com/KingYoSun/kukuri/issues/55`
- 既存 Issue #52 コメント: `https://github.com/KingYoSun/kukuri/issues/52#issuecomment-3913266183`
