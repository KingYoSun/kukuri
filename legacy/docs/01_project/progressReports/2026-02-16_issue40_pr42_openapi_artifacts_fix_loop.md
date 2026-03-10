# Issue #40 / PR #42 fix loop（OpenAPI Artifacts Check generated drift）

作成日: 2026年02月16日
最終更新日: 2026年02月16日

## 背景

- 対象PR: `https://github.com/KingYoSun/kukuri/pull/42`
- 失敗Run/Job:
  - `https://github.com/KingYoSun/kukuri/actions/runs/22064412365`
  - `https://github.com/KingYoSun/kukuri/actions/runs/22064412365/job/63752435217`
- 失敗ステップ: `Verify generated artifacts are up-to-date`

## 根因

- M-01 で `kukuri-community-node/Cargo.toml` の workspace license を `MIT` に更新した一方、`apps/admin-console/openapi/*.json` は更新前 (`Apache-2.0`) のまま残っていた。
- CI で `cn-cli openapi export` を再実行すると `info.license.name` が `MIT` へ変化し、`git diff --exit-code` が失敗した。

## 実施内容

- 失敗ジョブログを解析し、ドリフト箇所を特定。
- CI と同一コマンドで OpenAPI スナップショットを再生成。
- 差分が `admin-api.json` / `user-api.json` の `info.license.name` のみであることを確認。
- OpenAPI 生成物2件を更新し、M-01 スコープ外の変更を発生させない形で fix loop を完了。

## 変更ファイル

- `kukuri-community-node/apps/admin-console/openapi/admin-api.json`
- `kukuri-community-node/apps/admin-console/openapi/user-api.json`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-16_issue40_pr42_openapi_artifacts_fix_loop.md`

## 検証

- `cd /home/kingyosun/kukuri && gh run view 22064412365 --job 63752435217 --log`（pass: 差分原因を確認）
- `cd /home/kingyosun/kukuri/kukuri-community-node/apps/admin-console && pnpm install --frozen-lockfile`（pass）
- `cd /home/kingyosun/kukuri/kukuri-community-node && cargo run --locked -p cn-cli -- openapi export --service user-api --output apps/admin-console/openapi/user-api.json --pretty`（pass）
- `cd /home/kingyosun/kukuri/kukuri-community-node && cargo run --locked -p cn-cli -- openapi export --service admin-api --output apps/admin-console/openapi/admin-api.json --pretty`（pass）
- `cd /home/kingyosun/kukuri/kukuri-community-node/apps/admin-console && pnpm generate:api`（pass）
- `cd /home/kingyosun/kukuri && git diff -- kukuri-community-node/apps/admin-console/openapi/admin-api.json kukuri-community-node/apps/admin-console/openapi/user-api.json kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`（pass: JSON 2件のみ差分）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue40-pr42-openapi-fix-loop.log`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue40-pr42-openapi-fix-loop.log`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue40-pr42-openapi-fix-loop.log`（pass）

## スコープ確認

- 実装変更は OpenAPI 生成物のライセンス表記整合に限定。
- M-02 / M-03 は未着手のまま分離し、PR #42 の目的（M-01）を維持。
