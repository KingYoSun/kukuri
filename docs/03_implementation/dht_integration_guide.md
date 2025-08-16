# Distributed Topic Tracker統合実装ガイド

**作成日**: 2025年08月16日
**最終更新**: 2025年08月16日

## 1. 依存関係の追加

### Cargo.toml更新
```toml
# kukuri-tauri/src-tauri/Cargo.toml
[dependencies]
# 既存の依存関係
iroh = "0.90.0"
iroh-gossip = "0.90.0"
iroh-net = "0.90.0"
iroh-base = "0.90.0"

# 新規追加
distributed-topic-tracker = "0.1.1"
keyring = "2.0"  # シークレット管理用

# 必要に応じて
tracing = "0.1"  # デバッグログ用
```

## 2. P2Pモジュール実装

### 2.1 ディレクトリ構造
```
src-tauri/src/p2p/
├── mod.rs           # 既存
├── service.rs       # 既存
├── bootstrap.rs     # 新規作成
└── discovery.rs     # 新規作成
```

### 2.2 bootstrap.rs実装
```rust
// src-tauri/src/p2p/bootstrap.rs
use anyhow::Result;
use distributed_topic_tracker::{
    AutoDiscoveryBuilder, 
    AutoDiscoveryGossip, 
    DefaultSecretRotation, 
    TopicId
};
use iroh::net::Endpoint;
use iroh_gossip::GossipEvent;
use tokio::sync::mpsc;
use std::sync::Arc;

pub struct DhtBootstrap {
    gossip: Arc<AutoDiscoveryGossip>,
    event_rx: mpsc::Receiver<GossipEvent>,
}

impl DhtBootstrap {
    pub async fn new(endpoint: Endpoint) -> Result<Self> {
        // AutoDiscoveryBuilder設定
        let builder = AutoDiscoveryBuilder::default()
            .rate_limit(10)      // 分あたり10レコード
            .retries(3)          // 3回リトライ
            .jitter(500)         // 500msジッター
            .timeout(5000);      // 5秒タイムアウト

        // イベントチャンネル作成
        let (event_tx, event_rx) = mpsc::channel(100);

        // AutoDiscoveryGossip生成
        let gossip = AutoDiscoveryGossip::spawn(
            endpoint,
            builder,
            event_tx
        ).await?;

        Ok(Self {
            gossip: Arc::new(gossip),
            event_rx,
        })
    }

    pub async fn join_topic(
        &self,
        topic_name: &str,
        shared_secret: &[u8; 32]
    ) -> Result<()> {
        // トピックID生成
        let topic_id = TopicId::from_bytes(topic_name.as_bytes());
        
        // シークレットローテーション設定
        let secret_rotation = DefaultSecretRotation::new(shared_secret);
        
        // トピックに参加
        self.gossip.join(topic_id, secret_rotation).await?;
        
        tracing::info!("Joined topic: {}", topic_name);
        Ok(())
    }

    pub async fn leave_topic(&self, topic_name: &str) -> Result<()> {
        let topic_id = TopicId::from_bytes(topic_name.as_bytes());
        self.gossip.leave(topic_id).await?;
        
        tracing::info!("Left topic: {}", topic_name);
        Ok(())
    }

    pub async fn broadcast(&self, topic_name: &str, message: Vec<u8>) -> Result<()> {
        let topic_id = TopicId::from_bytes(topic_name.as_bytes());
        self.gossip.broadcast(topic_id, message).await?;
        Ok(())
    }
}
```

### 2.3 discovery.rs実装
```rust
// src-tauri/src/p2p/discovery.rs
use anyhow::Result;
use keyring::Entry;
use std::collections::HashMap;
use tokio::sync::RwLock;

const KEYRING_SERVICE: &str = "kukuri";
const SECRET_KEY_PREFIX: &str = "topic_secret_";

pub struct SecretManager {
    secrets: Arc<RwLock<HashMap<String, [u8; 32]>>>,
}

impl SecretManager {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_secret(&self, topic_name: &str) -> Result<[u8; 32]> {
        // メモリキャッシュから取得
        if let Some(secret) = self.secrets.read().await.get(topic_name) {
            return Ok(*secret);
        }

        // Keyringから取得
        let key_name = format!("{}{}", SECRET_KEY_PREFIX, topic_name);
        let entry = Entry::new(KEYRING_SERVICE, &key_name)?;
        
        let secret = match entry.get_password() {
            Ok(password) => {
                // Base64デコード
                let bytes = base64::decode(&password)?;
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&bytes[..32]);
                secret
            }
            Err(_) => {
                // 新規生成
                let secret = generate_secret();
                let encoded = base64::encode(&secret);
                entry.set_password(&encoded)?;
                secret
            }
        };

        // キャッシュに保存
        self.secrets.write().await.insert(topic_name.to_string(), secret);
        Ok(secret)
    }

    pub async fn rotate_secret(&self, topic_name: &str) -> Result<[u8; 32]> {
        let new_secret = generate_secret();
        
        // Keyringに保存
        let key_name = format!("{}{}", SECRET_KEY_PREFIX, topic_name);
        let entry = Entry::new(KEYRING_SERVICE, &key_name)?;
        let encoded = base64::encode(&new_secret);
        entry.set_password(&encoded)?;
        
        // キャッシュ更新
        self.secrets.write().await.insert(topic_name.to_string(), new_secret);
        
        Ok(new_secret)
    }
}

fn generate_secret() -> [u8; 32] {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut secret = [0u8; 32];
    rng.fill(&mut secret);
    secret
}
```

## 3. Service層への統合

### 3.1 P2PService更新
```rust
// src-tauri/src/services/p2p_service.rs の更新部分

use crate::p2p::bootstrap::DhtBootstrap;
use crate::p2p::discovery::SecretManager;

pub struct P2PService {
    // 既存フィールド
    endpoint: Endpoint,
    gossip: Gossip,
    
    // 新規追加
    dht_bootstrap: Arc<DhtBootstrap>,
    secret_manager: Arc<SecretManager>,
}

impl P2PService {
    pub async fn new() -> Result<Self> {
        let endpoint = Endpoint::builder()
            .discovery(Box::new(DnsDiscovery::default()))
            .bind(0)
            .await?;
        
        // DHT Bootstrap初期化
        let dht_bootstrap = DhtBootstrap::new(endpoint.clone()).await?;
        let secret_manager = SecretManager::new();
        
        // 既存のGossip初期化（変更なし）
        let gossip = Gossip::new(endpoint.clone());
        
        Ok(Self {
            endpoint,
            gossip,
            dht_bootstrap: Arc::new(dht_bootstrap),
            secret_manager: Arc::new(secret_manager),
        })
    }

    pub async fn join_topic(&self, topic_name: &str) -> Result<()> {
        // シークレット取得
        let secret = self.secret_manager.get_or_create_secret(topic_name).await?;
        
        // DHT経由でトピック参加
        self.dht_bootstrap.join_topic(topic_name, &secret).await?;
        
        // 既存のGossipトピック参加（互換性維持）
        let topic_id = TopicId::from_bytes(topic_name.as_bytes());
        self.gossip.join(topic_id, Vec::new()).await?;
        
        Ok(())
    }
}
```

## 4. Tauriコマンド実装

### 4.1 コマンド追加
```rust
// src-tauri/src/commands/p2p.rs

#[tauri::command]
pub async fn init_dht_discovery(
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    let p2p_service = &state.p2p_service;
    
    // デフォルトトピックに自動参加
    let default_topics = vec!["global", "announcements"];
    for topic in default_topics {
        p2p_service.join_topic(topic)
            .await
            .map_err(|e| e.to_string())?;
    }
    
    Ok("DHT discovery initialized".to_string())
}

#[tauri::command]
pub async fn get_peer_count(
    state: tauri::State<'_, AppState>
) -> Result<usize, String> {
    let count = state.p2p_service
        .get_connected_peers()
        .await
        .map_err(|e| e.to_string())?
        .len();
    
    Ok(count)
}

#[tauri::command]
pub async fn rotate_topic_secret(
    topic: String,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    state.p2p_service
        .secret_manager
        .rotate_secret(&topic)
        .await
        .map_err(|e| e.to_string())?;
    
    // 再参加でシークレット更新
    state.p2p_service
        .leave_topic(&topic)
        .await
        .map_err(|e| e.to_string())?;
    
    state.p2p_service
        .join_topic(&topic)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

## 5. イベントハンドリング

### 5.1 イベントループ実装
```rust
// src-tauri/src/p2p/event_handler.rs

use iroh_gossip::GossipEvent;
use tokio::sync::mpsc;

pub async fn handle_dht_events(
    mut event_rx: mpsc::Receiver<GossipEvent>,
    db: Arc<Database>,
    event_bus: Arc<EventBus>
) {
    while let Some(event) = event_rx.recv().await {
        match event {
            GossipEvent::Message { topic, from, content } => {
                handle_message(topic, from, content, &db, &event_bus).await;
            }
            GossipEvent::PeerConnected { topic, peer } => {
                handle_peer_connected(topic, peer, &event_bus).await;
            }
            GossipEvent::PeerDisconnected { topic, peer } => {
                handle_peer_disconnected(topic, peer, &event_bus).await;
            }
            GossipEvent::TopicJoined { topic } => {
                tracing::info!("Joined topic: {:?}", topic);
            }
            GossipEvent::TopicLeft { topic } => {
                tracing::info!("Left topic: {:?}", topic);
            }
        }
    }
}

async fn handle_message(
    topic: TopicId,
    from: PeerId,
    content: Vec<u8>,
    db: &Arc<Database>,
    event_bus: &Arc<EventBus>
) {
    // Nostrイベントとしてデシリアライズ
    match serde_json::from_slice::<NostrEvent>(&content) {
        Ok(event) => {
            // データベースに保存
            db.save_event(&event).await.ok();
            
            // UIに通知
            event_bus.emit("new_event", &event).await;
        }
        Err(e) => {
            tracing::warn!("Failed to parse event: {}", e);
        }
    }
}
```

## 6. フォールバック実装

### 6.1 フォールバック機構
```rust
// src-tauri/src/p2p/fallback.rs

const BOOTSTRAP_NODES: &[&str] = &[
    // 暫定的なブートストラップノードリスト
    "/ip4/1.2.3.4/tcp/4001",
    "/ip4/5.6.7.8/tcp/4002",
];

pub async fn connect_with_fallback(
    dht_bootstrap: &DhtBootstrap,
    topic: &str,
    secret: &[u8; 32]
) -> Result<()> {
    // DHT経由での接続試行
    match dht_bootstrap.join_topic(topic, secret).await {
        Ok(_) => return Ok(()),
        Err(e) => {
            tracing::warn!("DHT connection failed: {}", e);
        }
    }
    
    // ローカルキャッシュから既知ピア取得
    if let Some(cached_peers) = load_cached_peers(topic).await? {
        for peer in cached_peers {
            if try_connect_peer(&peer).await.is_ok() {
                return Ok(());
            }
        }
    }
    
    // ハードコードされたノードへの接続
    for node in BOOTSTRAP_NODES {
        if try_connect_bootstrap(node).await.is_ok() {
            return Ok(());
        }
    }
    
    Err(anyhow::anyhow!("All connection attempts failed"))
}
```

## 7. テスト実装

### 7.1 ユニットテスト
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dht_bootstrap() {
        let endpoint = create_test_endpoint().await;
        let bootstrap = DhtBootstrap::new(endpoint).await.unwrap();
        
        let secret = generate_secret();
        bootstrap.join_topic("test", &secret).await.unwrap();
        
        // ピア発見を待つ
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // ピア数確認
        let peers = bootstrap.get_peers("test").await.unwrap();
        assert!(!peers.is_empty());
    }

    #[tokio::test]
    async fn test_secret_rotation() {
        let manager = SecretManager::new();
        let topic = "test_topic";
        
        let secret1 = manager.get_or_create_secret(topic).await.unwrap();
        let secret2 = manager.rotate_secret(topic).await.unwrap();
        
        assert_ne!(secret1, secret2);
    }
}
```

### 7.2 E2Eテスト（Docker）
```yaml
# tests/docker-compose.yml
version: '3.8'
services:
  dht-node-1:
    build: .
    environment:
      - NODE_ID=1
      - TOPIC=test-topic
    ports:
      - "4001:4001"
  
  dht-node-2:
    build: .
    environment:
      - NODE_ID=2
      - TOPIC=test-topic
    ports:
      - "4002:4002"
    depends_on:
      - dht-node-1
```

## 8. デバッグとログ設定

### 8.1 環境変数
```bash
# .env
RUST_LOG=kukuri=debug,distributed_topic_tracker=debug,iroh_gossip=info
DHT_RATE_LIMIT=10
DHT_RETRIES=3
DHT_TIMEOUT=5000
```

### 8.2 ログ出力
```rust
// main.rsでの初期化
fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}
```

## 9. パフォーマンスチューニング

### 9.1 最適化ポイント
- レート制限の調整（デフォルト10→環境に応じて調整）
- リトライ回数とジッターの最適化
- バックグラウンドパブリッシャーの頻度調整

### 9.2 監視項目
- DHT応答時間
- ピア発見成功率
- メッセージ配信レイテンシ
- メモリ使用量

## 10. トラブルシューティング

### よくある問題と解決方法

#### DHT接続タイムアウト
```rust
// タイムアウト値を増やす
let builder = AutoDiscoveryBuilder::default()
    .timeout(10000);  // 10秒に増加
```

#### シークレット同期エラー
```rust
// 強制的にシークレット再生成
secret_manager.rotate_secret(topic).await?;
```

#### ピア発見失敗
```rust
// フォールバック機構を使用
connect_with_fallback(&dht_bootstrap, topic, &secret).await?;
```

## 次のステップ

1. 実装完了後、`cargo test`でユニットテスト実行
2. Docker ComposeでE2Eテスト実行
3. パフォーマンステスト実施
4. 本番環境へのデプロイ準備

## 参考資料

- [distributed-topic-tracker API Doc](https://docs.rs/crate/distributed-topic-tracker/latest/json)
- [iroh-gossip Documentation](https://docs.rs/iroh-gossip/latest/)
- [実装計画詳細](../01_project/activeContext/distributed-topic-tracker-plan.md)