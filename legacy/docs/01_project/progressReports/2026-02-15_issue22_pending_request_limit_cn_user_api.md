# Issue #22 Task1: `cn-user-api` pending 同時保留数上限実装

作成日: 2026年02月15日

## 概要

- 対象: `POST /v1/topic-subscription-requests`（`cn-user-api`）
- 目的: DoS 要件「申請の同時保留数上限（per pubkey）」を実装し、上限到達時の拒否契約を固定する。
- スコープ外: node-level 同時取込 topic 上限と、その広範なシナリオ行列テストは次PR（Issue #22 Task2以降）へ分離。

## 実施内容

- `create_subscription_request` に pending 件数判定を追加。
  - `cn_user.topic_subscription_requests` の `status='pending'` 件数を pubkey 単位で集計。
  - 競合時の過剰受理を抑えるため `pg_advisory_xact_lock(hashtext(pubkey))` で同一 pubkey の判定・挿入を直列化。
- 上限設定を `user-api` の service config から読取。
  - `config_json.subscription_request.max_pending_per_pubkey`
  - 未設定時デフォルト: `5`
- OpenAPI と契約テストへ 429 応答を反映。
- 最小回帰として「上限到達時に拒否し、追加行が挿入されない」契約テストを追加。

## 拒否契約（上限到達時）

- HTTP status: `429 Too Many Requests`
- `code`: `PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED`
- `details`:
  - `metric`: `topic_subscription_requests.pending`
  - `scope`: `pubkey`
  - `current`: 現在の pending 件数
  - `limit`: 設定上限

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `kukuri-community-node/crates/cn-user-api/src/lib.rs`
- `kukuri-community-node/crates/cn-user-api/src/openapi.rs`
- `kukuri-community-node/crates/cn-user-api/src/openapi_contract_tests.rs`
- `kukuri-community-node/apps/admin-console/openapi/user-api.json`
- `docs/03_implementation/community_nodes/topic_subscription_design.md`
- `docs/03_implementation/community_nodes/user_api.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`

## 検証

- Community Node（Docker 経路）:
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
  - `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
  - `docker run --rm --network kukuri_community-node-network ... cargo test --workspace --all-features; cargo build --release -p cn-cli`（成功）
- `gh act`:
  - `format-check` 成功
  - `native-test-linux` 成功
  - `community-node-tests` は初回 `tuple concurrently updated` で失敗、再実行で成功

## 次アクション

- Issue #22 Task2 で「同時保留数上限の広範シナリオ行列（approve/reject 後の再申請など）」を契約テストとして拡張する。
- node-level 同時取込 topic 上限の実装と回帰テストを別PRで実施する。
