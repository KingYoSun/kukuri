# 2026年02月10日 cn-relay WS バックフィル順序/limit/EOSE 整合対応

## 概要

`docs/03_implementation/community_nodes/services_relay.md` の NIP-01 整合要件に合わせて、`cn-relay` の WS バックフィル初期取得を `created_at` 降順（同値は `event.id` 辞書順）に修正した。あわせて `limit` 適用時の並び順と `EOSE` からリアルタイム配信への遷移を統合テストで固定した。

## 実装内容

- `kukuri-community-node/crates/cn-relay/src/ws.rs`
  - バックフィル取得 SQL を `ORDER BY e.created_at DESC, e.event_id ASC` に変更。
  - 複数フィルタの初期取得を一度集約し、重複除去後に `created_at DESC + id ASC` で再ソートして送信するよう変更。
  - 送信順比較用の `compare_backfill_events` を追加。
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
  - `insert_backfill_event` ヘルパーを追加。
  - 統合テスト `ws_backfill_orders_desc_applies_limit_and_transitions_to_realtime_after_eose` を追加。
    - `limit=2` で `newest -> tie(id辞書順)` の順序を検証。
    - 3件目が `EOSE` であることを検証。
    - `EOSE` 後に publish した新規イベントが同一購読へリアルタイム配信されることを検証。

## 検証結果

- `./scripts/test-docker.ps1 rust`（成功）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-relay -- --nocapture"`（成功: 11 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - 初回失敗（`cn-relay/src/integration_tests.rs` の rustfmt 差分）
  - `cargo fmt --all` 後の再実行は成功（ログ: `tmp/logs/gh-act-format-check-cn-relay-backfill-retry-20260211-062056.log`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功。ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-backfill-20260211-062217.log`）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功。ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-backfill-20260211-062834.log`）
