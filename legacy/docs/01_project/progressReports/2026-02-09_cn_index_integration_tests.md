# Community Nodes `cn-index` 統合テスト追加（outbox/期限切れ削除/reindex_jobs）

作成日: 2026年02月09日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-index` の統合テストを追加し、outbox `upsert/delete`・期限切れ削除・`reindex_jobs` の状態遷移（pending/running/succeeded/failed）までを Meilisearch 反映込みで検証する

を実装し、完了状態に更新した。

## 実装内容

- `cn-index` に統合テストモジュール `integration_tests.rs` を追加
  - DB マイグレーション実行 + 一意ID生成 + テスト排他ロックの共通ヘルパーを追加
  - Meilisearch API（`/indexes/*`、`/documents/*`）をテスト用 `axum` モックで再現し、反映結果をインメモリで検証可能にした
- outbox/期限切れ削除テストを追加
  - `outbox upsert` でドキュメント作成
  - `outbox delete` でドキュメント削除
  - `expire_events_once` 実行でドキュメント削除 + `cn_index.expired_events` 記録を確認
- reindex ジョブ遷移テストを追加
  - `pending -> running -> succeeded` の遷移、`total_events`/`processed_events` の更新、Meilisearch の再索引結果を確認
  - 強制エラー Meili モックを使って `pending -> running -> failed` の遷移と `error_message` 記録を確認
- `cn-index/src/lib.rs` に `#[cfg(test)] mod integration_tests;` を追加

## 変更ファイル

- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-09.md`

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo run -p cn-cli -- migrate"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-index -- --nocapture"`（成功: 5 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-20260209-151318.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260209-151318.log`

## 補足

- `cn-index` テストで Meilisearch 実体依存を増やさないため、HTTP インターフェース互換のモックサーバを使って反映内容を厳密検証している。
