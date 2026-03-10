# Issue #27 PR-02 投稿検索ドキュメント（PGroonga）

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-02 スコープとして、投稿検索の PGroonga 経路を追加した。
既存 Meilisearch 経路は維持し、`search_write_mode` / `search_read_backend` ランタイムフラグで段階的切替できるよう最小互換で実装した。

## 実施内容

1. 投稿検索ドキュメント用 migration 追加
- ファイル: `kukuri-community-node/migrations/20260216030000_m7_post_search_documents.sql`
- `cn_search.post_search_documents` テーブルを追加。
- `post_search_text_pgroonga_idx`（`USING pgroonga (search_text)`）を追加。
- `topic_id + created_at`、`visibility + is_deleted` 補助 index を追加。

2. 共通正規化モジュール追加
- ファイル: `kukuri-community-node/crates/cn-core/src/search_normalizer.rs`
- `SEARCH_NORMALIZER_VERSION=1` を導入。
- NFKC + lower-case + 制御文字/空白正規化 + 記号処理（`#`/`@` 保持）を実装。
- `normalize_search_text` / `normalize_search_terms` / `build_search_text` を追加。

3. `cn-index` outbox consumer に dual-write 実装
- ファイル: `kukuri-community-node/crates/cn-index/src/lib.rs`
- `SearchWriteMode`（`meili_only` / `dual` / `pg_only`）を追加。
- ランタイムフラグ読取失敗時は `meili_only` にフォールバック。
- upsert/delete/expiry で Meili と PG の書込をモード別に制御。
- stale version 削除時も PG 側 `is_deleted=true` を反映。

4. `/v1/search` の PG read backend 追加
- ファイル: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `search_read_backend=pg` のとき PG 経路へ分岐。
- クエリを `search_normalizer` で正規化し、`normalizer_version` 一致のみ検索対象化。
- 非空クエリは PGroonga `&@~` + 合成スコア（text/freshness/popularity）でソート。
- レスポンス形状（`topic/query/items/next_cursor/total` と item フィールド）を既存互換で維持。

5. ランタイムフラグ定数拡張
- ファイル: `kukuri-community-node/crates/cn-core/src/search_runtime_flags.rs`
- `SEARCH_READ_BACKEND_PG` / `SEARCH_WRITE_MODE_DUAL` / `SEARCH_WRITE_MODE_PG_ONLY` を追加。

6. 回帰テスト追加
- `cn-core`: `search_normalizer` ユニットテスト（4件）
- `cn-index`: `outbox_dual_write_updates_meili_and_post_search_documents`
- `cn-user-api`: `search_contract_pg_backend_switch_normalization_and_version_filter`

## 後方互換性

- write 側は `meili_only` を既定とし、既存 Meili indexing への影響を回避。
- read 側は `meili` 既定を維持し、フラグ有効化時のみ PG 検索へ切替。
- runtime flag 読取失敗時は Meili 経路へフォールバックするため、migration 適用順序の差異でも動作継続。

## 検証コマンド

- `cd kukuri-community-node && cargo fmt`
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-core search_normalizer -- --nocapture; cargo test -p cn-index outbox_dual_write_updates_meili_and_post_search_documents -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_switch_normalization_and_version_filter -- --nocapture"`
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr02.log`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr02.log`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr02.log`

## 検証結果

- 追加した正規化/dual-write/PG read の対象テストは pass。
- Community Node 全体 `cargo test --workspace --all-features` + `cargo build --release -p cn-cli` は pass。
- `gh act` の `format-check` / `native-test-linux` は pass。
- `gh act community-node-tests` は `cn-admin-api` 契約テスト `trust_contract_success_and_shape` で fail（今回変更範囲外の既知系不安定失敗としてログ保全）。

## 影響範囲

- 変更は `kukuri-community-node` の検索経路に限定。
- 既存 Meili 経路を残したため、段階移行（PR-06/PR-07）へ接続可能。
