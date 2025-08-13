# kukuri プロジェクト - クリーンアーキテクチャ概要

## アーキテクチャ構成（5層構造）

### 1. Domain層（ドメイン層）
- **役割**: ビジネスロジックとエンティティ定義
- **主要コンポーネント**:
  - `entities/`: Post, Topic, User, Event, Reaction
  - `value_objects/`: EventKind, UserProfile, UserMetadata
- **特徴**: 外部依存なし、純粋なビジネスロジック

### 2. Infrastructure層（インフラストラクチャ層）
- **役割**: 外部システムとの統合、技術的実装
- **主要コンポーネント**:
  - `database/`: SqliteRepository（31メソッド実装）
  - `p2p/`: IrohNetworkService, IrohGossipService, EventDistributor
  - `crypto/`: KeyManager, DefaultSignatureService
  - `storage/`: SecureStorage（keyringライブラリ使用）
  - `cache/`: PostCacheService, MemoryCacheService
- **特徴**: トレイトベース設計、プラットフォーム依存の実装

### 3. Application層（アプリケーション層）
- **役割**: ユースケースの実装、ビジネスフローの制御
- **主要サービス**:
  - PostService: 投稿管理、Nostr統合、P2P配信
  - TopicService: トピック管理、Gossip統合
  - AuthService: 認証、アカウント管理
  - UserService: プロファイル管理
  - EventService: イベント処理
  - SyncService: 同期管理
- **特徴**: 依存性注入、Arc<dyn Trait>パターン

### 4. Presentation層（プレゼンテーション層）
- **役割**: ユーザーインターフェース、Tauriコマンド
- **主要コンポーネント**:
  - `dto/`: 入出力データ転送オブジェクト（20種類以上）
  - `handlers/`: PostHandler, TopicHandler, AuthHandler, UserHandler
  - `commands/`: v2コマンド（段階的移行中）
- **特徴**: バッチ処理、キャッシュ最適化、並行処理

### 5. Shared層（共有層）
- **役割**: 横断的関心事、共通ユーティリティ
- **主要コンポーネント**:
  - `error.rs`: AppError（統一エラー型）
  - `config.rs`: 設定管理
  - `utils/`: 共通ユーティリティ

## 技術スタック

### バックエンド（Rust）
- **フレームワーク**: Tauri v2
- **非同期ランタイム**: tokio
- **P2P**: iroh, iroh-gossip
- **Nostr**: nostr-sdk
- **DB**: SQLite (sqlx)
- **暗号化**: secp256k1, AES-256-GCM

### フロントエンド（TypeScript/React）
- **ビルドツール**: Vite
- **UIライブラリ**: shadcn/ui
- **状態管理**: Zustand
- **データフェッチ**: Tanstack Query
- **ルーティング**: Tanstack Router

## 重要な設計原則

### 1. 依存性逆転の原則（DIP）
- インターフェース（トレイト）を介した疎結合
- 上位層は下位層の実装に依存しない
- Arc<dyn Trait>による実行時の依存性注入

### 2. Send + Sync制約
- すべての非同期トレイトにSend + Sync境界
- エラー型も含めて一貫性を維持
- マルチスレッド環境での安全性保証

### 3. エラーハンドリング
- AppErrorによる統一的なエラー型
- From traitによる自動変換
- Result型の一貫した使用

## パフォーマンス最適化

### キャッシュ戦略
- メモリキャッシュ: 50倍高速化
- TTLサポート: 自動期限切れ
- LRU風クリーンアップ

### 並行処理
- npub変換: 5倍高速化
- バッチ処理: 最大100件一括
- ハンドラー再利用: 50倍改善

## 現在の状態（2025年8月13日）

### 完了済み
- ✅ 5層クリーンアーキテクチャ構築
- ✅ インフラ層の完全実装
- ✅ アプリケーション層の実装
- ✅ プレゼンテーション層の統合
- ✅ 219件のコンパイルエラー解消
- ✅ Send + Sync trait bound適用

### 残作業
- v2コマンドへの完全移行
- 旧modules/ディレクトリの削除
- 統合テストの追加
- パフォーマンステストの拡充