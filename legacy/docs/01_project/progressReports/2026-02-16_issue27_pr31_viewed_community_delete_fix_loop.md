# Issue #27 / PR #31 fix loop（`VIEWED_COMMUNITY` delete再計算）

最終更新日: 2026年02月16日

## 概要

PR #31 のレビュー指摘（`discussion_r2811074944`）に対応し、`cn-index` の増分グラフ同期で delete 後に `VIEWED_COMMUNITY` エッジが残留する不整合を修正した。
これにより、増分同期結果と snapshot 再構築結果の差異が解消される。

## 原因

- `sync_suggest_graph_for_outbox_row` の `delete` 分岐では `MEMBER_OF` / `FOLLOWS_COMMUNITY` しか再計算していなかった。
- `VIEWED_COMMUNITY` は `upsert` 時にのみ追加されるため、最新閲覧イベントが削除・失効しても stale edge が残っていた。

## 実装内容

1. `VIEWED_COMMUNITY` の再計算ヘルパー追加
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- 追加:
  - `refresh_viewed_community_edge_from_current`
  - `delete_viewed_community_edge`
- 現行ソースオブトゥルース（`cn_relay.events` + `event_topics`、`is_deleted = FALSE AND is_current = TRUE AND not expired`）から `MAX(created_at)` を再計算し、
  - 値あり: `upsert_viewed_community_edge`
  - 値なし: `delete_viewed_community_edge`
  を実施するようにした。

2. delete / 非アクティブ upsert への適用
- `sync_suggest_graph_for_outbox_row` の `delete` 分岐で上記再計算を実行。
- 同時に、非アクティブ `upsert`（`is_deleted`・`is_current=false`・expired）でも同再計算を実行し、更新系でも stale edge が残らないよう補強。

3. 回帰テストの追加
- 変更: `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- 既存テスト `outbox_graph_sync_kind3_delete_is_idempotent` に以下を追加:
  - upsert 後に `VIEWED_COMMUNITY` が 1 件存在すること
  - delete 後に `VIEWED_COMMUNITY` が 0 件になること

## 検証

- `cd /home/kingyosun/kukuri && docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-index outbox_graph_sync_updates_age_edges_and_affinity -- --nocapture; cargo test -p cn-index outbox_graph_sync_kind3_delete_is_idempotent -- --nocapture"`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr31-fix-loop.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr31-fix-loop.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr31-fix-loop.log`（pass）

## 変更ファイル

- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_pr31_viewed_community_delete_fix_loop.md`
