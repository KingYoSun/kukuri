# cn-user-api 契約テスト拡張（bootstrap/search/reports）

日付: 2026年02月07日

## 概要

`cn-user-api` の未実装項目だった契約テストを補完し、`/v1/bootstrap/*` `/v1/search` `/v1/reports` の成功系とレスポンス shape 互換を検証できる状態にした。

## 実装内容

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - テスト用 `test_state_with_meili_url` を追加し、検索テストだけ Meilisearch 接続先を差し替え可能にした
  - bootstrap seed 挿入ヘルパー、公開 GET JSON ヘルパー、認証付き POST JSON ヘルパーを追加
  - `/v1/bootstrap/nodes` の成功系 shape 契約テストを追加
  - `/v1/bootstrap/topics/{topic_id}/services` の成功系 shape 契約テストを追加
  - `/v1/search` の成功系 shape 契約テストを追加（テスト内モック Meilisearch を起動）
  - `/v1/reports` の成功系 shape 契約テストを追加（200/201 互換）
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 該当未実装項目を `[x]` に更新

## 検証

- `./scripts/test-docker.ps1 rust` 成功
- `docker compose -f docker-compose.test.yml up -d community-node-postgres` 成功
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -c "source /usr/local/cargo/env && cargo test -p cn-user-api --tests -- --nocapture"` 成功（19 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check` 成功
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` 成功

## 備考

- `gh act` 実行時の `some refs were not updated` / `pnpm approve-builds` 警告、`native-test-linux` 中の `useRouter` 警告は既知で、ジョブは成功。
