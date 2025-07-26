# iroh-gossip統合設計

**作成日**: 2025年7月26日  
**最終更新**: 2025年7月26日

## 概要

本ドキュメントは、Kukuriアプリケーションにiroh-gossipを統合し、P2Pイベント配信機能を実装するための設計書です。

## 公式ドキュメント

- **iroh**: https://docs.rs/iroh/latest/iroh/
- **iroh-gossip**: https://docs.rs/iroh-gossip/latest/iroh_gossip/

## アーキテクチャ概要

### 1. 統合レイヤー

```
┌─────────────────────────────────────┐
│         Kukuri Application          │
├─────────────────────────────────────┤
│         Tauri Commands              │
├─────────────────────────────────────┤
│    P2P Event Manager (新規)         │
├─────────────────┬───────────────────┤
│  Nostr Manager  │  Gossip Manager   │
│                 │     (新規)        │
├─────────────────┴───────────────────┤
│  nostr-sdk      │  iroh-gossip     │
└─────────────────┴───────────────────┘
```

### 2. モジュール構成

```
src/modules/
├── nostr/           # 既存のNostrモジュール
├── p2p/             # 新規P2Pモジュール
│   ├── mod.rs
│   ├── gossip_manager.rs    # iroh-gossip管理
│   ├── topic_mesh.rs        # トピックメッシュ管理
│   ├── peer_discovery.rs    # ピア発見機能
│   └── event_sync.rs        # イベント同期
└── event_manager.rs # 既存（P2P統合を追加）
```

## 主要コンポーネント設計

### 1. GossipManager

```rust
pub struct GossipManager {
    endpoint: Endpoint,
    gossip: Gossip,
    router: Router,
    topics: Arc<RwLock<HashMap<String, TopicHandle>>>,
}

impl GossipManager {
    /// 新しいGossipManagerを作成
    pub async fn new(secret_key: SecretKey) -> Result<Self> {
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;
            
        let gossip = Gossip::builder().spawn(endpoint.clone());
        
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();
            
        Ok(Self {
            endpoint,
            gossip,
            router,
            topics: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// トピックに参加
    pub async fn join_topic(&self, topic_id: &str, peers: Vec<NodeAddr>) -> Result<()>
    
    /// メッセージをブロードキャスト
    pub async fn broadcast(&self, topic_id: &str, message: GossipMessage) -> Result<()>
    
    /// トピックから離脱
    pub async fn leave_topic(&self, topic_id: &str) -> Result<()>
}
```

### 2. TopicMesh

```rust
pub struct TopicMesh {
    topic_id: String,
    subscription: GossipSubscription,
    peers: Arc<RwLock<HashSet<PublicKey>>>,
    message_cache: Arc<RwLock<LruCache<MessageId, GossipMessage>>>,
}

impl TopicMesh {
    /// メッセージの受信処理
    pub async fn handle_message(&self, message: GossipMessage) -> Result<()>
    
    /// ピアの接続状態管理
    pub async fn update_peer_status(&self, peer: PublicKey, connected: bool)
    
    /// メッセージの重複チェック
    pub fn is_duplicate(&self, message_id: &MessageId) -> bool
}
```

### 3. EventSync

```rust
pub struct EventSync {
    nostr_manager: Arc<NostrManager>,
    gossip_manager: Arc<GossipManager>,
}

impl EventSync {
    /// NostrイベントをP2Pネットワークに配信
    pub async fn propagate_nostr_event(&self, event: Event) -> Result<()> {
        // 1. イベントをGossipMessage形式に変換
        let message = self.convert_to_gossip_message(event)?;
        
        // 2. 関連するトピックを特定
        let topic_id = self.extract_topic_id(&event)?;
        
        // 3. P2Pネットワークにブロードキャスト
        self.gossip_manager.broadcast(&topic_id, message).await?;
        
        Ok(())
    }
    
    /// P2Pで受信したメッセージをNostrイベントとして処理
    pub async fn handle_gossip_message(&self, message: GossipMessage) -> Result<()>
}
```

## メッセージフォーマット

### GossipMessage

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct GossipMessage {
    /// メッセージID（重複チェック用）
    pub id: MessageId,
    
    /// メッセージタイプ
    pub msg_type: MessageType,
    
    /// ペイロード
    pub payload: Vec<u8>,
    
    /// タイムスタンプ
    pub timestamp: u64,
    
    /// 送信者の公開鍵
    pub sender: PublicKey,
    
    /// 署名
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum MessageType {
    /// Nostrイベント
    NostrEvent,
    
    /// トピック情報の同期
    TopicSync,
    
    /// ピア情報の交換
    PeerExchange,
    
    /// ハートビート
    Heartbeat,
}
```

## トピック設計

### トピックID生成規則

```rust
/// トピックIDの生成
pub fn generate_topic_id(topic_name: &str) -> String {
    // Nostrのトピックタグと互換性のある形式
    format!("kukuri:topic:{}", topic_name.to_lowercase())
}

/// グローバルトピック（全体のタイムライン）
pub const GLOBAL_TOPIC: &str = "kukuri:global";

/// ユーザー固有トピック
pub fn user_topic_id(pubkey: &str) -> String {
    format!("kukuri:user:{}", pubkey)
}
```

## 実装フェーズ

### Phase 1: 基礎実装（2日間）

1. **依存関係の追加**
   ```toml
   [dependencies]
   iroh = "0.90.0"
   iroh-gossip = "0.90.0"
   ```

2. **基本モジュール構造の作成**
   - `p2p/mod.rs`: モジュール定義
   - `p2p/gossip_manager.rs`: 基本的な初期化とシャットダウン

3. **Tauriコマンドの追加**
   - `initialize_p2p`: P2P機能の初期化
   - `get_p2p_status`: 接続状態の取得

### Phase 2: トピック管理（3日間）

1. **トピック参加・離脱機能**
   - `join_topic`: トピックへの参加
   - `leave_topic`: トピックからの離脱
   - トピックごとのピア管理

2. **メッセージング基盤**
   - メッセージフォーマットの実装
   - 署名・検証機能
   - 重複排除メカニズム

### Phase 3: Nostr統合（3日間）

1. **イベント同期**
   - NostrイベントのP2P配信
   - P2P受信イベントのNostr処理
   - 双方向の変換ロジック

2. **ハイブリッド配信**
   - Nostrリレーとの並列送信
   - 配信優先度の管理
   - フォールバック処理

### Phase 4: 最適化とUI統合（2日間）

1. **パフォーマンス最適化**
   - メッセージキャッシュの実装
   - 接続プールの管理
   - 帯域幅の最適化

2. **UI統合**
   - P2P接続状態の表示
   - トピックメッシュの可視化
   - デバッグパネルの追加

## セキュリティ考慮事項

1. **認証とアクセス制御**
   - irohのNodeIdとNostrの公開鍵の紐付け
   - トピック参加時の権限確認
   - メッセージの署名検証

2. **プライバシー保護**
   - トピック参加情報の管理
   - メタデータの最小化
   - 選択的な情報開示

3. **攻撃対策**
   - スパム対策（レート制限）
   - シビル攻撃への対処
   - 悪意のあるピアの排除

## テスト計画

1. **単体テスト**
   - 各モジュールの機能テスト
   - メッセージ変換のテスト
   - エラーハンドリングのテスト

2. **統合テスト**
   - Nostr-P2P間の連携テスト
   - 複数ピア環境でのテスト
   - ネットワーク障害時のテスト

3. **パフォーマンステスト**
   - 大量メッセージ処理
   - 多数ピア接続時の動作
   - メモリ・CPU使用率の測定

## 監視とデバッグ

1. **メトリクス**
   - 接続ピア数
   - メッセージ送受信数
   - トピック参加者数
   - ネットワーク帯域使用量

2. **ログ出力**
   - 接続イベント
   - メッセージ処理
   - エラー情報
   - デバッグ情報

## 今後の拡張可能性

1. **高度な機能**
   - コンテンツベースのルーティング
   - 優先度付きメッセージング
   - 圧縮・暗号化の最適化

2. **統合機能**
   - ファイル共有（iroh-bytes）
   - リアルタイム同期（CRDT）
   - 分散検索インデックス

## リスクと課題

1. **技術的リスク**
   - iroh-gossipの安定性（比較的新しいライブラリ）
   - ネットワーク分断時の挙動
   - スケーラビリティの限界

2. **運用上の課題**
   - 初期ピアの発見方法
   - NAT越えの成功率
   - モバイル環境での接続維持

## まとめ

iroh-gossipの統合により、Kukuriは真の分散型ソーシャルプラットフォームとして機能します。Nostrプロトコルとの併用により、検閲耐性とユーザビリティを両立させた、次世代のP2Pアプリケーションを実現します。