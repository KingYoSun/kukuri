# cn-relay ingest→outbox→WS/gossip 統合テスト追加
日付: 2026年02月04日

## 概要
- cn-relay の ingest→outbox→WS/gossip 配信を統合テストで検証できるようにした。

## 対応内容
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs` に ingest→outbox→WS/gossip の統合テストを追加。
- gossip 配信の確認用に peer を明示登録するハーネスを追加。
- outbox 通知の `pg_notify` 呼び出しを `SELECT pg_notify('cn_relay_outbox', $1)` に合わせて修正。
- dev-dependencies に `tokio-tungstenite` を追加。

## 検証
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -c "cargo test -p cn-relay ingest_outbox_ws_gossip_integration -- --nocapture --test-threads=1"`
- `./scripts/test-docker.ps1 rust`
- `gh act --workflows .github/workflows/test.yml --job format-check`（NativeCommandError / git clone の some refs were not updated / pnpm approve-builds 警告）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（NativeCommandError / git clone の some refs were not updated / actions/cache restore warning（tar 失敗）/ useRouter 警告 / actions-cache save がハングしたため停止（テスト/リント完了済み））
