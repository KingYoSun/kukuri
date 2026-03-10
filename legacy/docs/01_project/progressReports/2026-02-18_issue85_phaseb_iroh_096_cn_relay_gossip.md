# Issue #85 Phase B: `iroh` / `iroh-gossip` 0.96 移行と `cn-relay` gossip 統合テスト修正

作成日: 2026年02月18日  
Issue: https://github.com/KingYoSun/kukuri/issues/85

## 実施概要

Issue #85 の Phase B として、`kukuri-tauri` と `kukuri-community-node` の iroh 系依存を 0.96 系へ更新し、API 移行を実施した。あわせて `cn-relay` gossip 統合テストの失敗要因を解消し、統合テストを green 化した。

## 依存更新

- `kukuri-tauri/src-tauri/Cargo.toml`
  - `iroh = "0.96.1"`
  - `iroh-gossip = "0.96.0"`
  - feature 名を 0.96 系へ更新:
    - `discovery-local-network` -> `address-lookup-mdns`
    - `discovery-pkarr-dht` -> `address-lookup-pkarr-dht`
- `kukuri-community-node/Cargo.toml`
  - `iroh = "0.96.1"`
  - `iroh-gossip = "0.96.0"`
  - feature 名を同様に更新
- lockfile
  - `kukuri-tauri/src-tauri/Cargo.lock`
  - `kukuri-community-node/Cargo.lock`

## API 移行対応

- `builder.discovery(...)` を `builder.address_lookup(...)` へ移行。
- `StaticProvider` を `MemoryLookup` へ置換し、`Endpoint` 初期化を `Endpoint::empty_builder(RelayMode::Default)` ベースへ統一。
- `bind_addr_v4` / `bind_addr_v6` を `bind_addr`（`Result`）へ置換し、エラーハンドリングを追加。
- `cn-cli` 側の discovery 適用関数を address lookup ベースへ更新。

## `cn-relay` gossip 統合テスト修正

- `crates/cn-relay/src/integration_tests.rs` の gossip 初期化を 0.96 推奨パターンへ変更。
- 直接 `connect` / `join_peers` 依存を外し、`MemoryLookup` と `gossip.subscribe(topic, vec![peer_id])` ベースで mesh を確立。
- 双方 `endpoint.addr()` を lookup に投入し、`router.endpoint().address_lookup().add(...)` で同一 lookup を利用。
- これにより以下の失敗していた統合ケースを含めて pass を確認:
  - `integration_tests::ingest_outbox_ws_gossip_integration`
  - `integration_tests::ephemeral_event_is_not_persisted_but_is_delivered_in_realtime`
  - `integration_tests::bootstrap_hint_notify_bridges_bootstrap_events_to_gossip`
  - `integration_tests::access_control_events_are_rejected_and_not_distributed`

## 検証

- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test --locked --workspace --all-features`（pass）
- `cd kukuri && docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `cd kukuri && docker compose -f docker-compose.test.yml build test-runner`（pass）
- `cd kukuri && docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh_act_format_check_issue85_phaseb_20260218.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh_act_native_test_linux_issue85_phaseb_20260218.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh_act_community_node_tests_issue85_phaseb_20260218.log`（初回 fail: `cn-user-api` の契約テスト 1 件が 428/402 不一致）
- `cd kukuri && docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api --lib subscriptions::api_contract_tests::auth_consent_quota_metrics_regression_counters_increment -- --exact --nocapture"`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh_act_community_node_tests_issue85_phaseb_20260218_retry.log`（retry pass）

## 変更ファイル

- `kukuri-tauri/src-tauri/Cargo.toml`
- `kukuri-tauri/src-tauri/Cargo.lock`
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/discovery_options.rs`
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs`
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_gossip_service.rs`
- `kukuri-tauri/src-tauri/src/application/shared/tests/p2p/bootstrap.rs`
- `kukuri-tauri/src-tauri/src/domain/p2p/tests/gossip_tests.rs`
- `kukuri-community-node/Cargo.toml`
- `kukuri-community-node/Cargo.lock`
- `kukuri-community-node/crates/cn-cli/src/main.rs`
- `kukuri-community-node/crates/cn-relay/src/gossip.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/completed/2026-02-18.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-18_issue85_phaseb_iroh_096_cn_relay_gossip.md`
