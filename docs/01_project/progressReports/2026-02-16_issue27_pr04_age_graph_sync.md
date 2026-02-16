# Issue #27 PR-04 AGE グラフ同期

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-04 スコープとして、コミュニティサジェスト向けの AGE グラフ同期経路を `cn-index` に実装した。
既存 trust グラフとは分離した suggest 専用グラフ（`kukuri_cn_suggest`）を導入し、outbox 差分同期と checkpoint 再開、`user_community_affinity` の再計算を追加した。

## 実施内容

1. schema/migration 追加
- 追加: `kukuri-community-node/migrations/20260216060000_m10_age_graph_sync.sql`
- 追加テーブル:
  - `cn_search.graph_sync_offsets`（consumer 別 checkpoint）
  - `cn_search.user_community_affinity`（`(user_id, community_id)` 主キー）
- 追加 index:
  - `user_community_affinity_score_idx`

2. `cn-index` の suggest graph 初期化とブートストラップ
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- suggest graph 定数:
  - `SUGGEST_GRAPH_NAME = "kukuri_cn_suggest"`
  - `GRAPH_SYNC_CONSUMER_NAME = "index-age-suggest-v1"`
- 起動時に `ensure_suggest_graph` を実行し、必要なラベル/エッジ種別を初期化。
- 初回のみ `bootstrap_suggest_graph_if_needed` で snapshot 構築後、`MAX(seq)` を checkpoint として記録。

3. outbox 差分同期（deterministic + 再開可能）
- `handle_outbox_row` 成功後に `sync_suggest_graph_for_outbox_row` を実行。
- シグナル同期対象:
  - `MEMBER_OF`（topic membership）
  - `FOLLOWS_COMMUNITY`（topic subscription）
  - `VIEWED_COMMUNITY`（event activity）
  - `FOLLOWS_USER`（kind=3 contact list）
- outbox 1行処理ごとに `graph_sync_offsets` を更新し、再起動時は checkpoint から再開。
- migration 未適用環境では `42P01/3F000` を検知して warn し、既存 index 処理を継続（後方互換維持）。

4. affinity 定期再計算
- 変更: `kukuri-community-node/crates/cn-index/src/config.rs`
- `graph_affinity.recompute_interval_seconds` を runtime config に追加（default 300秒）。
- `spawn_affinity_recompute_worker` を追加し、`cn_search.user_community_affinity` を deterministic に全再生成。

5. テスト追加
- 変更: `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- 追加テスト:
  - `outbox_graph_sync_updates_age_edges_and_affinity`
  - `outbox_graph_sync_kind3_delete_is_idempotent`
- AGE セッション初期化付き helper と cleanup を追加し、再実行時の event_id 衝突を回避して安定化。

## 後方互換性

- suggest graph は trust 既存グラフと分離し、既存 trust 経路へ影響を与えない。
- `graph_sync_offsets` / `user_community_affinity` が未作成でも、既存 outbox index 処理は停止せず継続する。
- 既定 runtime config に `graph_affinity` を追加したが、未設定時は default 値で互換動作する。

## 検証

- `cd /home/kingyosun/kukuri && docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-index outbox_graph_sync_updates_age_edges_and_affinity -- --nocapture; cargo test -p cn-index outbox_graph_sync_kind3_delete_is_idempotent -- --nocapture; cargo test -p cn-index outbox_upsert_updates_community_search_terms -- --nocapture"`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `cd /home/kingyosun/kukuri/kukuri-tauri/src-tauri && cargo test`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr04.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr04.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr04.log`（pass）

## 変更ファイル（主要）

- `kukuri-community-node/migrations/20260216060000_m10_age_graph_sync.sql`
- `kukuri-community-node/crates/cn-index/src/config.rs`
- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
