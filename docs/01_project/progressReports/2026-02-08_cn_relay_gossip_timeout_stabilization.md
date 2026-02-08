# cn-relay gossip timeout 安定化（Community Node Tests）

作成日: 2026年02月08日

## 概要

GitHub Actions の `Community Node Tests` で断続的に失敗していた
`integration_tests::ingest_outbox_ws_gossip_integration` の `gossip timeout` を調査し、
接続確立待機のレースを解消した。

## 原因

- `setup_gossip` 内で `receiver_b.joined()` の結果を握りつぶしており、接続未確立のままテスト本体へ進行するケースがあった。
- 接続未確立時は WS 側の `broadcast_to_gossip` が送信できず、`wait_for_gossip_event` が timeout していた。

## 実装内容

- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `receiver_a.joined()` / `receiver_b.joined()` を `timeout + expect` で厳密待機に変更。
- `wait_for_gossip_event` の timeout メッセージに `expected_id` と `last_received_id` を追加し、再発時の原因調査を容易化。

## 検証結果

- `gh run view 21796109648 --log-failed` で CI 失敗ログを確認し、失敗点が `gossip timeout` であることを再確認。
- `gh act --workflows .github/workflows/test.yml --job community-node-tests` を 2 回連続実行し成功。
- `gh act --workflows .github/workflows/test.yml --job format-check` 成功。
- ログ: `tmp/logs/gh-act-format-check-20260208-200124.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` 成功。
- ログ: `tmp/logs/gh-act-native-test-linux-20260208-200241.log`

## 備考

- `gh act` 実行時に `some refs were not updated` と `pnpm approve-builds` 警告が表示されるが、既知事象でありジョブ本体は成功。
