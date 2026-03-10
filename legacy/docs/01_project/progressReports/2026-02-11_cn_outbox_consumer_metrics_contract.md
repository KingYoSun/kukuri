# Community Nodes `cn-index` / `cn-moderation` / `cn-trust` outbox consumer メトリクス契約固定

作成日: 2026年02月11日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- Runbook 必須メトリクス（outbox consumer別エラー率/処理レイテンシ/batch size）を `cn-index` / `cn-moderation` / `cn-trust` に追加し、`/metrics` 契約テストでメトリクス名とラベル互換を固定する

を実装し、完了状態へ更新した。

## 実装内容

- `cn-core` メトリクス基盤を拡張
  - `outbox_consumer_batches_total{service,consumer,result}`
  - `outbox_consumer_processing_duration_seconds{service,consumer,result}`
  - `outbox_consumer_batch_size{service,consumer}`
  - result ラベル値を共通化する定数（`success` / `error`）を追加

- outbox consumer 計測を 3サービスへ追加
  - 対象: `cn-index` / `cn-moderation` / `cn-trust`
  - 非空 batch の処理時に batch size histogram を記録
  - batch の成功/失敗を `outbox_consumer_batches_total` へ記録
  - batch 処理時間（成功/失敗）を `outbox_consumer_processing_duration_seconds` へ記録
  - fetch エラー・処理エラー・offset commit エラーを `result="error"` として記録

- `/metrics` 契約テストを拡張
  - `cn-index` / `cn-moderation` / `cn-trust` で新メトリクスを事前に記録し、`metrics_endpoint` 応答の body に対してメトリクス名とラベル組み合わせを検証
  - Prometheus 出力のラベル順（`consumer` 先頭）に合わせて検証文字列を固定

- Runbook/タスク更新
  - `docs/03_implementation/community_nodes/ops_runbook.md` の outbox 必須メトリクス節に実メトリクス名を追記
  - `community_nodes_roadmap.md` の該当未実装項目を `[x]` に更新

## 変更ファイル

- `kukuri-community-node/crates/cn-core/src/metrics.rs`
- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-moderation/src/lib.rs`
- `kukuri-community-node/crates/cn-trust/src/lib.rs`
- `kukuri-community-node/crates/cn-trust/src/integration_tests.rs`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-11.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 初回: 失敗（`cargo fmt` 差分）
    - ログ: `tmp/logs/gh-act-format-check-20260211-122304.log`
  - 修正後: 成功
    - ログ: `tmp/logs/gh-act-format-check-20260211-122443.log`
  - 最終確認: 成功
    - ログ: `tmp/logs/gh-act-format-check-20260211-124219.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260211-122600.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - 初回: 失敗（`cn-index` の契約テストでラベル順期待値不一致）
    - ログ: `tmp/logs/gh-act-community-node-tests-20260211-123255.log`
  - 修正後: 成功
    - ログ: `tmp/logs/gh-act-community-node-tests-20260211-123647.log`
