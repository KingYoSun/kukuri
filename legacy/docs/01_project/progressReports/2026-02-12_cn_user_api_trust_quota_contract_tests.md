# Community Nodes / `cn-user-api` trust クォータ契約テスト追加

最終更新日: 2026年02月12日

## 概要

- `cn-user-api` の `trust.requests` クォータ要件に対して、以下 2 エンドポイントの `402 QUOTA_EXCEEDED` + `X-Request-Id` 冪等契約テストを追加した。
  - `GET /v1/trust/report-based`
  - `GET /v1/trust/communication-density`
- 既存の `search/trending/report` と同等に、同一 `X-Request-Id` 再送時の `reset_at` 不変と `usage_events` の重複計上なし（1件）を検証する。
- タスク管理上は `community_nodes_roadmap.md` の該当未完了項目を `[x]` へ更新した。

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 実施内容

- `subscriptions::api_contract_tests` に以下 2 テストを追加:
  - `trust_report_based_quota_contract_payment_required_with_request_id_idempotent`
  - `trust_communication_density_quota_contract_payment_required_with_request_id_idempotent`
- いずれも以下を検証:
  - 初回リクエストで `402` と `code=QUOTA_EXCEEDED`
  - `details.metric == trust.requests`
  - 同一 `X-Request-Id` で再送しても `details.reset_at` が不変
  - `cn_user.usage_events` の同一 `request_id` レコード件数が `1`

## 検証

- `./scripts/test-docker.ps1 rust -NoBuild`（成功）
- `docker compose -f docker-compose.test.yml run --rm -e DATABASE_URL=postgres://cn:cn_password@localhost:5432/cn -e MEILI_URL=http://localhost:7700 test-runner bash -lc "source /usr/local/cargo/env && cd /app/kukuri-community-node && cargo test --workspace --all-features && cargo build --release -p cn-cli"`（成功）
- `docker run --rm --network host -v ${PWD}:/workspace -v kukuri-cargo-registry:/usr/local/cargo/registry -v kukuri-cargo-git:/usr/local/cargo/git -v kukuri-cargo-target:/workspace/kukuri-community-node/target -w /workspace/kukuri-community-node -e DATABASE_URL=postgres://cn:cn_password@localhost:5432/cn -e MEILI_URL=http://localhost:7700 kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api -- --nocapture"`（成功、37 tests passed）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - `tmp/logs/gh-act-format-check-trust-quota-20260212-203149.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知の `useRouter` 警告のみ）
  - `tmp/logs/gh-act-native-test-linux-trust-quota-20260212-203311.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - `tmp/logs/gh-act-community-node-tests-trust-quota-20260212-204003.log`

