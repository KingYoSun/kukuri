# iroh v0.90.0 仕様書

作成日：2025年07月27日

## 概要

irohは、Rustで実装されたピアツーピアQUIC接続を確立するためのライブラリです。直接接続、ホールパンチング、リレーサーバーを使用した高度なネットワーキング技術により、ノード間の安全で効率的な通信を実現します。

## コア機能

### 1. ピアツーピア通信
- **QUICプロトコル**: 低遅延で信頼性の高い通信を提供
- **直接接続**: ホールパンチングによる直接的なピア間通信
- **暗号化接続**: 各ノードの固有キーを使用したTLS暗号化
- **リレーサーバー**: 直接接続が困難な場合の中継機能
- **軽量ストリーム管理**: 双方向・単方向ストリームの効率的な管理

### 2. アーキテクチャ

#### 中心的な構造体
- **`Endpoint`**: 接続管理の中核となる構造体
- **接続確立に必要な情報**:
  1. NodeID（公開鍵ベース）
  2. アドレス情報（リレーURLまたは直接アドレス）
  3. ALPN（Application-Layer Protocol Negotiation）

#### 接続ライフサイクル
1. Endpoint作成とバインド
2. 接続の確立（connect）または受信（accept）
3. ストリームの作成と管理
4. データ転送
5. 接続のクローズ

## 主要モジュール

### 1. `endpoint`モジュール
**役割**: 接続管理とノード間通信の中核機能

**主要構造体**:
- `Endpoint`: ローカルirohノードのメインAPI
- `Builder`: Endpointの設定と作成
- `Connection`: 確立されたQUIC接続
- `Connecting`: 接続試行中の状態管理
- `ConnectOptions`: 接続オプションの設定

**主要機能**:
- ノードへの接続（`connect`）
- 着信接続の受け入れ（`accept`）
- 双方向/単方向ストリームの管理
- ゼロRTT接続のサポート
- トランスポートパラメータの設定

### 2. `discovery`モジュール
**役割**: ノードアドレスの自動検出と解決

**検出メカニズム**:
1. **StaticProvider**: 手動でノードアドレスを管理
2. **DnsDiscovery**: 標準的なDNSルックアップ
3. **PkarrResolver**: 指定されたリレーサーバーからの検索
4. **MdnsDiscovery**: ローカルネットワーク内のノード検出
5. **DhtDiscovery**: Mainline DHTを介した記録の公開/検索

**主要トレイト**:
- `Discovery`: ノード検出メソッドの定義
- `IntoDiscovery`: 構造体をDiscoveryサービスに変換
- `ConcurrentDiscovery`: 複数のDiscoveryサービスの並行実行

**使用例**:
```rust
let ep = Endpoint::builder()
    .add_discovery(PkarrPublisher::n0_dns())
    .add_discovery(DnsDiscovery::n0_dns())
    .bind()
    .await?;
```

### 3. `protocol`モジュール
**役割**: 着信ネットワークリクエストの適切なプロトコルハンドラへのルーティング

**主要コンポーネント**:
- `Router`: 異なるプロトコルハンドラの管理
- `RouterBuilder`: Routerの構築
- `ProtocolHandler`: 着信接続の処理方法を定義

**特徴**:
- 動的なプロトコル登録
- 柔軟な接続処理
- カスタムプロトコル実装のサポート

**使用例**:
```rust
let router = Router::builder(endpoint)
    .accept(b"/my/alpn", Echo)
    .spawn();
```

### 4. `net_report`モジュール
**役割**: ネットワーク接続性と状態の評価

**主要機能**:
- IPv4/IPv6経由でのインターネットアクセスの確認
- NAT状況の調査
- 設定されたリレーノードへの到達可能性の評価
- 包括的なネットワークレポートの生成

**主要構造体**:
- `Report`: ネットワーク評価の全体結果
- `Metrics`: ネットワーク関連の測定値
- `Options`: ネットワークプロービングパラメータ
- `RelayLatencies`: リレーノードへの接続時間測定

### 5. その他のモジュール
- **`dns`**: DNS解決機能
- **`net`**: ネットワーク関連のユーティリティ

## セキュリティ機能

### 1. ノード識別
- 各ノードは固有の`PublicKey`/`NodeId`で識別
- 公開鍵暗号による安全な識別

### 2. 通信の暗号化
- TLSベースの暗号化接続
- エンドツーエンドの暗号化通信

### 3. 認証
- ピア間の相互認証
- 中間者攻撃への耐性

## 統合例

### 基本的なEndpointの作成
```rust
use iroh::{Endpoint, NodeId};

// Endpointの作成
let endpoint = Endpoint::builder()
    .discovery_n0()  // N0 Discoveryサービスを使用
    .bind()          // ローカルアドレスにバインド
    .await?;

// 他のノードへの接続
let connection = endpoint
    .connect(node_id, b"my-protocol")
    .await?;

// 双方向ストリームの開始
let (send, recv) = connection.open_bi().await?;
```

### Discoveryサービスの複数利用
```rust
let endpoint = Endpoint::builder()
    .add_discovery(DnsDiscovery::n0_dns())
    .add_discovery(PkarrPublisher::n0_dns())
    .add_discovery(MdnsDiscovery::new()?)
    .bind()
    .await?;
```

## iroh-gossip v0.90.0との統合

iroh-gossipは、irohの上に構築されたゴシッププロトコル実装です。

### iroh-gossipの主要機能

#### 1. ゴシッププロトコル
- **epidemic broadcast trees**プロトコルベース
- トピックベースのメッセージ配信
- 効率的なメッセージ伝播

#### 2. プロトコル実装
**HyParView（メンバーシップ管理）**:
- Active View: 5ピア（デフォルト）
- Passive View: 30ピア（デフォルト）
- 双方向接続の保証
- 自動的なピア検出と回復

**PlumTree（メッセージ配信）**:
- Eager peers: 積極的なメッセージ転送
- Lazy peers: メッセージハッシュの共有
- ネットワーク遅延に基づく自己最適化

#### 3. 主要構造体
- `State`: プロトコルステートマシン
- `InEvent`/`OutEvent`: 入出力イベント型
- `Message`: プロトコルメッセージ
- `TopicId`: 32バイトのトピック識別子

### 統合例
```rust
use iroh::{Endpoint, Router};
use iroh_gossip::{Gossip, ALPN};

// Endpointの作成
let endpoint = Endpoint::builder()
    .discovery_n0()
    .bind()
    .await?;

// Gossipインスタンスの作成
let gossip = Gossip::builder()
    .spawn(endpoint.clone());

// Routerでゴシッププロトコルを受け入れ
let router = Router::builder(endpoint.clone())
    .accept(ALPN, gossip.clone())
    .spawn();

// トピックへの参加
let topic = TopicId::from([0u8; 32]);
let (send, recv) = gossip.join(topic).await?;

// メッセージの送受信
send.send(b"Hello, P2P world!".to_vec()).await?;
```

## パフォーマンス考慮事項

### 1. 接続管理
- 接続プールの効率的な利用
- アイドル接続のタイムアウト管理
- 接続の再利用

### 2. ストリーム管理
- 適切なストリームタイプの選択（双方向/単方向）
- ストリームの明示的なクローズ
- バックプレッシャーの処理

### 3. Discovery最適化
- 複数のDiscoveryサービスの並行利用
- キャッシュの活用
- 適切なタイムアウト設定

## ライセンス
- iroh: MIT OR Apache-2.0
- iroh-gossip: Apache-2.0 AND MIT

## 参考リンク
- [iroh ドキュメント](https://docs.rs/iroh/latest/iroh/)
- [iroh-gossip ドキュメント](https://docs.rs/iroh-gossip/latest/iroh_gossip/)
- [GitHub リポジトリ](https://github.com/n0-computer/iroh)
- [Crates.io](https://crates.io/crates/iroh)