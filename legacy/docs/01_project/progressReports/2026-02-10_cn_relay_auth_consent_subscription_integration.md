# `cn-relay` 認証必須モードの consent/subscription 強制テスト追加

作成日: 2026年02月10日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目だった、`cn-relay` の認証必須モードにおける consent/subscription 強制の統合テストを追加した。

## 実装内容

- 変更ファイル: `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- 追加テスト:
  - `auth_required_enforces_consent_and_subscription`
- 追加ヘルパー:
  - `ensure_required_policies`
  - `ensure_consents`
  - `insert_topic_subscription`
  - `wait_for_auth_challenge`
  - `send_auth`
- 検証シナリオ:
  - AUTH 成功 + 未同意 -> `consent-required`
  - 同意済み未購読 -> `restricted: subscription required`
  - 同意済み購読済み -> 受理（`OK` true）

## ドキュメント更新

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 該当未実装項目を `[x]` に更新

## 検証結果

- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "${PWD}:/work" -w /work/kukuri-community-node rust:1.88-bookworm bash -lc '... cargo test --locked -p cn-relay auth_required_enforces_consent_and_subscription -- --nocapture'`（成功: 1 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-relay-consent-20260210-150451.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-consent-20260210-150606.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-consent-20260210-151410.log`

## 備考

- `gh act` 実行時に `some refs were not updated` が出るが、ジョブ本体はすべて成功。
