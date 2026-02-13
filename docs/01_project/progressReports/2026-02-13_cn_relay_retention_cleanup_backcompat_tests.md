# Community Nodes 進捗レポート（`cn-relay` retention クリーンアップ統合テスト）
作業日: 2026年02月13日

## 対象
`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目:
- `cn-relay`: `retention` クリーンアップ（`events/event_topics`、`event_dedupe`、`events_outbox`、`deletion_tombstones`）の統合テストを追加し、保持期間ポリシーの後方互換を固定する。

## 実施内容
1. `cn-relay` 統合テストを追加
- `cleanup_cleans_all_retention_targets_and_preserves_policy_backcompat` を追加。
- 以下の4系統を1テストで同時検証:
  - `events/event_topics`（non-current 旧イベントの削除）
  - `event_dedupe`（`last_seen_at` ベースの削除）
  - `events_outbox`（`ingested_at` ベースの削除）
  - `deletion_tombstones`（`requested_at` ベースの削除）

2. 保持期間ポリシーの後方互換を固定
- topic ingest policy で `retention_days: 0` の場合はトピック個別上書きとして扱わず、グローバル `events_days` にフォールバックする挙動を検証。
- あわせて `retention_days > 0` のトピック上書きが維持されることも検証。

3. ロードマップ更新
- `community_nodes_roadmap.md` の該当タスクを `[x]` へ更新。

## 変更ファイル
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/progressReports/2026-02-13_cn_relay_retention_cleanup_backcompat_tests.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-13.md`

## 検証
- `./scripts/test-docker.ps1 rust`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-relay cleanup_cleans_all_retention_targets_and_preserves_policy_backcompat -- --nocapture --test-threads=1"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job format-check`
  - ログ: `tmp/logs/gh-act-format-check-cn-relay-retention-20260213-2.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-retention-20260213-1.log`
- `$env:NPM_CONFIG_PREFIX='/tmp/npm-global'; gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-retention-20260213-1.log`
