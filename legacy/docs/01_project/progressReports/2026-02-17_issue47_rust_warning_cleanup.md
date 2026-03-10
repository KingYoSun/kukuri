# Issue #47 Rust warning解消（clippy/コンパイラ警告の整理）

最終更新日: 2026年02月17日

## 概要

Issue #47（`[rust] Warning解消（clippy/コンパイラ警告の整理）`）に対して、`kukuri-tauri/src-tauri` と `kukuri-community-node` の warning を棚卸しし、挙動変更を伴わない最小修正で警告負債を削減した。

## 監査結果

- `kukuri-tauri/src-tauri`:
  - `cargo clippy --all-targets --all-features` で `collapsible_if` が多数（既存負債）に加え、`deprecated/unused/useless conversion` などの修正容易な warning を確認。
- `kukuri-community-node`:
  - Docker 内 `cargo clippy --workspace --all-targets --all-features` で `await_holding_lock`（主に統合テスト）と `too_many_arguments`（設計由来）に加え、`deprecated`・不要借用・単純 if 等を確認。

## 実施内容

1. `kukuri-tauri` で以下を最小修正。
   - `criterion::black_box` 非推奨呼び出しを `std::hint::black_box` へ置換。
   - 未使用変数、不要 `.into()`、同型キャスト、`unwrap_or_else(Vec::new)` を整理。
   - `sort_by` を `sort_by_key` へ置換。
   - `repeat().take()` を `std::iter::repeat_n()` へ置換。
   - テストファイル先頭の重複属性を削除。
2. `kukuri-community-node` で以下を最小修正。
   - `Timestamp::as_u64()` を `as_secs()` へ置換（非推奨対応）。
   - `Copy` 型への不要 `clone()` を削除。
   - `&vec![...]` の不要借用を削除。
   - `max().min()` の clamp パターンを `clamp()` へ統一。
   - 単純な入れ子 `if` と `Vec::new(); push(...)` を簡潔化。

## 変更ファイル

- `kukuri-tauri/src-tauri/benches/command_optimization.rs`
- `kukuri-tauri/src-tauri/src/application/services/p2p_service/tests.rs`
- `kukuri-tauri/src-tauri/src/application/services/user_service.rs`
- `kukuri-tauri/src-tauri/src/application/shared/tests/event/manager_tests.rs`
- `kukuri-tauri/src-tauri/src/infrastructure/event/event_manager_gateway.rs`
- `kukuri-tauri/src-tauri/tests/common/offline_support.rs`
- `kukuri-tauri/src-tauri/tests/common/performance/offline_seed.rs`
- `kukuri-tauri/src-tauri/tests/performance/sync.rs`
- `kukuri-community-node/crates/cn-moderation/src/lib.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-user-api/src/personal_data.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue47_rust_warning_cleanup.md`

## 検証

- `cd kukuri-tauri/src-tauri && cargo fmt`（pass）
- `cd kukuri-community-node && cargo fmt`（pass）
- `cd kukuri-tauri/src-tauri && cargo clippy --all-targets --all-features --message-format=short`（pass / warningは既存負債中心に継続）
- `DOCKER_CONFIG=/tmp/docker-config docker run ... -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "cargo clippy --workspace --all-targets --all-features --message-format=short"`（pass / warningは主に `await_holding_lock` と `too_many_arguments` が継続）
- `cd kukuri-tauri/src-tauri && cargo test`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run ... -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

ログ:

- `tmp/logs/gh-act-format-check-issue47.log`
- `tmp/logs/gh-act-native-test-linux-issue47.log`
- `tmp/logs/gh-act-community-node-tests-issue47.log`
