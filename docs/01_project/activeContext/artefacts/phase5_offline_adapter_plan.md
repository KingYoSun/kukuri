# Phase 5 OfflineService Adapter 設計
最終更新日: 2025年10月23日

## 背景
- `application::services::OfflineService` は `modules::offline::{OfflineManager, models::*}` に直接依存しており、Application 層内で SQLx 構造体（`OfflineAction`, `CacheMetadata` など）をそのまま公開している。
- `phase5_dependency_inventory_template.md` で `OfflineService` / `OfflineManager` を High 難易度項目として特定済み。Infrastructure への移行と抽象化が求められている。
- Phase 4 で追加された機能（SyncQueue、OptimisticUpdate など）が Manager に集中し、責務が肥大化している。

## 現状の課題
1. Application 層の公開型が Legacy モジュール依存 (`modules::offline::models::*`) になっている。
2. `serde_json::Value` をサービス内で多用し、ドメインモデルが不明瞭。
3. OfflineManager への呼び出しが同期ロジック/キャッシュロジック/キュー操作で混在し、テスト容易性が低い。
4. SQLx 構造体の変更が Application 層まで波及するため、Phase 5 のレイヤ再構成に不向き。

## 提案アーキテクチャ

### 1. ポート（抽象化）設計
- 新規追加: `kukuri-tauri/src-tauri/src/application/ports/offline_store.rs`
- 役割: オフライン関連の永続化操作を抽象化し、サービスはポートのみに依存。
- 抽象化を4つに分割し責務を明確化する。
  ```rust
  #[async_trait]
  pub trait OfflineActionStore {
      async fn save_action(&self, payload: OfflineActionDraft) -> Result<OfflineActionRecord, AppError>;
      async fn list_actions(&self, filter: OfflineActionFilter) -> Result<Vec<OfflineActionRecord>, AppError>;
      async fn mark_synced(&self, action_id: OfflineActionId, remote_id: Option<String>) -> Result<(), AppError>;
  }

  #[async_trait]
  pub trait SyncQueueStore {
      async fn enqueue(&self, item: SyncQueueItemDraft) -> Result<SyncQueueId, AppError>;
      async fn ensure_enqueued(&self, action: &OfflineActionRecord) -> Result<bool, AppError>;
      async fn pending_items(&self) -> Result<Vec<SyncQueueItem>, AppError>;
  }

  #[async_trait]
  pub trait CacheMetadataStore {
      async fn upsert_metadata(&self, update: CacheMetadataUpdate) -> Result<(), AppError>;
      async fn list_stale(&self) -> Result<Vec<CacheMetadataRecord>, AppError>;
      async fn cleanup_expired(&self) -> Result<u32, AppError>;
  }

  #[async_trait]
  pub trait OptimisticUpdateStore {
      async fn save(&self, update: OptimisticUpdateDraft) -> Result<OptimisticUpdateId, AppError>;
      async fn confirm(&self, id: OptimisticUpdateId) -> Result<(), AppError>;
      async fn rollback(&self, id: OptimisticUpdateId) -> Result<Option<String>, AppError>;
      async fn unresolved(&self) -> Result<Vec<OptimisticUpdateRecord>, AppError>;
  }
  ```
- Application 層ではこれらのポートを束ねた `OfflinePersistence` を注入し、`OfflineService` は値オブジェクトとドメインロジックに集中する。

### 2. ドメイン値オブジェクトの整備
- 既存の `OfflineActionRecord`, `SyncResult`, `CacheStatusData` などを `domain::entities::offline`（新設）へ移し、SQLx 由来の `i64` / `String` を型付けする。
  - 例: `OfflineActionId(String)`, `CacheKey(String)`, `SyncStatus(enum)`。
- `serde_json::Value` を直接返さず、`OfflinePayload`（内部で `serde_json::Value` を保持する newtype）でラップしバリデーションポイントを明示する。

### 3. Infrastructure 実装の段階移行
- 新ディレクトリ: `kukuri-tauri/src-tauri/src/infrastructure/offline/`
  - `sqlite_store.rs`: SQLx ベースの実装。既存の `OfflineManager` ロジックを分割移植。
  - `mappers.rs`: SQLx Row ↔ ドメイン値オブジェクトの変換。
  - `mod.rs`: `OfflinePersistenceImpl` を公開。
- 既存の `modules/offline` は Legacy 化し、段階的に削除。

### 4. サービス層リファクタリング方針
- `OfflineService` では以下を徹底:
  - ポート経由でデータ取得し、`SyncResult` などの集計のみ担当。
  - UI から受け取る `serde_json::Value` は新設 mapper でバリデーション後に `OfflinePayload` へ変換。
  - `manager` フィールドを `Arc<dyn OfflinePersistence>` に置換。

## 段階的移行計画

### Stage 0（準備: 1日）
1. ドメイン値オブジェクトとドラフト/フィルタ構造体を追加（`domain::entities::offline`）。
2. `phase5_dependency_inventory_template.md` で `OfflineService` の課題項目にリンク。

#### Stage 0 タスク一覧（2025年10月23日追加）
| ID | 作業内容 | 対象パス/モジュール | チェックポイント |
| --- | --- | --- | --- |
| OFF-S0-01 | オフライン関連の値オブジェクト／エンティティ（`OfflineActionId`, `OfflinePayload`, `SyncStatus` など）を Domain 層へ追加。 | `kukuri-tauri/src-tauri/src/domain/entities/offline/*`<br>`kukuri-tauri/src-tauri/src/domain/value_objects/offline/*` | 既存 Application 型との互換性を確認し、Serde 派生を付与。 |
| OFF-S0-02 | 新 VO への変換ヘルパを Application 層に仮実装し、`OfflineService` からの利用箇所を洗い出す。 | `kukuri-tauri/src-tauri/src/application/services/offline_service.rs` | 変換箇所の TODO をコメントで残し、後続 Stage 1 で差し替え可能な状態にする。 |
| OFF-S0-03 | `.sqlx` ディレクトリの既存ファイルと OfflineManager の SQL を棚卸しし、準備段階での再生成要否を判定。 | `kukuri-tauri/src-tauri/.sqlx/`<br>`kukuri-tauri/src-tauri/src/modules/offline/manager.rs` | 動的 SQL のため即時再生成不要だが、Stage 2 で `query!` 導入時に `cargo sqlx prepare` が必要になる点をメモ。 |

#### OFF-S0-01 実装メモ（2025年10月24日）
- `domain::value_objects::offline` に `OfflineActionId`・`OfflinePayload`・`SyncStatus`・`SyncQueueStatus`・`CacheKey` などの値オブジェクトを追加し、従来の `String`／`i64` フィールドを型付けした。
- `domain::entities::offline` では `OfflineActionRecord`・`SyncQueueItem`・`CacheMetadataRecord`・`OptimisticUpdateRecord`・`SyncStatusRecord`・`CacheStatusSnapshot` などを定義。`chrono::DateTime<Utc>` でタイムスタンプを扱い、`serde_json::Value` は `OfflinePayload` 経由で包んだ。
- 新規スキーマ付きで `cargo check` を実行し、ビルド可能な状態を確認。Stage 1 以降はこれらの型を `OfflineService` から参照するよう変換ヘルパを実装予定。

### Stage 1（Adapter 導入: 3日）
1. `application::ports::offline_store` を追加し、`OfflineService` に注入ポイントを用意（既存 `OfflineManager` をラップする暫定 Adapter を実装）。
2. 暫定 Adapter (`LegacyOfflineManagerAdapter`) を `modules/offline` 上に実装し、テストをモック経由に更新。
3. `OfflineService` 内の型変換をドメイン値オブジェクトへ置換（一部旧構造体を newtype で包む）。

### Stage 2（Infrastructure 移行: 4日）
1. `infrastructure/offline/sqlite_store.rs` を作成し、`OfflineManager` の CRUD ロジックを移植。
2. `LegacyOfflineManagerAdapter` から新インフラ実装へ差し替え。`modules/offline` は read-only ラッパに縮退。
3. SQLx テスト（`offline_service` の結合テスト）を `tests/integration/offline/*` に移し、Docker スクリプトを更新。

#### Stage 2 実装メモ（2025年10月24日）
- `SqliteOfflinePersistence` を実装し、`modules::offline::manager` から主要 CRUD ロジック（オフラインアクション保存・同期、キャッシュメタデータ更新、同期キュー操作、楽観的更新管理、同期ステータス更新）を移植した。`uuid` / `chrono` / `sqlx::QueryBuilder` を活用しつつ、戻り値を `AppError` ベースに整備。
- `state.rs` の DI を新実装に切り替え、`Arc<dyn OfflinePersistence>` へ SQLite プールを直接注入。プレゼンテーション層のハンドラーや既存サービスはインターフェイスのみ参照する構成に統一。
- OfflineService のユニットテストを `LegacyOfflineManagerAdapter` 依存から `SqliteOfflinePersistence` へ差し替え、インメモリ SQLite スキーマ初期化後に同等のシナリオ（保存・同期・キュー・キャッシュ・楽観的更新）を検証。
- `infrastructure/offline/mod.rs` から Legacy アダプタの再エクスポートを外し、新 API を優先使用できるよう公開面を整理。Stage 3 での Legacy 縮退を容易にするため、旧アダプタはモジュール直下で維持。
- ビルド整合性確認として `cargo fmt` → `cargo clippy -- -D warnings` → `./scripts/test-docker.ps1 rust` を実行。Docker Rust テスト完走で互換性を確認（既知の `Nip10Case::description` 警告のみ）。

### Stage 3（Legacy 解体: 2日）
1. `modules/offline` を削除し、`infrastructure/offline::{rows,mappers,sqlite_store,reindex_job}` に統合。OfflineReindexJob は新 Persistence を直接利用する。
2. `state.rs` の DI とテスト資産を刷新し、`SqliteOfflinePersistence` / `OfflineReindexJob` を共有。
3. Documentation 更新（本ドキュメント、`phase5_dependency_inventory_template.md` など）とテストの補完。

#### Stage 3 実装メモ（2025年10月25日）
- `modules/offline` 一式（manager/models/reindex/tests）を削除し、行構造は `infrastructure/offline/rows.rs` と mapper へ移植。Legacy adapter も撤去。
- `SqliteOfflinePersistence` に同期キュー・キャッシュ・楽観的更新・同期状態の取得 API を追加し、`OfflineReindexJob` を新設モジュールで再実装。`AppState` からは `Arc<SqliteOfflinePersistence>` を共有してジョブを生成。
- `state.rs` から Legacy `OfflineManager` 依存を除去し、DI を `OfflineReindexJob` + `OfflineService` の二経路に整理。
- Rust ユニットテストを `sqlite_store.rs` / `reindex_job.rs` 内へ再配置してカバレッジを維持。`cargo test` はローカルリンクエラーで失敗（`cc` 経由で iroh 依存ライブラリ link 不可）だが、テストコード自体はビルドまで確認済み。

### `.sqlx` 影響メモ（2025年10月23日調査開始）
- 現行 `OfflineManager` は `sqlx::query` / `query_as` を動的 SQL 文字列で呼び出しており、`.sqlx/` にプリコンパイル済みクエリは存在しない。
- Stage 2 で Repository を Infrastructure 層へ移行し、`query!` / `query_as!` を採用する場合は `cargo sqlx prepare` を再実行して `.sqlx/query-*.json` を生成する必要がある。
- `.sqlx` 再生成時は `scripts` 配下の CI 手順（`./scripts/test-docker.ps1 rust`）と整合を確認し、アーティファクトをリポジトリに含めること。

## テスト方針
- Stage 1: 既存ユニットテストをモック差し替えで通過させる。
- Stage 2: 新設の integration テスト（SQLite 実 DB）を Docker ジョブに追加。
- Stage 3: Regression 用に `scripts/test-docker.ps1 offline` を追加し、CI に登録。

## オープン課題
- オフラインキューの再送ポリシーをどこで管理するか（Service か Infrastructure）。Stage 2 で決定。
- `serde_json::Value` のバリデーションスキーマ化（JSON Schema or custom validator）については別タスク化。
- マイグレーション段階で `.sqlx/` の再生成が必要になるため、Stage 2 完了後に対応。
