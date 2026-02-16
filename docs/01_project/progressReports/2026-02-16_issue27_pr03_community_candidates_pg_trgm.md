# Issue #27 PR-03 コミュニティ候補生成（pg_trgm + prefix）

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-03 スコープとして、コミュニティ候補生成を Meili 依存から分離し、PostgreSQL（`pg_trgm` + prefix）で段階移行できる経路を追加した。
既存経路は `suggest_read_backend=legacy` で維持し、`pg` 切替時も候補ゼロなら legacy fallback する構成で blast radius を最小化した。

## 実施内容

1. schema/migration 追加
- 追加: `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql`
- `cn_search.community_search_terms` を追加。
- index:
  - `community_search_terms_trgm_idx` (`gin (term_norm gin_trgm_ops)`)
  - `community_search_terms_prefix_idx` (`btree (term_norm text_pattern_ops)`)
- 既存データの初期バックフィル:
  - `cn_admin.node_subscriptions`
  - `cn_user.topic_subscriptions (status='active')`

2. 共通候補語生成ロジックを `cn-core` へ集約
- 追加: `kukuri-community-node/crates/cn-core/src/community_search_terms.rs`
- `build_terms_from_topic_id` で `name`/`alias` 候補語を生成し、`search_normalizer` で正規化。
- `kukuri:<64hex>` は alias 展開を抑制（ハッシュ topic で誤候補を増やさない）。

3. write path（`cn-index`）へ同期追加
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- outbox upsert 時に `upsert_community_search_terms` を実行し、候補語テーブルへ UPSERT。
- migration 未適用環境向けに `42P01/3F000` を握って処理継続（後方互換維持）。

4. read path（`cn-user-api`）実装
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- 追加 endpoint: `GET /v1/communities/suggest`
- ランタイムフラグ:
  - `suggest_read_backend=legacy`（既定）
  - `suggest_read_backend=pg`
- Stage-A 候補生成:
  - 入力長 1-2 文字: prefix 優先（`LIKE q%` + 高しきい値 similarity）
  - 入力長 3 文字以上: prefix + trgm（`%` 演算子）
- pg 側が空結果の場合は `legacy_fallback` へ自動フォールバック。

5. ルーティング / OpenAPI
- 変更: `kukuri-community-node/crates/cn-user-api/src/lib.rs`
- 変更: `kukuri-community-node/crates/cn-user-api/src/openapi.rs`
- 変更: `kukuri-community-node/crates/cn-user-api/src/openapi_contract_tests.rs`

## 後方互換性

- 既定 backend は `legacy` のままで既存挙動を維持。
- `pg` 有効化時も、候補語テーブル未作成・空結果時に legacy fallback を実施。
- 既存検索 API（`/v1/search`）には影響を与えない実装に限定。

## 検証

- `cd kukuri-community-node && cargo fmt`（pass）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `docker compose -f docker-compose.test.yml build test-runner`（pass）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-core community_search_terms; cargo test -p cn-index outbox_upsert_updates_community_search_terms; cargo test -p cn-user-api community_suggest_pg_backend_supports_exact_prefix_and_trgm; cargo test -p cn-user-api community_suggest_legacy_backend_uses_topic_sources; cargo test -p cn-user-api openapi_contract_contains_user_paths"`（pass, `tmp/logs/issue27-pr03-targeted-tests.log`）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass, `tmp/logs/issue27-pr03-community-node-full.log`）
- `cd kukuri-tauri/src-tauri && cargo test`（pass, `tmp/logs/issue27-pr03-tauri-cargo-test.log`）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr03.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr03.log`（pass）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr03.log`（初回 fail: `auth_consent_quota_metrics_regression_counters_increment`）
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr03-rerun.log`（rerun pass）

## 変更ファイル（主要）

- `kukuri-community-node/migrations/20260216050000_m9_community_search_terms.sql`
- `kukuri-community-node/crates/cn-core/src/community_search_terms.rs`
- `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-user-api/src/lib.rs`
- `kukuri-community-node/crates/cn-user-api/src/openapi.rs`
- `kukuri-community-node/crates/cn-user-api/src/openapi_contract_tests.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
