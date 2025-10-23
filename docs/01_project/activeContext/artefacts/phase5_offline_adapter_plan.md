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

### Stage 1（Adapter 導入: 3日）
1. `application::ports::offline_store` を追加し、`OfflineService` に注入ポイントを用意（既存 `OfflineManager` をラップする暫定 Adapter を実装）。
2. 暫定 Adapter (`LegacyOfflineManagerAdapter`) を `modules/offline` 上に実装し、テストをモック経由に更新。
3. `OfflineService` 内の型変換をドメイン値オブジェクトへ置換（一部旧構造体を newtype で包む）。

### Stage 2（Infrastructure 移行: 4日）
1. `infrastructure/offline/sqlite_store.rs` を作成し、`OfflineManager` の CRUD ロジックを移植。
2. `LegacyOfflineManagerAdapter` から新インフラ実装へ差し替え。`modules/offline` は read-only ラッパに縮退。
3. SQLx テスト（`offline_service` の結合テスト）を `tests/integration/offline/*` に移し、Docker スクリプトを更新。

### Stage 3（Legacy 解体: 2日）
1. `modules/offline` の不要コードを削除し、残存するユーティリティ（例: DTO for API）を Infrastructure か Shared mapper へ移行。
2. Documentation 更新（`tauri_app_implementation_plan.md`, Runbook への追記）。

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
