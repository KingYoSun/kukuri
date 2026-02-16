# Issue #43 in_progress 運用衛生化と AGENTS 更新

作成日: 2026年02月16日

## 背景

- `docs/01_project/activeContext/tasks/status/in_progress.md` に完了済みの Issue #27 記録が残存し、作業中タスク一覧としての視認性と運用ルールの一貫性を損なっていた。

## 実施内容

- `docs/01_project/activeContext/tasks/status/in_progress.md` から、完了済み見出し `2026年02月16日 Issue #27 検索PG移行計画の初期監査（完了）` 配下を削除。
- `AGENTS.md` の「タスク管理ルール」に、`in_progress.md` へ完了状態を残さない明示ルールと、完了時の移管順序を追記。
- `AGENTS.md` の「作業完了チェックリスト」に、完了状態残留チェックと移管先記録チェックを追記。

## 変更ファイル

- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `AGENTS.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue43_in_progress_hygiene_and_agents_update.md`

## 検証

- `date "+%Y年%m月%d日"`（pass）
- `rg -n "状態: 完了|（完了）" docs/01_project/activeContext/tasks/status/in_progress.md`（pass: 一致なし）
- `git diff --check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache`（pass, `tmp/logs/gh-act-format-check-issue43.log`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache`（pass, `tmp/logs/gh-act-native-test-linux-issue43.log`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache`（pass, `tmp/logs/gh-act-community-node-tests-issue43.log`）

## 結果

- `in_progress.md` は未完了タスクのみの状態に整理完了。
- 完了タスクの移管先と再発防止ルールを `AGENTS.md` に明文化。
