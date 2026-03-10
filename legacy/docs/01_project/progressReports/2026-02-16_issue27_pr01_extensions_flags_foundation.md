# Issue #27 PR-01 拡張導入とランタイムフラグ基盤

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-01 スコープとして、検索 PG 移行の土台（拡張導入・runtime flags 基盤）を最小差分で実装した。
検索機能本体の切替は行わず、既存挙動（Meilisearch read）を維持して後続 PR のための準備のみ追加している。

## 実施内容

1. Postgres イメージに PGroonga 導入
- ファイル: `kukuri-community-node/docker/postgres-age/Dockerfile`
- `groonga-apt-source-latest-bookworm.deb` を導入し、`postgresql-16-pgdg-pgroonga` をインストール。
- 既存 AGE ビルド手順は維持。

2. 拡張/フラグ migration 追加
- ファイル: `kukuri-community-node/migrations/20260216020000_m6_search_runtime_flags.sql`
- `CREATE EXTENSION IF NOT EXISTS pg_trgm;`
- `CREATE EXTENSION IF NOT EXISTS pgroonga;`
- `CREATE EXTENSION IF NOT EXISTS age;`
- `cn_search.runtime_flags` 作成と初期値 seed（`search_read_backend=meili` など）を追加。

3. runtime flags 読取基盤を `cn-core` に追加
- ファイル: `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
- `SearchRuntimeFlags` / `watch_search_runtime_flags` を実装。
- `cn_search.runtime_flags` が未作成の場合は互換デフォルト値へフォールバックするため、後方互換を確保。
- migration 後に `age` / `pg_trgm` / `pgroonga` が存在することをテストで検証。

4. `cn-user-api` / `cn-index` の読取経路を共通化
- ファイル: `kukuri-community-node/crates/cn-user-api/src/lib.rs`
- ファイル: `kukuri-community-node/crates/cn-index/src/lib.rs`
- 起動時に `cn_core::search_runtime_flags::watch_search_runtime_flags` を起動し、同一読取経路へ統一。

5. モジュール公開
- ファイル: `kukuri-community-node/crates/cn-core/src/lib.rs`
- `pub mod search_runtime_flags;` を追加。

## 設計判断

- 検索ランタイムフラグの正本は `cn_search.runtime_flags` とした。
- `cn_admin.service_configs` は既存用途を維持し、検索移行フラグの読取は新設 `cn_search.runtime_flags` へ統一した。
- この PR では read/write 切替ロジック自体は導入せず、観測可能な挙動変更を避けた。

## 検証コマンド

- `docker compose -f docker-compose.test.yml build community-node-postgres test-runner`
- `docker compose -f docker-compose.test.yml up -d --force-recreate community-node-postgres community-node-meilisearch`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

## 検証結果

- Community Node コンテナ経路: `cargo test --workspace --all-features` / `cargo build --release -p cn-cli` は pass。
- `gh act format-check`: pass（`tmp/logs/gh-act-format-check-issue27-pr01.log`）
- `gh act native-test-linux`: pass（`tmp/logs/gh-act-native-test-linux-issue27-pr01.log`）
- `gh act community-node-tests`: fail（2回再実行、いずれも `cn-admin-api` 契約テストの既存不安定失敗）
  - 1回目: `trust_contract_success_and_shape`
  - 2回目: `subscription_request_approve_rejects_when_node_topic_limit_already_exceeded`
  - ログ: `tmp/logs/gh-act-community-node-tests-issue27-pr01.log`, `tmp/logs/gh-act-community-node-tests-issue27-pr01-rerun.log`

## 影響範囲

- 検索移行の基盤（DB 拡張・フラグ読取）に限定。
- 既存 API の検索結果ロジック・書込みロジックは未変更。
- 後続 PR（PR-02 以降）で PG 検索実装へ段階移行可能な状態を用意。
