# iroh-gossip v0.90.0 API ドキュメント

## 概要

iroh-gossip v0.90.0は、Irohネットワークライブラリ上でepidemic broadcast trees（疫学的ブロードキャストツリー）を実装するGossipプロトコルライブラリです。トピックベースのメッセージ配信を提供し、各トピックは独立したブロードキャストツリーとメンバーシップを持ちます。

## 主要コンポーネント

### 1. Gossip構造体
Gossipプロトコルのメインインスタンスです。

#### 作成方法
```rust
use iroh_gossip::net::Gossip;

// Gossipインスタンスの作成
let gossip = Gossip::builder()
    .spawn(endpoint.clone());
```

#### 主要メソッド
- `subscribe(topic_id: TopicId, bootstrap_nodes: Vec<NodeId>) -> impl Stream<Item = Result<Event, RecvError>>` - トピックを購読（デフォルトオプション）
- `subscribe_with_opts(topic_id: TopicId, opts: Options) -> impl Stream<Item = Result<Event, RecvError>>` - カスタムオプションでトピックを購読
- `subscribe_and_join(topic_id: TopicId, bootstrap_nodes: Vec<NodeId>) -> Result<GossipTopic>` - トピックを購読し、少なくとも1つのノードとの接続を待つ

### 2. GossipTopic構造体
購読されたトピックを表し、メッセージの送受信を行います。

#### 主要メソッド
- `broadcast(data: impl Into<Bytes>) -> Result<()>` - すべてのピアにメッセージをブロードキャスト
- `broadcast_neighbors(data: impl Into<Bytes>) -> Result<()>` - 直接接続された隣接ノードのみにメッセージを送信
- `joined() -> impl Future<Output = Result<()>>` - 少なくとも1つのノードとの接続を待つ
- `is_joined() -> bool` - 少なくとも1つのノードと接続されているかチェック
- `split() -> (GossipSender, GossipReceiver)` - 送信と受信を分離

### 3. Event列挙型
Gossipトピックから受信するイベントです。

```rust
use iroh_gossip::api::Event;

pub enum Event {
    /// 新しい直接隣接ノードが追加された
    NeighborUp(NodeId),
    
    /// 直接隣接ノードが削除された
    NeighborDown(NodeId),
    
    /// メッセージを受信した
    Received(Message),
    
    /// メッセージの処理が遅れ、いくつかのメッセージが失われた
    Lagged,
}
```

### 4. Message構造体
受信したメッセージの詳細情報を含みます。

```rust
use iroh_gossip::api::Message;

pub struct Message {
    /// メッセージの内容
    pub content: Bytes,
    
    /// 配信スコープ（直接隣接ノードまたはGossip経由）
    pub scope: DeliveryScope,
    
    /// メッセージを配信したノード（必ずしも元の送信者ではない）
    pub delivered_from: NodeId,
}
```

## 使用例

### 基本的な使用フロー

```rust
use iroh::{Endpoint, protocol::Router};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN_BYTES};
use iroh_gossip::proto::TopicId;
use futures::StreamExt;

// 1. Endpointの作成
let endpoint = Endpoint::builder()
    .secret_key(secret_key)
    .discovery_n0()
    .bind()
    .await?;

// 2. Gossipインスタンスの作成
let gossip = Gossip::builder()
    .spawn(endpoint.clone());

// 3. Routerの設定
let router = Router::builder(endpoint.clone())
    .accept(GOSSIP_ALPN_BYTES.to_vec(), gossip.clone())
    .spawn();

// 4. トピックの購読
let mut stream = gossip.subscribe(topic_id, bootstrap_nodes).await?;

// 5. イベントの処理
while let Some(event) = stream.next().await {
    match event? {
        Event::Received(message) => {
            println!("Received: {:?}", message.content);
        },
        Event::NeighborUp(node_id) => {
            println!("Neighbor joined: {}", node_id);
        },
        Event::NeighborDown(node_id) => {
            println!("Neighbor left: {}", node_id);
        },
        Event::Lagged => {
            println!("Some messages were dropped");
        }
    }
}
```

## 現在の実装の問題点と修正方法

### 1. APIインターフェースの変更

v0.90.0では、`GossipApi`と`GossipTopic`を介したAPIアクセスが推奨されています。現在のコードは直接`Gossip`を使用しているため、以下の修正が必要です：

```rust
// 現在の実装（問題あり）
let mut receiver = self.gossip.subscribe(iroh_topic_id, bootstrap_peers)
    .await
    .map_err(|e| P2PError::JoinTopicFailed(format!("Failed to subscribe to topic: {}", e)))?;

// 修正後（推奨）
// GossipApiを使用
let topic = self.gossip.subscribe_and_join(iroh_topic_id, bootstrap_peers)
    .await
    .map_err(|e| P2PError::JoinTopicFailed(format!("Failed to subscribe to topic: {}", e)))?;

// ストリームの分割
let (sender, mut receiver) = topic.split();
```

### 2. メッセージのブロードキャスト

```rust
// 現在の実装（broadcastメソッドが存在しない）
// TODO: iroh-gossip 0.90.0ではbroadcastメソッドがない。APIを調査

// 修正後
// GossipTopicまたはGossipSenderを使用
sender.broadcast(bytes).await?;
```

### 3. トピックからの離脱

```rust
// 現在の実装（leaveメソッドが存在しない）
// TODO: iroh-gossip 0.90.0ではleaveメソッドがない。APIを調査

// 修正後
// GossipTopicをドロップすることで自動的に離脱
drop(topic); // または、スコープ外に出る
```

### 4. インポートパスの修正

```rust
// 現在のインポート
use iroh_gossip::net::Event; // 正しくない

// 修正後
use iroh_gossip::api::Event; // apiモジュールから
```

## 重要な注意点

1. **接続管理**: デフォルトでは、各トピックにつき最大5つのピア接続を維持します
2. **メッセージ配信**: メッセージは自動的に他のピアにリレー（ゴシップ）されます
3. **ブートストラップ**: トピックに参加するには、少なくとも1つのピアの公開鍵が必要です
4. **バッファリング**: 内部チャネルが満杯になると、最も古いメッセージが削除されます

## 今後の実装タスク

1. `GossipApi`と`GossipTopic`を使用した新しいAPIへの移行
2. メッセージ送信機能の実装（`broadcast`メソッドの使用）
3. トピック管理の改善（適切なライフサイクル管理）
4. エラーハンドリングの強化
5. インテグレーションテストの更新