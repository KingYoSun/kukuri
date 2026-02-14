# Community Nodes 進捗レポート（bootstrap hint 受信→HTTP再取得）

作成日: 2026年02月14日

## 概要

`cn-bootstrap` の `pg_notify('cn_bootstrap_hint')` publish 経路に対し、受信側（`cn-user-api` + クライアント）を実装した。  
クライアントは hint を受信すると `/v1/bootstrap/*` キャッシュを stale 化し、同一呼び出し内で HTTP 再取得して最新化する。

## 実装内容

1. `cn-user-api` に bootstrap hint 受信経路を追加
- `PgListener` で `cn_bootstrap_hint` を購読し、最新 hint を in-memory で保持する `BootstrapHintStore` を追加
- `GET /v1/bootstrap/hints/latest?since=<seq>` を追加（更新なしは `204 No Content`、更新ありは `seq/received_at/hint` を返却）
- hint endpoint は bootstrap 認証・公開 rate limit と同じ境界で運用

2. `kukuri-tauri` `CommunityNodeHandler` に hint 起点の再取得トリガを追加
- bootstrap 取得前に `/v1/bootstrap/hints/latest` を問い合わせ
- hint payload（`kukuri-bootstrap-update-hint-v1`）を検証し、`/v1/bootstrap/nodes` と `/v1/bootstrap/topics/{topic_id}/services` の該当キャッシュを stale 化
- stale 化後は既存ロジックで即時 HTTP 再取得し、キャッシュを更新

3. テスト追加
- `kukuri-tauri/src-tauri` に unit test を追加し、hint 受信後に `list_bootstrap_services` が再取得され event id が更新されることを固定
- `cn-user-api` に契約テストを追加し、`/v1/bootstrap/hints/latest` の `200/204` 互換を固定
- `tests/e2e/specs/community-node.bootstrap.spec.ts` を拡張し、hint 受信後に `/v1/bootstrap/topics/{topic_id}/services` キャッシュが更新されることを実ノード経路で検証

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `./scripts/test-docker.ps1 e2e-community-node`（成功、`Spec Files: 17 passed, 17 total`）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
