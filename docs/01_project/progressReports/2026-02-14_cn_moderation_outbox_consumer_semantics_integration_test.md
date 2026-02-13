# 2026年02月14日 `cn-moderation` outbox consumer セマンティクス統合テスト

## 概要

`cn-moderation` の outbox consumer について、`outbox_notify_semantics.md` で定義した v1 要件（起動時 catch-up / NOTIFY 起床 / offset commit / replay）を 1 本の統合テストで固定した。

## 実装内容

- `kukuri-community-node/crates/cn-moderation/src/lib.rs`
  - テストヘルパーを追加:
    - `insert_outbox_row(...)`
    - `cleanup_outbox_consumer_artifacts(...)`
  - 統合テスト `outbox_consumer_semantics_cover_catch_up_notify_commit_and_replay_idempotency` を追加し、以下を検証:
    - `load_last_seq` で offset を初期化できること
    - `fetch_outbox_batch` で起動時 catch-up を昇順取得できること
    - `commit_last_seq` で offset を更新し、未処理行が空になること
    - `connect_listener` + `wait_for_notify` で NOTIFY を受けて起床できること
    - offset 巻き戻し後の replay でも `enqueue_job` が重複ジョブを作らず、at-least-once 前提で冪等であること

## ドキュメント更新

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 該当タスクを `[x]` に更新
- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 完了に伴い着手メモを削除し、最終更新日を更新
- `docs/01_project/activeContext/tasks/completed/2026-02-14.md`
  - 完了タスクと検証結果を追記

## 検証コマンド

- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-moderation outbox_consumer_semantics_cover_catch_up_notify_commit_and_replay_idempotency -- --nocapture --test-threads=1"`
- `./scripts/test-docker.ps1 rust`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test --workspace --all-features && cargo build --release -p cn-cli"`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`
  - `tmp/logs/gh-act-format-check-cn-moderation-outbox-20260214-final2.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - `tmp/logs/gh-act-native-test-linux-cn-moderation-outbox-20260214-final.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - `tmp/logs/gh-act-community-node-tests-cn-moderation-outbox-20260214-final.log`
