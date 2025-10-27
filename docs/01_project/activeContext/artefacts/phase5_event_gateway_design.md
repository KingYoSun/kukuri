# Phase 5 EventGateway 設計メモ
最終更新日: 2025年10月23日

## 背景
- EventService は `modules::event::manager::EventManager` に直接依存しており、Presentation 層の DTO（`presentation::dto::event::NostrMetadataDto`）や `nostr_sdk` の型を介して操作している。
- Phase 5 で目指すレイヤ分離では、Application 層はドメイン値オブジェクトとポート（抽象化インターフェース）のみに依存する必要がある。
- EventManager は Legacy 扱い（`phase5_dependency_inventory_template.md` 参照）であり、段階的に Infrastructure 化／置換が必要。

## 課題整理（現状把握より）
- 直接依存しているメソッド群: `handle_p2p_event`, `publish_text_note`, `publish_topic_post`, `send_reaction`, `update_metadata`, `delete_events`, `disconnect`, `set_default_p2p_topic_id`, `get_public_key`。
- `EventService` が Presentation DTO (`NostrMetadataDto`) を受け取り、そのまま EventManager へ渡しているため、DTO の構造変更が Application 層へ波及する。
- Nostr 由来の構造体 (`nostr_sdk::EventId`, `Metadata` など) が Application 層の境界を越えている。

## 提案アーキテクチャ

### 1. EventGateway ポート定義
- 追加場所: `kukuri-tauri/src-tauri/src/application/ports/event_gateway.rs`
- 役割: Application 層が EventManager 等の具体実装へ依存しないようにする抽象化。
- 想定インターフェース（抜粋）:
  ```rust
  #[async_trait]
  pub trait EventGateway: Send + Sync {
      async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError>;
      async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError>;
      async fn publish_topic_post(
          &self,
          topic_id: TopicId,
          content: &TopicContent,
          reply_to: Option<EventId>,
      ) -> Result<EventId, AppError>;
      async fn send_reaction(&self, target: EventId, reaction: ReactionValue) -> Result<EventId, AppError>;
      async fn update_profile_metadata(&self, metadata: ProfileMetadata) -> Result<EventId, AppError>;
      async fn delete_events(&self, targets: Vec<EventId>, reason: Option<String>) -> Result<EventId, AppError>;
      async fn disconnect(&self) -> Result<(), AppError>;
      async fn get_public_key(&self) -> Result<Option<PublicKey>, AppError>;
      async fn set_default_topics(&self, topics: Vec<TopicId>) -> Result<(), AppError>;
      async fn list_default_topics(&self) -> Result<Vec<TopicId>, AppError>;
  }
  ```
  - `DomainEvent`, `TopicId`, `TopicContent`, `ReactionValue`, `ProfileMetadata`, `PublicKey` は Domain 層の値オブジェクト（既存の `domain::entities` / `value_objects` を活用し、不足分は Phase 5 内で追加）。
  - `handle_incoming_event` は現在の `handle_p2p_event` を置き換え、Infrastructure から提供される `DomainEvent` を受け取る。

### 2. mapper 層の整理
- 追加場所: `kukuri-tauri/src-tauri/src/application/shared/mappers/event/`
  - `nostr_to_domain.rs`: `nostr_sdk::Event` → `domain::entities::Event` 変換（現在 `modules/event/manager` の `conversions` を移設）
  - `metadata_mapper.rs`: Presentation DTO ↔ Domain 値オブジェクト変換
  - `event_id_mapper.rs`: `nostr_sdk::EventId` ↔ `domain::value_objects::EventId`
- `application/services/event_service` は Presentation DTO を受け取った時点で mapper を通して Domain 値へ変換し、Gateway にのみ渡す。
- 既存の `application::shared::nostr::factory` / `publisher` は、Infrastructure 実装（`EventManagerGateway`）から利用する形に整理。

### 3. Infrastructure 実装
- 新規モジュール: `kukuri-tauri/src-tauri/src/infrastructure/event/event_manager_gateway.rs`
  - `EventGateway` を実装し、内部で既存の `EventManager` を委譲する。
  - `EventManager` 固有の型変換は mapper 経由で吸収する。
- DI (`state/application_container.rs`) では `Arc<dyn EventGateway>` を生成し `EventService` に注入。
- 旧 `set_event_manager` / `set_subscription_invoker` は廃止し、SubscriptionInvoker は別ポートに切り出す（後続タスク）。

## 依存方向
- Application 層は `application::ports::EventGateway` に依存。
- Infrastructure 層が `EventGateway` を実装し、Legacy `EventManager` への依存を閉じ込める。
- Presentation 層は DTO → Application 層 mapper → Gateway → Infrastructure の順に流れる。

## 実装スプリント粒度

### Sprint 1（3〜4日想定）
1. `EventGateway` ポートと Domain 値オブジェクト（不足分）を追加。
2. 新 mapper (`application/shared/mappers/event/*`) を新設し、`EventService` から DTO 変換処理を切り出す。
3. `EventService` を Gateway 経由で動くように修正（`event_manager` フィールドを削除、コンストラクタに Gateway 注入）。
4. 既存テスト（`tests/unit/application/event_service/*`）を Gateway モックベースに更新。

#### Sprint 1 タスク一覧（2025年10月23日着手）
| ID | 作業内容 | 対象パス/モジュール | チェックポイント |
| --- | --- | --- | --- |
| EG-S1-01 | `application::ports::event_gateway.rs` を追加し、trait と必要なドメイン型（`DomainEvent`, `TopicId`, `ReactionValue` など）を定義する。 | `kukuri-tauri/src-tauri/src/application/ports/event_gateway.rs`<br>`kukuri-tauri/src-tauri/src/domain/{entities,value_objects}/event_gateway/*` | trait 定義とドキュメントコメント／`phase5_event_gateway_design.md` のインターフェース差分が一致すること。 |
| EG-S1-02 | Event mapper を新設し、既存の `modules::event::manager::conversions` 依存を解消する。 | `kukuri-tauri/src-tauri/src/application/shared/mappers/event/{nostr_to_domain.rs,metadata_mapper.rs,event_id_mapper.rs}` | Nostr DTO 変換が Application 層内で完結し、`EventService` からの参照が新 mapper 経由になること。 |
| EG-S1-03 | `EventService` に Gateway を注入する DI パスを整備し、`set_event_manager` などの Legacy 依存を暫定的にラップする。 | `kukuri-tauri/src-tauri/src/application/services/event_service/*`<br>`kukuri-tauri/src-tauri/src/state/{application_container.rs,state.rs}` | 既存テストが Gateway モックで通り、Legacy EventManager へのアクセスが `LegacyEventManagerGateway`（仮称）経由になること。 |
| EG-S1-04 | テスト群を Gateway モックベースに更新し、CI での `cargo test --package kukuri-tauri --test event_service` がグリーンであることを確認。 | `kukuri-tauri/src-tauri/tests/unit/application/event_service/*` | Mock 実装が `EventGateway` trait を実装し、既存のビヘイビア検証を維持する。 |

#### EG-S1-01 実装メモ（2025年10月24日）
- `application::ports::event_gateway.rs` に `EventGateway` trait を追加し、`handle_incoming_event`／`publish_topic_post`／`send_reaction` など Phase 5 で想定する操作を整理。
- ドメイン型は `domain::{entities,value_objects}::event_gateway` 配下に新設。`DomainEvent` は `Event` エンティティとの変換ヘルパを備え、タグは `EventTag` で正規化した。
- `PublicKey`・`ReactionValue`・`TopicContent`・`ProfileMetadata` など値オブジェクト／エンティティを導入し、Nostr 由来の検証（hex長や文字数上限）をドメイン層で担保。

### Sprint 2（3日想定）
1. `infrastructure/event/event_manager_gateway.rs` を実装し、Legacy EventManager への委譲ロジックと mapper 呼び出しを移設。
2. DI (`state.rs` / `application_container.rs`) を更新し、Gateway を生成・注入。
3. `modules/event/manager` 内の Presentation 依存（`tauri::AppHandle` など）を EventGateway 実装側に閉じ込めるためのラッパを追加。
4. 結合テスト（Mainline DHT, EventService integration）を Gateway 経由で実行するように更新。

#### EG-S2-01 実装メモ（2025年10月24日）
- `EventManager` から `AppHandle` 保持・UI emit ロジックを削除し、`LegacyEventManagerGateway` 側に `set_app_handle` を追加して UI への `nostr://event/p2p` 通知を橋渡し。
- ApplicationContainer で生成した Gateway を `EventService` へ DI しつつ `AppHandle` を注入する初期化フローを再構築。`EventService` 側では Legacy 依存を意識せず `Arc<dyn EventGateway>` のみを扱う。
- Gateway で DomainEvent→Nostr イベント変換後に UI emit を担保するテスト（ハンドル未設定時のノップ/ペイロード変換）を追加し、`cargo test` で網羅的に検証。

#### EG-S2-02 実装メモ（2025年10月25日）
- `tests/integration/test_event_gateway.rs` を追加し、P2P（Mainline DHT）経路で受信した DomainEvent が `LegacyEventManagerGateway` → `EventManager` → SQLite (`events` / `event_topics`) へ正しく反映されることを検証。`ConnectionPool` を実際にマイグレーションし、タグ `t` → Hashtag 変換や `event_topics` 登録まで通過することを確認した。
- ランブック／タスクでは本テストを Mainline DHT フローの再現ステップとして扱い、Sprint 2 要件であった「Gateway 経由の結合テスト」完了のエビデンスとする。

### Sprint 3（2日）

- SubscriptionInvoker を `application::ports` へ切り出し、Gateway から Legacy EventManager への依存を遮断。
- `modules/event/manager::conversions` に残っていた Nostr ↔ Domain 変換を `application/shared/mappers` に移し、Application 層で一元管理。
- EventGateway にメトリクス記録フックを追加し、成功/失敗統計を `tests` で検証可能にする。

#### EG-S3-01 実装メモ（2025年10月25日）
- `application::ports::subscription_invoker.rs` を追加し、EventService から参照する購読ポートを定義。`set_subscription_invoker` など Application 層の公開 API は `Arc<dyn SubscriptionInvoker>` のみを扱う。
- 具象実装 `EventManagerSubscriptionInvoker` を `infrastructure::event::subscription_invoker.rs` へ移管し、DI（`state.rs`）とテスト（`tests/common/mocks/event_service.rs` 等）の import を更新。`LegacyEventManagerGateway` との依存関係は `EventManagerHandle` に限定された。
- `EventService` やユニットテストは新ポート経由で参照するように差し替え、`cargo fmt` / `cargo test --package kukuri-tauri --test event_service_gateway` で互換性を確認。

#### EG-S3-02 実装メモ（2025年10月25日）
- `modules/event/manager/conversions.rs` を削除し、Nostr イベント→Domain Event の変換ロジックを `application/shared/mappers/event/nostr_to_domain.rs` に移植。`nostr_event_to_domain_event` を追加し、`AppError::ValidationError` でバリデーション失敗を表現。
- `modules/event/manager/p2p.rs` は新 mapper を参照し、`anyhow!` で AppError をラップする形に更新。Legacy モジュールに Application 層の mapper を導入したことで、後続の EventManager 分割時に再利用できる構成になった。

#### EG-S3-03 実装メモ（2025年10月25日）
- `infrastructure::event::metrics` を新設し、Incoming/PUBLISH/Reaction/Metadata/Delete/Disconnect それぞれの成功・失敗回数と最終時刻を原子的に記録。`EventGateway` 実装からは `metrics::record_outcome` を介して計測する。
- `LegacyEventManagerGateway` の各メソッドに計測フックを追加し、`tests/infrastructure/event/event_manager_gateway.rs` へメトリクス検証ケース（成功/失敗）を追加。`metrics::snapshot()` で取得できる統計を Runbook から参照できるようにした。

## Stage3（2025年10月25日）実装メモ
- `infrastructure::event::manager_handle::EventManagerHandle` を新設し、`AppState` / `EventManagerSubscriptionInvoker` / `LegacyEventManagerGateway` から `modules::event` への直接参照を排除。DI では `LegacyEventManagerHandle` のみを生成する。
- Gateway 経由の送信パスを `tests/integration/test_event_service_gateway.rs` でカバー。Publish/TopicPost/Reaction/Metadata/Delete の各メソッドが Gateway 実装を通じて `EventManagerHandle` に委譲されることを検証し、`EventService` のリグレッションを自動化。
- Mainline DHT テスト群と合わせて Stage3 の完了条件（レガシー参照の封じ込め＋Gateway 経路の結合テスト）を満たした。以降は mapper/SubscriptionInvoker のポート化を順次行う。

## 残課題・フォローアップ
- 2025年10月27日: `LegacyEventManagerGateway` の Publish/Reaction/Delete/Metadata/Repost を `infrastructure::p2p::metrics` へ連動させ、Gateway 層で P2P 成功率を観測できるようにした。Runbook では EventGateway メトリクスに加えて Gossip 成功率の取得手順を追記する。
- 2025年10月27日: `modules/event/manager/tests` を廃止し、`application/shared/tests/event/manager_tests.rs` に統合。Gateway/Publisher/DefaultTopics 挙動の結合テストを crate 内共通モックで再構成したため、新レイヤ構成でも継続的に検証可能。
- 2025年10月27日: Presentation DTO に `Nip65RelayDto` を追加し、ProfileMetadata/Mapper 双方で Relay List（NIP-65）を扱えるよう拡張。Relay URL の検証を DTO/Domain の双方で行い、Mapper の単体テストで JSON ラウンドトリップを保証。
