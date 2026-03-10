# Issue #27 / PR #30 fix loop（alias backfill の hashed-tail ノイズ除去）

最終更新日: 2026年02月16日

## 概要

PR #30 の review comment（`discussion_r2810818018`）で、migration の alias バックフィルが `kukuri:<64hex>` topic を取り込み、runtime 側（`build_terms_from_topic_id`）の挙動と不整合になる問題を修正した。
本対応で初期バックフィル段階から hex ノイズ候補が残留しないように揃えた。

## 原因

- migration `20260216050000_m9_community_search_terms.sql` の alias backfill は `alias_norm <> ''` と `alias_norm <> name_norm` のみで投入していた。
- そのため `kukuri:<64hex>` でも alias 行が作成される。
- 一方 runtime は `cn_core::community_search_terms::build_terms_from_topic_id` で hashed tail alias を抑制済み。
- 結果として「初期バックフィルでのみ残る alias ノイズ」が発生していた。

## 実施内容

1. migration backfill 条件を修正
- 対象: `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql`
- alias INSERT の WHERE に以下を追加:
  - `LOWER(TRIM(topic_id)) !~ '^kukuri:[0-9a-f]{64}$'`
- `kukuri:<64hex>` は初期バックフィル対象から除外。

2. 回帰テスト追加
- 対象: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- 追加:
  - helper `run_alias_backfill_for_topic`
  - test `community_search_alias_backfill_skips_kukuri_hashed_tail_topics`
- 検証内容:
  - `kukuri:<64hex>` は alias 0件
  - 通常 topic（`kukuri:tauri:...`）は alias が生成される

## 検証

- `cd kukuri-community-node && cargo fmt`（pass）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-core community_search_terms -- --nocapture; cargo test -p cn-user-api community_search_alias_backfill_skips_kukuri_hashed_tail_topics -- --nocapture; cargo test -p cn-user-api community_suggest_pg_backend_supports_exact_prefix_and_trgm -- --nocapture; cargo test -p cn-user-api community_suggest_legacy_backend_uses_topic_sources -- --nocapture" | tee tmp/logs/issue27-pr30-fix-loop-targeted-tests.log`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli" | tee tmp/logs/issue27-pr30-fix-loop-community-node-full.log`（pass）
- `cd kukuri-tauri/src-tauri && cargo test | tee /home/kingyosun/kukuri/tmp/logs/issue27-pr30-fix-loop-tauri-cargo-test.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr30-fix-loop.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr30-fix-loop.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr30-fix-loop.log`（pass）

## 影響範囲

- schema 追加・削除なし（既存 migration の条件調整のみ）。
- runtime API 仕様変更なし。
- 影響は `community_search_terms` 初期バックフィル時の alias 生成条件に限定。

## 変更ファイル

- `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_pr30_alias_backfill_hashed_tail_fix_loop.md`
