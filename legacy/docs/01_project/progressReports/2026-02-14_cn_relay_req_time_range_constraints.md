# Community Nodes 進捗レポート（`cn-relay` REQ `since/until` 時間範囲制約）

作成日: 2026年02月14日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `services_relay.md` の REQ 制約にある `since/until` 時間範囲の上限を実装する（`filters.rs` は現状 `#t`/filter数/値数/`limit` のみ制約し、時間範囲は未制約）。`since > until`・過大lookback・過大window の拒否理由を固定し、unit/integration テストを追加する。

を実装し、完了状態へ更新した。

## 実装内容

1. `filters.rs` に時間範囲バリデーションを追加
- `since > until` を拒否（固定理由: `invalid since/until range`）
- 過大 lookback を拒否（固定理由: `lookback too large`）
- 過大 window を拒否（固定理由: `time window too large`）

2. unit test を追加（拒否理由の固定）
- `parse_filters_rejects_since_greater_than_until_with_stable_reason`
- `parse_filters_rejects_lookback_too_large_with_stable_reason`
- `parse_filters_rejects_time_window_too_large_with_stable_reason`

3. integration test を追加（WebSocket REQ 経路の拒否理由固定）
- `req_filter_time_range_rejections_use_stable_notice_reasons`
- `NOTICE` reason が上記 3 種で安定することを検証

4. タスク管理更新
- `community_nodes_roadmap.md` の該当未実装項目を `[x]` に更新

## 変更ファイル

- `kukuri-community-node/crates/cn-relay/src/filters.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-14.md`
- `docs/01_project/progressReports/2026-02-14_cn_relay_req_time_range_constraints.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（成功）
  - `cn-relay` の追加テスト（unit/integration）を含めて通過
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`
  - 初回: `tmp/logs/gh-act-format-check-cn-relay-time-range-20260214.log`（`rustfmt` 差分で失敗）
  - 再実行: `tmp/logs/gh-act-format-check-cn-relay-time-range-20260214-final.log`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-time-range-20260214-final.log`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-time-range-20260214-final.log`（成功）

補足: `gh act` 実行時に PowerShell 側で `NativeCommandError` が表示されるが、ログ終端の `Job succeeded` によりジョブ成功を確認済み。
