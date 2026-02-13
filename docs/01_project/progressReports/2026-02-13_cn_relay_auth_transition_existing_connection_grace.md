# Community Nodes 進捗レポート（`cn-relay` 認証遷移: 既存接続の猶予切断固定）

作成日: 2026年02月13日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-relay`: `auth_transition_design.md` の既存接続要件（`disconnect_unauthenticated_at = enforce_at + grace_seconds`）と実装を一致させる。`ws_auth_timeout_seconds` との責務分離を明確化し、既存接続が猶予期間を満たしてから切断されることを統合テストで固定する。

を実装し、完了状態へ更新した。

## 実装内容

1. `cn-relay` WS 認証遷移の責務分離
- `ws_auth_timeout_seconds` を「AUTH 必須状態で開始した接続（新規接続）」中心の待機タイマーに限定
- `enforce_at` 前から継続している既存接続は、`disconnect_unauthenticated_at (= enforce_at + grace_seconds)` 到来までタイムアウト切断しないよう修正
- AUTH challenge 送出ロジックを整理し、猶予付き接続で challenge を再送し続けないよう修正

2. `cn-core` 認証設定判定の明確化
- `AuthConfig::disconnect_deadline()` が `auth_mode=required` のときのみ有効になるように変更
- 判定の単体テストを追加し、`off` 時は deadline を返さないことを固定

3. 統合テストの固定化
- 既存接続ケースを `auth_enforce_existing_connection_disconnects_after_grace_period` に置換
  - `AUTH` challenge 後も `ws_auth_timeout_seconds` では切断されず、猶予満了後に `auth-required: deadline reached` で切断されることを検証
- 新規接続のタイムアウト責務を `auth_required_new_connection_times_out_without_auth` として分離
  - `auth-required: timeout` の切断経路を独立して保証

4. ドキュメント整合
- `auth_transition_design.md` に「既存接続は猶予期限判定を優先し、猶予中に `ws_auth_timeout_seconds` で切断しない」旨を明記
- `services_relay.md` に `ws_auth_timeout_seconds` と `disconnect_unauthenticated_at` の役割分離を追記
- `community_nodes_roadmap.md` の該当項目を `[x]` に更新

## 変更ファイル

- `kukuri-community-node/crates/cn-core/src/service_config.rs`
- `kukuri-community-node/crates/cn-relay/src/ws.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `docs/03_implementation/community_nodes/auth_transition_design.md`
- `docs/03_implementation/community_nodes/services_relay.md`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 検証

- `./scripts/test-docker.ps1 rust`
  - ログ: `tmp/logs/test-docker-rust-cn-relay-auth-transition-20260213-144235.log`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/workspace -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "source /usr/local/cargo/env && cargo test --workspace --all-features && cargo build --release -p cn-cli"`
  - 初回ログ: `tmp/logs/community-node-rust-workspace-auth-transition-20260213-144302.log`
  - 既知の失敗理由: `cn-relay` 統合テストで既存 DB データ残留により `cn_relay.events` の重複キー衝突
  - DB 再作成後の再実行ログ: `tmp/logs/community-node-rust-workspace-auth-transition-rerun-20260213-145345.log`
  - 再実行結果: 成功
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-format-check-cn-relay-auth-transition-20260213-151015.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-native-test-linux-cn-relay-auth-transition-20260213-151134.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests --env NPM_CONFIG_PREFIX=/tmp/npm-global`
  - ログ: `tmp/logs/gh-act-community-node-tests-cn-relay-auth-transition-20260213-151759.log`
