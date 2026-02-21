# 2026-02-21 Issue #111 PR-3 NIP-85 trust invalid-case テスト拡張

作成日: 2026年02月21日

## 概要

- 目的: NIP-85 trust 経路で無効データを安全に reject し、誤った trust score が採用されないことを統合レイヤー中心に固定する。
- 対象: `kukuri-tauri` の community-node trust 集約処理、`cn-user-api` の trust endpoint 契約。

## 実装内容

- `kukuri-tauri/src-tauri/tests/community_node_trust_invalid_integration.rs` を追加。
- 高優先 invalid-case を追加検証。
  - invalid subject（`<kind>:<value>` 形式不正）
  - invalid assertion kind（30382以外）
  - invalid claim tag（algorithm mismatch）
  - malformed assertion `event_json`（構造破損）
  - unexpected provider pubkey（設定providerと署名者不一致）
  - invalid source混在時に valid source のみ採用（false positive防止）
- テスト安定化のため、テスト内 `TestSecureStorage`（`SecureStorage` trait実装）を導入し、並列テスト間で secure storage state が干渉しないよう分離。
- `cn-user-api` の API contract tests に invalid subject 2ケースを追加。
  - `trust_report_based_contract_rejects_invalid_subject`
  - `trust_communication_density_contract_rejects_invalid_subject`
- 追加契約で `400 INVALID_SUBJECT` と `score/assertion` 不在を固定。

## 技術的ポイント

- 失敗すべき assertion が存在しても、trust集約結果の `sources` 配列と `score` が有効なノード由来のみに限定されることを明示的に検証。
- これにより invalid assertion が混入しても trust score の false positive が発生しない回帰防止を追加。

## 検証結果

- `cd kukuri-tauri/src-tauri && cargo fmt`（pass）
- `cd kukuri-tauri/src-tauri && cargo test --test community_node_trust_invalid_integration -- --nocapture`（pass）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（pass）
- `docker compose -f docker-compose.test.yml build test-runner`（pass）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api trust_report_based_contract_rejects_invalid_subject -- --nocapture; cargo test -p cn-user-api trust_communication_density_contract_rejects_invalid_subject -- --nocapture"`（pass）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo fmt --all -- --check"`（pass）
- `cd kukuri-tauri/src-tauri && cargo fmt -- --check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`（pass, `tmp/logs/gh-act-format-check-20260221-094906.log`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass, `tmp/logs/gh-act-native-test-linux-20260221-095446.log`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass, `tmp/logs/gh-act-community-node-tests-20260221-095820.log`）

## まとめ

- PR-3範囲として、NIP-85 trust invalid-case の安全拒否と false positive 抑止を統合/契約層で固定した。
- Tauri v2 E2E 制約を踏まえ、層別テスト方針に沿って高価値 invalid-case を優先実装した。
