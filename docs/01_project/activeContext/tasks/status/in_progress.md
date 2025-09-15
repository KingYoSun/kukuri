[title] 作業中タスク（in_progress）

最終更新日: 2025年09月15日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## Iroh DHT/Discovery 残タスク（着手）

- [x] `bootstrap_nodes.json` の形式定義・検証・読み込み導線の確定（CLI/アプリ双方 実装）
- [x] ブートストラップUIの実装（n0デフォルト、任意ノードをUIから `node_id@host:port` 指定）
- [x] iroh-gossip: quit の意味整理と API 連動実装（`dht_bootstrap.rs::leave_topic` に Sender ドロップで退出を実装）
- [x] iroh-gossip: broadcast の意味整理と API 連動実装（`dht_bootstrap.rs::broadcast` に Sender 利用で送信を実装）
- [ ] NIPs 準拠イベントモデルの確定とバリデーション（NIP-01/10/19/30078 など）
- [ ] P2P 経路のみの投稿/閲覧/返信/引用の結合テスト（Rust/TS）と契約テストでの検証
- [ ] スモークテスト最小化（Tauri起動を伴わない形で `docker-compose.test.yml` の test-runner を用いた検証）
- [ ] Windows 環境では `./scripts/test-docker.ps1` による Docker 経由実行の既定化
- [ ] DHT メトリクス/ログの整備（tracing, counters, レベル設定）

関連: `docs/01_project/activeContext/iroh-native-dht-plan.md`

メモ/進捗ログ:
- 2025年09月15日: テスト戦略を更新（Tauri v2 の E2E は困難のため、ユニット/結合/契約テスト中心＋最小スモークへ移行）。
- 2025年09月15日: 方針更新（Nostr リレー非接続・P2P 優先、内部イベントは NIPs 準拠）。
- 2025年09月15日: critical.md から本タスク群を移動し、着手を明示しました。
- 2025年09月15日: DhtGossip にトピック別 Sender 管理を追加。`join_topic` で Sender を保持、`leave_topic` で削除、`broadcast` で送信（未参加時は自動参加）。
- 2025年09月15日: `discovery_dht()` を有効化（Tauri）。`bootstrap_nodes.json` の仕様（NodeId@host:port 推奨）・検証/読み込み導線を Tauri/CLI 双方に実装。development の localhost ノード設定を削除（n0 優先運用）。
- 2025年09月15日: ブートストラップUIを追加。Tauriコマンド `get_bootstrap_config` / `set_bootstrap_nodes` / `clear_bootstrap_nodes` 実装、ユーザーデータ配下 `user_bootstrap_nodes.json` に保存。設定画面に `BootstrapConfigPanel` を追加。フォールバック優先順は「ユーザー設定 → 同梱JSON → なし（n0依存）」に統一。
