# Kukuri: irohビルトインDHTディスカバリー活用計画

## 最終更新日: 2025年09月15日

## 1. 背景と変更理由

### 1.1 従来の計画
- distributed-topic-trackerを使用した分散型ブートストラップ
- 外部ライブラリ依存によるDHT実装

### 1.2 新しいアプローチ
- irohのビルトインDHTディスカバリー機能を活用
- 公式ドキュメント: https://www.iroh.computer/docs/concepts/discovery
- distributed-topic-trackerは不要（削除済み/非推奨化済み）

### 1.3 メリット
- 依存関係の削減
- irohとのより良い統合
- 公式サポートとドキュメント
- メンテナンスの簡素化

### 1.4 本フェーズの方針（2025年09月15日 更新）
- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマ準拠）。

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
   - BitTorrent Mainline DHTを利用
   - デフォルトでは無効（コードで明示的に有効化が必要）
   - "discovery-pkarr-dht" フィーチャーフラグが必要（Cargoで設定済み）

### 2.2 現在の実装状況（2025年09月15日時点）
- Cargo.toml:
  - iroh = { version = "0.93.1", features = ["discovery-pkarr-dht"] }（設定済み）
  - iroh-gossip = "0.93.1"
  - distributed-topic-tracker はコメントアウト済み（非推奨化）
- エンドポイント初期化: `discovery_n0()` + `discovery_dht()` を併用（有効化済み）。
  ```rust
  // kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs
  let endpoint = Endpoint::builder()
      .secret_key(secret_key)
      .discovery_n0()      // n0の公開ディスカバリー
      .discovery_dht()     // BitTorrent Mainline DHT を利用
      .bind()
      .await?;
  ```
- DHT統合（最小）: `dht_bootstrap.rs` の `DhtGossip` で `subscribe` ベースの join は実装済み。
  - `leave_topic` / `broadcast` は TODO（API連動の意味整理待ち）。
- ブートストラップ設定: `bootstrap_config.rs` 実装済み（`bootstrap_nodes.json` 読み込み、ソケットアドレスリスト）。
- Gossip移行: 旧 `GossipManager` は廃止、`IrohGossipService` へ移行完了（進捗レポート参照）。
 - Nostrリレー接続: 現時点では未接続（本フェーズの方針に基づく）。

追加（本日反映）:
- DHTディスカバリー: `discovery_n0()` と `discovery_dht()` を併用するよう有効化済み。
- DHT Gossip API: `DhtGossip` に `leave_topic`（Senderドロップ）/`broadcast`（Sender利用、未参加時は自動参加）を実装。
- ブートストラップUI: 設定画面に `BootstrapConfigPanel` を追加し、UIから `node_id@host:port` を保存可能。
  - Tauriコマンド: `get_bootstrap_config` / `set_bootstrap_nodes` / `clear_bootstrap_nodes`
  - 保存先: ユーザーデータ配下 `user_bootstrap_nodes.json`
  - フォールバック優先順: ユーザー設定 → 同梱 `bootstrap_nodes.json` → なし（= n0 に依存）
  - development の同梱JSONは `nodes: []`（n0優先運用）

関連:
- `docs/01_project/activeContext/tasks/status/in_progress.md`（残タスクの最新ソース）
- 進捗: `docs/01_project/progressReports/2025-09-15_gossip_manager_deprecation_completion.md`

## 3. 実装計画

### 3.1 Phase 1: DHTディスカバリーの有効化（実施済み）

#### Cargo.tomlの更新（実施済み）
```toml
iroh = { version = "0.93.1", features = ["discovery-pkarr-dht"] }
iroh-gossip = "0.93.1"
# distributed-topic-tracker = "0.1.1"  # Deprecated
```

#### エンドポイント初期化の更新（実施済み）
```rust
// src-tauri/src/infrastructure/p2p/iroh_network_service.rs
let endpoint = Endpoint::builder()
    .secret_key(secret_key)
    .discovery_n0()      // DNSディスカバリー（プライマリ）
    .discovery_dht()     // DHTディスカバリー（追加）
    .bind()
    .await?;
```

### 3.2 Phase 2: ブートストラップノード設定（実装済み/要整備）

#### 設定ファイルの活用（設計例）
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

#### 動的エンドポイント構築（設計例）
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

### 3.3 Phase 3: フォールバック機構の改善（継続）

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

## 4. 移行手順（現状と残タスク）

### 4.1 完了済みの削除/非推奨化
- [x] distributed-topic-tracker の依存関係・利用箇所（非推奨化/除去）
- [x] 旧 `GossipManager` の削除（`IrohGossipService` へ移行）

### 4.2 更新対象（未完了/継続）
- [x] Cargo.toml - irohフィーチャーフラグ追加（実施済み）
- [x] iroh_network_service.rs - `discovery_dht()` 有効化（実施済み）
- [ ] dht_bootstrap.rs - quit/broadcast のAPI連動実装（意味整理含む）
- [x] dht_bootstrap.rs - quit/broadcast 実装（`GossipSender` 管理で実装済み）
- [x] bootstrap_nodes.json - 形式定義（NodeId@Addr 推奨）と検証導線（実装済み：Tauri/CLI）
- [x] ブートストラップUI - ユーザー指定を保存/読込（Tauriコマンド + Settings画面）
- [x] config.rs - DHT関連設定の追加（有効化フラグ、優先度）

### 4.3 新規追加（短期）
- [ ] ブートストラップピアの初期リスト（`bootstrap_nodes.json`）
- [x] DHTメトリクス/ログ（tracing, counters, レベル設定）
- [ ] ディスカバリーメソッドの切り替え（設定/起動オプション）
  - 環境変数: `KUKURI_ENABLE_DHT`, `KUKURI_ENABLE_DNS`, `KUKURI_ENABLE_LOCAL`（bool）
  - ブートストラップピア: `KUKURI_BOOTSTRAP_PEERS`（カンマ区切り `nodeid@host:port`）

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
- Docker/ローカルで複数ノード起動（`scripts/start-bootstrap-nodes.ps1`）
- DHT経由でのピア発見確認（`discovery_dht()` 有効時）

### 5.3 スモークテスト（Docker）
- 目的: Tauri起動なしでP2Pの最低限動作（join/broadcast/receive）を検証
- 実行: `docker compose -f docker-compose.test.yml up --build rust-test`（重め）
- 簡易: `docker compose -f docker-compose.test.yml up --build test-runner`（Rust/TSまとめ・最小）
- 設定: `ENABLE_P2P_INTEGRATION=1`（`./scripts/test-docker.ps1 integration` 実行時に `docker-compose.test.yml` の `test-runner`/`rust-test` へ上書き）
- 補足: 受信側で NIP-01/10/19 バリデーションを行い、不正イベントは破棄
- Gossip経由のメッセージ交換の検証（`IrohGossipService`）

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

## 9. 残タスク（集約）
`docs/01_project/activeContext/tasks/status/in_progress.md` を最新版としつつ、本計画に直結する残りを抜粋:
- [ ] iroh-gossip: quit の意味整理と API 連動実装（例: `dht_bootstrap.rs::leave_topic`）
- [ ] iroh-gossip: broadcast の意味整理と API 連動実装（例: `dht_bootstrap.rs::broadcast`）
- [ ] NIPs 準拠イベントスキーマの確定・検証（NIP-01/10/19/30078 など）
- [ ] DHT メトリクス/ログの整備（tracing, counters, レベル設定）

備考:
- Nostr リレー連携（Kukuri ↔ Nostr ブリッジのリレー接続機能）はバックログ。外部インデックスサーバー等の導入時に検討する。

## 10. 結論
irohのビルトインDHTディスカバリーを中核に、DNS/ブートストラップ/ローカル発見をハイブリッドで併用する方針は維持します。Cargo設定は完了済みのため、次はコード側で `discovery_dht()` を有効化し、quit/broadcastの意味整理とメトリクス整備を優先して仕上げます。
- NIP検証方針（受信）: NIP-01（ID/hex）に加え、NIP-10（e/pタグ markerとrelay_url）、NIP-19（note/nevent/npub/nprofile 形式）を最低限検証。`nprofile`はbech32形式の厳格性で代替（TLVの完全検証は今後）。
  - 追記: `nprofile`/`nevent` は bech32 decode + TLV(0x00=32byte)を必須化（最小）。
