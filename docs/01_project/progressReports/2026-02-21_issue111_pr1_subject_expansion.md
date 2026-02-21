# 2026-02-21 Issue #111 PR-1 trust subject expansion

## 概要

- 目的: `cn-user-api` の trust endpoint で `pubkey` 以外の subject（`event` / `relay` / `topic` / `addressable`）を受理し、既存クライアント互換を崩さずにレスポンス返却できるようにする。
- スコープ: `kukuri-community-node/crates/cn-user-api` の parser/validation/response/OpenAPI/contract test、および `kukuri-tauri` の trust subject 検証互換。

## 実装内容

- `cn-user-api`:
  - `parse_trust_subject` を `ParsedTrustSubject` 化し、`pubkey/event/relay/topic/addressable` を canonical 化して受理。
  - `trust_report_based` / `trust_communication_density` を更新し、
    - pubkey subject は既存 score table を優先。
    - 非 pubkey subject は `cn_trust.attestations` の active assertion を fallback 取得。
    - pubkey でも score row 連携 assertion が失効/欠落時は attestation fallback を利用。
  - OpenAPI parameter description を拡張形式に更新。
  - contract test を追加し、event/addressable subject で API shape が維持されることを固定。
- OpenAPI artifact / docs:
  - `apps/admin-console/openapi/user-api.json` を同期。
  - `docs/03_implementation/community_nodes/user_api.md` に受理 subject 形式を追記。
- Tauri:
  - `community_node_handler` の trust subject parser を `addressable` 対応。
  - trust assertion 検証で canonical subject id を使うよう変更（pubkey/event 正規化含む）。
  - addressable subject の parser/集約テストを追加。

## 追加・更新テスト

- `cn-user-api`
  - `trust_subject_tests::parse_trust_subject_accepts_event_topic_relay_and_addressable`
  - `trust_subject_tests::parse_trust_subject_rejects_invalid_addressable_value`
  - `api_contract_tests::trust_report_based_contract_supports_event_subject`
  - `api_contract_tests::trust_communication_density_contract_supports_addressable_subject`
  - `openapi_contract_tests::openapi_contract_contains_user_paths`（trust path/subject description 検証強化）
- `kukuri-tauri`
  - `community_node_handler_tests::trust_report_based_accepts_addressable_subject`
  - `community_node_handler_tests::parse_trust_subject_accepts_addressable_subject`
  - `community_node_handler_tests::parse_trust_subject_rejects_invalid_addressable_subject`

## 検証結果

- `docker run ... cargo test -p cn-user-api -- --nocapture`: pass（23 passed）
- `docker run ... cargo test --workspace --all-features; cargo build --release -p cn-cli`: pass
- `gh act`:
  - `format-check`: pass（`tmp/logs/gh-act-format-check-20260221-062943.log`）
  - `native-test-linux`: pass（`tmp/logs/gh-act-native-test-linux-20260221-063013.log`）
  - `community-node-tests`: pass（`tmp/logs/gh-act-community-node-tests-20260221-063424.log`）

## 補足

- trust assertion kind（NIP-85）は既存の `30382-30385` 前提を維持し、`addressable` subject は現行アーキテクチャ上 `event` 系 assertion 検証として扱う。
