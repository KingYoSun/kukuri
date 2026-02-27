# Community Node bootstrap runtime fallback 修正

作成日: 2026年02月27日

## 概要

Tauri クライアントで Community Node を設定して `Authenticate` しても、relay 接続状態の bootstrap source が `n0デフォルト` のままになる問題を修正した。  
原因は Community Node の `descriptor.endpoints.p2p` 未設定時に Tauri 側の bootstrap ノード抽出が 0 件となることだった。

## 実装内容

1. `cn-relay` に runtime P2P 情報エンドポイントを追加
- 追加: `GET /v1/p2p/info`
- 返却: `node_id`, `bind_addr`, `bootstrap_nodes`（`node_id@host:port`）
- 変更ファイル:
  - `kukuri-community-node/crates/cn-relay/src/lib.rs`
  - `kukuri-community-node/crates/cn-relay/src/gossip.rs`

2. `cn-user-api` の bootstrap レスポンスに runtime bootstrap ノードを同梱
- `GET /v1/bootstrap/nodes` のレスポンスに `bootstrap_nodes` 配列を追加
- `RELAY_P2P_INFO_URL`（未設定時は `RELAY_HEALTH_URL` から導出）へ問い合わせて runtime 候補を取得
- 変更ファイル:
  - `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs`
  - `kukuri-community-node/crates/cn-user-api/Cargo.toml`
  - `kukuri-community-node/Cargo.lock`

3. Tauri 側で runtime bootstrap ノードをフォールバック取り込み
- `BootstrapHttpResponse` に `bootstrap_nodes` を追加
- descriptor の `p2p` が欠落していても `bootstrap_nodes` を `normalize_bootstrap_node_candidate` 経由で user bootstrap に反映
- 変更ファイル:
  - `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`

## テスト

- `./scripts/test-docker.ps1 rust`（pass）
- Community Node コンテナ手順:
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres`（pass）
  - `docker compose -f docker-compose.test.yml build test-runner`（pass）
  - `docker run ... cargo test --workspace --all-features; cargo build --release -p cn-cli`（pass）
- 追加/更新テスト:
  - `set_config_uses_runtime_bootstrap_nodes_when_descriptor_has_no_p2p`（Tauri, pass）
  - `bootstrap_nodes_contract_includes_runtime_bootstrap_nodes_field`（cn-user-api, pass）
  - `extract_host_from_url_like_*`（cn-relay, pass）

## CI 補足（gh act）

- `native-test-linux`: pass
- `format-check`: fail（既存 TypeScript ファイル 142 件の Prettier 不整合）
- `community-node-tests`: fail（`/workspace/kukuri-community-node` 配下で `Cargo.toml` 解決失敗）

## 追記: Relay / Bootstrap Connected 数不整合の修正

Tauri 側で接続成功（Bootstrap 1 / Relay topic 参加済み）でも Admin UI の Relay/Bootstrap 接続数が 0 になるケースに対し、以下を追加した。

1. `cn-admin-api` の node-subscriptions 応答を runtime 補完
- `cn_admin.service_health(service='relay').details_json` の `p2p_runtime.bootstrap_nodes` と `auth_transition.ws_connections` を参照。
- topic 側の接続情報が空で、かつ `enabled = true` / `ref_count > 0` のトピックに限定して `connected_node_count` / `connected_user_count` を補完。
- これにより Relay topic 行で count が 0 固定になる問題を回避。

2. Admin Console の表示補完
- `BootstrapPage` は課金 `subscriptions` ではなく topic 接続情報を基準に Connected users を計算し、必要時に relay runtime へフォールバック。
- `RelayPage` は `connected_user_count > 0` かつ `connected_users` 空配列の行で `Runtime metrics only (pubkeys unavailable)` を表示。

3. 回帰テスト
- `cn-admin-api`: `node_subscriptions_list_falls_back_to_runtime_connectivity_when_topic_data_is_empty`
- `admin-console`: `BootstrapPage.test.tsx` / `RelayPage.test.tsx` の runtime 補完ケース更新

4. 検証結果（追記分）
- `docker run ... cargo test -p cn-admin-api --all-features`: pass
- `docker run ... cargo test --workspace --all-features; cargo build --release -p cn-cli`: pass
- `docker run ... pnpm vitest run src/pages/BootstrapPage.test.tsx src/pages/RelayPage.test.tsx`: pass
- `gh act --job native-test-linux`: pass
- `gh act --job format-check`: fail（既存の `kukuri-tauri` 側 Prettier 差分 142 件）
- `gh act --job community-node-tests`: fail（`/workspace/kukuri-community-node` で `Cargo.toml` が見つからない既知の `gh act` マウント差異）

## 追記: Connected Users が 0 のまま残るケースの修正

Admin UI で Relay / Bootstrap の `Connected Users` が 0 のまま残るケースに対し、`cn-user-api` 認可経路で public topic 購読を自動補完する処理を追加した。

1. `auth_verify` 成功時の補完
- `ensure_default_public_topic_subscription` を追加し、`cn_user.topic_subscriptions` の public topic 行を `active` で idempotent に作成/復帰するようにした。
- 初回有効化時のみ `cn_admin.node_subscriptions.ref_count` を +1 する。

2. `require_auth` 経由の補完
- 既存トークン利用（再 Authenticate していないセッション）でも public topic 購読が不足している場合に補完されるよう、`require_auth` でも同処理を呼び出すようにした。

3. 回帰テスト
- `cn-user-api`: `auth::tests::ensure_default_public_topic_subscription_is_idempotent`
  - 初回呼び出しは有効化されること
  - 2回目以降は重複加算しないこと
  - `topic_subscriptions` が `active` になること
  - `node_subscriptions.enabled` が `true` になること

4. 検証結果（本追記分）
- `docker run ... cargo test -p cn-user-api --all-features`: pass
- `gh act --job native-test-linux`: pass
- `gh act --job format-check`: fail（既存の `kukuri-tauri` 側 Prettier 差分 142 件）
- `gh act --job community-node-tests`: fail（`/workspace/kukuri-community-node` で `Cargo.toml` が見つからない既知の `gh act` マウント差異）
