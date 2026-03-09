# Community Node 認証時の relay URL 解決と iroh path warning 修正レポート

作成日: 2026年03月09日
最終更新日: 2026年03月09日

## 1. 概要

- Community Node 認証後に `wss://api.kukuri.app/relay` 404 が再発する問題を修正した。
- relay-only transport 下で canary relay と custom relay が lookup に共存し、`MaxPathIdReached` / `sent PATH_ABANDON after path was already discarded` を誘発し得る問題を修正した。
- いずれも shortcut ではなく、Rust 再現 test と Community Node end-to-end E2E の両方で回帰防止した。

## 2. 実施内容

### 2.1 relay URL 解決ロジックの修正

- `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`
  - `resolve_nostr_relay_urls_for_config()` に bootstrap descriptor fallback を追加し、`roles.bootstrap` の node では `config.base_url` と descriptor `endpoints.http` が一致しなくても `endpoints.ws` を優先利用するよう変更。
  - `base_url + /relay` fallback の適用条件を
    - matching descriptor がある node
    - bootstrap node
    - single-node legacy config
    のみに制限した。
- 追加 test
  - `resolve_nostr_relay_urls_skips_base_url_fallback_for_non_bootstrap_nodes_in_multi_node_config`
  - `resolve_nostr_relay_urls_uses_bootstrap_descriptor_ws_when_http_endpoint_differs`
  - `resolve_nostr_relay_urls_falls_back_to_base_url_for_single_non_bootstrap_node`

### 2.2 relay-only lookup 正規化の修正

- `kukuri-tauri/src-tauri/src/infrastructure/p2p/utils.rs`
  - `sanitize_remote_endpoint_addr_with_preferred_relays()` を追加し、configured custom relay がある場合は remote endpoint の relay URLs を custom relay のみに正規化。
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs`
  - `add_peer` 時の address lookup 登録で上記正規化を適用。
  - `effective_node_relay_urls()` は configured relay がある場合に endpoint reported relay を無視し、configured relay のみを採用するよう変更。
  - connectivity test `relay_only_add_peer_prefers_configured_custom_relay_in_lookup` を追加。
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_gossip_service.rs`
  - topic join 時の initial peer 登録にも preferred relay 正規化を適用。

## 3. 検証

### 3.1 Rust / Docker

- `./scripts/test-docker.ps1 rust`: pass
- `cargo test resolve_nostr_relay_urls --lib -- --nocapture` 相当の Docker 実行: pass
- `cargo test relay_only_add_peer --lib -- --nocapture` 相当の Docker 実行: pass

### 3.2 Required jobs

- `gh act --workflows .github/workflows/test.yml --job format-check`: pass
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: pass
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: pass

### 3.3 Live-path E2E

- `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.end-to-end.spec.ts ./scripts/test-docker.ps1 e2e-community-node`: pass
  - ログ: `tmp/logs/community-node-e2e/20260309-001451.log`
  - 同ログを検索し、以下が不在であることを確認:
    - `api.kukuri.app/relay`
    - `404 Not Found`
    - `MaxPathIdReached`
    - `sent PATH_ABANDON after path was already discarded`

## 4. タスク管理反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 完了済みの「relay URL 汚染不具合」「iroh path 異常」を削除。
- `docs/01_project/activeContext/tasks/completed/2026-03-09.md`
  - 完了内容と検証結果を追記。

## 5. 残課題

- Linux の account / Community Node 設定永続化の live-path 確認
- Windows reload crash (`iroh-quinn ... PoisonError`)
- Admin UI connected users / health の live-path 確認
