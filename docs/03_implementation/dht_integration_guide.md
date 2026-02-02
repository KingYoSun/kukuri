# irohネイティブDHT統合実装ガイド

**作成日**: 2025年08月16日
**最終更新**: 2026年02月02日

> **注意**: distributed-topic-tracker を使った実装から、iroh のビルトイン DHT ディスカバリーへ移行済み。

## 1. 依存関係の追加

### Cargo.toml更新
```toml
# kukuri-tauri/src-tauri/Cargo.toml
[dependencies]
# P2P Networking（DHT / Local Discovery）
iroh = { version = "0.95.1", features = ["discovery-pkarr-dht", "discovery-local-network"] }
iroh-gossip = "0.95.0"
```

## 2. DHTディスカバリーの設定

### 2.1 エンドポイント初期化（現行）
```rust
use crate::infrastructure::p2p::DiscoveryOptions;
use iroh::{Endpoint, SecretKey, discovery::static_provider::StaticProvider};
use std::sync::Arc;

pub async fn create_endpoint(
    secret_key: SecretKey,
    options: DiscoveryOptions,
    static_provider: Arc<StaticProvider>,
) -> Result<Endpoint, AppError> {
    let builder = Endpoint::builder().secret_key(secret_key);
    let builder = options.apply_to_builder(builder);
    let builder = builder.discovery(static_provider);
    let endpoint = builder.bind().await?;
    Ok(endpoint)
}
```

### 2.2 設定オプション
`NetworkConfig` がユーザー設定の入口で、内部では `DiscoveryOptions` に変換されます。`enable_dht` は **Mainline DHT** を表し、`DiscoveryOptions.enable_mainline` にマッピングされます。

```rust
// src-tauri/src/shared/config.rs
pub struct NetworkConfig {
    pub bootstrap_peers: Vec<String>,
    pub max_peers: u32,
    pub connection_timeout: u64,
    pub retry_interval: u64,
    pub enable_dht: bool,   // Mainline DHT
    pub enable_dns: bool,
    pub enable_local: bool,
}

// src-tauri/src/infrastructure/p2p/discovery_options.rs
pub struct DiscoveryOptions {
    pub enable_dns: bool,
    pub enable_mainline: bool,
    pub enable_local: bool,
}
```

### 2.3 DiscoveryOptions の適用
```rust
let options = DiscoveryOptions::from(&network_config);
let builder = Endpoint::builder().secret_key(secret_key);
let builder = options.apply_to_builder(builder);
let builder = builder.discovery(static_discovery.clone());
let endpoint = builder.bind().await?;
```

`apply_to_builder()` 内部では以下を使用しています。
- `PkarrPublisher::n0_dns()` / `DnsDiscovery::n0_dns()`
- `DhtDiscovery::builder().n0_dns_pkarr_relay()`
- `MdnsDiscovery::builder()`（enable_local が true の場合）

## 3. ブートストラップ実装

### 3.1 フォールバック機構
- まず `bootstrap_nodes.json` / `user_bootstrap_nodes.json` を確認
- `NodeId@host:port` 形式のみ接続対象とする
- 失敗時はフォールバック用の静的ノードへ接続

### 3.2 bootstrap_nodes.json の仕様（NodeId@host:port 推奨）

- 目的: DHTディスカバリーがつながらない場合のフォールバック接続先を環境別に宣言
- 置き場所: `kukuri-tauri/src-tauri/bootstrap_nodes.json`
- 環境変数（CLI向け）: `KUKURI_BOOTSTRAP_CONFIG` / `KUKURI_ENV` または `ENVIRONMENT`
- スキーマ:
  - ルートに `development`/`staging`/`production` 等の環境キーを持つ
  - 各環境は `description` と `nodes` を持つ
  - nodes のエントリ形式は2種のうち、NodeIdを含む形式のみ採用
    - 推奨（採用される）: `"<node_id>@<host:port>"`
    - 参考（採用しない）: `"<host:port>"`（NodeId 不在のため接続対象外。検証時に警告）

例:
```json
{
  "development": {
    "description": "Local development bootstrap nodes",
    "nodes": [
      "npub1xy...@127.0.0.1:11223",
      "npub1ab...@127.0.0.1:11224"
    ]
  },
  "staging": { "description": "staging", "nodes": [] },
  "production": { "description": "prod", "nodes": [] }
}
```

### 3.3 UIからのブートストラップ指定（推奨ルート）

- 目的: ユーザーが JSON を直接編集せず、UIからカスタムのブートストラップノードを指定
- 既定: ユーザー指定がない限りフォールバックは無効（空）

実装:
- Tauriコマンド: `get_bootstrap_config` / `set_bootstrap_nodes` / `clear_bootstrap_nodes`
- 保存先: ユーザーデータディレクトリ配下 `user_bootstrap_nodes.json`
- 優先順: `ユーザー設定` → `プロジェクト同梱 JSON` → `なし`（n0 による発見へ委譲）

## 4. Gossipとの統合

### 4.1 役割分担（IrohGossipService と DhtGossip）

方針:
- `IrohGossipService`: UI/ドメイン層向けの高レベルAPI（購読/配信/重複排除）
- `DhtGossip`: ネットワーク層の補助（低レベル送信やフォールバック）

利用ルール:
- アプリ機能は `IrohGossipService` を優先使用
- `DhtGossip` はフォールバック接続や低レベル送信で利用

## 5. イベントハンドリング（概略）

```rust
pub async fn handle_p2p_events(
    mut receiver: mpsc::Receiver<GossipApiEvent>,
) -> Result<()> {
    while let Some(event) = receiver.recv().await {
        match event {
            GossipApiEvent::Message { from, content } => {
                handle_message(from, content).await?;
            }
            GossipApiEvent::PeerConnected(peer) => {
                info!("Peer connected: {:?}", peer);
            }
            GossipApiEvent::PeerDisconnected(peer) => {
                info!("Peer disconnected: {:?}", peer);
            }
            _ => {}
        }
    }
    Ok(())
}
```

## 6. テスト戦略

### 6.1 ユニットテスト
- DiscoveryOptions の変換・フラグ確認
- ブートストラップ設定の検証（with_id/socket_only/invalid）

### 6.2 統合テスト
```bash
# WindowsではDocker経由で実行
./scripts/test-docker.ps1 rust
```

## 7. テストとトラブルシューティング

### 7.1 ログ設定
```bash
RUST_LOG=kukuri=debug,iroh=info,iroh_gossip=info cargo run
```

### 7.2 よくある問題

#### DHTディスカバリーが機能しない
- Cargo 側で `discovery-pkarr-dht` が有効か確認
- ファイアウォール/NAT 設定を確認
- DNSディスカバリーも併用する

#### ピアが見つからない
- ブートストラップノードが正しく設定されているか確認
- `NodeId@host:port` の形式で登録されているか確認
- ローカルネットワークでテスト

## 8. 参考資料

- [iroh Discovery Documentation](https://www.iroh.computer/docs/concepts/discovery)
- [BitTorrent DHT BEP-5](https://www.bittorrent.org/beps/bep_0005.html)
- [iroh API Documentation](https://docs.rs/iroh/latest/iroh/)
- [iroh-gossip Documentation](https://docs.rs/iroh-gossip/latest/iroh_gossip/)

## 9. 移行チェックリスト

- [x] Cargo.tomlのDHTフィーチャーフラグ追加
- [x] エンドポイント初期化コードの更新
- [x] distributed-topic-tracker関連コードの削除
- [x] フォールバック機構の実装
- [ ] テストの更新と実行
- [x] ドキュメントの更新
