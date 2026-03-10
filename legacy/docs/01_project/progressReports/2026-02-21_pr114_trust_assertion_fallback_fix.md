# PR #114 trust assertion fallback 修正レポート

作成日: 2026年02月21日

## 概要

- 対象: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `load_assertion_by_id` が参照先 attestation 行欠損時に `Some({ event_json: null })` を返していたため、`resolve_trust_assertion` のフォールバックが止まる不具合を修正。
- 参照行がない場合は `None` を返すよう変更し、`load_latest_active_assertion` へ継続できるようにした。

## 実装詳細

- `load_assertion_by_id` の DB 取得後、`event_json` が `None` の場合は `Ok(None)` を返却するよう修正。
- 有効期限チェック（`assertion_exp <= now` の場合に `None`）は既存どおり維持。
- API 契約テストに以下を追加。
  - `trust_report_based_contract_falls_back_to_latest_assertion_when_attestation_missing`
  - `trust_communication_density_contract_falls_back_to_latest_assertion_when_attestation_missing`
  - `trust_report_based_contract_keeps_referenced_attestation_when_present`

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api trust_report_based_contract_falls_back_to_latest_assertion_when_attestation_missing -- --nocapture; cargo test -p cn-user-api trust_report_based_contract_keeps_referenced_attestation_when_present -- --nocapture; cargo test -p cn-user-api trust_communication_density_contract_falls_back_to_latest_assertion_when_attestation_missing -- --nocapture"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
