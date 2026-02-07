# cn-user-api 契約テスト拡張（auth/policies/consents/topic-subscription/personal-data）

最終更新日: 2026年02月07日

## 概要

`cn-user-api` の契約テストを拡充し、未対応だった `/v1/auth/*` `/v1/policies/*` `/v1/consents*` `/v1/topic-subscription*` `/v1/personal-data-*` の成功系とレスポンス shape 互換を検証できる状態にした。

## 実装内容

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - `api_contract_tests` に以下の成功系契約テストを追加:
    - `auth_contract_challenge_verify_success_shape_compatible`
    - `policies_consents_contract_success_shape_compatible`
    - `topic_subscription_contract_success_shape_compatible`
    - `personal_data_export_contract_success_shape_compatible`
    - `personal_data_deletion_contract_success_shape_compatible`
  - 契約テスト用ヘルパーを追加:
    - `insert_current_policy`
    - `ensure_active_subscriber`
    - `post_json_public`
    - `delete_json`
  - `test_state_with_meili_url` で `export_dir` をテストごとにユニーク作成し、`create_export_request` の `500` を解消。
  - `personal_data_deletion` の `GET` は削除済みアカウントで `410` になる仕様のため、別アカウントに seed した deletion request で成功契約を検証。

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 監査追記の該当未実装項目を `[x]` に更新。

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -c "source /usr/local/cargo/env && cargo test -p cn-user-api --tests -- --nocapture"`（成功: 24 passed）
- `./scripts/test-docker.ps1 rust -NoBuild`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功。ログ: `tmp/logs/gh-act-format-check-20260207-231112.log`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。ログ: `tmp/logs/gh-act-native-test-linux-20260207-231231.log`）

## 備考

- `gh act` 実行時の `some refs were not updated` / `pnpm approve-builds` 警告、`native-test-linux` の `useRouter` 警告は既知で、ジョブ結果は成功。
