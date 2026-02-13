# 2026年02月13日 `cn-user-api` bootstrap `ETag` ハッシュ契約対応

## 背景

- `docs/03_implementation/community_nodes/services_bootstrap.md` の HTTP キャッシュ要件では、`ETag` は event JSON/レスポンスボディのハッシュを正とする。
- 既存の `cn-user-api` は `updated_at（秒） + 件数` で `ETag` を生成しており、同秒更新かつ件数不変の更新を検出できない。

## 実施内容

- `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs`
  - `bootstrap::respond_with_events` の `ETag` 生成をレスポンスボディ（`items` + `next_refresh_at`）の BLAKE3 ハッシュへ変更。
  - `ETag` は `W/"<hash>"` 形式で返却。
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - 契約テスト `bootstrap_services_etag_changes_when_body_changes_with_same_count_and_second` を追加。
  - 同一件数・同秒更新（`updated_at = date_trunc('second', updated_at)`）で event JSON を更新した場合に `ETag` が変化することを固定。
  - 旧 `ETag` での `If-None-Match` は `200`、新 `ETag` での `If-None-Match` は `304` になることを検証。

## 仕様タスク更新

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 2026年02月13日「調査追記」の該当未実装項目を `[x]` に更新。

## 検証コマンド

- `./scripts/test-docker.ps1 rust -NoBuild`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test -p cn-user-api bootstrap_services_ -- --nocapture --test-threads=1"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test --workspace --all-features && cargo build --release -p cn-cli"`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`
