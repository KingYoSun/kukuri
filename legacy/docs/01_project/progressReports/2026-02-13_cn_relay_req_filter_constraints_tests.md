# Community Nodes 進捗レポート（`cn-relay` REQ 制約テスト互換固定）

作成日: 2026年02月13日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `services_relay.md` / `topic_subscription_design.md` の REQ 制約（`#t` 必須・filter上限・limit上限）に対して、`filters.rs` の単体テストを追加し、拒否理由（`missing #t filter` / `too many filters` / `too many filter values`）の互換を固定する。

を実装し、完了状態へ更新した。

## 実装内容

1. `filters.rs` 単体テスト追加
- `parse_filters` が `#t` 未指定時に `missing #t filter` を返すことを固定
- filter 個数が上限超過時に `too many filters` を返すことを固定
- filter 値個数が上限超過時に `too many filter values` を返すことを固定

2. `limit` 上限挙動の固定
- `limit` が `MAX_LIMIT` を超える場合に `MAX_LIMIT` へクランプされることを固定

3. タスク管理更新
- `community_nodes_roadmap.md` の該当未実装項目を `[x]` に更新

## 変更ファイル

- `kukuri-community-node/crates/cn-relay/src/filters.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-13.md`

## 検証

- `./scripts/test-docker.ps1 rust`
  - ログ: `tmp/logs/test-docker-rust-cn-relay-filters-20260213-060257.log`
- `docker run --rm --network kukuri_community-node-network -v ${PWD}:/workspace ... kukuri-test-runner bash -lc "source /usr/local/cargo/env && cd /workspace/kukuri-community-node && cargo test --workspace --all-features && cargo build --release -p cn-cli"`
  - 初回は DB 未接続で `PoolTimedOut`
  - `community-node-postgres` / `community-node-meilisearch` 起動後の再実行で成功
- `docker run --rm -v ${PWD}:/workspace ... kukuri-test-runner bash -lc "source /usr/local/cargo/env && cd /workspace/kukuri-community-node && cargo test -p cn-relay filters::tests -- --nocapture"`
  - ログ: `tmp/logs/cn-relay-filters-unit-20260213-060159.log`
  - 結果: 4 tests passed
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-cn-relay-filters-20260213-054917.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知の `useRouter` 警告のみ）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-filters-20260213-055036.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-filters-20260213-055724.log`
