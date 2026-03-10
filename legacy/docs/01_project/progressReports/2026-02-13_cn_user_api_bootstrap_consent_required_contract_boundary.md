# 2026年02月13日 `cn-user-api` bootstrap `428 CONSENT_REQUIRED` 契約境界追加

## 概要

`docs/03_implementation/community_nodes/auth_transition_design.md` と `docs/03_implementation/community_nodes/user_api.md` の要件に合わせ、bootstrap 認証必須モードで「認証済みだが未同意」のとき `428 CONSENT_REQUIRED` を返す契約を `cn-user-api` テストで固定した。

## 実施内容

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs` の `api_contract_tests` に以下を追加。
  - `bootstrap_nodes_contract_requires_consent_when_authenticated`
  - `bootstrap_services_contract_requires_consent_when_authenticated`
- どちらのテストも以下を検証。
  - `StatusCode::PRECONDITION_REQUIRED (428)`
  - `payload.code == "CONSENT_REQUIRED"`
  - `details.required` が空でない（同意不足の詳細を返す）
  - `WWW-Authenticate` ヘッダが付与されない（既存の `401 + WWW-Authenticate` 契約との境界固定）
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当項目を `[x]` に更新。

## 技術的ポイント

- `test_state_with_bootstrap_auth_required()` で bootstrap `auth.mode=required` を有効化。
- `insert_current_policy(... terms/privacy ...)` を使って current policy を明示投入し、「未同意」状態を再現。
- Bearer token は `issue_token` で発行し、未認証ケース（401）ではなく認証済み未同意ケース（428）を狙って検証。

## 検証

- `./scripts/test-docker.ps1 rust`（成功、終了コード `-1` は既知）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api bootstrap_nodes_contract_requires_consent_when_authenticated -- --nocapture --test-threads=1 && cargo test -p cn-user-api bootstrap_services_contract_requires_consent_when_authenticated -- --nocapture --test-threads=1"`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`（初回 rustfmt 差分で失敗→再実行成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知の `useRouter` 警告のみ）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）

## 反映ドキュメント

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 未実装項目の bootstrap `428 CONSENT_REQUIRED` 契約テストタスクを `[x]` 化。
