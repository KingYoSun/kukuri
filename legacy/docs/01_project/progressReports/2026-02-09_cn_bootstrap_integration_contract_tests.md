# 2026年02月09日 `cn-bootstrap` 統合/契約テスト追加

## 概要

- `cn-bootstrap` の未実装/不足事項だった統合/契約テストを追加し、以下を自動検証可能にした。
  - `refresh_bootstrap_events` の DB 反映
  - `topic_services` 0件時の stale `39001` 削除
  - `/healthz` `/metrics` のレスポンス shape と依存異常時ステータス遷移

## 実装内容

- `kukuri-community-node/crates/cn-bootstrap/src/lib.rs`
  - DB/マイグレーション初期化ヘルパーを追加し、`cn-bootstrap` 内で統合テストを実行可能化。
  - 追加テスト:
    - `refresh_bootstrap_events_reflects_topic_services_into_db`
    - `refresh_bootstrap_events_deletes_stale_39001_when_topic_services_empty`
    - `healthz_contract_status_transitions_when_dependency_fails`
    - `metrics_contract_prometheus_content_type_shape_compatible`
  - 依存ヘルス用の簡易HTTPモック起動ヘルパーを追加し、`200 -> 503` の遷移を1テストで確認。
- `kukuri-community-node/crates/cn-bootstrap/Cargo.toml`
  - テスト用に `uuid` を `dev-dependencies` へ追加。

## 検証

- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`  
  - 成功ログ: `tmp/logs/gh-act-community-node-bootstrap-tests-20260209-225056.log`
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`  
  - 成功ログ: `tmp/logs/gh-act-format-check-cn-bootstrap-tests-20260209-225413.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`  
  - 成功ログ: `tmp/logs/gh-act-native-test-linux-cn-bootstrap-tests-20260209-225531.log`

## 結果

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当項目を `[x]` に更新。
