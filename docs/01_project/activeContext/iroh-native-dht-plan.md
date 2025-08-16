# Kukuri: irohビルトインDHTディスカバリー活用計画

## 更新日: 2025年08月16日

## 1. 背景と変更理由

### 1.1 従来の計画
- distributed-topic-trackerを使用した分散型ブートストラップ
- 外部ライブラリ依存によるDHT実装

### 1.2 新しいアプローチ
- **irohのビルトインDHTディスカバリー機能を活用**
- 公式ドキュメント: https://www.iroh.computer/docs/concepts/discovery
- distributed-topic-trackerは不要（削除予定）

### 1.3 メリット
- 依存関係の削減
- irohとのより良い統合
- 公式サポートとドキュメント
- メンテナンスの簡素化

## 2. irohディスカバリー機能の概要

### 2.1 利用可能なディスカバリーメカニズム

1. **DNSディスカバリー** (`discovery_n0()`)
   - Number 0の公開DNSサーバーを使用
   - デフォルトで有効
   - 現在kukuriで使用中

2. **ローカルディスカバリー**
   - mDNSライクなシステム
   - ローカルネットワーク内のみ
   - 明示的な有効化が必要

3. **Pkarrディスカバリー**
   - HTTPベースのPkarrリレーサーバー
   - NodeIdの解決に使用

4. **DHTディスカバリー** (`discovery_dht()`)
   - BitTorrent Mainline DHT使用
   - デフォルトでは無効
   - "discovery-pkarr-dht"フィーチャーフラグが必要

### 2.2 現在の実装状況
```rust
// 現在のkukuri実装 (iroh_network_service.rs)
let endpoint = Endpoint::builder()
    .secret_key(secret_key)
    .discovery_n0()  // DNSディスカバリーのみ
    .bind()
    .await?;
```

## 3. 実装計画

### 3.1 Phase 1: DHTディスカバリーの有効化

#### Cargo.tomlの更新
```toml
[dependencies]
# distributed-topic-trackerを削除
# distributed-topic-tracker = "0.1.1"  # 削除

# irohにDHTフィーチャーを追加
iroh = { version = "0.91.1", features = ["discovery-pkarr-dht"] }
iroh-gossip = "0.91.0"
```

#### エンドポイント初期化の更新
```rust
// src-tauri/src/infrastructure/p2p/iroh_network_service.rs
let endpoint = Endpoint::builder()
    .secret_key(secret_key)
    .discovery_n0()      // DNSディスカバリー（プライマリ）
    .discovery_dht()     // DHTディスカバリー（追加）
    .bind()
    .await?;
```

### 3.2 Phase 2: ブートストラップノード設定

#### 設定ファイルの活用
```rust
// src-tauri/src/shared/config.rs
pub struct NetworkConfig {
    pub bootstrap_peers: Vec<String>,  // 初期ピアリスト
    pub enable_dht: bool,              // DHT有効化フラグ
    pub enable_dns: bool,              // DNS有効化フラグ
    pub enable_local: bool,            // ローカル発見有効化フラグ
    // ...
}
```

#### 動的エンドポイント構築
```rust
pub async fn create_endpoint(
    secret_key: iroh::SecretKey,
    config: &NetworkConfig
) -> Result<Endpoint, AppError> {
    let mut builder = Endpoint::builder()
        .secret_key(secret_key);
    
    if config.enable_dns {
        builder = builder.discovery_n0();
    }
    
    if config.enable_dht {
        builder = builder.discovery_dht();
    }
    
    if config.enable_local {
        // ローカルディスカバリーの有効化（APIによる）
    }
    
    builder.bind().await
        .map_err(|e| AppError::P2PError(format!("Failed to bind endpoint: {:?}", e)))
}
```

### 3.3 Phase 3: フォールバック機構の改善

#### ハイブリッドアプローチ
1. **プライマリ**: DHT + DNSディスカバリー
2. **セカンダリ**: 設定ファイルのブートストラップピア
3. **ターシャリ**: ローカルディスカバリー（同一ネットワーク）

#### 実装例
```rust
// src-tauri/src/infrastructure/p2p/dht_bootstrap.rs
pub async fn bootstrap_with_fallback(
    endpoint: &Endpoint,
    config: &NetworkConfig,
) -> Result<Vec<NodeAddr>, AppError> {
    // 1. DHTディスカバリーが自動的に動作
    
    // 2. 設定ファイルのブートストラップピアに接続
    if !config.bootstrap_peers.is_empty() {
        for peer_addr in &config.bootstrap_peers {
            if let Ok(node_addr) = parse_node_addr(peer_addr) {
                let _ = endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await;
            }
        }
    }
    
    // 3. 接続状態を確認
    // ...
    
    Ok(vec![])
}
```

## 4. 移行手順

### 4.1 削除対象
- [ ] distributed-topic-trackerの依存関係
- [ ] `use distributed_topic_tracker::*`のインポート
- [ ] AutoDiscoveryBuilderベースのコード

### 4.2 更新対象
- [ ] Cargo.toml - irohフィーチャーフラグ追加
- [ ] iroh_network_service.rs - DHTディスカバリー有効化
- [ ] dht_bootstrap.rs - フォールバック機構の実装
- [ ] config.rs - DHT関連設定の追加

### 4.3 新規追加
- [ ] ブートストラップピアの初期リスト（設定ファイル）
- [ ] DHT状態監視メトリクス
- [ ] ディスカバリーメソッドの切り替えUI（オプション）

## 5. テスト計画

### 5.1 ユニットテスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dht_discovery_enabled() {
        // DHT有効化の確認
    }
    
    #[tokio::test]
    async fn test_fallback_to_bootstrap_peers() {
        // フォールバック動作の確認
    }
}
```

### 5.2 統合テスト
- Docker環境での複数ノード起動
- DHT経由でのピア発見確認
- メッセージ交換の検証

## 6. リスクと対策

### 6.1 リスク
- DHTディスカバリーの初回接続遅延
- ファイアウォール/NAT traversalの問題
- DHT spamming攻撃への脆弱性

### 6.2 対策
- 複数ディスカバリーメソッドの併用
- リレーサーバーの活用
- レート制限とピア検証

## 7. 今後の拡張

### 7.1 短期
- Pkarrリレーサーバーの自前ホスティング
- ブートストラップピアの動的更新

### 7.2 中長期
- カスタムディスカバリーサービスの実装
- モバイル最適化（バッテリー効率）
- プライベートDHTネットワークのサポート

## 8. 参考資料
- [iroh Discovery Documentation](https://www.iroh.computer/docs/concepts/discovery)
- [BitTorrent DHT BEP-5](https://www.bittorrent.org/beps/bep_0005.html)
- [Pkarr Project](https://github.com/number0/pkarr)

## 9. 結論
irohのビルトインDHTディスカバリー機能を活用することで、外部依存を減らしながら、より堅牢な分散型ピア発見を実現できます。段階的な移行により、リスクを最小化しつつ、完全分散型のアーキテクチャへ移行します。