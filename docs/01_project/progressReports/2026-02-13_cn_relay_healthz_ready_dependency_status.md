# Community Nodes 進捗レポート（`cn-relay` `/healthz` ready 依存判定拡張）

作成日: 2026年02月13日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `ops_runbook.md` の `/healthz` ready 要件に合わせ、DB到達性だけでなく relay 依存（gossip 参加状態・topic購読同期状態）の劣化を `degraded/unavailable` として返せるようにする。あわせて `/healthz` 契約テストを拡張する。

を実装し、完了状態へ更新した。

## 実装内容

1. `cn-relay` `/healthz` 判定の3段階化
- `db::check_ready` に加え、relay 依存を評価する `ReadyStatus`（`ok` / `degraded` / `unavailable`）を導入
- gossip 参加状態は `gossip_senders` と DB 上の有効 topic 集合の一致度で判定
- topic購読同期状態は DB 有効 topic / `node_topics` / `gossip_senders` の整合で判定
- `degraded` / `unavailable` は ready 不成立として `503` を返すように変更

2. `/healthz` 契約テスト拡張
- 既存の success/unavailable（DB障害）に加え、以下を追加
  - gossip 未参加時: `status = unavailable`
  - topic 同期ドリフト時: `status = degraded`
- テスト前に `cn_admin.node_subscriptions` と in-memory topic 状態をリセットするヘルパーを追加し、既存データに依存しない形で固定

3. タスク管理更新
- `community_nodes_roadmap.md` の該当未実装項目を `[x]` に更新

## 変更ファイル

- `kukuri-community-node/crates/cn-relay/src/lib.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 検証

- `./scripts/test-docker.ps1 rust`
  - ログ: `tmp/logs/test-docker-rust-relay-healthz-20260213-050522.log`
- `docker run --rm --network host -v ${PWD}:/workspace ... kukuri-test-runner bash -lc "source /usr/local/cargo/env && cd /workspace/kukuri-community-node && cargo test -p cn-relay --all-features -- --nocapture"`
  - ログ: `tmp/logs/cn-relay-healthz-contract-mounted-20260213-051222.log`
  - 結果: `cn-relay` 16 tests passed
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-format-check-relay-healthz-20260213-051312.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-native-test-linux-relay-healthz-20260213-051312.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-community-node-tests-relay-healthz-20260213-051312.log`
