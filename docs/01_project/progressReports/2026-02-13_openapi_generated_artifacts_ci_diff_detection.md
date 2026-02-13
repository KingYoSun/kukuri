# OpenAPI 生成物 CI 差分検知（2026年02月13日）

最終更新日: 2026年02月13日

## 概要

`api_server_stack.md` の要件に合わせて、OpenAPI 生成物の更新漏れを検知する CI ジョブを `.github/workflows/test.yml` に追加した。  
対象は `kukuri-community-node/apps/admin-console/openapi/*.json` と `kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`。

## 実装内容

- `test.yml` に `openapi-artifacts-check` ジョブを追加
  - Rust 1.88.0 / Node 20 / pnpm 10.16.1 をセットアップ
  - `cargo run --locked -p cn-cli -- openapi export ...` で `user-api.json` / `admin-api.json` を再生成
  - `pnpm generate:api` で `src/generated/admin-api.ts` を再生成
  - `git diff --exit-code -- kukuri-community-node/apps/admin-console/openapi/*.json kukuri-community-node/apps/admin-console/src/generated/admin-api.ts` で差分検知
- `pr-required-checks` の依存に `openapi-artifacts-check` を追加（`docs_only` で skip の場合は許容）
- 実装との差分があったため、生成物を更新
  - `kukuri-community-node/apps/admin-console/openapi/admin-api.json`
  - `kukuri-community-node/apps/admin-console/src/generated/admin-api.ts`
- タスク管理更新
  - `community_nodes_roadmap.md` の該当未実装項目を `[x]` に更新

## 検証

- `gh act --workflows .github/workflows/test.yml --job openapi-artifacts-check`（失敗）
  - ログ: `tmp/logs/gh-act-openapi-artifacts-check-20260213-104423.log`
  - ログ: `tmp/logs/gh-act-openapi-artifacts-check-20260213-104939.log`
  - 理由: `act` 実行時の `actions/checkout` はローカル未コミット差分を反映しないため、HEAD 基準で更新漏れを検知して失敗（ジョブ意図どおり）
- 生成コマンド再実行後のハッシュ一致を確認（再生成の再現性確認）
  - `admin-api.json` / `admin-api.ts` とも再実行前後で SHA-256 が一致
- セッション必須の `gh act` 3ジョブ
  - `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
    - ログ: `tmp/logs/gh-act-format-check-openapi-ci-20260213-105407.log`
  - `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
    - ログ: `tmp/logs/gh-act-native-test-linux-openapi-ci-20260213-105531.log`
  - `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
    - ログ: `tmp/logs/gh-act-community-node-tests-openapi-ci-20260213-110222.log`
