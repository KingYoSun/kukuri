# GossipManager 廃止計画（具体化）

最終更新: 2025年09月15日

## 背景
- 旧経路 `modules/p2p/GossipManager + EventSync` は、独自のEndpoint生成やイベント配信経路を持つ。
- 新方針では `infrastructure/p2p/IrohGossipService` を統一実装として採用し、Endpointは `IrohNetworkService` が一元管理。

## 目的
- 実行経路を IrohGossipService に一本化し、重複コードを削減。
- 将来のAPI変更（iroh/iroh-gossip）の追随点を1箇所へ集約。

## スコープ
- 対象コード: `modules/p2p/gossip_manager.rs`, `modules/p2p/event_sync.rs` とそれらのテスト。
- 関連: `state.initialize_p2p` と UI へのイベント配信経路。

## フェーズ計画
### Phase 1（完了）
- 実行経路の切替: `initialize_p2p` を no-op 化し、旧経路を起動しない。
- IrohGossipService の join/leave/broadcast 実装と最小テスト追加。

### Phase 2（完了）
- ドキュメント化（本ファイル）と型レベル非推奨化（`#[deprecated]`）。
- IrohGossipServiceに subscribe API と互換P2PEvent送出を実装。
- P2PEventを`modules/p2p/events.rs`に一元化し、`mod.rs`で再エクスポート。

### Phase 3（移行中→一部完了）
- UI配信を IrohGossipService へ移行（AppStateで購読管理・Tauri Emit接続を実装、lib.rsの旧イベント名は`p2p://message/raw`へ退避）。
- 旧テストの等価カバレッジを IrohGossipService 系へ移植（integrationの置換を含む）。

### Phase 4（削除）
- `event_sync.rs` と対応テストの削除（完了）。
- `gossip_manager.rs` の削除（テスト完全移行後に実施）。
- `spawn_p2p_event_handler` の旧チャネル依存を整理。

## タスクリスト
- [x] `initialize_p2p` を no-op 化（state.rs）
- [x] IrohGossipService: join/leave/broadcast 実装
- [x] IrohGossipService: subscribe 実装＋互換P2PEvent送出
- [x] P2PEventの一元化（`modules/p2p/events.rs` 新設、`mod.rs`再エクスポート更新）
- [x] 旧テストの移植（等価テストの第1弾：join/leave/get_joined_topics/broadcast）
- [x] 未参加トピックの`leave`は冪等に（期待値を修正）
- [x] 旧EventSyncおよび対応テストの削除
- [x] 非トピック系イベントの配信先を既定トピックに統一（初期値`public`）
- [x] 既定トピック切替APIを追加（Tauri: `set_default_p2p_topic`）
- [x] UI配信の接続（`IrohGossipService::subscribe` → UI/handlers）
- [x] EventManagerのP2P配信を`GossipService`へ直結（`EventSync`完全撤去）
- [ ] integrationテスト群の移行（`GossipManager`依存の除去）
- [ ] `gossip_manager.rs` の削除（最終）

## 進捗概要（2025年09月15日）
- 等価テストを IrohGossipService へ移植（join/leave/get_joined_topics/broadcast）。
- IrohGossipService に購読APIを追加。受信を domain::Event として配布しつつ、互換の P2PEvent も送出。
- AppStateにUI購読管理（`ensure_ui_subscription`/`stop_ui_subscription`）を実装。起動時`public`購読を確立し、`join_p2p_topic`/`join_topic_by_name`で自動購読、`leave_p2p_topic`で停止。
- UI向けイベントの形状を`useP2PEventListener`の期待に合わせて`p2p://message`にEmit。旧`GossipMessage`経路は`p2p://message/raw`へ移して競合を回避。
- P2PEvent を modules/p2p/events.rs に集約。重複定義を解消。
- 露出整理：modules/p2p/mod.rs から GossipManager の再エクスポートを削除。
- `state.initialize_p2p` は no-op 化（旧経路の実行抑止）。
- 旧EventSync（およびテスト）を削除し、IrohGossipService への移行を前提化。

## 互換性/仕様メモ
- `leave`：未参加トピックに対しても成功扱い（冪等）。
- 受信イベント：UI向け（domain::Event）と互換用途（P2PEvent）の二経路を暫定維持。
- 非トピック系の送信イベント（テキストノート/メタデータ/リアクション等）は既定トピックで流通（初期値`public`、アプリ起動時に作成・参加を保証）。

## テスト
- ローカルRustテスト：147 passed / 0 failed / 6 ignored。
- Docker実行（前回）：166 passed / 0 failed / 6 ignored。

## リスクと対策
- リスク: UIのイベント配信経路が空になる可能性
  - 対策: Phase 3でイベントチャネル導線を先行実装し、UI影響を抑制
- リスク: 旧テスト削除に伴うカバレッジ低下
  - 対策: 等価テストをIrohGossipService側で先行準備

## ロールバック
- `initialize_p2p` を戻すだけで旧経路を再起動可能（当面はコード残置）
  - 注意: UIイベント名は`p2p://message`を新UIが使用中。旧`GossipMessage`経路は`p2p://message/raw`へ変更済みのため、戻す際はUI側のリスナを合わせて切替が必要。
