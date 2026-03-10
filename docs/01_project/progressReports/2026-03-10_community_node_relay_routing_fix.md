# Community Node relay 誤ルーティングと起動後 auth failure 修正レポート

作成日: 2026年03月10日
最終更新日: 2026年03月10日

## 1. 概要

- Tauri クライアント起動後に Community Node 認証を行うと、`wss://api.kukuri.app/relay` へ接続しようとして 404 になり、認証コマンド自体が失敗していた。
- 原因は、Community Node 側 bootstrap descriptor の既定値が `localhost` 固定だったことと、クライアント側が bootstrap node に対しても `base_url + /relay` fallback を使っていたことの 2 つだった。
- これを修正し、ローカル・公開環境のどちらでも `PUBLIC_BASE_URL` と `RELAY_PUBLIC_URL` の役割に沿った relay ルーティングになるよう整理した。

## 2. 原因

### 2.1 Community Node 側

- `kukuri-community-node/crates/cn-core/src/admin.rs`
  - bootstrap descriptor seed の `http/ws` endpoint は `BOOTSTRAP_DESCRIPTOR_HTTP_URL` / `BOOTSTRAP_DESCRIPTOR_WS_URL` が未設定だと
    - `http://localhost:8080`
    - `ws://localhost:8082/relay`
    へ固定で落ちていた。
  - `.env.home-vps-edge.example` では `RELAY_PUBLIC_URL=wss://relay.kukuri.app/relay` がある一方、descriptor seed はそれを使っていなかった。
  - そのため公開環境でも bootstrap descriptor が loopback relay を返し得た。

### 2.2 Tauri クライアント側

- `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`
  - bootstrap descriptor の `ws` が空、または loopback として破棄された場合、bootstrap node でも `base_url + /relay` へ fallback していた。
  - `https://api.kukuri.app` を設定すると、この fallback が `wss://api.kukuri.app/relay` を生成する。
  - 実運用の Nostr relay は `https://relay.kukuri.app` なので、ここで 404 が発生していた。

### 2.3 認証失敗への波及

- `kukuri-tauri/src-tauri/src/presentation/commands/community_node_commands.rs`
  - `authenticate` 完了後に relay 同期を実施し、その失敗をそのままコマンド失敗として返していた。
  - そのため、auth 自体は成功していても relay warning があるだけで UI 上は認証失敗になっていた。

## 3. 修正内容

### 3.1 bootstrap descriptor を公開 URL から自動導出

- `cn-core::admin`
  - descriptor `http` は `BOOTSTRAP_DESCRIPTOR_HTTP_URL` が無ければ `PUBLIC_BASE_URL` を使う。
  - descriptor `ws` は `BOOTSTRAP_DESCRIPTOR_WS_URL` が無ければ `RELAY_PUBLIC_URL` を使う。
  - 既存の bootstrap service config がある場合も descriptor endpoint だけを部分更新し、auth / exp など他の設定は保持する。

### 3.2 bootstrap node の `base_url + /relay` fallback を停止

- `community_node_handler`
  - bootstrap node では descriptor に有効な `ws` endpoint が無い限り `base_url + /relay` fallback を使わない。
  - legacy single-node 非 bootstrap config だけは既存互換のため fallback を維持する。

### 3.3 Community Node auth の relay 同期を best-effort 化

- `community_node_commands`
  - relay URL 解決や relay 接続失敗は warning ログに留め、`set_config` / `authenticate` を失敗させないようにした。

### 3.4 公開構成サンプルの修正

- `kukuri-community-node/.env.home-vps-edge.example`
  - `PUBLIC_BASE_URL=https://api.kukuri.app`
  - `RELAY_PUBLIC_URL=wss://relay.kukuri.app/relay`
  という役割分離を明示した。

## 4. テスト

- `./scripts/test-docker.ps1 rust`: PASS
- `./scripts/test-docker.ps1 e2e-community-node`: PASS
- `gh act --workflows .github/workflows/test.yml --job format-check`: PASS
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: PASS
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: PASS

## 5. 残課題

- `https://api.kukuri.app` 実機での live-path 再検証は未実施。
- 次の確認では、Community Node 認証後の relay status が `wss://relay.kukuri.app/relay` を向くこと、`wss://api.kukuri.app/relay` 404 と `No configured Nostr relay connected within 3s` warning が解消していることを確認する必要がある。
