# 2026年02月09日 `cn-bootstrap` 39001 クリーンアップ補完

## 概要

- `cn_admin.topic_services` が 0 件のときに stale な `cn_bootstrap.events(kind=39001)` が残る問題を修正。
- `cn_admin_config` の `pg_notify` を `cn-bootstrap` が `LISTEN` し、`bootstrap` 設定更新時に即時リフレッシュされるように変更。

## 実装内容

- `kukuri-community-node/crates/cn-bootstrap/src/lib.rs`
  - 39001 クリーンアップ処理を関数化し、active tag が空の場合は `kind = 39001` を全削除するよう変更。
  - refresh ループに `PgListener` を追加し、`cn_admin_config` 通知で `bootstrap` 関連 payload を受信した場合に即時で `refresh_bootstrap_events` を実行。
  - 通知経路断時の再接続処理（リトライ）を追加。
  - 通知判定とクリーンアップ分岐のユニットテストを追加。

## 検証

- `cargo test -p cn-bootstrap`（5 passed）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`  
  - ログ: `tmp/logs/gh-act-community-node-39001-fix-20260209-211926.log`
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`  
  - ログ: `tmp/logs/gh-act-format-check-39001-fix-20260209-212253.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`  
  - ログ: `tmp/logs/gh-act-native-test-linux-39001-fix-20260209-212408.log`

## 結果

- `community_nodes_roadmap.md` の該当未実装項目を完了（[x]）に更新。
