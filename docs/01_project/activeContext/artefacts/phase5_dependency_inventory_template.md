# Phase 5 依存関係棚卸しテンプレート
最終更新日: 2025年10月31日

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
| AuthService | `application/services/auth_service.rs` | `infrastructure::crypto::KeyManager`, `infrastructure::storage::SecureStorage`, `application::ports::auth_lifecycle::AuthLifecyclePort` | Application | Medium | 2025年10月26日: AuthLifecyclePort/DefaultAuthLifecycle を追加し、User/TopicService への直接依存を排除。アカウント生成とログインはイベント経由でプロビジョニングされる。 |
| EventService | `application/services/event_service/*` | `infrastructure::database::EventRepository`, `infrastructure::crypto::SignatureService`, `infrastructure::p2p::EventDistributor`, `application::services::{SubscriptionStateMachine,SubscriptionStateStore}`, `infrastructure::event::EventManagerHandle` | Application | High | 2025年10月25日 Stage3: EventManagerHandle で Legacy 依存を封じ、`tests/integration/test_event_service_gateway.rs` で Publish/Reactions/Metadata/Delete の Gateway 結合テストを追加。 |
| OfflineService | `application/services/offline_service.rs` | `application::ports::offline_store::OfflinePersistence`, `infrastructure::offline::SqliteOfflinePersistence`, `infrastructure::offline::OfflineReindexJob`, `shared::error::AppError` | Application | Medium | 2025年10月25日 Stage3: Legacy OfflineManager 依存を解消し、ドメイン値オブジェクトでの永続化に移行。テストは新Persistenceベースに更新済み。 |
| P2PService & Builder | `application/services/p2p_service.rs` | `infrastructure::p2p::{NetworkService,GossipService,DiscoveryOptions,IrohNetworkService,IrohGossipService}`, `modules::p2p::events::P2PEvent`, `shared::config::NetworkConfig`, `tokio::sync`, `iroh::SecretKey` | Application | Medium | Iroh 固有型がサービス層にリーク。Builder 内に閉じ込めつつ、P2PEvent をドメインイベントへ置換する。2025年10月27日: `P2PStack` の公開フィールドを trait object 化し、GossipService に `local_peer_hint` を追加してテストを含む外部依存を抽象化。2025年10月31日: ファイル長が797行のため、Peer 管理/DI/テスト補助を別モジュールへ分割するタスクを継続。 |
| PostService | `application/services/post_service.rs` | `domain::entities::{Post,Event,User}`, `application::ports::cache::PostCache`, `infrastructure::database::PostRepository`, `infrastructure::p2p::EventDistributor`, `application::services::event_service::EventServiceTrait` | Application | Medium | 2025年10月27日: `PostCache` ポートを追加し、キャッシュ実装の DI を `AppState` に集約。publish 成功/失敗の整合性をユニットテストで検証。2025年10月31日: トピック別投稿キャッシュの TODO を解消し、トピック単位のフェッチ／再利用を `PostCacheService` に統合済み。 |
| TopicService | `application/services/topic_service.rs` | `domain::entities::Topic`, `infrastructure::database::TopicRepository`, `infrastructure::p2p::GossipService`, `shared::error::AppError` | Application | Medium | Gossip 参加/離脱を直接呼び出すため、P2PService 経由のイベント発行に置き換える。2025年10月27日: `topicStore` / `syncEngine` を Tauri `join_topic` / `leave_topic` コマンド経由に統一し、AppState の UI 購読制御を補完。 |
| SyncService | `application/services/sync_service.rs` | `infrastructure::p2p::NetworkService`, `application::services::{PostService,EventService}`, `tokio::sync::RwLock`, `chrono::Utc` | Application | Medium | 2025年10月27日: `SyncServiceTrait` を導入し、AppState/コマンド層を含む呼び出し元を trait object 経由に統一。循環依存を防止する構成へ移行済み。 |
| SubscriptionStateMachine / Store | `application/services/subscription_state.rs` | `application::ports::subscription_state_repository::SubscriptionStateRepository`, `infrastructure::database::SqliteSubscriptionStateRepository`, `domain::value_objects::subscription`, `shared::error::AppError` | Application | Medium | 2025年10月25日 SSR-01/02 完了: SubscriptionStateRepository ポート＋ SQLite 実装を導入し、`SubscriptionStateMachine` は Repository 経由で遷移管理を行う。SQL 直書きを排除し、再同期バックオフはドメイン値オブジェクトに集約した。 |
| UserService | `application/services/user_service.rs` | `domain::entities::{User,UserMetadata}`, `infrastructure::database::UserRepository`, `shared::error::AppError` | Application | Low | 2025年10月26日: フォロー/アンフォロー処理を `UserRepository` ポート経由で実装し、Self follow 検証や NotFound 応答を含むユースケースを整備。Tauri コマンドは UserService 経由に統一済み。 |
| AppState（legacy aggregator） | `state.rs` | `modules::{auth,event,p2p}`, `infrastructure::offline::{SqliteOfflinePersistence,OfflineReindexJob}`, `application::services::*`, `presentation::handlers::*`, `infrastructure::{crypto,database,p2p}`, `tauri::AppHandle` | Legacy | Medium | 2025年10月26日: P2PBootstrapper へ初期化を委譲し、AppState から Iroh SecretKey/Builder 参照を排除。2025年10月27日: `SyncService` 具象依存を外し、trait object で公開するよう整理。残タスクは Legacy EventManager 依存の Infrastructure 化。 |
| ApplicationContainer | `state/application_container.rs` | `application::services::p2p_service::{P2PService,P2PStack}`, `modules::p2p::P2PEvent`, `shared::config::AppConfig`, `tokio::fs`, `anyhow` | Application | Medium | P2P イベント型の差し替えとメトリクス初期化統合が必要。Phase 5 でブートストラップ専用モジュールへ再配置する。 |
| EventManager | `infrastructure/event/manager` | `application::shared::{default_topics,nostr::EventPublisher}`, `application::ports::event_topic_store::EventTopicStore`, `infrastructure::event::{handler::EventHandler,nostr_client_manager::NostrClientManager}`, `infrastructure::p2p::GossipService`, `application::ports::key_manager::KeyManager`, `infrastructure::database::ConnectionPool` | Infrastructure | Medium | 2025年10月27日: Legacy モジュールを Infrastructure 層へ移設し、Gateway/DI から新パスを参照するよう統合。P2P ブロードキャスト・購読復元のテストは `SQLX_OFFLINE=true cargo test` でグリーンを確認。 |
| SqliteOfflinePersistence | `infrastructure/offline/sqlite_store.rs` | `sqlx`, `chrono`, `serde_json`, `uuid`, `application::ports::offline_store::OfflinePersistence` | Infrastructure | Medium | 2025年10月25日 Stage3: Legacy OfflineManager の SQL を移植。OfflineReindexJob とサービス双方から利用。`.sqlx` 再生成タイミングと DRY 化の追跡を継続。 |
| Legacy KeyManager | ~~`modules/auth/key_manager.rs`~~（2025年10月25日削除） | `nostr_sdk::Keys`, `tokio::sync::RwLock`, `anyhow` | Legacy | 完了 | 2025年10月25日: `application::ports::key_manager` + `DefaultKeyManager` へ完全移行。AppState/Tauri/EventManager/SubscriptionInvoker からの依存を解消。 |
| Legacy Database Connection | `modules/database/connection.rs` | `sqlx`, `std::fs`, `Path`, `tracing` | Retired (2025年10月25日) | - | `ConnectionPool` への移行完了に伴い削除。参照先は `infrastructure::database::connection_pool` に集約済み。 |
| Legacy BookmarkManager | **完了 2025年10月26日**: `modules/bookmark` ディレクトリを削除し、Bookmark API は `PostService` + `BookmarkRepository` 経由へ統一。 | `domain::entities::bookmark`, `infrastructure::database::BookmarkRepository`, `application::services::PostService` | Archived | 完了 | Stage0〜3 の移行とドキュメント更新を完了。`.sqlx` の追加生成は不要で、Runbook/タスクリストにも反映済み。 |
| Legacy SecureStorage Module | ~~`modules/secure_storage`~~（Removed 2025年10月25日） | `keyring`, `serde_json`, `anyhow`, `tokio::sync::RwLock` | Archived | Low | Debug 用 `clear_all_accounts` は `infrastructure::storage::secure_storage::DefaultSecureStorage::clear_all_accounts_for_test` へ統合済み。タスク完了後はドキュメント参照のみ。 |
| EncryptionManager (Legacy) | ~~`modules/crypto/encryption.rs`~~（2025年10月27日削除） | `aes-gcm`, `sha2`, `base64`, `anyhow` | Retired | 完了 | `DefaultEncryptionService` への移行が完了し、Legacy モジュール/テスト/`pub mod crypto` を削除。依存表・タスクリストを更新し dead code を解消。 |
| Crypto Hash Stack | `infrastructure/crypto` | `sha2 0.10`, `aes-gcm 0.10`, `argon2 0.5`, `generic-array 0.14` | Infrastructure | Medium | 2025年10月24日: RustCrypto 系は generic-array 1.x をまだ stable 提供しておらず、`aes-gcm`/`sqlx`/`iroh` の依存も 0.14 系を前提。非推奨警告は `GenericArray::as_slice` 呼び出しを `&*key` 参照へ置換して解消済み。RustCrypto の stable リリースで 1.x 対応が揃い次第、依存引き上げを再評価する。 |
| SQLiteRepository | `infrastructure/database/sqlite_repository/*` | `sqlx`, `infrastructure::database::ConnectionPool`, `domain::entities::*`, `shared::error::AppError`, `async_trait` | Infrastructure | High | 2025年10月27日: Repository トレイトを `application::ports::repositories` へ移し、Sqlite 実装・AppState・EventTopicStore が application ポート経由で動作するよう更新。`domain::repositories` を削除し、インフラ層の domain 直接依存を解消。 |
| ConnectionPool | `infrastructure/database/connection_pool.rs` | `sqlx::SqlitePool`, `std::sync::Arc` | Infrastructure | Low | 旧 DbPool 利用箇所をすべて差し替え、環境変数による設定注入をサポートする。 |
| EventDistributor | `infrastructure/p2p/event_distributor.rs` | `domain::entities::Event`, `tokio::sync::RwLock`, `metrics`, `shared::error::AppError` | Infrastructure | Medium | 2025年10月27日: DistributionStrategy/DistributionMetrics を `domain::p2p` へ移設し、Default/P2P/Nostr distributor から共通メトリクスフックを呼び出す実装に刷新。2025年10月31日: Gossip/Relay/P2P 送信処理の `TODO` が残っており、実装完了とテスト整備が必要。 |
| IrohNetworkService | `infrastructure/p2p/iroh_network_service.rs` | `iroh::{Endpoint,protocol::Router}`, `tokio::sync::{RwLock,broadcast}`, `shared::config::NetworkConfig`, `shared::error::AppError`, `super::dht_bootstrap` | Infrastructure | Medium | 2025年10月27日: `P2PEvent::NetworkConnected/Disconnected` を broadcast 送出し、AppState/OfflineService/EventService が P2P イベントバスから再接続シグナルを購読できるよう統合。 |
| IrohGossipService | `infrastructure/p2p/iroh_gossip_service.rs` | `iroh_gossip::{Gossip,GossipSender,GossipTopic}`, `iroh::protocol::Router`, `domain::entities::Event`, `shared::error::AppError`, `tokio::sync::{broadcast,RwLock,mpsc}` | Infrastructure | Medium | 2025年10月27日: Gossip 受信ループで Domain `P2PEvent` を broadcast 送信し、モック/API 双方が同じイベントバス（broadcast Receiver）を利用する構成に統一。 |
| Gossip Metrics | `infrastructure/p2p/metrics.rs` | `metrics_exporter_prometheus`, `tokio`, `shared::time`, `serde` | Infrastructure | Low | 2025年10月27日: `p2p_metrics_export` バイナリと `scripts/metrics/export-p2p` を追加し、CI から JSON を収集できるようにした。AppState での初期化と CLI エクスポートを Runbook に統合。 |
| KeyManager（infrastructure） | `infrastructure/crypto/key_manager.rs` | `nostr_sdk`, `secp256k1`, `keyring`, `shared::error::AppError`, `rand_core` | Infrastructure | Medium | 2025年10月27日: `KeyMaterialStore` ポートを追加し、DefaultKeyManager は KeyStore 依存＋InMemory 実装を備える構成へ刷新。SecureStorage が同ポートを実装し、鍵保存/削除/現在値の責務を分離。 |
| SecureStorage | `infrastructure/storage/secure_storage.rs` | `keyring`, `serde`, `async_trait`, `chrono`, `anyhow` | Infrastructure | Medium | 2025年10月27日: `KeyMaterialLedger`/`KeyMaterialRecord` を導入し、鍵台帳を Domain 値オブジェクトで永続化。アカウント追加/切替/削除と KeyManager 保存が同じレコードを更新する構成に統一。 |
| Command Modules (Topics/Posts/Events) | `presentation/commands/*_commands.rs` | `application::services::*`, `presentation::handlers::*`, `state::AppState`, `presentation::dto::*`, `tauri::command` | Presentation | Medium | AppState からの直接 clone に依存。レイヤ分離後は DI でハンドラーを注入し、依存を明示する。 |
| Presentation Handlers | `presentation/handlers/*_handler.rs` | `application::services::*`, `presentation::dto::*`, `shared::error::AppError`, `serde_json` | Presentation | Medium | DTO バリデーションとサービス呼び出しが混在。Mapper/Validator を共通化し、例外処理を `errorHandler` と整合させる。 |

### カバレッジメモ（2025年10月23日）
- Application 層: 9件（AuthService, EventService, OfflineService, P2PService, PostService, TopicService, SyncService, SubscriptionStateMachine/Store, UserService）
- Infrastructure 層: 8件（SQLiteRepository, ConnectionPool, EventDistributor, IrohNetworkService, IrohGossipService, Gossip Metrics, KeyManager, SecureStorage）
- Presentation 層: 2件（Command Modules, Presentation Handlers）
- Legacy 橋渡し: 3件（AppState, EventManager, Legacy Database Connection）※Legacy KeyManager は 2025年10月25日に解体済み
- ブートストラップ: ApplicationContainer（P2P スタック初期化）を別管理とし、Phase 5 Workstream A/B の対象に含める。

## 外部クレート棚卸し（主要カテゴリ）
| カテゴリ | 主なクレート | 役割 | Phase 5 観点 |
| --- | --- | --- | --- |
| データベース | `sqlx`（`runtime-tokio-native-tls`,`sqlite`）, `sqlite` | SQLite アクセスとマイグレーション | すべて `ConnectionPool` 経由で利用し、Repository 層に閉じ込める。 |
| P2P / DHT | `iroh`, `iroh-gossip`, `bytes`, `lru` | Mainline DHT・Gossip 通信 | P2PService のビルダー以外から直接呼ばず、trait モックでテスト可能にする。2025年10月31日: `infrastructure/p2p/dht_integration.rs` の TODO 実装が未完了のため、mainline 経路のハンドシェイク/announce 実装を継続。 |
| プロトコル / イベント | `nostr-sdk`, `bech32`, `bincode` | Nostr イベント生成・鍵変換・ペイロード直列化 | EventService 専用の mapper に集約し、フロントへの露出を DTO 層に限定する。 |
| 暗号 / セキュリティ | `secp256k1`, `aes-gcm`, `argon2`, `keyring`, `rand_core` | 鍵生成・暗号化・ストレージ保護 | KeyManager/ SecureStorage の境界を明確化し、フォールバック実装を検討する。 |
| 非同期 / エラーハンドリング | `tokio`, `async-trait`, `anyhow`, `thiserror`, `chrono` | 非同期実行・共通エラーモデル・時刻計算 | Phase 5 後も `shared`/`infrastructure` 層に限定し、presentation 層での `anyhow` 直接使用を禁止する。 |
| 観測 / 運用 | `metrics_exporter_prometheus`, `tracing`, `tracing-subscriber` | メトリクス収集とロギング | ApplicationContainer で初期化を一元化し、CI/ローカル両方の計測パスを確認する。 |

## TODOメモ
- [x] 主要サービス／Repository／コマンドをすべて洗い出す
- [x] 切り離し難易度 High の項目について、対策案を `tauri_app_implementation_plan.md` に記録する
- [x] 依存関係ライブラリ（外部クレート）の棚卸し結果を別途追記する
