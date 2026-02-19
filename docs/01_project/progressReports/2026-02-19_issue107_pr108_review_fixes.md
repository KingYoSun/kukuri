# Issue #107 PR #108 レビュー指摘対応（テスト復旧）

作成日: 2026年02月19日  
Issue: https://github.com/KingYoSun/kukuri/issues/107  
PR: https://github.com/KingYoSun/kukuri/pull/108  
対象ブランチ: `chore/issue-107-remove-meilisearch-dependency`

## 背景

- PR #108 の Codex Review で、以下 2 点の回帰リスクが指摘された。
  - `cn-user-api` の `api_contract_tests` が `#[cfg(any())]` で常時無効化されていた。
  - `cn-index` の統合テストファイルとモジュール参照が削除され、outbox/reindex/backfill の自動回帰検知が失われていた。

## 実施内容

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - `#[cfg(any())] mod api_contract_tests {}` を廃止し、PG-only 前提で `api_contract_tests` を復旧。
  - `/v1/search` 契約（認証必須、レスポンス形状、normalizer_version フィルタ、同一 post_id の topic 分離）を再検証するテストを追加。
- `kukuri-community-node/crates/cn-index/src/lib.rs`
  - `#[cfg(test)] mod integration_tests;` を再追加。
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
  - 削除されていた統合テストを復元し、PG-only 経路に合わせて再構成。
  - outbox / reindex / backfill / healthz / metrics の回帰テストを維持。
- いずれも Meilisearch 依存を再導入せず、Issue #107 の PG-only 目標は維持。

## 検証

- `cd kukuri-community-node && cargo fmt --all`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml up -d community-node-postgres`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api subscriptions::api_contract_tests -- --nocapture; cargo test -p cn-index -- --nocapture"`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

## 結果

- Codex Review で指摘された 2 点（`api_contract_tests` の無効化、`cn-index` 統合テスト削除）を解消。
- PG-only 移行方針を維持したまま、CI で契約・統合テストが継続実行される状態に復旧した。
