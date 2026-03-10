# Issue #27 / PR #30 fix loop second pass（OpenAPI generated artifacts drift）

最終更新日: 2026年02月16日

## 概要

PR #30 の CI Run `22052290440`（OpenAPI Artifacts Check）で、生成物整合チェック `Verify generated artifacts are up-to-date` が失敗していたため、差分原因を特定して OpenAPI 生成物を最小差分で更新した。

## 原因

- 失敗ジョブ: https://github.com/KingYoSun/kukuri/actions/runs/22052290440/job/63712785423
- `gh run view` ログ上で、`kukuri-community-node/apps/admin-console/openapi/user-api.json` に `/v1/communities/suggest` path が未反映。
- PR #30 で追加した community suggest endpoint の OpenAPI 反映漏れが原因。

## 実施内容

1. CIログ解析
- `gh run view 22052290440 --job 63712785423 --log` で差分出力を確認。

2. 生成物再作成（CI同等）
- `kukuri-community-node` で `cargo run --locked -p cn-cli -- openapi export --service user-api/admin-api ... --pretty` を実行。
- `apps/admin-console` で `pnpm generate:api` を実行。

3. 差分確認
- `git diff --stat` により、更新対象が `kukuri-community-node/apps/admin-console/openapi/user-api.json` のみであることを確認。

## 変更ファイル

- `kukuri-community-node/apps/admin-console/openapi/user-api.json`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_pr30_second_pass_openapi_artifacts_fix_loop.md`

## 検証

- `cd /home/kingyosun/kukuri && gh run view 22052290440 --job 63712785423 --log`（pass）
- `cd /home/kingyosun/kukuri/kukuri-community-node/apps/admin-console && pnpm install --frozen-lockfile`（pass）
- `cd /home/kingyosun/kukuri/kukuri-community-node && cargo run --locked -p cn-cli -- openapi export --service user-api --output apps/admin-console/openapi/user-api.json --pretty && cargo run --locked -p cn-cli -- openapi export --service admin-api --output apps/admin-console/openapi/admin-api.json --pretty`（pass）
- `cd /home/kingyosun/kukuri/kukuri-community-node/apps/admin-console && pnpm generate:api`（pass）
- `cd /home/kingyosun/kukuri && git diff --stat`（pass: `user-api.json` のみ変更）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr30-second-pass-openapi.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr30-second-pass-openapi.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr30-second-pass-openapi.log`（pass）
