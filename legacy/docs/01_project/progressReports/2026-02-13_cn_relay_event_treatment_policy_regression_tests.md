# Community Nodes 進捗レポート（`cn-relay` `event_treatment_policy` 回帰テスト補完）

作成日: 2026年02月13日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `event_treatment_policy.md` の回帰テストを補完する（replaceable/addressable の `created_at` 同値タイブレーク、`kind=5` の `e/a` 削除、`expiration` 到来後の `delete` 通知、ephemeral 非永続化）。

を実装し、完了状態へ更新した。

## 実装内容

1. replaceable/addressable 同値タイブレークの回帰テストを追加
- `cn-relay` 統合テスト `replaceable_and_addressable_tiebreak_prefers_lexicographically_smaller_event_id` を追加
- `created_at` が同値の場合に `event.id` 辞書順で winner が確定することを `replaceable_current` / `addressable_current` と `is_current` で固定
- 固定時刻イベント生成用の `build_event_at(...)` helper を追加

2. `kind=5` の `e/a` 削除回帰テストを追加
- 統合テスト `kind5_deletion_applies_e_and_a_targets` を追加
- `e` 対象（通常イベント）と `a` 対象（addressable）双方が `is_deleted=true` になることを固定
- `events_outbox` の `op='delete'` / `reason='nip09'` と `addressable_current` の削除を固定

3. `expiration` 到来後 delete 通知の補完
- `retention::cleanup_once` を `pub(crate)` 化し、先頭で expiration sweep を実行するよう更新
- `expire_events` / `expire_events_batch` を追加し、期限到達イベントを soft delete + index cleanup + outbox delete (`reason='expiration'`) に反映
- 統合テスト `expiration_reaches_and_enqueues_delete_outbox_notification` で期限到達後の delete 反映を固定

4. ephemeral 非永続化の回帰テストを追加
- 統合テスト `ephemeral_event_is_not_persisted_but_is_delivered_in_realtime` を追加
- WS realtime と gossip には配信される一方で `events/event_topics/events_outbox/event_dedupe` に永続化されないことを固定

5. タスク管理更新
- `community_nodes_roadmap.md` の該当項目を `[x]` に更新
- `completed/2026-02-13.md` に完了記録を追記

## 変更ファイル

- `kukuri-community-node/crates/cn-relay/src/retention.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-13.md`
- `docs/01_project/progressReports/2026-02-13_cn_relay_event_treatment_policy_regression_tests.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
  - ログ: `tmp/logs/test-docker-rust-cn-relay-event-policy-transcript-20260213-171937.log`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（再実行で成功）
  - 初回ログ: `tmp/logs/community-node-rust-workspace-event-policy-20260213-165129.log`
  - 備考: 既存 DB データ残留により `cn_relay.events` の重複キー衝突（既知）
  - DB 初期化: `docker exec kukuri-community-node-postgres psql -U cn -d cn -c 'DO $$ DECLARE r RECORD; BEGIN FOR r IN (SELECT schemaname, tablename FROM pg_tables WHERE schemaname IN (''cn_admin'',''cn_bootstrap'',''cn_index'',''cn_moderation'',''cn_relay'',''cn_trust'',''cn_user'')) LOOP EXECUTE format(''TRUNCATE TABLE %I.%I RESTART IDENTITY CASCADE'', r.schemaname, r.tablename); END LOOP; END $$;'`（成功）
  - 再実行ログ: `tmp/logs/community-node-rust-workspace-event-policy-rerun-20260213-165737.log`
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-relay-event-policy-final-20260213-170709.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功、既知の `useRouter` 警告のみ）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-event-policy-final-20260213-170830.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-event-policy-final-20260213-171459.log`
