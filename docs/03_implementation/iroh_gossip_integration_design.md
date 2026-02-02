# iroh-gossip統合設計

**作成日**: 2025年07月26日  
**最終更新**: 2026年02月02日

## 概要

本ドキュメントは、KukuriのP2Pイベント配信における現行の iroh-gossip 統合設計をまとめたものです。旧設計で扱っていた「GossipManager」単体構成は廃止され、現在は **GossipService trait + IrohGossipService 実装** を中心に、NetworkService と統合した構成へ移行しています。

## 参照

- iroh: https://docs.rs/iroh/latest/iroh/
- iroh-gossip: https://docs.rs/iroh-gossip/latest/iroh_gossip/

## 対応バージョン

- iroh: `0.95.1`
- iroh-gossip: `0.95.0`

## アーキテクチャ概要

```
┌──────────────────────────────────────────┐
│ Presentation (Tauri Commands / DTO)      │
├──────────────────────────────────────────┤
│ Application (P2PService / Use Cases)     │
├───────────────────────┬──────────────────┤
│ Infrastructure        │ Infrastructure   │
│ NetworkService        │ GossipService    │
│ (IrohNetworkService)  │ (IrohGossipService)
└───────────────────────┴──────────────────┘
```

### 主要な責務分担

- **NetworkService / IrohNetworkService**
  - Endpoint の生成、DiscoveryOptions 適用、ブートストラップ接続
  - DhtGossip（低レベルのDHTブロードキャスト補助）を保持
- **GossipService / IrohGossipService**
  - トピック参加/離脱、イベント配信、購読
  - TopicMesh による重複排除・統計
- **P2PService / P2PStack**
  - NetworkService と GossipService を束ね、アプリ層で利用
- **EventManager / EventDistributor**
  - アプリイベントを GossipService 経由で配信

## モジュール構成（現行）

```
src-tauri/src/
├── application/
│   └── services/p2p_service/
│       ├── bootstrap.rs      # P2PStackの組み立て
│       └── core.rs           # P2PService本体
├── infrastructure/
│   ├── p2p/
│   │   ├── discovery_options.rs
│   │   ├── gossip_service.rs
│   │   ├── iroh_gossip_service.rs
│   │   ├── iroh_network_service.rs
│   │   └── dht_bootstrap.rs
│   └── event/
│       └── manager/          # EventManagerのP2P配信統合
└── state.rs                  # AppStateへの配線
```

## 主要コンポーネント設計

### 1. GossipService（trait）

```rust
#[async_trait]
pub trait GossipService: Send + Sync {
    fn local_peer_hint(&self) -> Option<String> { None }
    async fn join_topic(&self, topic: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
    async fn leave_topic(&self, topic: &str) -> Result<(), AppError>;
    async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), AppError>;
    async fn subscribe(&self, topic: &str) -> Result<mpsc::Receiver<Event>, AppError>;
    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError>;
    async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, AppError>;
    async fn get_topic_stats(&self, topic: &str) -> Result<Option<TopicStats>, AppError>;
    async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError>;
}
```

### 2. IrohGossipService（実装）

- `iroh_gossip::Gossip` を内部保持
- `GossipTopic::split()` で sender/receiver を保持し、TopicMesh へ反映
- `static_discovery` を介して初期ピアのヒントを適用

### 3. IrohNetworkService（実装）

- `DiscoveryOptions` を `apply_to_builder()` で Endpoint に反映
- `StaticProvider` を使い、`NodeId@Addr` 形式のブートストラップを登録
- `DhtGossip` を補助的に保持（低レベル配信/フォールバック）

### 4. P2PStack 組み立て

```rust
let iroh_network = IrohNetworkService::new(secret_key, net_cfg, options, event_tx).await?;
let endpoint = iroh_network.endpoint().clone();
let static_discovery = iroh_network.static_discovery();
let mut gossip = IrohGossipService::new(endpoint, static_discovery)?;

let network_service: Arc<dyn NetworkService> = Arc::new(iroh_network);
let gossip_service: Arc<dyn GossipService> = Arc::new(gossip);
let p2p_service = Arc::new(P2PService::with_discovery(network_service.clone(), gossip_service.clone(), options));
```

## イベント配信フロー（概略）

1. EventManager がイベントを生成
2. EventDistributor が GossipService を介してトピックへ配信
3. IrohGossipService が TopicMesh で重複を排除しつつ broadcast
4. 購読側は `subscribe()` の Receiver で受信

## 旧設計との差分（重要）

- **GossipManager / EventSync / p2p/gossip_manager.rs は廃止**
- 現在は **GossipService trait + IrohGossipService** が中心
- P2P配線は `state.rs` と `P2PStack` で統合され、単体の「GossipManager」責務は分割済み

## 補足

- DHTディスカバリーの設定やブートストラップ詳細は `docs/03_implementation/dht_integration_guide.md` を参照
- 旧設計（v0.90.0前提）のAPI利用例は現行コードと一致しないため、参照しないこと
