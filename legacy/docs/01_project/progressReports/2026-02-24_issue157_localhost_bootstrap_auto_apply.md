# Issue #157 localhost Bootstrap 自動適用修正レポート

作成日: 2026年02月24日

## 概要

- 対象:
  - `kukuri-tauri/src-tauri/src/infrastructure/p2p/bootstrap_config.rs`
  - `kukuri-tauri/src-tauri/src/infrastructure/p2p/utils.rs`
  - `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs`
  - `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`
- Community Node の base URL が `localhost` の場合でも、bootstrap ノード登録を `127.0.0.1` と同等に扱い、自動適用されるよう修正した。
- Tauri クライアント側で `node_id@host:port` の hostname 解決を共通化し、peer 追加経路の解析不整合を解消した。

## 実装詳細

- `bootstrap_config.rs`
  - `resolve_socket_addr` を追加し、`localhost` 正規化と `ToSocketAddrs` による hostname 解決を実装。
  - bootstrap descriptor / 設定値読み込み / バリデーション / sanitize の全経路で `resolve_socket_addr` を利用。
  - loopback 判定を `localhost`/`127.0.0.1` 混在に強い挙動へ統一。

- `utils.rs`
  - `parse_node_addr` 内の socket 解析を `resolve_socket_addr` 化。
  - `node_id@localhost:port` を正しく loopback 解決できるよう修正。

- `iroh_network_service.rs`
  - `add_peer` の個別 parse 実装を削除し、`utils::parse_node_addr` に統一。
  - 解析ロジックの重複を解消し、hostname サポートを一貫適用。

- `community_node_handler.rs`
  - `set_config_treats_localhost_bootstrap_nodes_as_loopback` を追加。
  - `localhost` 由来 descriptor が保存時に `127.0.0.1` へ正規化され、重複が増えないことを検証。

## 実行コマンド

- `cd kukuri-tauri/src-tauri && cargo test set_config_treats_localhost_bootstrap_nodes_as_loopback -- --nocapture`
- `cd kukuri-tauri/src-tauri && cargo test test_parse_node_addr_localhost -- --nocapture`
- `cd kukuri-tauri/src-tauri && cargo test sanitize_bootstrap_nodes_rewrites_unspecified_addresses -- --nocapture`
- `cd kukuri-tauri/src-tauri && cargo test`
- `cd kukuri-tauri/src-tauri && cargo clippy -- -D warnings`
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
