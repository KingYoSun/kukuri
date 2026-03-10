# iroh-gossip実装計画

**作成日**: 2025年07月26日  
**最終更新**: 2025年07月27日

## 概要

本ドキュメントは、iroh-gossip統合の具体的な実装計画とタスクブレークダウンです。

## 実装スケジュール（10日間）

### Day 1-2: 基礎実装 ✅ 完了

#### Day 1: セットアップと基本構造
- [x] Cargo.tomlへの依存関係追加
  ```toml
  [dependencies]
  iroh = "0.90.0"
  iroh-gossip = "0.90.0"
  ```
- [x] P2Pモジュールディレクトリ作成
- [x] 基本的なモジュール構造の実装
- [x] エラー型の定義

#### Day 2: GossipManager基本実装
- [x] GossipManager構造体の実装
  - [x] iroh-gossip v0.90.0 API対応
  - [x] Event型のインポートパス修正（api::Event使用）
  - [x] GossipSenderの可変性対応（Arc<Mutex<GossipSender>>）
- [x] 初期化・シャットダウン機能
- [x] 基本的なTauriコマンド作成
- [x] ユニットテスト作成

### Day 3-5: トピック管理 ✅ 完了

#### Day 3: トピック参加機能
- [x] TopicMesh構造体の実装
- [x] join_topic機能の実装
  - [x] iroh-gossip v0.90.0のsubscribe API対応
  - [x] GossipTopic返却とsplit()による送受信分離
- [x] leave_topic機能の実装
- [x] トピック状態管理

#### Day 4: メッセージング基盤
- [x] GossipMessageフォーマット実装
- [x] メッセージ署名・検証（secp256k1使用）
- [x] broadcast機能の実装
- [x] 受信ハンドラーの実装
  - [x] NeighborUp/NeighborDownイベント対応
  - [x] Tauriイベントエミッター統合

#### Day 5: メッセージ処理
- [x] 重複排除メカニズム（LRUキャッシュ）
- [x] メッセージキャッシュ実装（最大1000件）
- [x] エラーハンドリング
- [x] 統合テスト作成（ピア間通信、マルチノード等）

### Day 6-8: Nostr統合（進行中）

#### Day 6: イベント変換
- [x] NostrイベントからGossipMessageへの変換
- [x] GossipMessageからNostrイベントへの変換
- [x] トピックID抽出ロジック（kind:30078使用）
- [x] 変換テスト作成

#### Day 7: 双方向同期 ✅ 完了
- [x] EventSync実装
- [x] Nostrイベント送信時のP2P配信
- [x] P2P受信イベントのNostr処理
- [x] 同期状態管理

#### Day 8: ハイブリッド配信 ✅ 完了
- [x] 並列配信メカニズム
- [x] 配信優先度管理
- [x] フォールバック処理
- [x] 統合テスト実装

### Day 9-10: UI統合と最適化

#### Day 9: UI統合
- [ ] P2P状態管理（p2pStore）作成
- [ ] 接続状態表示コンポーネント
- [ ] トピックメッシュ可視化
- [ ] デバッグパネル実装

#### Day 10: 最適化とテスト
- [ ] パフォーマンステスト
- [ ] メモリ使用量最適化
- [ ] ドキュメント作成
- [ ] 最終統合テスト

## 詳細実装ガイド

### 1. ディレクトリ構造

```
src-tauri/src/modules/p2p/
├── mod.rs                  # モジュール定義 ✅
├── error.rs               # P2P固有のエラー型 ✅
├── gossip_manager.rs      # GossipManager実装 ✅
├── topic_mesh.rs          # TopicMesh実装 ✅
├── message.rs             # メッセージ型定義 ✅
├── event_sync.rs          # Nostr連携（実装中）
├── peer_discovery.rs      # ピア発見（将来）
├── commands.rs            # Tauriコマンド ✅
└── tests/                 # テストモジュール ✅
    ├── mod.rs
    └── gossip_tests.rs
```

### 2. 主要インターフェース

#### Tauriコマンド

```rust
// src-tauri/src/commands/p2p.rs

#[tauri::command]
pub async fn initialize_p2p(
    state: State<'_, AppState>,
) -> Result<(), String> {
    // P2P機能の初期化
}

#[tauri::command]
pub async fn join_p2p_topic(
    state: State<'_, AppState>,
    topic_id: String,
    initial_peers: Vec<String>,
) -> Result<(), String> {
    // トピックへの参加
}

#[tauri::command]
pub async fn broadcast_to_topic(
    state: State<'_, AppState>,
    topic_id: String,
    content: String,
) -> Result<(), String> {
    // メッセージのブロードキャスト
}

#[tauri::command]
pub async fn get_p2p_status(
    state: State<'_, AppState>,
) -> Result<P2PStatus, String> {
    // P2P接続状態の取得
}
```

#### TypeScript API

```typescript
// src/lib/api/p2p.ts

export interface P2PStatus {
  connected: boolean;
  endpoint_id: string;
  active_topics: TopicStatus[];
  peer_count: number;
}

export interface TopicStatus {
  topic_id: string;
  peer_count: number;
  message_count: number;
  last_activity: number;
}

export const p2pApi = {
  initialize: () => invoke<void>('initialize_p2p'),
  
  joinTopic: (topicId: string, peers?: string[]) => 
    invoke<void>('join_p2p_topic', { topicId, initialPeers: peers || [] }),
    
  broadcast: (topicId: string, content: string) =>
    invoke<void>('broadcast_to_topic', { topicId, content }),
    
  getStatus: () => invoke<P2PStatus>('get_p2p_status'),
  
  leaveTopic: (topicId: string) =>
    invoke<void>('leave_p2p_topic', { topicId }),
};
```

### 3. 実装の重要ポイント

#### エラーハンドリング

```rust
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("Failed to initialize endpoint: {0}")]
    EndpointInit(#[from] iroh::net::endpoint::Error),
    
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    #[error("Failed to broadcast message: {0}")]
    BroadcastFailed(String),
    
    #[error("Invalid peer address: {0}")]
    InvalidPeerAddr(String),
    
    #[error("Gossip error: {0}")]
    Gossip(String),
    
    #[error("Invalid signature")]
    InvalidSignature,
}
```

#### 状態管理

```rust
pub struct P2PState {
    /// GossipManager instance
    manager: Arc<RwLock<Option<GossipManager>>>,
    
    /// Active topic subscriptions
    topics: Arc<RwLock<HashMap<String, TopicState>>>,
    
    /// Message event channel
    event_tx: mpsc::UnboundedSender<P2PEvent>,
}

pub struct TopicState {
    handle: TopicHandle,
    peers: HashSet<PublicKey>,
    stats: TopicStats,
}
```

#### イベント処理（v0.90.0対応）

```rust
#[derive(Clone, Debug)]
pub enum P2PEvent {
    MessageReceived {
        topic_id: String,
        message: GossipMessage,
    },
    // iroh-gossip v0.90.0: NeighborUp/NeighborDown使用
    PeerJoined {
        topic_id: String,
        peer_id: PublicKey,
    },
    PeerLeft {
        topic_id: String,
        peer_id: PublicKey,
    },
    TopicJoined {
        topic_id: String,
    },
    TopicLeft {
        topic_id: String,
    },
}
```

### 4. テスト戦略

#### 単体テスト例

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_gossip_manager_initialization() {
        let secret_key = SecretKey::generate();
        let manager = GossipManager::new(secret_key).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_topic_join_leave() {
        let manager = create_test_manager().await;
        let topic_id = "test-topic";
        
        // Join topic
        let result = manager.join_topic(topic_id, vec![]).await;
        assert!(result.is_ok());
        
        // Leave topic
        let result = manager.leave_topic(topic_id).await;
        assert!(result.is_ok());
    }
}
```

#### 統合テスト

```rust
// tests/p2p_integration.rs

#[tokio::test]
async fn test_peer_to_peer_messaging() {
    // 1. 2つのノードを作成
    let node1 = create_test_node("node1").await;
    let node2 = create_test_node("node2").await;
    
    // 2. 両ノードが同じトピックに参加
    let topic = "test-topic";
    node1.join_topic(topic, vec![node2.addr()]).await.unwrap();
    node2.join_topic(topic, vec![]).await.unwrap();
    
    // 3. メッセージを送信
    let message = "Hello from node1";
    node1.broadcast(topic, message).await.unwrap();
    
    // 4. 受信を確認
    let received = node2.receive_message(topic).await.unwrap();
    assert_eq!(received.content, message);
}
```

### 5. パフォーマンス最適化

#### メッセージキャッシュ

```rust
use lru::LruCache;

pub struct MessageCache {
    cache: Arc<Mutex<LruCache<MessageId, Instant>>>,
    ttl: Duration,
}

impl MessageCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            ttl,
        }
    }
    
    pub fn is_duplicate(&self, id: &MessageId) -> bool {
        let mut cache = self.cache.lock().unwrap();
        
        if let Some(&timestamp) = cache.get(id) {
            if timestamp.elapsed() < self.ttl {
                return true;
            }
        }
        
        cache.put(*id, Instant::now());
        false
    }
}
```

#### 接続プール管理

```rust
pub struct ConnectionPool {
    max_connections_per_topic: usize,
    connections: Arc<RwLock<HashMap<String, Vec<Connection>>>>,
}

impl ConnectionPool {
    pub async fn get_or_create_connection(
        &self,
        topic: &str,
        peer: &NodeAddr,
    ) -> Result<Connection> {
        // 既存の接続を再利用するか、新規作成
    }
    
    pub async fn prune_inactive(&self) {
        // 非アクティブな接続を定期的に削除
    }
}
```

### 6. UI統合詳細

#### P2P Store (Zustand)

```typescript
// src/stores/p2pStore.ts

interface P2PState {
  initialized: boolean;
  status: P2PStatus | null;
  topics: Map<string, TopicStatus>;
  error: string | null;
}

interface P2PActions {
  initialize: () => Promise<void>;
  joinTopic: (topicId: string, peers?: string[]) => Promise<void>;
  leaveTopic: (topicId: string) => Promise<void>;
  broadcast: (topicId: string, content: string) => Promise<void>;
  updateStatus: () => Promise<void>;
}

export const useP2PStore = create<P2PState & P2PActions>()((set, get) => ({
  initialized: false,
  status: null,
  topics: new Map(),
  error: null,
  
  initialize: async () => {
    try {
      await p2pApi.initialize();
      set({ initialized: true, error: null });
      await get().updateStatus();
    } catch (error) {
      set({ error: error.message });
    }
  },
  
  // ... その他のアクション
}));
```

#### P2P状態表示コンポーネント

```tsx
// src/components/P2PStatus.tsx

export function P2PStatus() {
  const { status, topics } = useP2PStore();
  
  if (!status) return null;
  
  return (
    <div className="p-4 border rounded-lg">
      <h3 className="font-semibold mb-2">P2P ネットワーク</h3>
      <div className="space-y-1 text-sm">
        <div>状態: {status.connected ? '接続中' : '切断'}</div>
        <div>ピア数: {status.peer_count}</div>
        <div>アクティブトピック: {status.active_topics.length}</div>
      </div>
      
      {status.active_topics.map(topic => (
        <div key={topic.topic_id} className="mt-2 p-2 bg-gray-50 rounded">
          <div className="font-medium">{topic.topic_id}</div>
          <div className="text-xs text-gray-600">
            ピア: {topic.peer_count} | メッセージ: {topic.message_count}
          </div>
        </div>
      ))}
    </div>
  );
}
```

### 7. 設定とデプロイメント

#### 環境変数

```env
# .env
KUKURI_P2P_ENABLED=true
KUKURI_P2P_PORT=4001
KUKURI_P2P_DISCOVERY_URL=https://discovery.iroh.network/
KUKURI_P2P_MAX_PEERS_PER_TOPIC=20
KUKURI_P2P_MESSAGE_TTL=3600
```

#### ビルド設定

```toml
# Cargo.toml features
[features]
default = ["p2p"]
p2p = ["iroh", "iroh-gossip", "iroh-net"]
```

## デバッグとトラブルシューティング

### ログ設定

```rust
// 詳細なP2Pログを有効化
env_logger::builder()
    .filter_module("iroh", log::LevelFilter::Debug)
    .filter_module("iroh_gossip", log::LevelFilter::Debug)
    .filter_module("kukuri::p2p", log::LevelFilter::Trace)
    .init();
```

### よくある問題と対処法

1. **接続が確立できない**
   - ファイアウォール設定を確認
   - NAT越えのためのSTUNサーバー設定を確認
   - 初期ピアのアドレスが正しいか確認

2. **メッセージが届かない**
   - トピックIDが一致しているか確認
   - ピア間の接続状態を確認
   - メッセージサイズが制限内か確認

3. **パフォーマンスが低い**
   - 接続ピア数を調整
   - メッセージキャッシュサイズを増やす
   - 不要なトピックから離脱

## 成功指標

- [x] 10ピア間でのメッセージ配信成功率 > 99% ✅
- [x] メッセージ配信遅延 < 100ms（ローカルネットワーク） ✅
- [ ] メモリ使用量 < 100MB（1000メッセージ/分）
- [ ] CPU使用率 < 10%（通常運用時）

### 達成済み機能
- メッセージの署名検証機能
- 重複メッセージの自動除外
- Tauriイベント統合
- マルチノード通信の動作確認

## 次のステップ

実装完了後：
1. 大規模ネットワークでのテスト
2. モバイル環境での最適化
3. 高度な機能の実装（ファイル共有、CRDT同期等）