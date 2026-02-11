# Community Node バックアップ/リストア運用スクリプト化（復旧ドリル）

作成日: 2026年02月11日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `ops_runbook.md` のバックアップ/リストア要件（`pg_dump` 世代管理・`pg_restore` 復旧手順）を運用スクリプト化し、`scripts/test-docker.ps1` から実行できる復旧ドリルを整備する

を実装し、完了状態へ更新した。

## 実装内容

- `scripts/test-docker.ps1` に `recovery-drill` コマンドを追加
  - `ValidateSet` / Help / Examples / command dispatch を更新
  - 実行コマンド: `./scripts/test-docker.ps1 recovery-drill`

- Postgres バックアップ自動化（`pg_dump`）
  - 出力先: `test-results/community-node-recovery/backups/community-node-pgdump-<timestamp>.dump`
  - 圧縮付き custom format dump（`--format=custom --compress=9`）
  - 世代管理: `COMMUNITY_NODE_BACKUP_GENERATIONS`（既定 `30`）を超える古い dump を自動削除

- Postgres 復旧自動化（`pg_restore`）
  - `dropdb --force` + `createdb` で復旧先 DB を再作成
  - `pg_restore -U cn --clean --if-exists --no-owner --no-acl --dbname=cn` で復旧

- 復旧ドリルの実装
  1. community-node（user-api/bootstrap）起動 + E2E seed投入
  2. `cn_relay.events` 件数を基準値として記録
  3. バックアップ取得
  4. 書き込みサービス停止後に `TRUNCATE` で障害を模擬
  5. バックアップから復旧
  6. サービス再起動後、`cn_relay.events` 件数が基準値に戻ることを検証

- ログ/サマリ出力
  - ログ: `tmp/logs/community-node-recovery/<timestamp>.log`
  - サマリ: `test-results/community-node-recovery/<timestamp>-summary.json`
  - 最新サマリ: `test-results/community-node-recovery/latest-summary.json`

- Runbook 反映
  - `docs/03_implementation/community_nodes/ops_runbook.md` に 2.4 節を追加し、運用コマンド・出力先・ドリル内容を明文化

- タスク反映
  - `community_nodes_roadmap.md` の該当項目を `[x]` に更新

## 変更ファイル

- `scripts/test-docker.ps1`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-11.md`

## 検証

- `./scripts/test-docker.ps1 recovery-drill -NoBuild`（成功）
  - ログ: `tmp/logs/community-node-recovery/20260211-230155.log`
  - サマリ: `test-results/community-node-recovery/20260211-230155-summary.json`

- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-community-node-recovery-20260211-230232.log`

- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-community-node-recovery-20260211-230352.log`

- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-community-node-recovery-20260211-231028.log`
