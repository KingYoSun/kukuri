# インフラストラクチャ層の補完実装 - 進捗レポート

**作成日**: 2025年08月13日  
**フェーズ**: インフラストラクチャ層の補完実装

## 概要

新アーキテクチャへの移行の一環として、インフラストラクチャ層の未実装部分を補完しました。KeyManager、SecureStorage、EventDistributorの3つの重要なコンポーネントを実装・移行し、重複するコマンド定義の解消を行いました。

## 実装内容

### 1. KeyManagerの移行

**ファイル**: `infrastructure/crypto/key_manager.rs`

#### 実装内容
- 既存の`modules/auth/key_manager.rs`から新インフラ層への移行
- トレイトベースの設計によるインターフェース定義
- DefaultKeyManager実装の提供
- Nostr SDK統合による鍵管理機能

#### 主な機能
```rust
pub trait KeyManager: Send + Sync {
    async fn generate_keypair(&self) -> Result<KeyPair, Box<dyn std::error::Error>>;
    async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, Box<dyn std::error::Error>>;
    async fn export_private_key(&self, npub: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), Box<dyn std::error::Error>>;
    async fn list_npubs(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}
```

#### 互換性
- 旧インターフェース（`generate`、`login`、`logout`）との互換性維持
- 新旧両方のAPIをサポート

### 2. SecureStorageの統合

**ファイル**: `infrastructure/storage/secure_storage.rs`

#### 実装内容
- 既存の`modules/secure_storage/mod.rs`から移行
- トレイトベースのインターフェース定義
- DefaultSecureStorage実装
- keyringライブラリによるプラットフォーム依存のセキュアストレージ

#### 主な機能
```rust
pub trait SecureStorage: Send + Sync {
    async fn store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error>>;
    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    async fn clear(&self) -> Result<(), Box<dyn std::error::Error>>;
}
```

#### アカウント管理機能
- マルチアカウント対応
- アカウントメタデータの管理
- 現在のアカウント切り替え機能

### 3. EventDistributorの実装

**ファイル**: `infrastructure/p2p/event_distributor.rs`

#### 実装内容
- 新規実装によるイベント配信メカニズム
- 複数の配信戦略のサポート
- 失敗したイベントのリトライ機構

#### 配信戦略
```rust
pub enum DistributionStrategy {
    Broadcast,       // 全ピアに配信
    Gossip,         // Gossipプロトコルで配信
    Direct(String), // 特定のピアに直接配信
    Hybrid,         // NostrとP2Pの両方で配信
    Nostr,          // Nostrリレー経由のみ
    P2P,            // P2Pネットワークのみ
}
```

#### 実装クラス
- `DefaultEventDistributor`: 汎用実装
- `P2PEventDistributor`: P2P専用実装
- `NostrEventDistributor`: Nostr専用実装

### 4. PostCacheServiceの実装

**ファイル**: `infrastructure/cache/post_cache.rs`

#### 実装内容
- 投稿のメモリキャッシュ実装
- トピック別の投稿取得
- 非同期安全な実装（Arc + RwLock）

#### 主な機能
- 投稿の追加・取得・削除
- 複数投稿の一括処理
- トピックIDによるフィルタリング
- キャッシュサイズの管理

### 5. エンティティの補完

**修正ファイル**: 
- `domain/entities/mod.rs`
- `domain/entities/event.rs`

#### 追加エクスポート
- `UserMetadata`
- `UserProfile`
- `EventKind`

### 6. 重複コマンドの解消

**修正ファイル**:
- `lib.rs`: 旧コマンドのコメントアウト
- `presentation/commands/post_commands.rs`: 重複コマンドのコメントアウト
- `presentation/commands/topic_commands.rs`: 重複コマンドのコメントアウト

#### 解消したコマンド
- `create_post`
- `delete_post`
- `like_post`
- `boost_post`
- `create_topic`
- `delete_topic`

## 統計

### ファイル変更
- 新規作成: 3ファイル
  - `infrastructure/cache/post_cache.rs` (155行)
  - `infrastructure/p2p/event_distributor.rs` (367行)
  - `infrastructure/storage/secure_storage.rs` (408行)
- 更新: 8ファイル
- 合計追加行数: 約930行

### テスト
- KeyManager: 8テストケース
- SecureStorage: 5テストケース
- EventDistributor: 8テストケース
- PostCacheService: 5テストケース
- **合計**: 26テストケース

## 課題と今後の作業

### 残存エラー
現在もコンパイルエラーが残っているため、以下の対応が必要：

1. **型のミスマッチ**: state.rsで発生している型エラーの解消
2. **引数の不一致**: 関数呼び出しの引数数を修正
3. **依存関係の整理**: 循環参照や未解決の依存関係の解消

### 次のステップ
1. コンパイルエラーの完全解消
2. 統合テストの実施
3. v2コマンドの完全移行
4. パフォーマンステストの実施

## まとめ

インフラストラクチャ層の主要コンポーネントの実装が完了しました。KeyManager、SecureStorage、EventDistributorの3つの重要な基盤サービスが新アーキテクチャに統合され、クリーンアーキテクチャの原則に従った実装となっています。

重複コマンドの問題も解消し、新旧のアーキテクチャの共存が可能な状態になりました。ただし、まだ一部のコンパイルエラーが残っているため、これらの解消が次の優先タスクとなります。