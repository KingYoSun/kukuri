# 2026-02-12 `cn-cli` Node Key / Access Control 統合テスト追加

## 概要

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目
  - `cn-cli`: Node Key 生成/ローテーションと Access Control rotate/revoke の統合テスト
  - 監査ログ記録・DB 反映・CLI 出力後方互換の担保
  - を実装完了した。

## 実装内容

- `kukuri-community-node/crates/cn-cli/src/main.rs`
  - `access-control rotate` 実行時に `cn_admin.audit_logs` へ `access_control.rotate` を記録する処理を追加。
  - `access-control revoke` 実行時に `cn_admin.audit_logs` へ `access_control.revoke` を記録する処理を追加。
  - 既存 CLI 出力（`topic_id` / `scope` / `previous_epoch` / `new_epoch` / `recipients` / `revoked_pubkey`）は維持。

- `kukuri-community-node/crates/cn-cli/tests/cli_access_control_node_key_integration.rs`（新規）
  - `node-key generate/rotate/show` の統合テストを追加。
  - `access-control rotate/revoke` の統合テストを追加。
  - 検証内容:
    - 監査ログ（`node_key.generate` / `node_key.rotate` / `access_control.rotate` / `access_control.revoke`）がDBへ記録されること。
    - `topic_scope_state` / `topic_scope_keys` / `topic_memberships` / `key_envelopes` が期待通り更新されること。
    - CLI 出力が後方互換な top-level フィールド構造を維持していること。

- タスク管理
  - `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当項目を `[x]` に更新。

## 検証

- `./scripts/test-docker.ps1 rust -NoBuild`（成功）
  - `tmp/logs/test-docker-rust-cn-cli-access-control-20260211-235253.log`
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 初回は `rustfmt` 差分で失敗（既知・修正済み）
    - `tmp/logs/gh-act-format-check-cn-cli-access-control-20260211-235325.log`
  - `cargo fmt --all` 実行後に再実行し成功
    - `tmp/logs/gh-act-format-check-cn-cli-access-control-20260211-235448.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - `tmp/logs/gh-act-native-test-linux-cn-cli-access-control-20260211-235608.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - `tmp/logs/gh-act-community-node-tests-cn-cli-access-control-20260212-000304.log`
  - 追加テスト通過確認:
    - `test access_control_rotate_revoke_updates_db_audit_and_preserves_output_shape ... ok`
    - `test node_key_generate_rotate_records_audit_and_keeps_stdout_shape ... ok`

