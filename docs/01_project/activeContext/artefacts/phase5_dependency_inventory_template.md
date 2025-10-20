# Phase 5 依存関係棚卸しテンプレート
最終更新日: 2025年10月20日

## 記入ルール
- `モジュール/コンポーネント`: ファイルまたはモジュールの論理単位（例: `application/services/event_service`）
- `現行配置`: 既存ディレクトリ構造のパス
- `主な依存先`: 代表的な依存モジュールや外部クレート（箇条書き可）
- `想定レイヤ`: Phase 5 後に配置するレイヤ（Domain/Application/Infrastructure/Presentation）
- `切り離し難易度`: Low / Medium / High 等級で評価
- `課題・メモ`: 循環依存や責務分割上の懸念点などを記載

## 一覧表
| モジュール/コンポーネント | 現行配置 | 主な依存先 | 想定レイヤ | 切り離し難易度 | 課題・メモ |
| --- | --- | --- | --- | --- | --- |
| EventService | `application/services/event_service` | `infrastructure::database::EventRepository`, `infrastructure::crypto::SignatureService`, `infrastructure::p2p::EventDistributor`, `modules::event::manager::EventManager`, `presentation::dto::event`, `shared::error::AppError` | Application | Medium | Presentation DTO への直接依存を排除し、アプリ層専用 DTO/mapper を用意する必要あり。EventManager とはトrait境界での結合を維持。 |
| OfflineService | `application/services/offline_service.rs` | `modules::offline::OfflineManager`, `modules::offline::models::*`, `shared::error::AppError`, `serde_json::Value` | Application | Medium | モジュール層の型エイリアスが多く、変換ヘルパーを `shared::offline` に切り出すことで循環を予防。 |
| P2PService & Builder | `application/services/p2p_service.rs` | `infrastructure::p2p::{NetworkService,GossipService,DiscoveryOptions}`, `modules::p2p::events::P2PEvent`, `shared::config::NetworkConfig`, `iroh::SecretKey`, `tokio::sync` | Application | Medium | `DiscoveryOptions` の共有構造を `domain::usecases::p2p` に移し、Iroh 具象型との結合を Builder 内に閉じ込める。 |
| EventManager | `modules/event/manager` | `application::shared::{default_topics,nostr::EventPublisher}`, `infrastructure::database::EventRepository`, `infrastructure::p2p::GossipService`, `modules::auth::KeyManager`, `modules::database::DbPool`, `tauri::AppHandle` | Domain | High | Tauri `AppHandle` とインフラ層の両方へ依存。`AppHandle` をイベントブロードキャスト用インターフェースに抽象化し、リポジトリ依存はアプリ層経由に限定する。 |
| SQLiteRepository | `infrastructure/database/sqlite_repository.rs` | `sqlx`, `domain::entities::*`, `application::shared::nostr`, `modules::database::connection::DbPool`, `shared::error::AppError` | Infrastructure | High | 単一ファイルに複数リポジトリ実装が集約。責務分割と `domain::entities` への直接依存を解消するため、機能別モジュール化＋トrait境界の導入が必須。 |
| IrohNetworkService | `infrastructure/p2p/iroh_network_service.rs` | `iroh`, `iroh_gossip`, `tokio`, `shared::error::AppError`, `modules::p2p::events::P2PEvent` | Infrastructure | Medium | `P2PEvent` 依存をイベントバス経由に差し替え、インフラ層→モジュール層の逆流を防ぐ。 |
| Command Modules (Topics/Posts/Events) | `presentation/commands/*_commands.rs` | `application::services::*`, `shared::error::AppError`, `presentation::dto::*`, `tauri::command` | Presentation | Low | 共通のエラーハンドリングラッパーは導入済み。Phase 5 では DTO 連携を `presentation::mapper` に集約させる。 |
| AppState/Config | `state.rs`, `state/*` | `application::services::*`, `shared::config::*`, `tokio::sync`, `once_cell` | Application | Medium | グローバル状態の初期化順序が複雑。DI コンテナ初期化に合わせて責務を `application::bootstrap` へ寄せる。 |
| OfflineManager | `modules/offline` | `modules::database::connection::DbPool`, `shared::time`, `serde_json`, `sqlx`, `shared::error::AppError` | Domain | Medium | Application 層との境界を保つため、同期キュー API を trait 化してモジュール層からの参照を一方向化する。 |
| Gossip Metrics | `infrastructure/p2p/metrics.rs` | `metrics_exporter_prometheus`, `tokio`, `shared::time`, `serde` | Infrastructure | Low | メトリクスエクスポーターは Phase 5 後の CI との整合を確認するのみ。 |

## TODOメモ
- [ ] 主要サービス／Repository／コマンドをすべて洗い出す
- [ ] 切り離し難易度 High の項目について、対策案を `tauri_app_implementation_plan.md` に記録する
- [ ] 依存関係ライブラリ（外部クレート）の棚卸し結果を別途追記する
