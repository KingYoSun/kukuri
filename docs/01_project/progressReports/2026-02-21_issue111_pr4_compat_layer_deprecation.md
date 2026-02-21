# 2026-02-21 Issue #111 PR-4 互換レイヤー廃止（legacy trust path deprecation）

作成日: 2026年02月21日

## 概要

- 目的: Issue #111 の最終段として、legacy trust path の互換レイヤーを縮退し、実行時経路を NIP-85 assertion/provider list 前提へ整理する。
- 対象: `kukuri-community-node`（`cn-admin-api` / `cn-user-api`）、`kukuri-tauri`（`community_node_handler`）、関連ドキュメントと契約テスト。

## 実装内容

- `cn-admin-api`
  - `POST /v1/attestations` は trust job 作成エイリアスとして廃止。
  - 同パスは `410 Gone` + `DEPRECATED_PATH` を返す明示的 deprecation 境界へ変更。
  - 正規経路は `POST /v1/admin/trust/jobs` のみ。
- `cn-user-api`
  - `report_scores` / `communication_scores` が存在する場合、参照 `attestation_id` 欠損/失効時に「最新 assertion へフォールバック」しない仕様へ変更。
  - 上記ケースは `assertion: null` を返し、score/count は score table の値を返却。
  - score row が存在しない場合の最新 active assertion 参照（subject + claim）は維持。
- `kukuri-tauri`
  - `community_node_trust_anchor_v1`（legacy trust anchor, 39011 系）の読み取り互換を削除。
  - trust provider 移行は `community_node_trust_provider_v1` のみを許可。
  - `community_node_trust_anchor_v1` は読み取り対象外とし、検出時は削除のみ行う。

## 互換境界（最終）

- 維持する互換:
  - `community_node_trust_provider_v1` -> `community_node_trust_providers_v2` への移行
  - `cn-user-api` の score row 未存在時における assertion 直接参照
- 廃止した互換:
  - `POST /v1/attestations` の trust job 作成エイリアス
  - score row 参照 assertion 欠損時の latest assertion 自動フォールバック
  - `community_node_trust_anchor_v1`（39011）読み取り移行

## 追加・更新テスト

- `cn-admin-api`
  - `legacy_admin_path_aliases_contract_success_and_trust_attestations_deprecated`
- `cn-user-api`
  - `trust_report_based_contract_returns_null_when_referenced_attestation_missing`
  - `trust_communication_density_contract_returns_null_when_referenced_attestation_missing`
  - `trust_report_based_contract_prefers_latest_issued_at_when_score_row_is_missing`
- `kukuri-tauri`
  - `trust_provider_ignores_legacy_trust_anchor_without_provider_record`

## 検証結果

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（pass）
- `docker compose -f docker-compose.test.yml build test-runner`（pass）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-admin-api legacy_admin_path_aliases_contract_success_and_trust_attestations_deprecated -- --nocapture; cargo test -p cn-user-api trust_report_based_contract_returns_null_when_referenced_attestation_missing -- --nocapture; cargo test -p cn-user-api trust_report_based_contract_prefers_latest_issued_at_when_score_row_is_missing -- --nocapture; cargo test -p cn-user-api trust_report_based_contract_keeps_referenced_attestation_when_present -- --nocapture; cargo test -p cn-user-api trust_communication_density_contract_returns_null_when_referenced_attestation_missing -- --nocapture"`（pass）
- `cd kukuri-tauri/src-tauri && cargo test trust_provider_ -- --nocapture`（pass）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo fmt --all -- --check"`（pass）
- `cd kukuri-tauri/src-tauri && cargo fmt -- --check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`（pass, `tmp/logs/gh-act-format-check-20260221-101919.log`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass, `tmp/logs/gh-act-native-test-linux-20260221-101919.log`）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass, `tmp/logs/gh-act-community-node-tests-20260221-101919.log`）
