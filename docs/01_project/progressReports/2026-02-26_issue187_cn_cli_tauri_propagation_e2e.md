# Issue #187 Community Node bootstrap/relay 経路 `cn-cli -> Tauri` 伝播 E2E

作成日: 2026年02月26日

## 概要

Community Node の bootstrap/relay 経路を利用し、2ノード（`cn-cli` と Tauri クライアント）間でイベントが伝播する E2E を追加した。  
`cn-cli` 側から publish したイベントを Tauri クライアントで受信し、投稿一覧に表示されることを検証対象とした。

## 実装内容

1. `cn-cli` 側 publish 経路の追加
- `kukuri-community-node/crates/cn-cli/src/main.rs` に `p2p publish` サブコマンドを追加。
- NIP-01 互換イベント JSON を作成し、gossip message payload として送信可能にした。
- bootstrap node を relay として扱う既定購読経路を追加し、`cn-cli` publish の中継経路を明確化した。

2. Tauri 側受信・表示経路の安定化
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_gossip_service.rs` で初期 peer 接続と payload デコードの互換処理を強化。
- `kukuri-tauri/src/hooks/useP2PEventListener.ts` で raw payload パースの型互換を拡張し、重複受信抑止を追加。
- `kukuri-tauri/src/lib/utils/tauriEnvironment.ts` で `tauri://` 判定を補強。

3. E2E bridge と spec 追加
- `kukuri-tauri/src/testing/registerE2EBridge.ts` / `kukuri-tauri/tests/e2e/helpers/bridge.ts` に P2P 状態取得・topic join・投稿スナップショット取得 API を追加。
- `kukuri-tauri/tests/e2e/specs/community-node.cn-cli-propagation.spec.ts` を新規追加。
- spec では bootstrap/relay 経路の有効化確認、`cn-cli publish` 実行、Tauri 側 message/post snapshot と UI（`posts-list`）表示を検証。

## 検証

- `tmp/logs/community-node-e2e/20260226-123650-single-cn-cli-propagation.log`  
  - `PASSED ... community-node.cn-cli-propagation.spec.ts`
  - `Spec Files: 1 passed`
  - `posts-list` に publish 文字列が出現
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

## 補足

- `format-check` 初回失敗（TypeScript Prettier）を修正後、再実行で通過。
