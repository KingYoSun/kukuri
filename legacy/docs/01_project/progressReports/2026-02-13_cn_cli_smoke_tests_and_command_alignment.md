# Community Nodes 進捗レポート（`cn-cli` 統合スモーク + コマンド仕様整合）

作成日: 2026年02月13日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-cli`: `cn_cli_migration.md` の「有用サブコマンド維持 + daemon 起動対応」に対する回帰防止として、`migrate` / `config seed` / `admin bootstrap|reset-password` / `openapi export` / `p2p` 系の統合スモークテストを追加する。あわせて `cn bootstrap daemon` / `cn relay daemon` 形式をサポートするか、現行コマンド（`cn bootstrap` / `cn relay`）を正とするよう設計ドキュメントを更新して整合を取る。

を実装し、完了状態へ更新した。

## 実装内容

1. `cn-cli` 統合スモークテスト追加
- `kukuri-community-node/crates/cn-cli/tests/cli_access_control_node_key_integration.rs` に
  `cli_smoke_covers_migrate_config_admin_openapi_and_p2p_commands` を追加。
- 追加スモークで以下を実行検証:
  - `cn migrate`
  - `cn config seed`
  - `cn admin bootstrap`
  - `cn admin reset-password`
  - `cn openapi export --service user-api|admin-api`
  - `cn p2p --help`
  - `cn p2p node-id`（同一 secret key で node id が決定論的に一致すること）
  - `cn bootstrap --help` / `cn relay --help`（トップレベル起動コマンドの契約）

2. 設計ドキュメント整合
- `docs/03_implementation/community_nodes/cn_cli_migration.md` を更新し、
  現行仕様を `cn bootstrap` / `cn relay`（daemon サブコマンドなし）として明記。
- `cn bootstrap daemon` / `cn relay daemon` は現行未サポートであることを追記。

3. タスク管理更新
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当未実装項目を `[x]` に更新。

## 変更ファイル

- `kukuri-community-node/crates/cn-cli/tests/cli_access_control_node_key_integration.rs`
- `docs/03_implementation/community_nodes/cn_cli_migration.md`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 検証

- `./scripts/test-docker.ps1 rust -NoBuild`
  - ログ: `tmp/logs/test-docker-rust-cn-cli-smoke-20260213.log`
  - 備考: PowerShell 側の既知挙動で終了コードが `-1` 表示になることがあるが、ログ上の Rust テストは全件成功。
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test --workspace --all-features && cargo build --release -p cn-cli"`
  - ログ: `tmp/logs/community-node-workspace-cn-cli-smoke-20260213.log`
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 初回（`rustfmt` 差分検知で失敗）: `tmp/logs/gh-act-format-check-cn-cli-smoke-20260213.log`
  - `cargo fmt --all` 後の再実行（成功）: `tmp/logs/gh-act-format-check-cn-cli-smoke-rerun-20260213.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-cli-smoke-20260213.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-cli-smoke-20260213.log`
