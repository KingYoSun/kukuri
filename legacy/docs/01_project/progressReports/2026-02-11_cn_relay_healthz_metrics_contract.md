# Community Nodes `cn-relay` `/healthz` `/metrics` 契約固定と必須メトリクス互換固定

作成日: 2026年02月11日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `/healthz` `/metrics` の契約テストを追加し、Runbook 必須メトリクス（`ws_connections` / `ws_req_total` / `ws_event_total` / `ingest_received_total` / `ingest_rejected_total` / `gossip_received_total` / `gossip_sent_total` / `dedupe_hits_total` / `dedupe_misses_total`）の公開互換を固定する

を実装し、完了状態へ更新した。

## 実装内容

- `cn-relay` 統合テスト（`integration_tests.rs`）に `/healthz` 契約テストを追加
  - success: `200` + JSON `{ "status": "ok" }`
  - dependency unavailable: `503` + JSON `{ "status": "unavailable" }`

- `cn-relay` 統合テストに `/metrics` 契約テストを追加
  - content-type を `text/plain; version=0.0.4` で固定
  - 以下の Runbook 必須メトリクスを `service="cn-relay"` ラベル付きで公開することを検証
    - `ws_connections`
    - `ws_req_total`
    - `ws_event_total`
    - `ingest_received_total`（`source="contract"`）
    - `ingest_rejected_total`（`reason="contract"`）
    - `gossip_received_total`
    - `gossip_sent_total`
    - `dedupe_hits_total`
    - `dedupe_misses_total`

- タスク更新
  - `community_nodes_roadmap.md` の該当項目を `[x]` に更新
  - 完了記録を `tasks/completed/2026-02-11.md` に追記

## 変更ファイル

- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-11.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
  - ログ: `tmp/logs/test-docker-rust-cn-relay-metrics-20260211-191220.log`
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 初回/再実行: 失敗（`cn-relay` テストファイル整形差分）
    - `tmp/logs/gh-act-format-check-cn-relay-metrics-20260211-191324.log`
    - `tmp/logs/gh-act-format-check-cn-relay-metrics-20260211-191536.log`
  - 整形修正後: 成功
    - `tmp/logs/gh-act-format-check-cn-relay-metrics-20260211-191756.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-metrics-20260211-191917.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-metrics-20260211-192603.log`
