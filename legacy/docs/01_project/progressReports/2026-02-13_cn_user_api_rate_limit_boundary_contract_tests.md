# 2026年02月13日 `cn-user-api` 429 契約境界テスト追加

## 概要

`docs/03_implementation/community_nodes/rate_limit_design.md`（作成日: 2026年01月23日）で定義された 429 契約（`RATE_LIMITED` + `Retry-After`）に対し、`cn-user-api` の回帰テスト不足を補完した。

## 実施内容

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs` の `api_contract_tests` に以下を追加。
  - `auth_challenge_and_verify_rate_limit_boundary_contract`
    - `/v1/auth/challenge` 直後に `/v1/auth/verify` を叩いた境界で 429 契約を確認。
  - `bootstrap_nodes_and_services_rate_limit_boundary_contract`
    - `/v1/bootstrap/nodes` と `/v1/bootstrap/topics/{topic_id}/services` の境界で 429 契約を確認。
  - `protected_search_and_trending_rate_limit_boundary_contract`
    - protected API（`/v1/search` と `/v1/trending`）境界で 429 契約を確認。
- 429契約確認用の共通アサーションを追加。
  - `status=429`
  - `payload.code=RATE_LIMITED`
  - `Retry-After >= 1`
- レスポンスヘッダ検証のため、テストヘルパーを拡張。
  - public POST / auth付きGET のヘッダ取得ヘルパーを追加。

## 技術的詳細

- レート設定をテストごとに差し替えられるように、`test_state` 生成ヘルパーへ `user_config_json` 注入経路を追加。
- protected API 側は consent 条件で前段失敗しうるため、`PRECONDITION_REQUIRED` を再試行で吸収するヘルパーを追加し、rate limit 境界判定の安定性を確保。

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）

## 反映ドキュメント

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 未実装項目の `cn-user-api` 429 契約テスト補完タスクを `[x]` 化。
