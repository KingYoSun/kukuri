# Issue #22 Task2: `cn-user-api` pending limit 契約テスト拡張

作成日: 2026年02月15日

## 概要

- 対象: `POST /v1/topic-subscription-requests`（`cn-user-api`）
- 目的: per-pubkey pending 同時保留数上限の契約カバレッジを、境界/遷移シナリオまで拡張する。
- スコープ外: node-level 同時取込 topic 上限（実装/回帰）は次PRで実施。

## 実施内容

- `subscriptions.rs` の API 契約テストへ pending-limit 関連のシナリオを追加。
  - `topic_subscription_pending_request_limit_contract_accepts_under_limit_requests`
  - `topic_subscription_pending_request_limit_contract_rejects_at_limit`
  - `topic_subscription_pending_request_limit_contract_allows_rerequest_after_approve_or_reject`
- 既存の重複 SQL を減らすため、テスト専用ヘルパーを追加。
  - pending 申請作成
  - 申請ステータス更新
  - pubkey ごとの pending 件数取得

## 契約観点で固定した仕様

- under-limit (`pending < limit`) では申請が `200` + `status=pending` で受理される。
- at-limit (`pending == limit`) では `429` かつ `code=PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED` を返す。
- 既存 pending を `approved` / `rejected` に更新して pending 件数を解放した後は、再申請が再び受理される。

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`

## 検証

- Community Node（Docker 経路）
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
  - `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
  - `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api topic_subscription_pending_request_limit_contract_ -- --nocapture --test-threads=1"`
  - 結果: `3 passed; 0 failed`
- `gh act`（AGENTS セッション完了要件）
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - `XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`（失敗）
    - 失敗理由（既知 flaky）: `cn-admin-api` 契約テスト `admin_mutations_fail_when_audit_log_write_fails` で `trigger "test_audit_failures_trigger" for relation "audit_logs" already exists`
  - ログ:
    - `tmp/logs/gh-act-format-check-issue22-pending-limit-contract-tests.log`
    - `tmp/logs/gh-act-native-test-linux-issue22-pending-limit-contract-tests.log`
    - `tmp/logs/gh-act-community-node-tests-issue22-pending-limit-contract-tests.log`

## 次アクション

- Issue #22 残タスクとして、node-level 同時取込 topic 上限の実装と回帰テストを 1タスク=1PR で継続する。
