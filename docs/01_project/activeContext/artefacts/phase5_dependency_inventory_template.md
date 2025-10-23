# Phase 5 依存関係棚卸しテンプレート
最終更新日: 2025年10月23日

## 記入ルール
- `モジュール/コンポーネント`: ファイルまたはモジュールの論理単位（例: `application/services/event_service`）
- `現行配置`: 既存ディレクトリ構造のパス
- `主な依存先`: 代表的な依存モジュールや外部クレート（最大5件を目安）
- `想定レイヤ`: Phase 5 後に配置するレイヤ（Domain/Application/Infrastructure/Presentation/Legacy）
- `切り離し難易度`: Low / Medium / High 等級で評価
- `課題・メモ`: 循環依存や責務分割上の懸念点を記載

## 一覧表
| モジュール/コンポーネント | 現行配置 | 主な依存先 | 想定レイヤ | 切り離し難易度 | 課題・メモ |
| --- | --- | --- | --- | --- | --- |
| AuthService | `application/services/auth_service.rs` | `infrastructure::crypto::KeyManager`, `infrastructure::storage::SecureStorage`, `application::services::{UserService,TopicService}`, `shared::error::AppError` | Application | Medium | 認証と初期トピック参加が同期的に結合しており、イベント駆動化とサービス分離が必要。 |
| EventService | `application/services/event_service/*` | `infrastructure::database::EventRepository`, `infrastructure::crypto::SignatureService`, `infrastructure::p2p::EventDistributor`, `application::services::{SubscriptionStateMachine,SubscriptionStateStore}`, `modules::event::manager::EventManager` | Application | High | Presentation DTO とレガシー EventManager に直接依存。ドメイン用 mapper と `EventGateway` trait を導入して境界を整理する。 |
| OfflineService | `application/services/offline_service.rs` | `modules::offline::{OfflineManager,models::*}`, `shared::error::AppError`, `serde_json::Value`, `async_trait` | Application | High | 旧 OfflineManager の戻り値をそのまま公開しており、変換アダプタと新インフラ層への移行が必須（`artefacts/phase5_offline_adapter_plan.md` を参照）。 |
| P2PService & Builder | `application/services/p2p_service.rs` | `infrastructure::p2p::{NetworkService,GossipService,DiscoveryOptions,IrohNetworkService,IrohGossipService}`, `modules::p2p::events::P2PEvent`, `shared::config::NetworkConfig`, `tokio::sync`, `iroh::SecretKey` | Application | Medium | Iroh 固有型がサービス層にリーク。Builder 内に閉じ込めつつ、P2PEvent をドメインイベントへ置換する。 |
| PostService | `application/services/post_service.rs` | `domain::entities::{Post,Event,User}`, `infrastructure::database::PostRepository`, `infrastructure::p2p::EventDistributor`, `infrastructure::cache::PostCacheService`, `nostr_sdk::Keys` | Application | Medium | `nostr_sdk` 依存を共有ファクトリに集約し、イベント生成を EventService 経由に統一する。 |
| TopicService | `application/services/topic_service.rs` | `domain::entities::Topic`, `infrastructure::database::TopicRepository`, `infrastructure::p2p::GossipService`, `shared::error::AppError` | Application | Medium | Gossip 参加/離脱を直接呼び出すため、P2PService 経由のイベント発行に置き換える。 |
| SyncService | `application/services/sync_service.rs` | `infrastructure::p2p::NetworkService`, `application::services::{PostService,EventService}`, `tokio::sync::RwLock`, `chrono::Utc` | Application | Medium | サービス間の循環参照防止のため、同期オーケストレータ用 trait を別途定義する。 |
| SubscriptionStateMachine / Store | `application/services/subscription_state.rs` | `infrastructure::database::connection_pool::ConnectionPool`, `sqlx`, `chrono`, `shared::error::AppError` | Application | High | SQL を直書きしており、`SubscriptionStateRepository` を新設して再同期ロジックをドメイン値オブジェクトへ切り出す。 |
| UserService | `application/services/user_service.rs` | `domain::entities::{User,UserMetadata}`, `infrastructure::database::UserRepository`, `shared::error::AppError` | Application | Low | Phase 5 ではフォローデータ取得/更新をドメインユースケース化するだけで対応可能。 |
| AppState（legacy aggregator） | `state.rs` | `modules::{auth,event,offline,database,p2p}`, `application::services::*`, `presentation::handlers::*`, `infrastructure::{crypto,database,p2p}`, `tauri::AppHandle` | Legacy | High | 旧モジュールと新サービスが同居。状態管理とサービス DI を分割し、UI への公開は読み取り専用 ViewModel に絞る。 |
| ApplicationContainer | `state/application_container.rs` | `application::services::p2p_service::{P2PService,P2PStack}`, `modules::p2p::P2PEvent`, `shared::config::AppConfig`, `tokio::fs`, `anyhow` | Application | Medium | P2P イベント型の差し替えとメトリクス初期化統合が必要。Phase 5 でブートストラップ専用モジュールへ再配置する。 |
| EventManager | `modules/event/manager` | `application::shared::{default_topics,nostr::EventPublisher}`, `infrastructure::database::EventRepository`, `infrastructure::p2p::GossipService`, `modules::auth::key_manager::KeyManager`, `modules::database::connection::DbPool` | Legacy | High | `tauri::AppHandle` 依存をイベントブロードキャスタ trait に抽象化し、Repository 参照はアプリ層経由に限定する。 |
| OfflineManager | `modules/offline` | `sqlx`, `chrono`, `serde_json`, `uuid`, `modules::database::connection::DbPool` | Legacy | High | バッチ SQL が多数。`infrastructure::offline` に移行し、DTO 変換を `application::shared::offline` に集約する（`artefacts/phase5_offline_adapter_plan.md` を参照）。 |
| Legacy KeyManager | `modules/auth/key_manager.rs` | `nostr_sdk::Keys`, `tokio::sync::RwLock`, `anyhow` | Legacy | Medium | AppState からのみ利用。`infrastructure::crypto::KeyManager` に置換し、旧実装はテスト専用へ縮退。 |
| Legacy Database Connection | `modules/database/connection.rs` | `sqlx`, `std::fs`, `Path`, `tracing` | Legacy | Medium | `ConnectionPool` への全面移行とマイグレーション呼び出し位置の一本化が必要。 |
| SQLiteRepository | `infrastructure/database/sqlite_repository/*` | `sqlx`, `infrastructure::database::ConnectionPool`, `domain::entities::*`, `shared::error::AppError`, `async_trait` | Infrastructure | High | ドメイン構造体を丸ごと import しており、mapper 層で DTO 化して domain 依存を薄くする必要がある。 |
| ConnectionPool | `infrastructure/database/connection_pool.rs` | `sqlx::SqlitePool`, `std::sync::Arc` | Infrastructure | Low | 旧 DbPool 利用箇所をすべて差し替え、環境変数による設定注入をサポートする。 |
| EventDistributor | `infrastructure/p2p/event_distributor.rs` | `domain::entities::Event`, `tokio::sync::mpsc`, `metrics`, `shared::error::AppError` | Infrastructure | Medium | DistributionStrategy を domain 層で定義し、メトリクス発火を共通トレイトにまとめる。 |
| IrohNetworkService | `infrastructure/p2p/iroh_network_service.rs` | `iroh::{Endpoint,protocol::Router}`, `tokio::sync::{RwLock,broadcast}`, `shared::config::NetworkConfig`, `shared::error::AppError`, `super::dht_bootstrap` | Infrastructure | Medium | ネットワークイベント通知を `P2PService` 用イベントバスに統合し、直接 broadcast を expose しない。 |
| IrohGossipService | `infrastructure/p2p/iroh_gossip_service.rs` | `iroh_gossip::{Gossip,GossipSender,GossipTopic}`, `iroh::protocol::Router`, `domain::entities::Event`, `shared::error::AppError`, `tokio::sync::{mpsc,RwLock}` | Infrastructure | Medium | Gossip イベントを domain DTO に変換し、テストモックとの API 差異をなくす。 |
| Gossip Metrics | `infrastructure/p2p/metrics.rs` | `metrics_exporter_prometheus`, `tokio`, `shared::time`, `serde` | Infrastructure | Low | メトリクス登録を ApplicationContainer で一元化し、Phase 5 後の CI 指標へ反映する。 |
| KeyManager（infrastructure） | `infrastructure/crypto/key_manager.rs` | `nostr_sdk`, `secp256k1`, `keyring`, `shared::error::AppError`, `rand_core` | Infrastructure | Medium | SecureStorage と責務が重複。鍵管理とメタデータ更新を別トレイトに分離する。 |
| SecureStorage | `infrastructure/storage/secure_storage.rs` | `keyring`, `serde`, `async_trait`, `chrono`, `anyhow` | Infrastructure | Medium | 永続化スキーマが `AccountMetadata` 固定。domain 値オブジェクトと整合するマイグレーションが必要。 |
| Command Modules (Topics/Posts/Events) | `presentation/commands/*_commands.rs` | `application::services::*`, `presentation::handlers::*`, `state::AppState`, `presentation::dto::*`, `tauri::command` | Presentation | Medium | AppState からの直接 clone に依存。レイヤ分離後は DI でハンドラーを注入し、依存を明示する。 |
| Presentation Handlers | `presentation/handlers/*_handler.rs` | `application::services::*`, `presentation::dto::*`, `shared::error::AppError`, `serde_json` | Presentation | Medium | DTO バリデーションとサービス呼び出しが混在。Mapper/Validator を共通化し、例外処理を `errorHandler` と整合させる。 |

### カバレッジメモ（2025年10月23日）
- Application 層: 9件（AuthService, EventService, OfflineService, P2PService, PostService, TopicService, SyncService, SubscriptionStateMachine/Store, UserService）
- Infrastructure 層: 8件（SQLiteRepository, ConnectionPool, EventDistributor, IrohNetworkService, IrohGossipService, Gossip Metrics, KeyManager, SecureStorage）
- Presentation 層: 2件（Command Modules, Presentation Handlers）
- Legacy 橋渡し: 5件（AppState, EventManager, OfflineManager, Legacy KeyManager, Legacy Database Connection）
- ブートストラップ: ApplicationContainer（P2P スタック初期化）を別管理とし、Phase 5 Workstream A/B の対象に含める。

## 外部クレート棚卸し（主要カテゴリ）
| カテゴリ | 主なクレート | 役割 | Phase 5 観点 |
| --- | --- | --- | --- |
| データベース | `sqlx`（`runtime-tokio-native-tls`,`sqlite`）, `sqlite` | SQLite アクセスとマイグレーション | すべて `ConnectionPool` 経由で利用し、Repository 層に閉じ込める。 |
| P2P / DHT | `iroh`, `iroh-gossip`, `bytes`, `lru` | Mainline DHT・Gossip 通信 | P2PService のビルダー以外から直接呼ばず、trait モックでテスト可能にする。 |
| プロトコル / イベント | `nostr-sdk`, `bech32`, `bincode` | Nostr イベント生成・鍵変換・ペイロード直列化 | EventService 専用の mapper に集約し、フロントへの露出を DTO 層に限定する。 |
| 暗号 / セキュリティ | `secp256k1`, `aes-gcm`, `argon2`, `keyring`, `rand_core` | 鍵生成・暗号化・ストレージ保護 | KeyManager/ SecureStorage の境界を明確化し、フォールバック実装を検討する。 |
| 非同期 / エラーハンドリング | `tokio`, `async-trait`, `anyhow`, `thiserror`, `chrono` | 非同期実行・共通エラーモデル・時刻計算 | Phase 5 後も `shared`/`infrastructure` 層に限定し、presentation 層での `anyhow` 直接使用を禁止する。 |
| 観測 / 運用 | `metrics_exporter_prometheus`, `tracing`, `tracing-subscriber` | メトリクス収集とロギング | ApplicationContainer で初期化を一元化し、CI/ローカル両方の計測パスを確認する。 |

## TODOメモ
- [x] 主要サービス／Repository／コマンドをすべて洗い出す
- [x] 切り離し難易度 High の項目について、対策案を `tauri_app_implementation_plan.md` に記録する
- [x] 依存関係ライブラリ（外部クレート）の棚卸し結果を別途追記する
