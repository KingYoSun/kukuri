# Community Nodes M3 完了
日付: 2026年01月24日

## 概要
- relay outbox から Meilisearch 同期を行う index サービスと User API search/trending を完成させた。
- reindex/expiration sweep の運用導線を整備した。

## 対応内容
- `kukuri-community-node/crates/cn-index` を追加し、outbox consumer/再索引/期限切れ掃除を実装。
- `kukuri-community-node/crates/cn-core` に Meilisearch クライアントを追加。
- `kukuri-community-node/crates/cn-user-api` の `/v1/search` `/v1/trending` を Meilisearch/DB集計に切替え、metering を適用。
- `kukuri-community-node/crates/cn-admin-api` に `/v1/reindex` を追加。
- `kukuri-community-node/docker-compose.yml` に `index` と `meilisearch` を追加、`.env` / `.env.example` を更新。
- `kukuri-community-node/migrations/20260125000000_m3_index.sql` を追加。

## 検証
- `docker run --rm -v "<workspace>:/workspace" -w /workspace/kukuri-community-node rust:1.88-bookworm cargo test --workspace --all-features`
- `./scripts/test-docker.ps1 rust`（警告: `dead_code` など）
- `gh act --workflows .github/workflows/test.yml --job format-check`（警告: `git clone` の `some refs were not updated`、pnpm の `approve-builds` 注意）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（警告: `git clone` の `some refs were not updated`、`useRouter` 警告）

## 補足
- Rust テストは警告ありだが、失敗なく完了。
