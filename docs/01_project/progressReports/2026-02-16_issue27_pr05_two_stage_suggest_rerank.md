# Issue #27 PR-05 2段階サジェスト rerank

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-05 スコープとして、`/v1/communities/suggest` に Stage-A（候補生成）+ Stage-B（関係性再ランキング）を実装した。
PR-03 の候補生成を維持しつつ、`user_community_affinity` を用いた rerank と block/mute/visibility の最終フィルタを SQL 側へ集約し、`shadow`/`enabled` の段階移行を可能にした。

## 実施内容

1. runtime flag/migration 追加
- 追加: `kukuri-community-node/migrations/20260216070000_m11_suggest_rerank_runtime_flags.sql`
- 追加フラグ:
  - `suggest_rerank_mode`（`shadow`/`enabled`）
  - `suggest_relation_weights`（JSON）
- 既定値:
  - `suggest_rerank_mode=shadow`
  - `suggest_relation_weights={"is_member":1.20,"is_following_community":0.80,"friends_member":0.35,"two_hop_follow":0.25,"recent_view":0.15}`

2. runtime flag ローダー拡張
- 変更: `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
- `SearchRuntimeFlags` に `suggest_rerank_mode` / `suggest_relation_weights` を追加。
- seed/default 読み込みテストを更新し、旧フラグ運用との後方互換を維持。

3. suggest 2段階パイプライン実装
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- Stage-A:
  - 既存 prefix/trgm 候補生成を維持して topN を取得。
- Stage-B:
  - CTE で candidate を `user_community_affinity` と join。
  - weighted score（name/relation/popularity/recency）を計算。
  - SQL 側で block/mute/visibility を最終適用。
- mode 制御:
  - `enabled`: Stage-B 順位をレスポンスへ反映。
  - `shadow`: レスポンスは Stage-A 順位を維持しつつ、shadow rerank を計測ログとして記録。

4. observability 追加
- 変更: `kukuri-community-node/crates/cn-core/src/metrics.rs`
- 追加メトリクス:
  - `suggest_stage_a_latency_ms`
  - `suggest_stage_b_latency_ms`
  - `suggest_block_filter_drop_count`
- `cn-user-api` から実測値を emit するよう接続。

5. 回帰テスト拡張
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`（`api_contract_tests`）
- 追加/更新テスト:
  - `community_suggest_pg_backend_supports_exact_prefix_and_trgm`
  - `community_suggest_pg_rerank_enabled_prioritizes_affinity_and_visibility`
  - `community_suggest_pg_rerank_filters_muted_communities`
  - `community_suggest_pg_shadow_mode_preserves_stage_a_order_and_reports_shadow`
  - `community_suggest_legacy_backend_uses_topic_sources`
  - `community_search_alias_backfill_skips_kukuri_hashed_tail_topics`

## 後方互換性

- `suggest_read_backend=legacy` では既存 suggest 経路を維持。
- `suggest_rerank_mode=shadow`（既定）では、返却順は Stage-A を維持しつつ Stage-B を観測のみ行う。
- migration 未適用などで Stage-B を評価できない環境でも、Stage-A/legacy へフォールバックして API を継続可能。

## 検証

- `cd /home/kingyosun/kukuri && docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `cd /home/kingyosun/kukuri && docker compose -f docker-compose.test.yml build test-runner`（pass）
- `cd /home/kingyosun/kukuri && docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-core search_runtime_flags -- --nocapture; cargo test -p cn-user-api community_suggest_pg_backend_supports_exact_prefix_and_trgm -- --nocapture; cargo test -p cn-user-api community_suggest_pg_rerank_enabled_prioritizes_affinity_and_visibility -- --nocapture; cargo test -p cn-user-api community_suggest_pg_rerank_filters_muted_communities -- --nocapture; cargo test -p cn-user-api community_suggest_pg_shadow_mode_preserves_stage_a_order_and_reports_shadow -- --nocapture; cargo test -p cn-user-api community_suggest_legacy_backend_uses_topic_sources -- --nocapture; cargo test -p cn-user-api community_search_alias_backfill_skips_kukuri_hashed_tail_topics -- --nocapture"`（pass）
- `cd /home/kingyosun/kukuri && docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass。初回1回のみ `cn-admin-api::trust_contract_success_and_shape` が不安定失敗したため単体再実行後に full rerun で pass）
- `cd /home/kingyosun/kukuri/kukuri-tauri/src-tauri && cargo test`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr05.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr05.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr05.log`（pass）

## 変更ファイル（主要）

- `kukuri-community-node/migrations/20260216070000_m11_suggest_rerank_runtime_flags.sql`
- `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
- `kukuri-community-node/crates/cn-core/src/metrics.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/priority/search_pg_migration_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
