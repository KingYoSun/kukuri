# irohネイティブDHT統合実装ガイド

**作成日**: 2025年08月16日
**最終更新**: 2025年08月16日

> **注意**: distributed-topic-trackerを使った実装から、irohのビルトインDHTディスカバリーへ移行しました。

## 1. 依存関係の追加

### Cargo.toml更新
```toml
# kukuri-tauri/src-tauri/Cargo.toml
[dependencies]
# P2P Networking（DHTフィーチャー付き）
iroh = { version = "0.91.1", features = ["discovery-pkarr-dht"] }
iroh-gossip = "0.91.0"

# その他の依存関係
keyring = "3.6.3"  # シークレット管理用
tracing = "0.1"    # デバッグログ用
```

## 2. DHTディスカバリーの設定

### 2.1 エンドポイント初期化
```rust
// src-tauri/src/infrastructure/p2p/iroh_network_service.rs
use iroh::Endpoint;

pub async fn create_endpoint(secret_key: iroh::SecretKey) -> Result<Endpoint, AppError> {
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .discovery_n0()      // DNSディスカバリー（プライマリ）
        .discovery_dht()     // DHTディスカバリー（BitTorrent Mainline）
        .bind()
        .await?;
    
    Ok(endpoint)
}
```

### 2.2 設定オプション
```rust
// src-tauri/src/shared/config.rs
pub struct NetworkConfig {
    pub enable_dns: bool,        // DNSディスカバリー有効化
    pub enable_dht: bool,        // DHTディスカバリー有効化
    pub enable_local: bool,      // ローカルディスカバリー有効化
    pub bootstrap_peers: Vec<String>, // フォールバックピア
    pub max_peers: u32,
    pub connection_timeout: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enable_dns: true,
            enable_dht: true,    // デフォルトで有効
            enable_local: false,
            bootstrap_peers: vec![],
            max_peers: 50,
            connection_timeout: 30,
        }
    }
}
```

## 3. ブートストラップ実装

### 3.1 フォールバック機構
```rust
// src-tauri/src/infrastructure/p2p/dht_bootstrap.rs
use iroh::{Endpoint, NodeAddr};
use std::str::FromStr;

/// フォールバックノードに接続
pub async fn connect_to_fallback(
    endpoint: &Endpoint,
    bootstrap_peers: &[String],
) -> Result<Vec<NodeAddr>, AppError> {
    let mut connected_nodes = Vec::new();
    
    for node_str in bootstrap_peers {
        match parse_node_addr(node_str) {
            Ok(node_addr) => {
                match endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                    Ok(_) => {
                        info!("Connected to bootstrap node: {}", node_str);
                        connected_nodes.push(node_addr);
                    }
                    Err(e) => {
                        debug!("Failed to connect: {:?}", e);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to parse: {:?}", e);
            }
        }
    }
    
    Ok(connected_nodes)
}

/// ノードアドレスをパース（形式: "NodeId@Address"）
fn parse_node_addr(node_str: &str) -> Result<NodeAddr, AppError> {
    let parts: Vec<&str> = node_str.split('@').collect();
    if parts.len() != 2 {
        return Err(AppError::P2PError("Invalid format".into()));
    }
    
    let node_id = iroh::NodeId::from_str(parts[0])?;
    let socket_addr = parts[1].parse()?;
    
    Ok(NodeAddr::new(node_id).with_direct_addresses([socket_addr]))
}
```

### 3.2 動的エンドポイント構築
```rust
pub async fn create_endpoint_with_config(
    secret_key: iroh::SecretKey,
    config: &NetworkConfig,
) -> Result<Endpoint, AppError> {
    let mut builder = Endpoint::builder()
        .secret_key(secret_key);
    
    // DNSディスカバリー
    if config.enable_dns {
        builder = builder.discovery_n0();
    }
    
    // DHTディスカバリー
    if config.enable_dht {
        builder = builder.discovery_dht();
    }
    
    // ローカルディスカバリー（将来実装）
    // if config.enable_local {
    //     builder = builder.discovery_local();
    // }
    
    let endpoint = builder.bind().await?;
    
    // フォールバックピアに接続
    if !config.bootstrap_peers.is_empty() {
        connect_to_fallback(&endpoint, &config.bootstrap_peers).await?;
    }
    
    Ok(endpoint)
}
```

## 4. Gossipとの統合

### 4.1 Gossipサービス
```rust
use iroh_gossip::net::Gossip;

pub struct DhtGossip {
    gossip: Gossip,
    endpoint: Arc<Endpoint>,
}

impl DhtGossip {
    pub async fn new(endpoint: Arc<Endpoint>) -> Result<Self, AppError> {
        info!("Initializing DHT-integrated Gossip service");
        
        let gossip = Gossip::builder()
            .spawn(endpoint.as_ref().clone());
        
        Ok(Self { gossip, endpoint })
    }
    
    pub async fn join_topic(
        &self,
        topic: &[u8],
        neighbors: Vec<iroh::NodeAddr>,
    ) -> Result<(), AppError> {
        let topic_id = blake3::hash(topic);
        let topic_bytes = *topic_id.as_bytes();
        
        let peer_ids: Vec<_> = neighbors
            .iter()
            .map(|addr| addr.node_id)
            .collect();
        
        self.gossip
            .subscribe(topic_bytes.into(), peer_ids)
            .await?;
        
        info!("Joined DHT topic: {:?}", topic_id);
        Ok(())
    }
}
```

## 5. イベントハンドリング

### 5.1 イベント処理ループ
```rust
use tokio::sync::mpsc;

pub async fn handle_gossip_events(
    mut receiver: mpsc::Receiver<GossipEvent>,
) -> Result<()> {
    while let Some(event) = receiver.recv().await {
        match event {
            GossipEvent::Message { from, content } => {
                handle_message(from, content).await?;
            }
            GossipEvent::PeerConnected(peer) => {
                info!("Peer connected: {:?}", peer);
            }
            GossipEvent::PeerDisconnected(peer) => {
                info!("Peer disconnected: {:?}", peer);
            }
            _ => {}
        }
    }
    Ok(())
}

async fn handle_message(from: NodeId, content: Vec<u8>) -> Result<()> {
    // メッセージ処理ロジック
    // 1. 署名検証
    // 2. デシリアライズ
    // 3. イベント処理
    Ok(())
}
```

## 6. テスト戦略

### 6.1 ユニットテスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dht_discovery_enabled() {
        let config = NetworkConfig {
            enable_dht: true,
            ..Default::default()
        };
        
        // テスト実装
    }
    
    #[tokio::test]
    async fn test_fallback_connection() {
        // フォールバック接続のテスト
    }
}
```

### 6.2 統合テスト
```bash
# Docker環境でのテスト実行
.\scripts\test-docker.ps1 rust

# 複数ノードのシミュレーション
docker-compose up -d
cargo test --test integration
```

## 7. Pkarrリレーサーバーのローカル環境構築

### 7.1 概要
irohのビルトインDHTディスカバリー機能をローカル環境でテストするため、Pkarrリレーサーバーをセットアップします。

### 7.2 セットアップ手順
```bash
# サブモジュールの初期化（初回のみ）
git submodule update --init --recursive

# Pkarrリレーサーバーの起動
docker-compose up -d

# ログの確認
docker-compose logs -f pkarr

# 動作確認
curl http://localhost:8080/health
curl http://localhost:8080/stats

# サーバーの停止
docker-compose down
```

### 7.3 設定ファイル

#### docker-compose.yml
```yaml
services:
  pkarr:
    container_name: pkarr
    build:
      context: ./pkarr
      dockerfile: Dockerfile
    volumes:
      - ./config.toml:/config.toml
      - .pkarr_cache:/cache
    ports:
      - "8080:8080"  # HTTP API port
      - "6881:6881"  # Mainline DHT port
    command: pkarr-relay --config=/config.toml
    restart: unless-stopped
    environment:
      - RUST_LOG=info
```

#### config.toml
```toml
[http]
port = 8080

[mainline]
port = 6881

[cache]
path = "/cache"
size = 100_000
minimum_ttl = 300
maximum_ttl = 86400

[rate_limiter]
behind_proxy = false
burst_size = 10
per_second = 2
```

### 7.4 irohアプリケーション側の設定
```rust
// Pkarrリレーサーバーへの接続（将来実装）
pub async fn connect_to_pkarr(endpoint: &Endpoint) -> Result<()> {
    // Pkarrリレーサーバーは自動的にDHTディスカバリーで発見される
    // 明示的な接続設定は不要
    Ok(())
}
```

## 8. デバッグとトラブルシューティング

### 8.1 ログ設定
```bash
# 環境変数でログレベル設定
RUST_LOG=kukuri=debug,iroh=info,iroh_gossip=info cargo run
```

### 8.2 よくある問題

#### DHTディスカバリーが機能しない
- フィーチャーフラグ `discovery-pkarr-dht` が有効か確認
- ファイアウォール設定を確認
- NAT traversalの問題を確認
- Pkarrリレーサーバーが起動しているか確認（ローカル環境）

#### ピアが見つからない
- ブートストラップノードが正しく設定されているか確認
- DNSディスカバリーも併用する
- ローカルネットワークでテスト
- Pkarrリレーサーバーのログを確認

## 9. パフォーマンス最適化

### 9.1 接続管理
```rust
pub struct ConnectionManager {
    max_peers: usize,
    peer_timeout: Duration,
    // ...
}

impl ConnectionManager {
    pub async fn prune_inactive_peers(&mut self) {
        // 非アクティブなピアを削除
    }
    
    pub async fn optimize_peer_list(&mut self) {
        // ピアリストを最適化
    }
}
```

### 9.2 メトリクス収集
```rust
pub struct DhtMetrics {
    pub peers_discovered: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub discovery_time: Duration,
}
```

## 10. 参考資料

- [iroh Discovery Documentation](https://www.iroh.computer/docs/concepts/discovery)
- [BitTorrent DHT BEP-5](https://www.bittorrent.org/beps/bep_0005.html)
- [iroh API Documentation](https://docs.rs/iroh/latest/iroh/)
- [Pkarr GitHub Repository](https://github.com/Pubky/pkarr)
- [Pkarr Relay Configuration](https://github.com/Pubky/pkarr/blob/main/relay/src/config.example.toml)
- [iroh-gossip Documentation](https://docs.rs/iroh-gossip/latest/iroh_gossip/)

## 11. 移行チェックリスト

- [ ] Cargo.tomlのDHTフィーチャーフラグ追加
- [ ] エンドポイント初期化コードの更新
- [ ] distributed-topic-tracker関連コードの削除
- [ ] フォールバック機構の実装
- [ ] テストの更新と実行
- [ ] ドキュメントの更新