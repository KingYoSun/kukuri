# Community Node OpenAPI 契約統合（2026年02月06日）

最終更新日: 2026年02月06日

## 概要

Community Nodes の未実装項目だった OpenAPI 対応を完了した。`cn-user-api` / `cn-admin-api` で `utoipa` による spec 生成を実装し、`/v1/openapi.json` が実データを返すように変更。Admin Console は OpenAPI 由来の型/クライアント生成へ移行し、契約テストへ反映した。

## 実装内容

- `cn-user-api` / `cn-admin-api`:
  - `openapi.rs` を追加し、`utoipa::OpenApi` 定義を実装
  - `/v1/openapi.json` のレスポンスを実 spec 返却へ変更
  - schema 生成に必要な `ToSchema` derive を request/response 型へ追加
- 契約テスト:
  - `cn-admin-api`: `/v1/openapi.json` に主要管理 API path / schema が含まれることを検証
  - `cn-user-api`: `/v1/openapi.json` に主要ユーザー API path が含まれることを検証
- `cn-cli`:
  - `openapi export` サブコマンドを追加（`--service`/`--output`/`--pretty`）
- Admin Console:
  - `openapi-typescript` / `openapi-fetch` を導入
  - `generate:api` スクリプトを追加
  - 生成型（`src/generated/admin-api.ts`）と OpenAPI JSON（`openapi/*.json`）を反映
  - 既存 API クライアントを OpenAPI 由来型へ置換

## 検証

- `./scripts/test-docker.ps1 rust` 成功
- `./scripts/test-docker.ps1 ts` 成功
- OpenAPI 契約テスト:
  - `cargo test -p cn-admin-api openapi_contract_contains_admin_paths`
  - `cargo test -p cn-user-api openapi_contract_contains_user_paths`
  - `cargo test -p cn-cli`
- `gh act --workflows .github/workflows/test.yml --job format-check` 成功
  - ログ: `tmp/logs/gh-act-format-check-20260206-183630.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` 成功
  - ログ: `tmp/logs/gh-act-native-test-linux-20260206-182834.log`

## 備考

- `gh act` 実行時に `some refs were not updated` / `pnpm approve-builds` / `useRouter` 警告が出るが、いずれも既知でジョブは成功している。
