# Phase 3/4 ギャップ対応計画
最終更新日: 2025年10月19日

## 1. 背景と目的
- Phase 2 で主要TODOを解消した結果、Rust 側のコアモジュールが肥大化し、今後の機能拡張・テスト整備の阻害要因となっている。
- Phase 3/4 では「大規模ファイルの分割」と「重複処理のDRY化」を進め、保守性・テスト容易性・レビュー効率を向上させる。
- 本ドキュメントは対象ファイルの現状把握と分割方針を整理し、実装タスクへのブレイクダウンを提供することを目的とする。

## 2. スコープ

### 2.1 対象
- Phase 3 で700行超を解消することを優先する Rust ファイル群。
- Phase 4 で DRY 適用対象とするモジュールおよびテスト資産。
- `.sqlx/` を含むビルド成果物の更新および関連ドキュメントの同期。

### 2.2 対象外
- TypeScript 側のリファクタリングは Phase 4 以降のストア共通化（persistHelpers.ts 適用）に限定。
- 既存 API スキーマや DB スキーマの抜本的変更は本計画外。ただし分割の副作用として軽微なリネームが必要な場合は別途チケット化する。
- Nostr プロトコル拡張・新機能開発は本計画では扱わない。

## 3. 700行超ファイル再調査結果
- `kukuri-tauri/src-tauri/src/application/services/event_service.rs`（1216行）: イベントサービスのインターフェース実装・Nostr署名・サブスクリプション復元・大量のモックテストが同居。
- `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs`（1099行）: `Repository`系トレイト実装（Post/Topic/User/Event）が1ファイルに集中し、SQLとエンティティ変換コードが重複。
- `kukuri-tauri/src-tauri/src/modules/event/manager.rs`（813行）: EventManager本体に初期化、P2Pブロードキャスト、既定トピック管理、テスト群が内包。
- `kukuri-tauri/src-tauri/src/modules/p2p/tests/iroh_integration_tests.rs`（702行）: Irohベースの統合テストシナリオが単一モジュールに集約され、ヘルパー関数とシナリオが混在。

## 4. ファイル別 分割・DRY 方針

### 4.1 `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs`（1099行）
**現状課題**
- `SqliteRepository`単体が `Repository` トレイトと4つの専門リポジトリトレイトをまとめて実装しており、責務が肥大化。
- トピック・イベント・ユーザーそれぞれで `Row` → ドメイン変換処理が繰り返され、DRY違反が顕著。
- SQL文のリテラルが分散しており、将来のマイグレーション変更時に影響範囲が把握しづらい。

**分割/リファクタリング案**
- `sqlite_repository/` ディレクトリ化し、`mod.rs` を薄いファサードに変更。`posts.rs` / `topics.rs` / `users.rs` / `events.rs` へ各トレイト実装を分割。
- 行共通の `Row` マッピングを `mapper.rs`（例：`fn map_topic_row(..)`）に切り出し、`Topic`/`Post`/`Event` の生成ロジックを共通化（Phase 4 DRY と直結）。
- SQL文を `queries.rs` に定数化、もしくは `include_str!` ベースで整理し、変更差分の特定を容易にする。
- `ConnectionPool` 依存を `Arc<ConnectionPool>` で共通注入できるよう `SqliteRepository` の保持型を再検討（スレッドセーフ化）。

**実行タスク（ドラフト）**
1. `sqlite_repository.rs` を `mod.rs` 化し、既存実装をサブファイルへ移動。
2. `Topic`/`Event` 変換のユーティリティを `mapper.rs` に実装し、既存コードを置換。
3. SQL文字列を `queries.rs` に抽出し、テストで最低限のクエリ回帰を確認。
4. 影響範囲テスト（`cargo test`）と `sqlx prepare` 再生成を行い、`.sqlx/` を更新。

**成果物・完了条件**
- `src-tauri/src/infrastructure/database/sqlite_repository/` 配下に `mod.rs`・`posts.rs`・`topics.rs`・`users.rs`・`events.rs`・`mapper.rs`・`queries.rs` を配置し、既存APIを維持したままビルドが通ること。
- ドメインエンティティ変換ロジックが `mapper.rs` に集約され、元ファイルでの重複実装が削除されていること。
- `sqlx-data.json` を含む `.sqlx/` データが最新化され、CI で `cargo sqlx prepare` が不要な差分を報告しないこと。

**依存/前提**
- 既存マイグレーションファイルとの整合性を事前確認 (`src-tauri/migrations/` の見直し)。
- Repository を利用するサービス層（EventService, OfflineService, TopicService 等）の API 変更が不要であることを確認。

**検証**
- `cd kukuri-tauri/src-tauri && cargo fmt && cargo clippy -D warnings && cargo test` を通過。
- `DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare` を再実行し `.sqlx/` の更新をレビュー。

### 4.2 `kukuri-tauri/src-tauri/src/application/services/event_service.rs`（1216行）
**現状課題**
- サービスインターフェース、Nostr署名/検証ロジック、サブスクリプション状態管理、P2Pディストリビューター連携が1モジュールに内包。
- `SubscriptionInvoker` や `EventManager` 依存注入コードが直列化しており、モックテストも同一ファイルに存在。
- 非同期テストが大量に配置され、本体コードの可読性が低下。

**分割/リファクタリング案**
- `event_service/` ディレクトリ化：`mod.rs`（トレイト定義と構成ルート）、`core.rs`（イベント発行/取得API）、`subscription.rs`（購読状態遷移と復元ロジック）、`distribution.rs`（EventDistributor連携）、`factory.rs`（Nostrイベント生成・署名補助）といった責務別モジュールに分解。
- `SubscriptionInvoker` の具象実装（`EventManagerSubscriptionInvoker`）を `invoker.rs` へ移動し、`EventManager` 依存を明示化。
- テストを `tests.rs` に分離し、`mockall` モック生成は `tests/support` 配下に共通化して繰り返し定義を削減。
- DRY観点で Nostrイベント生成部分を専用ファクトリにまとめ、`create_event` / `publish_topic_post` 関連コードの重複を除去。

**実行タスク（ドラフト）**
1. `event_service` ディレクトリ構成を整備し、`EventService` 本体を `core.rs` に抽出。
2. サブスクリプション処理を `subscription.rs` へ移し、`EventService` からは薄い委譲に変更。
3. テストを `tests.rs` へ移動し、モック生成ヘルパーを `tests/support/mocks.rs` として再利用化。
4. 署名・検証ヘルパーを `factory.rs` にまとめ、`process_received_event` 等の重複ロジックを簡素化。

**成果物・完了条件**
- `event_service/` 配下に `mod.rs`・`core.rs`・`subscription.rs`・`distribution.rs`・`factory.rs`・`invoker.rs` を作成し、公開トレイトと型エイリアスのエントリポイントを `mod.rs` に定義。
- サービス本体が `core.rs` へ移行し、購読復元ロジックが `subscription.rs` に集約されていること。
- テストが `event_service/tests.rs` + `tests/support/mocks.rs` へ移動し、モック定義の重複が解消されていること。
- `EventServiceTrait` を利用するコマンド群（Tauri commands）がビルドエラーなくリンク。

**依存/前提**
- `EventManager` 側 API の再利用を前提とし、依存注入（DI）設定を `ApplicationContainer` で再確認。
- サブスクリプション状態機構（`SubscriptionStateStore`）の既存実装に変更がないこと。

**検証**
- `cargo test -p kukuri-tauri -- event_service` で該当テストが成功。
- `pnpm test` の TypeScript 側で EventService 呼び出しモックに変更がないことを確認。

### 4.3 `kukuri-tauri/src-tauri/src/modules/event/manager.rs`（813行）
**現状課題**
- EventManager が Tauri AppHandle 管理、GossipService連携、既定トピック集合の操作、Nostrイベント生成を一括で担っている。
- 内部状態管理（`selected_default_topic_ids`）とブロードキャスト処理が密結合で、テストでは多数のモック構築が必要。
- テスト群が同一ファイルに存在し、本体APIの読み取りが困難。

**分割/リファクタリング案**
- `modules/event/manager/` 配下に `mod.rs`（公開API）、`core.rs`（初期化・イベント発行）、`p2p.rs`（GossipService連携）、`default_topics.rs`（既定トピック管理 state）、`tests.rs`（ユニットテスト）を新設。
- `DefaultTopicsState`（仮称）を導入し、`HashSet` 操作を専任構造体に切り出すことでロック粒度を明確化。
- P2Pブロードキャストとイベント生成の境界を整理し、`publish_*` 系メソッドの責務を軽量化。
- テストでは TestGossipService などのモックを再利用可能なサポートモジュールに移動。

**実行タスク（ドラフト）**
1. `EventManager` の基本APIを `core.rs` に移し、P2P関連メソッドを `p2p.rs` に再配置。
2. `selected_default_topic_ids` 操作を `DefaultTopicsRegistry`（仮）に分離し、読み書きロック管理をカプセル化。
3. テストを `tests.rs` に移し、`tests/support/gossip.rs` 等でモックを共用化。
4. 変更後のAPIを利用する呼び出し側（フロントコマンド等）を調整し、`cargo test` で回帰確認。

**成果物・完了条件**
- `modules/event/manager/` 配下に `mod.rs`・`core.rs`・`p2p.rs`・`default_topics.rs`・`tests.rs` を作成。
- `DefaultTopicsRegistry`（仮称）が導入され、`HashSet` 直接操作が本体から排除されていること。
- GossipService 連携APIが `p2p.rs` に分離され、`EventManager` の公開APIは `mod.rs` で再エクスポートされる。
- 既存テストが `tests.rs` へ移動し、モック GossipService が共通サポートに移された状態で全テスト成功。

**依存/前提**
- GossipService 実装（`IrohGossipService`）への影響を事前に洗い出し、インターフェース変更有無を確認。
- フロントエンドの Tauri コマンド (`topic_commands.rs`, `event_commands.rs` 等) が新APIに追随可能であること。

**検証**
- `cargo test --lib modules::event::manager` を実行し成功。
- `pnpm test` で EventManager 呼び出しを含むフロントテストが回帰していないことを確認。

### 4.4 `kukuri-tauri/src-tauri/src/modules/p2p/tests/iroh_integration_tests.rs`（702行）
**現状課題**
- 環境変数ベースの統合テストシナリオが1モジュール内に集中し、各シナリオでブートストラップや待機ロジックが重複。
- ヘルパー関数（`bootstrap_context`, `wait_for_topic_membership` 等）がファイル内で散在し、他テストから再利用不可。
- ENABLEフラグ未設定時にはテストスキップが多発し、メンテナンス判断が取りづらい。

**分割/リファクタリング案**
- `modules/p2p/tests/iroh/` ディレクトリを設け、`support.rs` に共通ユーティリティ（ブートストラップ、待機、イベント変換）を集約。
- シナリオ別に `connectivity.rs`（接続検証）、`broadcast.rs`（ブロードキャスト系）、`multi_peer.rs`（多ノードシナリオ）といったテストファイルへ分割。
- `cfg` フラグ（例: `#[cfg(feature = "p2p-integration-tests")]`）を導入し、ビルド時の明示的ON/OFF制御を検討。
- テストロギングを `tracing` ベースに切り替え、`log_step!` マクロは `support` で定義・再利用する。

**実行タスク（ドラフト）**
1. `iroh_integration_tests.rs` を `tests/iroh/mod.rs` + サブモジュール構成に再編成。
2. 共通ヘルパーを `support.rs` に移動し、既存テストからの呼び出しを差し替え。
3. シナリオごとにテストをグループ化し、不要な重複タイムアウト値・待機処理を調整。
4. CIでの実行条件（環境変数）を整理し、ドキュメント化。

**成果物・完了条件**
- `modules/p2p/tests/iroh/` 配下に `mod.rs`・`support.rs`・`connectivity.rs`・`broadcast.rs`・`multi_peer.rs` などシナリオ別ファイルを配置。
- 待機ロジックやログ出力が `support.rs` に統一され、重複コードが削減されていること。
- テストのスキップ条件が整理され、`ENABLE_P2P_INTEGRATION` 未設定時は早期 return で明示的にスキップ理由をログ。

**依存/前提**
- `iroh` クレートの API 変更がないことを前提に、テストで利用するサポートユーティリティの安定性を確認。
- CI（GitHub Actions）でのテスト実行条件（環境変数）について、運用担当と合意を取る。

**検証**
- `ENABLE_P2P_INTEGRATION=1 KUKURI_BOOTSTRAP_PEERS=... cargo test --tests modules::p2p::tests::iroh` をローカル/CI で実行し成功。
- `cargo test -- --skip modules::p2p::tests::iroh` のような部分実行でもビルドが通ることを確認。

## 5. 今後の進め方
- 上記タスクを Phase 3（ファイル分割）→ Phase 4（DRY適用）の順でチケット化し、優先度は `sqlite_repository` → `event_service` → `event_manager` → `p2p/integration tests` の順で着手。
- 各ファイルの分割前後で `cargo fmt` / `cargo clippy -D warnings` / `cargo test` を必ず実行し、`.sqlx/` やモック定義の再生成が必要な場合は差分を明示。
- テスト分割後は `docs/03_implementation/error_handling_guidelines.md` ほか関連ドキュメントに差分影響がないか確認し、必要に応じて更新。

## 6. 実行ロードマップ（目安）
| フェーズ | 期間目安 | 主タスク | 完了条件 | 依存 |
| --- | --- | --- | --- | --- |
| Phase 3A | 1週間 | `sqlite_repository` 分割／mapper抽出／`.sqlx` 更新 | 新ディレクトリ構成で `cargo test` & `sqlx prepare` 合格 | DBマイグレーションの再確認 |
| Phase 3B | 1週間 | `event_service` モジュール化＋テスト分離 | `event_service/tests.rs` が安定し、Tauriコマンドが再ビルド | Phase 3A 完了（Repository API維持） |
| Phase 3C | 1週間 | `event_manager` 分割＋DefaultTopicsRegistry導入 | Gossip連携テスト成功・フロントビルド成功 | Phase 3B 完了（EventServiceのAPI確定） |
| Phase 3D | 0.5週間 | `iroh` 統合テスト再編とドキュメント更新 | シナリオ別テストが成功し、CI条件を明文化 | Phase 3C までのAPI安定化 |
| Phase 4 | 1週間 | mapper/モック再利用など DRY 適用と TypeScript persist 共通化準備 | 重複コード削減指標達成、Zustand persist 設計案提示 | Phase 3 全完了 |

※ 期間は目安。CI やレビュー状況に応じて調整。

## 7. リスクと対応策
- **`.sqlx/` の破損・差分漏れ**  
  - 対応: 各PRで `sqlx prepare` を必ず再実行し、レビュー時に生成物を確認。必要ならローカルDBを `scripts/` に沿って再初期化。
- **モジュール分割に伴う API 破壊的変更**  
  - 対応: `ApplicationContainer`／Tauriコマンド／テストから事前に依存箇所を列挙、コンパイルエラーで検知しながら段階的に適用。
- **統合テストのタイムアウト増加**  
  - 対応: `support.rs` にタイムアウト値を一本化し、必要に応じて `tracing` でログ出力を強化。CI ではジョブタイムアウトを再設定。
- **レビュー負荷の増大**  
  - 対応: 各フェーズでPRを細切れに出し、ドキュメント更新とコード変更を分離。大規模差分は設計図（本ドキュメント）へのリンクを添付。

## 8. 成果測定指標（KPI）
- 700行超ファイル数: 実施前 4ファイル → 実施後 0ファイル。
- `cargo clippy -D warnings` および `cargo test` の連続成功率: 100% を維持。
- `modules/event` 配下での重複テストモック定義数: 現状 3 箇所 → 実施後 1 箇所（サポートモジュールに集約）。
- `.sqlx/` 差分発生件数: リファクタリング完了後は 0 件（余計な再生成が発生しない状態）。
- 平均レビュー所要時間（目標値）: 1PR あたり 30分以内（レビュアーフィードバック）。

## 9. 即時アクション
1. Phase 3A 用のチケットを `tasks/status/in_progress.md` に追加し、担当・予定期間を明記。
2. `ApplicationContainer` と `Tauri` コマンドの依存一覧を洗い出すリストアップタスクを別途作成（EventService/EventManager 差分検証用）。
3. CI 設定（GitHub Actions）の統合テストジョブで必要な環境変数 (`ENABLE_P2P_INTEGRATION`, `KUKURI_BOOTSTRAP_PEERS`) の指定を確認し、未設定であればワークフロー定義更新タスクを起票。
