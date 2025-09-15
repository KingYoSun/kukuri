[title] 作業中タスク（in_progress）

最終更新日: 2025年09月15日

## Iroh DHT/Discovery 残タスク（着手）

- [x] `bootstrap_nodes.json` の形式定義・検証・読み込み導線の確定（CLI/アプリ双方 実装）
- [x] iroh-gossip: quit の意味整理と API 連動実装（`dht_bootstrap.rs::leave_topic` に Sender ドロップで退出を実装）
- [x] iroh-gossip: broadcast の意味整理と API 連動実装（`dht_bootstrap.rs::broadcast` に Sender 利用で送信を実装）
- [ ] Kukuri ↔ Nostr ブリッジの設計/実装（`bridge::kukuri_to_nostr`, `bridge::nostr_to_kukuri`）
- [ ] DHT メトリクス/ログの整備（tracing, counters, レベル設定）

関連: `docs/01_project/activeContext/iroh-native-dht-plan.md`

メモ/進捗ログ:
- 2025年09月15日: critical.md から本タスク群を移動し、着手を明示しました。
- 2025年09月15日: DhtGossip にトピック別 Sender 管理を追加。`join_topic` で Sender を保持、`leave_topic` で削除、`broadcast` で送信（未参加時は自動参加）。
- 2025年09月15日: `discovery_dht()` を有効化（Tauri）。`bootstrap_nodes.json` の仕様（NodeId@host:port 推奨）・検証/読み込み導線を Tauri/CLI 双方に実装。development の localhost ノード設定を削除（n0 優先運用）。
