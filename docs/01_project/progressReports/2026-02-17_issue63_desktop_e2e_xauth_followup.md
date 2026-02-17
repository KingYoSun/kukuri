# Issue #63 Desktop E2E 再発対応（xauth 欠落）

最終更新日: 2026年02月17日

## 概要

Run `22103489567` では OpenAPI 側は修正済みで成功した一方、`Desktop E2E (Community Node, Docker)` が再失敗した。

失敗ログを再解析した結果、`xvfb-run` 実行時に `xauth command not found` が発生しており、前回修正で導入した `--no-install-recommends` により `xvfb` の推奨依存 `xauth` が test-runner image に入らなくなったことが再発根因だった。

## 実施内容

1. `gh run view 22103489567 --job 63879454427 --log-failed` で失敗行を特定（`xvfb-run: error: xauth command not found`）。
2. `git show 03b007a4 -- Dockerfile.test` で前回変更の差分を確認し、`--no-install-recommends` 導入後に依存不足が発生したことを確定。
3. `Dockerfile.test` の APT 依存へ `xauth` を追加して Desktop E2E 経路のみ最小修正。

## 変更ファイル

- `Dockerfile.test`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/progressReports/2026-02-17_issue63_desktop_e2e_xauth_followup.md`

## 検証

- `DOCKER_CONFIG=/tmp/docker-config bash ./scripts/test-docker.sh e2e-community-node`（pass, `Spec Files: 17 passed, 17 total`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue63-followup.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue63-followup.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue63-followup.log`（pass）

## 参照

- tracking issue #63: `https://github.com/KingYoSun/kukuri/issues/63`
- CI Run: `https://github.com/KingYoSun/kukuri/actions/runs/22103489567`
- Desktop E2E failed job: `https://github.com/KingYoSun/kukuri/actions/runs/22103489567/job/63879454427`
