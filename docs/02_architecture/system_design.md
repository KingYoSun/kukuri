# kukuri システム設計書

## ドキュメント情報
- **プロジェクト名**: kukuri
- **バージョン**: 1.0
- **作成日**: 2025年7月25日
- **最終更新**: 2025年7月27日
- **目的**: kukuriアプリケーションの技術的アーキテクチャとシステム設計の詳細定義

## 1. システムアーキテクチャ概要

### 1.1 レイヤー構成

```
┌─────────────────────────────────────────────┐
│          Client Layer (Tauri App)           │
│  ┌─────────────┐  ┌──────────────────────┐  │
│  │   UI Layer  │  │   Business Logic     │  │
│  │   (React)   │  │      (Rust)          │  │
│  └─────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────┘
              │                    │
              ▼                    ▼
┌─────────────────────┐  ┌──────────────────┐
│  Discovery Layer    │  │   P2P Network    │
│ (Workers/Container) │  │     (iroh)       │
└─────────────────────┘  └──────────────────┘
              │                    │
              ▼                    ▼
┌─────────────────────────────────────────────┐
│           Marketplace Layer                 │
│   (Search Nodes, Suggestion Nodes)         │
└─────────────────────────────────────────────┘
```

### 1.2 コンポーネント責務

#### クライアント層
- **UIレイヤー**: React/TypeScript/shadcnによるユーザーインターフェース
- **ビジネスロジック**: Rustによるコア機能実装（暗号化、署名、P2P通信）
- **データ管理**: Zustandによる状態管理、Tanstack Queryによるキャッシュ

#### 発見層
- **ピア発見**: トピック別のピアリスト管理
- **登録/検索API**: RESTful/WebSocket API
- **分散運用**: 複数インスタンスによる冗長性

#### P2Pネットワーク層 ✅ 実装完了
- **ゴシッププロトコル**: iroh-gossip v0.90.0によるトピックベース配信 ✅
- **イベント同期**: Nostrイベントの相互変換とハイブリッド配信 ✅
- **メッセージ署名検証**: secp256k1によるメッセージ完全性保証 ✅
- **重複排除**: LRUキャッシュによる効率的な重複メッセージ処理 ✅
- **NAT traversal**: irohによる自動接続確立 ✅
- **UI統合**: P2P状態表示、トピックメッシュ可視化 ✅

#### マーケットプレイス層
- **検索ノード**: 分散インデックス管理
- **サジェストノード**: AI/MLベースの推薦
- **インセンティブ**: トークンエコノミー（将来実装）

## 2. データモデル

### 2.1 Nostrイベント構造

```typescript
interface NostrEvent {
  id: string;              // イベントID（SHA256ハッシュ）
  pubkey: string;          // 公開鍵
  created_at: number;      // UNIXタイムスタンプ
  kind: number;            // イベントタイプ
  tags: string[][];        // タグ配列
  content: string;         // コンテンツ
  sig: string;             // 署名
}

// kukuri固有のイベントタイプ
enum KukuriEventKind {
  TOPIC_CREATE = 30000,    // トピック作成
  TOPIC_POST = 30001,      // トピック内投稿
  TOPIC_REACTION = 30002,  // リアクション
  TOPIC_COMMENT = 30003,   // コメント
}
```

### 2.2 トピックデータ構造

```typescript
interface Topic {
  id: string;              // トピックID
  name: string;            // トピック名
  description: string;     // 説明
  creator: string;         // 作成者公開鍵
  created_at: number;      // 作成日時
  tags: string[];          // タグ
  category: string;        // カテゴリー
  members_count: number;   // メンバー数
}
```

### 2.3 ユーザープロファイル

```typescript
interface UserProfile {
  pubkey: string;          // 公開鍵
  name?: string;           // 表示名
  about?: string;          // 自己紹介
  picture?: string;        // アバターURL
  nip05?: string;          // NIP-05識別子
  created_at: number;      // 作成日時
}
```

### 2.4 ローカルストレージスキーマ

```sql
-- SQLite Database Schema

-- ユーザーテーブル
CREATE TABLE users (
  pubkey TEXT PRIMARY KEY,
  privkey_encrypted TEXT,
  name TEXT,
  about TEXT,
  picture TEXT,
  created_at INTEGER
);

-- トピックテーブル
CREATE TABLE topics (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  creator TEXT NOT NULL,
  created_at INTEGER,
  tags TEXT,  -- JSON配列
  category TEXT
);

-- イベントテーブル
CREATE TABLE events (
  id TEXT PRIMARY KEY,
  pubkey TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  kind INTEGER NOT NULL,
  tags TEXT,  -- JSON配列
  content TEXT,
  sig TEXT NOT NULL,
  topic_id TEXT,
  FOREIGN KEY (topic_id) REFERENCES topics(id)
);

-- インデックス
CREATE INDEX idx_events_pubkey ON events(pubkey);
CREATE INDEX idx_events_created_at ON events(created_at);
CREATE INDEX idx_events_topic_id ON events(topic_id);
```

## 3. API設計

### 3.1 発見層API

#### ピア登録
```http
POST /api/v1/peers/register
Content-Type: application/json

{
  "peer_id": "12D3KooW...",
  "pubkey": "npub1...",
  "topics": ["politics", "technology"],
  "endpoint": "192.168.1.1:8080"
}
```

#### ピア検索
```http
GET /api/v1/peers?topic=politics&limit=50
```

#### WebSocket接続
```javascript
ws://discovery.kukuri.app/ws
// リアルタイムピア更新通知
```

### 3.2 Tauri IPC API

```rust
// Rust側の関数定義
#[tauri::command]
async fn create_nostr_event(
    content: String,
    kind: u32,
    tags: Vec<Vec<String>>
) -> Result<NostrEvent, String> {
    // イベント作成ロジック
}

#[tauri::command]
async fn connect_to_peer(
    peer_id: String,
    endpoint: String
) -> Result<bool, String> {
    // P2P接続ロジック
}
```

### 3.3 P2P通信プロトコル

#### iroh-gossipアーキテクチャ
```rust
// iroh-gossipのトピックベース配信
use iroh_gossip::{Gossip, TopicId};

// Nostrイベントアダプター
struct NostrGossipAdapter {
    gossip: Gossip,
    topics: HashMap<String, TopicId>,
}

impl NostrGossipAdapter {
    // トピックへのイベント配信
    async fn broadcast_event(&self, topic: &str, event: NostrEvent) -> Result<()> {
        let topic_id = self.get_or_create_topic(topic)?;
        let message = GossipMessage::from_nostr_event(event);
        self.gossip.broadcast(topic_id, message).await
    }
    
    // トピックからのイベント受信
    async fn subscribe_topic(&self, topic: &str) -> impl Stream<Item = NostrEvent> {
        let topic_id = self.get_or_create_topic(topic)?;
        self.gossip.subscribe(topic_id)
            .map(|msg| NostrEvent::from_gossip_message(msg))
    }
}

// メッセージ構造
struct GossipMessage {
    event_id: String,
    event_data: Vec<u8>,  // シリアライズされたNostrEvent
    timestamp: u64,
}
```

## 4. セキュリティ設計

### 4.1 鍵管理
- **秘密鍵暗号化**: AES-256-GCMによるローカル暗号化
- **パスワード導出**: Argon2idによるKDF
- **メモリ保護**: 秘密鍵使用後の即座のメモリクリア

### 4.2 通信セキュリティ
- **P2P暗号化**: irohのNoiseプロトコル
- **署名検証**: 全イベントのsecp256k1署名検証
- **レート制限**: DDoS攻撃対策

### 4.3 コンテンツフィルタリング
- **クライアント側フィルタ**: ローカルでのコンテンツ検証
- **スパム対策**: PoW（Proof of Work）要求（オプション）
- **ブロックリスト**: ユーザー/コンテンツのブロック機能

## 5. パフォーマンス設計

### 5.1 データ同期戦略
- **ゴシップベース同期**: iroh-gossipのEagerセット/Lazyセットによる効率的配信
- **トピックベース購読**: 関心のあるトピックのみを選択的に同期
- **イベント永続化**: 受信イベントのローカルSQLite保存
- **履歴同期**: 新規ピア参加時の過去イベント提供（追加実装）

### 5.2 キャッシュ戦略
- **メモリキャッシュ**: Tanstack Queryによる自動キャッシュ
- **ディスクキャッシュ**: SQLiteによる永続化
- **CDN活用**: 静的コンテンツのエッジ配信

### 5.3 最適化技術
- **仮想スクロール**: 大量投稿の効率的表示
- **遅延読み込み**: 画像/動画の遅延ロード
- **WebAssembly**: 暗号処理の高速化

## 6. 状態管理設計

### 6.1 Zustandストア構造

```typescript
interface AppState {
  // ユーザー状態
  currentUser: UserProfile | null;
  privateKey: string | null;  // 暗号化済み
  
  // トピック状態
  topics: Map<string, Topic>;
  currentTopic: string | null;
  
  // イベント状態
  events: Map<string, NostrEvent>;
  
  // P2P状態
  peers: Map<string, PeerInfo>;
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  
  // アクション
  login: (privateKey: string) => Promise<void>;
  createTopic: (name: string, description: string) => Promise<Topic>;
  postToTopic: (topicId: string, content: string) => Promise<void>;
  connectToPeer: (peerId: string) => Promise<void>;
}
```

### 6.2 データフロー

```
User Action → Zustand Action → Tauri Command → Rust Backend
                    ↓                               ↓
                UI Update ← Zustand State ← Event/Response
```

## 7. エラーハンドリング

### 7.1 エラー分類
```typescript
enum ErrorType {
  NETWORK_ERROR = 'NETWORK_ERROR',
  AUTH_ERROR = 'AUTH_ERROR',
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  STORAGE_ERROR = 'STORAGE_ERROR',
  P2P_ERROR = 'P2P_ERROR',
}

interface AppError {
  type: ErrorType;
  message: string;
  code?: string;
  details?: any;
}
```

### 7.2 リトライ戦略
- **指数バックオフ**: ネットワークエラー時の再試行
- **サーキットブレーカー**: 連続失敗時の一時停止
- **フォールバック**: 代替ピア/サービスへの切り替え

## 8. モニタリングとログ

### 8.1 メトリクス収集
- **パフォーマンスメトリクス**: 応答時間、スループット
- **エラーレート**: エラー発生頻度
- **ユーザーメトリクス**: アクティブユーザー数、セッション時間

### 8.2 ログレベル
```rust
#[derive(Debug)]
enum LogLevel {
    ERROR,   // エラー情報
    WARN,    // 警告情報
    INFO,    // 一般情報
    DEBUG,   // デバッグ情報
    TRACE,   // 詳細トレース
}
```

## 9. 拡張性設計

### 9.1 プラグインアーキテクチャ
- **拡張ポイント**: カスタムイベントハンドラー
- **APIフック**: 処理前後のフック機能
- **テーマシステム**: UIカスタマイズ

### 9.2 マイクロサービス対応
- **サービス分離**: 検索、サジェスト、分析の独立化
- **メッセージング**: イベント駆動アーキテクチャ
- **スケールアウト**: 水平スケーリング対応

## 10. デプロイメントアーキテクチャ

### 10.1 クライアント配布
- **デスクトップ**: Tauriによるネイティブバイナリ
- **モバイル**: Tauri v2のモバイルサポート
- **自動更新**: アプリ内アップデート機能

### 10.2 インフラストラクチャ
```yaml
# Docker Compose例
version: '3.8'
services:
  discovery:
    image: kukuri/discovery:latest
    ports:
      - "8080:8080"
    environment:
      - NODE_ENV=production
    
  marketplace-search:
    image: kukuri/search-node:latest
    ports:
      - "8081:8081"
    
  marketplace-suggest:
    image: kukuri/suggest-node:latest
    ports:
      - "8082:8082"
```

## 更新履歴

- 2025年7月28日: P2P実装完了に伴う更新（v1.2）
- 2025年7月26日: iroh-gossip統合に伴う更新（v1.1）
- 2025年7月25日: 初版作成（v1.0）