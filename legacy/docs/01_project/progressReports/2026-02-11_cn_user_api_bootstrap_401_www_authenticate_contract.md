# 2026-02-11 `cn-user-api` bootstrap `401 + WWW-Authenticate` 契約実装

## 概要

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未完了項目
  - `cn-user-api`: bootstrap 認証必須時の `401 + WWW-Authenticate` 契約
  - `/v1/bootstrap/nodes` と `/v1/bootstrap/topics/{topic_id}/services` の契約テストでヘッダ互換を固定
  - を実装完了した。

## 実装内容

- `kukuri-community-node/crates/cn-user-api/src/auth.rs`
  - `require_auth` の `AUTH_REQUIRED` エラー生成を共通化し、`WWW-Authenticate: Bearer realm="cn-user-api"` を付与するように変更。
  - これにより bootstrap 認証必須モードで未認証アクセス時に、HTTP 認証チャレンジヘッダを返す契約を満たす。

- `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs`
  - `api_contract_tests` を拡張し、`/v1/bootstrap/nodes` と `/v1/bootstrap/topics/{topic_id}/services` の `401` 応答に `WWW-Authenticate` が含まれることを検証。

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - bootstrap 認証必須設定のテスト state を追加。
  - 2つの bootstrap エンドポイントに対する契約テストを追加し、`401` 応答の `code=AUTH_REQUIRED` と `WWW-Authenticate` ヘッダ値を固定。

- タスク管理
  - `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当未実装項目を `[x]` へ更新。

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（初回は `cargo fmt` 差分で失敗、整形後の再実行で成功）
  - `tmp/logs/gh-act-format-check-20260211-115842.log`
  - `tmp/logs/gh-act-format-check-20260211-120021.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - `tmp/logs/gh-act-native-test-linux-20260211-120200.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - `tmp/logs/gh-act-community-node-tests-20260211-120854.log`
