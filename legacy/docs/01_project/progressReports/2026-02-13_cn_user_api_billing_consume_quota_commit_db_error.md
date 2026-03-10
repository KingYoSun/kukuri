# 2026年02月13日 `cn-user-api` `billing::consume_quota` commit 失敗時の `DB_ERROR` 返却対応

## 概要

`docs/03_implementation/community_nodes/billing_usage_metering.md` の「メータリング/監査の整合」に合わせ、`billing::consume_quota` が commit 失敗を握り潰して成功/超過レスポンスを返してしまう経路を解消した。

## 実施内容

- `kukuri-community-node/crates/cn-user-api/src/billing.rs`
  - `consume_quota` の超過分岐（`outcome='rejected'` 記録後）で `tx.commit().await.ok()` を廃止し、commit 失敗を `ApiError(DB_ERROR, 500)` に変換して返すように修正。
  - `consume_quota` の成功分岐（`usage_counters_daily` + `usage_events(outcome='ok')` 記録後）も同様に commit 失敗を `DB_ERROR` で返すように修正。
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - commit 失敗注入用のテストヘルパーを追加。
    - `install_usage_events_commit_failure_trigger`
    - `remove_usage_events_commit_failure_trigger`
    - `usage_counter_daily_count`
  - 以下の回帰テストを追加。
    - `search_quota_commit_failure_on_success_path_returns_db_error_and_rolls_back`
    - `search_quota_commit_failure_on_quota_exceeded_path_returns_db_error_and_rolls_back`
  - 上記2テストで、commit 失敗時に `500 + code=DB_ERROR` を返し、`usage_events`/`usage_counters_daily` に副作用が残らないことを検証。
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 未実装/不足事項の該当項目（`billing::consume_quota` commit 失敗伝播）を `[x]` に更新。

## 検証

- `./scripts/test-docker.ps1 rust`
  - ログ: `tmp/logs/test-docker-rust-cn-user-api-billing-20260213-120259.log`
  - 備考: スクリプトの終了コードは `-1` だが、ログに `Rust tests passed!` を確認。
- `docker run --rm --network kukuri_community-node-network -v ${PWD}:/workspace ... cargo test -p cn-user-api search_quota_commit_failure_on_* -- --nocapture --test-threads=1`
  - ログ: `tmp/logs/cn-user-api-commit-failure-tests-20260213-120342.log`
  - 結果: 追加した2テストがともに `ok`。
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`
  - ログ: `tmp/logs/gh-act-format-check-cn-user-api-billing-20260213-120136.log`
  - 結果: `Job succeeded`。
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-user-api-billing-20260213-115127.log`
  - 結果: `Job succeeded`。
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-user-api-billing-20260213-115757.log`
  - 結果: `Job succeeded`。
