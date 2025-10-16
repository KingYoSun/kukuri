[title] 作業中タスク（in_progress）

最終更新日: 2025年10月16日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## GitHub Actions ワークフロー障害調査
- [ ] `gh run list` / `gh run view` で直近の失敗ワークフローとジョブを特定
- [ ] `gh act` で失敗ジョブをローカル再現
- [ ] 原因を修正し、GitHub Actions / `gh act` 双方で成功を確認

## Iroh DHT/Discovery 残タスク（着手）

- [x] `bootstrap_nodes.json` の形式定義・検証・読み込み導線の確定（CLI/アプリ双方 実装）
- [x] ブートストラップUIの実装（n0デフォルト、任意ノードをUIから `node_id@host:port` 指定）
- [x] iroh-gossip: quit の意味整理と API 連動実装（`dht_bootstrap.rs::leave_topic` に Sender ドロップで退出を実装）
- [x] iroh-gossip: broadcast の意味整理と API 連動実装（`dht_bootstrap.rs::broadcast` に Sender 利用で送信を実装）
- [x] DHT 設定フラグの導入（`NetworkConfig.enable_{dht,dns,local}` と `IrohNetworkService` ビルダー反映）
- [x] NIPs 準拠イベントモデルの受信バリデーション（NIP-01/10/19）
- [x] P2P 経路のみの投稿/閲覧/返信/引用の結合テスト（Rust/TS）と契約テストでの検証
  - [x] GitHub Actions に `smoke-tests.yml` を追加（`test-runner` 実行）
- [x] DHT メトリクス/ログの整備（tracing, counters, レベル設定）
  - [x] GossipメトリクスAPI（`get_p2p_metrics`）とフロントAPIラッパを追加
  - [x] P2PDebugPanelにメトリクス自動更新（10秒）と手動更新を追加

### 今後の作業予定（短期）

- [ ] NIP-19 TLVの詳細検証拡張（複数relay_urlsの扱い、文字列長上限、UTF-8検証など）
- [ ] `get_p2p_status` にメトリクス要約を含めるか別APIで集約（要UI検討）
- [ ] Rust/TSの契約テストを追加（NIP-10のmarker/relay_url整合の境界ケース）
- [ ] Windows: `./scripts/test-docker.ps1` に `metrics` / `contracts` オプションを追加
- [x] modules/p2p/tests/iroh_integration_tests.rs を NodeAddr ヒント対応（connect_peers の戻り値で初期ピア再設定）
- [x] P2P受信確認テストの安定化（DHTブートストラップコンテナ経由で discovery_dht() のみを使用。詳細: docs/03_implementation/p2p_dht_test_strategy.md）
- [ ] TypeScript契約テストの追加と Docker スモークテスト構成の縮小タスク化

関連: `docs/01_project/activeContext/iroh-native-dht-plan.md`

メモ/進捗ログ:
- 2025年10月16日: GitHub Actions が失敗しているため調査を開始。gh と `gh act` を用いた再現手順を整理。
- 2025年09月15日: テスト戦略を更新（Tauri v2 の E2E は困難のため、ユニット/結合/契約テスト中心＋最小スモークへ移行）。
- 2025年09月15日: 方針更新（Nostr リレー非接続・P2P 優先、内部イベントは NIPs 準拠）。
- 2025年09月15日: critical.md から本タスク群を移動し、着手を明示しました。
- 2025年09月15日: DhtGossip にトピック別 Sender 管理を追加。`join_topic` で Sender を保持、`leave_topic` で削除、`broadcast` で送信（未参加時は自動参加）。
- 2025年09月15日: `discovery_dht()` を有効化（Tauri）。`bootstrap_nodes.json` の仕様（NodeId@host:port 推奨）・検証/読み込み導線を Tauri/CLI 双方に実装。development の localhost ノード設定を削除（n0 優先運用）。
- 2025年09月15日: ブートストラップUIを追加。Tauriコマンド `get_bootstrap_config` / `set_bootstrap_nodes` / `clear_bootstrap_nodes` 実装、ユーザーデータ配下 `user_bootstrap_nodes.json` に保存。設定画面に `BootstrapConfigPanel` を追加。フォールバック優先順は「ユーザー設定 → 同梱JSON → なし（n0依存）」に統一。
- 2025年09月15日: DHT 設定フラグを `shared/config.rs::NetworkConfig` に追加（`enable_dht`, `enable_dns`, `enable_local`）。`IrohNetworkService::new` に反映（ビルダーに `discovery_n0` / `discovery_dht` を条件付け）。
- 2025年09月15日: P2P受信経路に NIP-01 準拠バリデーションを追加。`domain::entities::Event::validate_nip01` を実装し、`IrohGossipService` の受信処理で検証・不正ドロップ。
- 2025年09月15日: DHT/Gossip メトリクスの軽量カウンタを追加（`infrastructure/p2p/metrics.rs`）。`join/leave/broadcast/received` を計測し `tracing` に集約ログ出力。
- 2025年09月15日: NIP-10/19 検証を受信経路に追加（e/pタグの基本整合性）。統合テストをNIP-01準拠イベントで送信するよう修正。Dockerスモークで `ENABLE_P2P_INTEGRATION=1` を有効化。
- 2025年09月15日: GitHub Actions に Docker スモークテストを追加（`.github/workflows/smoke-tests.yml`）。
- 2025年09月15日: NIP-19（nprofile/npubなど）のエンコード検証が必要なケースを洗い出し中。
- 2025年10月15日: DHT メトリクスを成功/失敗件数と最終時刻を保持できる構造に刷新。`dht_bootstrap.rs` と `iroh_gossip_service.rs` へ成功・失敗ログを追加し、Tauri コマンドとフロントエンド（P2PDebugPanel）を拡張。`pnpm test` と `cargo test` を実行し、いずれも成功。
