# Community Node iroh bootstrap/relay 実装チェックリスト完了レポート

作成日: 2026年03月04日
最終更新日: 2026年03月04日

## 1. 概要

- `docs/01_project/activeContext/tasks/status/in_progress.md` の「実装チェックリスト（2026年03月04日 追加）」を Phase 0 から Phase 5 まで実装した。
- 実装後の検証として、Windows ルール準拠の Docker 経路テストおよび `gh act` 必須 3 ジョブを完走した。

## 2. 実施内容

### 2.1 Phase 0-3: relay/bootstrap/join 基盤

- `cn-iroh-relay` 雛形を追加し、compose と環境変数を整備。
  - `kukuri-community-node/docker-compose.yml`
  - `kukuri-community-node/.env.example`
  - `kukuri-community-node/docker/iroh-relay/{Dockerfile,entrypoint.sh}`
  - `kukuri-community-node/README.md`
- `kukuri-tauri` / `cn-relay` の iroh endpoint を custom relay 対応へ拡張し、未指定時は `RelayMode::Default` へフォールバック。
  - `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs`
  - `kukuri-community-node/crates/cn-relay/src/lib.rs`
- `/v1/p2p/info` / `/v1/bootstrap/nodes` 契約を `relay_urls` / `bootstrap_hints` 対応へ拡張し、互換フィールドを維持。
  - `kukuri-community-node/crates/cn-relay/src/lib.rs`
  - `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs`
  - `kukuri-community-node/crates/cn-bootstrap/src/lib.rs`
  - `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`
- `cn-relay` の topic join を seed 付き `subscribe` ベースへ更新し、`cn_bootstrap_hint` 受信時の再 join 制御とメトリクス追加を実施。
  - `kukuri-community-node/crates/cn-relay/src/gossip.rs`
  - `kukuri-community-node/crates/cn-relay/src/ingest.rs`
  - `kukuri-community-node/crates/cn-core/src/metrics.rs`

### 2.2 Phase 4-5: E2E 経路統一と運用

- E2E で bridge 経由の peer 直注入を禁止し、bootstrap API -> hint 反映 -> join 経路へ統一。
  - `kukuri-tauri/src/testing/registerE2EBridge.ts`
  - `kukuri-tauri/tests/e2e/helpers/bridge.ts`
  - `kukuri-tauri/tests/e2e/specs/community-node.multi-peer.spec.ts`
- 必須シナリオを追加。
  - `kukuri-tauri/tests/e2e/specs/community-node.ipv6-relay-fallback.spec.ts`
- canary/feature flag/rollback を Runbook に反映。
  - `docs/03_implementation/community_nodes/ops_runbook.md`

### 2.3 追加修正（検証時の不整合解消）

- `cn-bootstrap` の unstable 構文（`if ... && let ...`）を安定構文に修正。
  - `kukuri-community-node/crates/cn-bootstrap/src/lib.rs`
- `cn-admin-api` の runtime fallback 契約テストをデータ前提固定化。
  - `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- Rust 整形差分を Docker 内 `cargo fmt` で解消。
  - `kukuri-community-node/crates/cn-relay/src/{gossip.rs,lib.rs}`
  - `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs`

## 3. 検証結果

- `./scripts/test-docker.ps1 rust`: pass
- `./scripts/test-docker.ps1 ts`: pass
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`: pass
- `docker compose -f docker-compose.test.yml build test-runner`: pass
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "<repo>:/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`: pass

## 4. タスク管理反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 完了済み「実装チェックリスト（2026年03月04日 追加）」を削除。
- `docs/01_project/activeContext/tasks/completed/2026-03-04.md`
  - 完了内容と検証ログを追記。

## 5. まとめ

- iroh custom relay と gossip bootstrap peer を同時に扱うための API 契約・join 制御・E2E 経路・運用手順を実装し、チェックリスト要求を満たした。
- 追加で発見されたテスト前提差分とフォーマット差分も修正し、ローカル検証と `gh act` の必須ジョブまで green を確認した。
