# Community Nodes `cn-user-api` 認証・同意・課金メータ回帰テスト

作成日: 2026年02月11日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-user-api`: 認証/同意/課金メータの回帰テストを追加し、API 実行で `auth_success_total` / `auth_failure_total` / `consent_required_total` / `quota_exceeded_total` の増分を検証する

を実装し、完了状態へ更新した。

## 実装内容

- `cn-user-api` 契約テスト（`subscriptions.rs`）に以下を追加
  - `prometheus_counter_value` ヘルパー
    - Prometheus exposition text から、指定メトリクス名 + ラベル組み合わせのカウンタ値を抽出
  - `auth_consent_quota_metrics_regression_counters_increment`
    - `POST /v1/auth/challenge` + `POST /v1/auth/verify` 成功で `auth_success_total` 増分を確認
    - 不正 kind の `POST /v1/auth/verify` 失敗で `auth_failure_total` 増分を確認
    - 同意未取得状態で `POST /v1/topic-subscription-requests` を実行し `consent_required_total` 増分を確認
    - `max_topics` 上限超過の `POST /v1/topic-subscription-requests` を実行し `quota_exceeded_total{metric="max_topics"}` 増分を確認
    - いずれも `/metrics` の実測値を before/after 比較（`after >= before + 1`）で検証

- タスク更新
  - `community_nodes_roadmap.md` の該当項目を `[x]` に更新
  - 完了記録を `tasks/completed/2026-02-11.md` に追記

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-11.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker build -t kukuri-postgres-age ./kukuri-community-node/docker/postgres-age`（成功）
- `docker run --rm -v "${PWD}:/app" -w /app/kukuri-community-node -e DATABASE_URL=postgres://cn:cn_password@host.docker.internal:15432/cn rust:1.88-bookworm /usr/local/cargo/bin/cargo test -p cn-user-api auth_consent_quota_metrics_regression_counters_increment -- --nocapture --test-threads=1`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-user-api-metrics-20260211-222708.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-user-api-metrics-20260211-222859.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-user-api-metrics-20260211-223536.log`
