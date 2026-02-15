# Issue #22 / PR #26 Community Node Tests fix loop

最終更新日: 2026年02月15日

## 概要

PR #26（`feat/issue22-node-level-limit-regression-tests`）で発生していた `Community Node Tests` の失敗ループを解析し、同PRスコープで最小修正を適用して CI 経路を安定化した。

対象ジョブ:
- workflow run: `https://github.com/KingYoSun/kukuri/actions/runs/22038074347`
- job: `Community Node Tests`（`63674268397`）

## 失敗原因

1. `cn-core` テストの UB 起因クラッシュ
- `crates/cn-core/src/health.rs` のテストが `unsafe std::env::set_var/remove_var` を使用していた。
- 並列テスト中にプロセス環境変数へ同時アクセスが発生し、`SIGSEGV`（signal 11）を誘発。

2. `cn-admin-api` 契約テストの並列 DDL 競合
- `test_audit_failures_trigger` / `test_commit_failures_trigger` の準備で同時 DDL 実行が競合し、`trigger already exists` / `tuple concurrently updated` が発生。

3. PR #26 追加回帰テストの過剰な同値前提
- `subscription_request_approve_rejects_when_node_topic_limit_already_exceeded` が共有状態（global DB）前提で `current` / `limit` の厳密同値を要求していたため、並列実行で不安定化。

## 実施した修正

### 1) `cn-core` テストの環境変数依存除去
- ファイル: `kukuri-community-node/crates/cn-core/src/health.rs`
- `parse_health_targets_with` を追加し、`parse_health_targets` は env accessor を注入して再利用。
- テストは in-memory `HashMap` を使う方式に変更し、`unsafe set_var/remove_var` を削除。

### 2) `cn-admin-api` 契約テスト初期化の直列化
- ファイル: `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `ensure_audit_failure_trigger` / `ensure_commit_failure_trigger` で `pg_advisory_xact_lock` を取得して trigger 準備を直列化。
- 並列 DDL 実行による競合失敗を回避。

### 3) PR #26 追加 over-limit 回帰テストの安定化
- ファイル: `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `current > limit` 契約（本質要件）を維持しつつ、共有状態揺れに依存する厳密同値アサーションを緩和。

## 検証コマンド

- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-core --all-features"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-admin-api --all-features"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests`

## 検証結果

- `cn-core`: pass
- `cn-admin-api`: pass（43 passed）
- `cargo test --workspace --all-features` + `cargo build --release -p cn-cli`: pass
- `gh act format-check`: pass（`tmp/logs/gh-act-format-check-issue22-pr26-fix-loop.log`）
- `gh act native-test-linux`: pass（`tmp/logs/gh-act-native-test-linux-issue22-pr26-fix-loop.log`）
- `gh act community-node-tests`: pass（`tmp/logs/gh-act-community-node-tests-issue22-pr26-fix-loop.log`）

## 影響範囲

- 変更は `cn-core`/`cn-admin-api` のテスト補助ロジックと回帰テストアサーションに限定。
- 本番機能の挙動は変更なし。
