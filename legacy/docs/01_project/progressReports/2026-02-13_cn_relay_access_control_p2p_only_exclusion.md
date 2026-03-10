# Community Nodes 進捗レポート（`cn-relay` Access Control P2P-only 除外）

作成日: 2026年02月13日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `event_treatment_policy.md` の P2P-only 要件に合わせ、`39020/39021/39022`（Access Control）を relay の保存/outbox/WS/gossip 配布対象から除外する（拒否理由の互換を含む）。統合テストを追加する。

を実装し、完了状態へ更新した。

## 実装内容

1. `ingest` で Access Control kind を明示 reject
- `cn-relay` の ingest 経路に `39020/39021/39022` 判定を追加
- `restricted: access control p2p-only` で reject し、保存/outbox/gossip を停止
- reject は metrics の `reason=restricted` で集計

2. WS 配信経路でも Access Control kind を除外
- `is_allowed_event` で `39020/39021/39022` を常時除外
- DB に過去データが残っている場合でも backfill/realtime で配信しない

3. 統合テスト追加（拒否理由互換 + 非配布保証）
- `access_control_events_are_rejected_and_not_distributed` を追加
- kind `39020/39021/39022` それぞれで次を固定:
  - `OK false` + `restricted: access control p2p-only`
  - `cn_relay.events` / `cn_relay.event_topics` / `cn_relay.events_outbox` に保存されない
  - WS subscriber へ EVENT が届かない
  - gossip receiver へ配信されない

4. タスク管理更新
- `community_nodes_roadmap.md` の該当項目を `[x]` に更新
- `completed/2026-02-13.md` に完了記録を追加

## 変更ファイル

- `kukuri-community-node/crates/cn-relay/src/ingest.rs`
- `kukuri-community-node/crates/cn-relay/src/ws.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-13.md`

## 検証

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml run --rm --workdir /app/kukuri-community-node -v ./kukuri-community-node:/app/kukuri-community-node rust-test cargo fmt --all -- --check`（成功）
- `docker compose -f docker-compose.test.yml run --rm --workdir /app/kukuri-community-node -v ./kukuri-community-node:/app/kukuri-community-node -e DATABASE_URL=postgres://cn:cn_password@127.0.0.1:15432/cn rust-test cargo test --workspace --all-features -p cn-relay -- --nocapture --test-threads=1`（成功）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-20260213-154714.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功、既知の `useRouter` 警告のみ）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260213-154833.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-20260213-155503.log`
