# 進行中タスク

**最終更新**: 2025年09月14日

## n0ディスカバリー利用＋IrohGossipService統一（着手）
- 目的: 当面はn0提供ノードを利用し、安定したブートストラップを確保。並行してGossip実装をIrohGossipServiceへ早期統一。
- 参照: iroh Builder（discovery_n0）https://docs.rs/iroh/latest/iroh/endpoint/struct.Builder.html
- 期待成果: discovery_n0での接続安定化、join/leave/broadcastの実装完了とサービス層からの一貫利用。

### 次のアクション（進行中）
- [x] タスク方針の反映（本ファイル更新）
- [x] Endpointをn0優先に変更（`.discovery_dht()`を一時無効化）
- [x] IrohGossipServiceで`GossipTopic::split()`を用いた`broadcast`実装
- [x] gossip_testsをIrohGossipServiceへ移行（join/leave/get_joined_topics/broadcast）
- [x] 未参加トピック`leave`は冪等に（IrohGossipService仕様）テスト期待値を調整
- [x] `modules/p2p/events.rs`を新設しP2PEventを一元化（`mod.rs`で再エクスポート）
- [x] `modules/p2p/mod.rs`から`GossipManager`の再エクスポートを削除（露出整理）
- [x] `state.initialize_p2p`をno-op化（旧経路の実起動を停止）
- [x] 旧EventSyncおよび対応テストを削除（段階移行完了）
- [x] 最小ユニットテスト（join/broadcast/leave）追加・通過
- [x] EventManagerのP2P配信を`GossipService`直結に変更（`EventSync`完全撤去）
- [x] 非トピック系イベントの配信先を既定トピックに統一（初期値`public`、起動時に作成/参加保証）
- [x] 既定トピック切替のTauriコマンド追加（`set_default_p2p_topic`）
- [x] UI受信導線の接続（`IrohGossipService::subscribe`→UI/handlers）
- [ ] 旧`GossipManager`利用箇所の棚卸しと段階的無効化（integration_testsの移行含む）
  - [ ] integration_tests（Iroh版）の骨子追加（subscribe受信テストを追加、現状はmulti-node未配線のためignore）
- [ ] P2Pイベント配信/購読ルーティングの実装（設計は作成済: `docs/03_implementation/p2p_event_routing_design.md`。実装着手はGossipManager廃止完了後）

### メモ
- ブートストラップノード（staging/production）は未運用のため、当面はn0のDNSディスカバリーを使用。
- 統一の最終段では旧P2P経路（modules/p2p/*）を除去し、プレゼンテーション層は`P2PService`→`IrohGossipService`に一本化する。
- RustテストはDockerで実行（DBはインメモリ）。オフライン系スキーマに合わせてマイグレーションを更新。Linux環境で不適合なキーリング依存の一部ユニットテストは除外（Windows実行を想定）。

### テスト状況メモ
- Docker実行結果（前回）：166 passed / 0 failed / 6 ignored

## 最近完了したタスク
- ✅ DHTブートストラップCLIノード実装（2025年08月20日完了）
  - 詳細: [進捗レポート](../../progressReports/2025-08-20_dht-bootstrap-cli-node.md)
- ✅ irohネイティブDHTへの移行（2025年08月16日完了）
  - 詳細: [進捗レポート](../../progressReports/2025-08-16_iroh-dht-migration.md)
