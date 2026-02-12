# `cn-admin-api` + Admin Console の `service_configs` 秘匿キー保存防止

作成日: 2026年02月12日

## 概要

`service_configs` に `OPENAI_API_KEY` などの秘匿キーを保存しない要件を実装で強制した。`cn-admin-api` では `PUT /v1/admin/services/{service}/config` で secret-like key を reject し、Admin Console でも保存前に同等チェックでブロックするようにした。あわせて契約テストと UI テストを追加し、後方互換を固定した。

## 変更内容

1. `cn-admin-api` の secret-like key reject
- 対象: `kukuri-community-node/crates/cn-admin-api/src/services.rs`
- `config_json` を再帰走査し、secret-like key（`OPENAI_API_KEY`, `clientSecret`, `password` など）を JSON Pointer で検出するロジックを追加。
- 検出時は `400 BAD_REQUEST`、`code=SECRET_CONFIG_FORBIDDEN` を返却し、DB 更新前に処理を中断。
- 単体テストを追加し、検出ケースと許可ケース（`max_tokens` など）を固定。

2. OpenAPI 契約の更新
- 対象: `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
- `PUT /v1/admin/services/{service}/config` に `400 ErrorResponse` を明示し、契約上も reject パスを公開。

3. 契約テスト追加
- 対象: `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `services_update_contract_rejects_secret_keys_and_preserves_storage` を追加。
- 検証内容:
  - secret-like key を含む更新が `400 + SECRET_CONFIG_FORBIDDEN` になること
  - `cn_admin.service_configs` に保存されないこと
  - `audit_logs` が記録されないこと
- OpenAPI 契約テストにも `responses/400` の確認を追加。

4. Admin Console の保存前バリデーション
- 対象: `kukuri-community-node/apps/admin-console/src/pages/ServicesPage.tsx`
- 保存時に `config_json` を解析し、secret-like key を検出した場合は API 呼び出し前にエラー表示して中断。

5. Admin Console UI テスト追加
- 対象: `kukuri-community-node/apps/admin-console/src/pages/ServicesPage.test.tsx`
- `OPENAI_API_KEY` を含む入力で Save したときに:
  - エラーが表示されること
  - `api.updateServiceConfig` が呼ばれないこと
  を検証。

## 検証結果

- `./scripts/test-docker.ps1 ts`: 成功
- `./scripts/test-docker.ps1 rust`: 成功
- `docker compose -f docker-compose.test.yml run --rm -v ${PWD}:/workspace -w /workspace/kukuri-community-node/apps/admin-console -e CI=true node:20-bookworm bash -lc "corepack enable && pnpm install --frozen-lockfile && pnpm vitest run src/pages/ServicesPage.test.tsx"`: 成功
- `docker compose -f docker-compose.test.yml run --rm -v ${PWD}:/workspace -w /workspace/kukuri-community-node rust-test /bin/sh -c "cd /workspace/kukuri-community-node && /usr/local/cargo/bin/cargo test --locked -p cn-admin-api -- --nocapture"`: 成功
- `gh act --workflows .github/workflows/test.yml --job format-check`: 成功
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: 成功
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: 成功

## 後方互換の固定ポイント

- API: `PUT /v1/admin/services/{service}/config` は secret-like key を許可しない（`400 + SECRET_CONFIG_FORBIDDEN`）
- 永続化: reject 時に `service_configs` へ副作用を残さない
- UI: Admin Console は保存前に同じ制約を適用し、危険な設定を送信しない
