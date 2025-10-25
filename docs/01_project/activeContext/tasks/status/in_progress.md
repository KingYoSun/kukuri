[title] 作業中タスク（in_progress）

最終更新日: 2025年10月24日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## v2 アプリ Phase 7 - Mainline DHT 統合（着手）

- [x] Iroh Mainline DHT を有効化する Builder 抽象を整理し、`P2PService` から `discovery_mainline` を切り替え可能にする。
- [x] `ApplicationContainer` / `IrohNetworkService` の DI を見直し、ノード ID・ブートストラップ設定を mainline 向けに注入できる初期化シーケンスを実装。
- [x] Mainline DHT ハンドシェイクとルーティングの統合テストを追加し、既存 DHT/Gossip テストと並行実行できるよう CI 設定を更新。
- [x] Mainline DHT のメトリクス項目（接続数、ルーティング成功率、再接続統計）を収集し、`get_p2p_metrics` に反映。

## OfflineService 再索引ジョブ整備（着手）

- [x] 現状の Repository キャッシュ構造を棚卸しし、再接続時に必要な再索引対象をリストアップ。
- [x] オフライン再索引ジョブの設計（スケジューラ、バックオフ、失敗時リカバリ）を `docs/01_project/activeContext/iroh-native-dht-plan.md` に追記。
- [x] 再起動／再接続シナリオの結合テストを作成し、P2P 経路でのイベント整合性を確認。

## EventService DHT購読・復元の強化

- [x] DHT購読状態を永続化するステートマシンを設計し、`EventService` に実装（8d97b15で `SubscriptionStateMachine` を追加）。
- [x] 再接続時の購読復元シーケンスをテストで検証（離脱→再接続→履歴同期）し、ConnectionEvent監視から呼び出す復元経路を整備。
- [x] UI 側で購読状態と同期状況を可視化するフックを追加し、P2PDebugPanel に購読一覧を表示。
- 2025年10月24日: EventService を `EventGateway` 経由に切り替えるため DI・モック・テストを更新。LegacyEventManagerGateway を追加し、Gateway モックベースで `cargo test` / Docker テストを通過確認。
- 2025年10月25日: Stage3（EventManagerHandle 導入 + Gateway 送信パス結合テスト + Mainline DHT builder 検証）を実施し、`LegacyEventManagerGateway`/`AppState`/`SubscriptionInvoker` から `modules::event` への直接依存を除去。

## エラーハンドリング統一タスク

- [x] フロントエンドの主要フローを `errorHandler` ベースに移行し、`console.error` を廃止。
- [x] Rust/Tauri 側のドメインエラーを `thiserror` でラップし、コマンド境界の共通レスポンスを整理。
- [x] `docs/03_implementation/error_handling_guidelines.md` を更新し、統一フローと実装例を追記。

## 運用/品質・観測（着手）

- [x] Windows での `./scripts/test-docker.ps1` 実行を基本ラインとする運用ガイドを策定し、CI とローカルの手順差異を吸収。
  - 2025年10月20日: `docs/03_implementation/windows_test_docker_runbook.md` を作成し、PowerShell 運用手順と GitHub Actions との主な差分を記録。
  - 2025年10月20日: `windows_test_docker_runbook.md` に Linux/macOS ガイドとの共通化ポイントを整理し、`docker_test_environment.md` との統合方針を検討。
  - 2025年10月20日: `scripts/run-rust-tests.ps1` を追加し、Windows から Docker 経由で Rust テストを呼び出す自動化フローを整備。`docker_test_environment.md` / `windows_test_docker_runbook.md` に運用例を追記。
  - 2025年10月21日: `windows_test_docker_runbook.md` と `docker_test_environment.md` の重複セクションを精査し、コマンド表・トラブルシュート統合の対応順を整理。
  - 2025年10月21日: `scripts/metrics/collect-metrics.{ps1,sh}` を追加し、PowerShell 版で TODO / `any` / `#[allow(dead_code)]` 集計を自動化。成果物を `artefacts/metrics/2025-10-21-collect-metrics.json` に保存。
  - 2025年10月22日: `windows_test_docker_runbook.md` に CI 対応表とチェックリストを追加し、`docker_test_environment.md` の Windows 節を runbook 参照に整理。
  - 2025年10月22日: `.\scripts\test-docker.ps1` を実行し、Runbook の CI 等価性チェックリスト（全テストフロー・成果物確認・ログ確認）を完了したことを検証。
- [x] ドキュメントの日付表記を `YYYY年MM月DD日` に統一するルールを整理し、主要ドキュメントの棚卸しを行う。
  - 2025年10月21日 着手: `rg '202[0-9]-[0-9]{2}-[0-9]{2}' docs` で旧表記の候補を抽出し、`tasks/README.md`・`refactoring_plan_2025-08-08_v3.md` など修正対象リストを作成開始。
  - 2025年10月22日: `docs/01_project/activeContext/artefacts/document_date_format_inventory.md` を作成し、ルール整理と主要ドキュメントの棚卸し結果を記録。Python スクリプトによるゼロ埋め未対応箇所の一覧を整理。
  - 2025年10月22日: 棚卸しで「要修正」となっていた主要ドキュメントのゼロ埋め対応を完了し、同 inventory を対応済みに更新。
  - 2025年10月22日: ProgressReports や完了タスクアーカイブ等を含む `docs/**/*.md` をスクリプトで一括整形し、非ゼロ埋め表記を完全排除。
  - 2025年10月22日: `scripts/check_date_format.py` を追加し、`format-check` ワークフローから自動検証するよう CI を更新。
- [x] Phase 5 CI/ローカルスクリプトのテストモジュール移行対応
  - 2025年10月21日: `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` を確認し、PowerShell スクリプト→Bash→Compose→Runbook の順で更新する方針を設定。
  - [x] `scripts/docker/run-smoke-tests.sh` を `tests/` 配下の `p2p_mainline_smoke` 等へ切り替え、P2P ランブックを最新手順に更新。
  - [x] `scripts/test-docker.sh` の `TESTS` 既定値と `cargo --lib` 呼び出しを新しい `tests::integration::p2p::*` 構成へ移行し、ヘルプ出力と `docs/03_implementation/docker_test_environment.md` を修正。
  - [x] `scripts/test-docker.ps1` の `cargo test` 呼び出しを `--test` ベースに更新し、ログ文言と PowerShell オプション説明を調整。
  - [x] `docker-compose.test.yml` へ `./kukuri-tauri/src-tauri/tests` マウントを追加し、Rust テスト編集を即時反映できるようにする。
- [x] `docs/03_implementation/p2p_mainline_runbook.md` 等 Phase 5 連動ドキュメントのスモークテスト記述を `tests/p2p_*` バイナリに統一。
- 2025年10月22日: 新テストバイナリ（`p2p_gossip_smoke` / `p2p_mainline_smoke`）を追加し、旧スモークテスト資産を移設。Docker Compose のマウント更新とランブック改訂を完了。
  - 2025年10月22日: `scripts/docker/run-smoke-tests.sh` / `test-docker.{sh,ps1}` のフォールバックロジックを撤廃し、新バイナリ固定実行に切り替え。
  - [x] TypeScript 側の Phase 5 テスト再編（`src/__tests__` → `src/tests/*` への移行、重複整理、DI 周りの統合ケース追加）を実施し、`phase5_test_inventory.md` の不足項目（Hooks/Stores/Integration のギャップ）を解消する。進捗は artefact 更新で追跡。

## リファクタリング計画フォローアップ

- [x] Phase 2 TODO 解消: `event_service` の未実装処理（Post変換・メタデータ更新・Reaction/Repost処理）を完了し、`EventManager` との連携を整備する（`kukuri-tauri/src-tauri/src/application/services/event_service.rs:122`）。
- [x] Phase 2 TODO 解消: `offline_service` の Repository 統合タスクを実装し、同期/キャッシュ関連の TODO を解消する（`kukuri-tauri/src-tauri/src/application/services/offline_service.rs:134`）。
- [x] Phase 2 TODO 解消: トピック更新・削除コマンドの未実装部分を実装し、フロントからの操作を完了させる（`kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs:99`）。
- [x] Phase 3/4 ギャップ対応: 700行超のファイル（`kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs:1003`, `kukuri-tauri/src-tauri/src/application/services/event_service.rs:341`, `kukuri-tauri/src-tauri/src/modules/event/manager.rs:240`）の分割計画を策定し、リファクタリングタスクへ落とし込む。
- [x] Phase 3D: `modules/p2p/tests/iroh/` への統合テスト再編（support抽出・シナリオ別ファイル分割・Runbook/Planの更新）を完了させる。
- [x] Phase 4 DRY 適用
  - [x] 共有モジュール `application/shared` を追加し、Sqliteマッパーと Nostr ファクトリの基盤を共通化。
  - [x] EventService / EventManager のイベント生成ロジックを `shared::nostr` に統合し、DefaultTopicsRegistry を共有ユーティリティ化する。
  - [x] `modules/event` / `modules/p2p` テスト支援コードを `application/shared/tests` に集約し、重複モック・ロガーを解消する。
  - [x] Zustand 永続化テンプレート（`withPersist` / `config/persist.ts`）を整備し、Map 含むストアで `createMapAwareStorage` を適用。テスト用 `setupPersistMock` を導入する。
  - [x] `.sqlx/` 更新手順とローカルストレージキー移行のリスク評価を `docs/03_implementation/sqlx_best_practices.md` 等へ反映し、後方互換検証結果を記録する。
  - [x] `docs/03_implementation/p2p_mainline_runbook.md` に共有モジュール化後の P2P テスト/運用手順を追記する。
  - [x] `docs/01_project/activeContext/tauri_app_implementation_plan.md` に Zustand 永続化共通化の設計と移行手順をまとめる。
- [x] Phase 5 成果測定: dead code 数やテストカバレッジといった指標を `tasks/metrics/` 配下で定期記録する運用を整備する。
  - [x] Phase5-01 依存関係棚卸しテンプレートを定義し、artefact 保存ディレクトリ（`docs/01_project/activeContext/artefacts/`）を準備する。
  - [x] Phase5-02 `cargo tree --edges features` などで基礎データを取得し、artefact を生成して共有する。
  - [x] Phase5-03 既存テストを種別ごとに分類した表を作成し、移動対象と不足領域を洗い出す。
  - [x] Phase5-04 CI／ローカルスクリプトのパス依存箇所を調査し、修正候補をリスト化する。
- [x] Phase 5 テスト移行: `kukuri-tauri/src-tauri/src/application/services/event_service/tests` を `tests/unit/application/event_service` へ移設し、EventManager 依存を `tests/common` モックへ集約する（`docs/01_project/activeContext/artefacts/phase5_test_inventory.md`）。
- [x] Phase 5 テスト移行: `kukuri-tauri/src-tauri/tests/integration` 配下に Mainline DHT 向けの P2P シナリオを追加し、Docker/CI 手順を `phase5_test_inventory.md` の更新内容と整合させる。
- [x] Phase 5 テスト移行: 契約テスト（`tests/contract/nip10.rs`）を再配置し、モジュール参照と CI / Runbook / Docker スクリプトをレイヤ構成に沿って更新する（`docs/01_project/activeContext/artefacts/phase5_test_inventory.md`）。
  - 2025年10月23日: 旧 `tests/nip10_contract_tests.rs` を `tests/contract.rs` 経由のモジュール構成へ移し、`env!("CARGO_MANIFEST_DIR")` 連結で `testdata` を参照するよう修正。
  - 2025年10月23日: `scripts/test-docker.ps1` の `contracts` コマンドを Rust 新モジュールと TypeScript 契約テスト（`src/tests/unit/lib/nip10.contract.test.ts`）へ追随させ、`test-runner` サービス経由で実行するよう調整。
  - 2025年10月23日: `./scripts/test-docker.ps1 contracts` を実行し、Rust / TypeScript 契約テストを Docker で完走（Rust 側は既知の `description` 未使用警告のみ発生）。
- [x] Phase 5 依存関係棚卸し TODO 対応（`docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md`）
  - 2025年10月23日: 主要サービス/Repository/コマンド/Legacy 25件を棚卸しし、High 難易度項目の対策メモを `tauri_app_implementation_plan.md` に追記。外部クレートもカテゴリ別に整理した。
  - 2025年10月23日: EventService/EventManager 向けの `EventGateway` 抽象と mapper 整理案を `docs/01_project/activeContext/artefacts/phase5_event_gateway_design.md` にまとめ、Sprint 1〜3 の粒度を定義した。
  - 2025年10月23日: Sprint 1 着手タスク（EG-S1-01〜04）を `phase5_event_gateway_design.md` に起票し、`application::ports::event_gateway.rs` / `application/shared/mappers/event/*` の実装前準備を完了。
  - 2025年10月23日: OfflineService/OfflineManager の adapter 方針と `infrastructure::offline` への段階移行計画を `docs/01_project/activeContext/artefacts/phase5_offline_adapter_plan.md` に記載し、Stage 0〜3 のタスクを整理した。
  - 2025年10月23日: Offline Stage 0 タスク（OFF-S0-01〜03）を起票し、`.sqlx` 影響調査メモを同 artefact に追記して再生成タイミングを明確化。
  - 対応方針: 棚卸し結果を基に Workstream A/B の移行順序を定義し、優先度の高い High 項目からリファクタリングに着手する。
- [x] Phase 5 Workstream A: Rustモジュール再構成（`docs/01_project/refactoring_plan_2025-08-08_v3.md`）
  - [x] **WSA-01 EventGateway Stage 2**: `LegacyEventManagerGateway` を `infrastructure::event` へ移設し、`state/application_container.rs`・各ハンドラーを `Arc<dyn EventGateway>` 注入に切り替える（参照: `phase5_event_gateway_design.md` Sprint 2）。
    - 2025年10月24日: `LegacyEventManagerGateway` を `infrastructure/event` 配下へ移設し、`application/services/event_service` からは trait のみを参照。`state.rs` の DI も `Arc<dyn EventGateway>` を介す構成に更新し、`cargo fmt` / `./scripts/test-docker.ps1 rust` を実行して回帰確認。
  - [x] **WSA-02 Offline Persistence Stage 1/2**: `application::ports::offline_store`・`LegacyOfflineManagerAdapter` を実装し、続けて `infrastructure/offline/sqlite_store.rs` へ移行する（参照: `phase5_offline_adapter_plan.md` Stage1-2）。
    - 2025年10月24日: `OfflineService` を `Arc<dyn OfflinePersistence>` で初期化できるよう刷新し、`LegacyOfflineManagerAdapter` を infrastructure/offline に追加。DI からの注入を更新し、Docker 経由の Rust テストで検証。Stage 2 向けに `sqlite_store.rs` をプレースホルダーとして追加（実実装は後続タスク）。
    - 2025年10月24日: `SqliteOfflinePersistence` を実装して Legacy 依存を排除。`state.rs` からの DI・OfflineService テストを新実装へ切り替え、`cargo fmt` → `cargo clippy -- -D warnings` → `./scripts/test-docker.ps1 rust` を実行しレグレッションを確認。`infrastructure/offline/mod.rs` の公開 API も新レイヤ構成に合わせて整理。
    - 2025年10月25日: ドメイン型ポートと `infrastructure/offline/mappers.rs` を追加し、OfflineService/Handler を新インターフェイスで再配線。Docker Rust テスト（`./scripts/test-docker.ps1 rust`）で回帰確認し、Windows 既知の `STATUS_ENTRYPOINT_NOT_FOUND` は Docker 実行で回避。
  - [x] **WSA-03 Bookmark Repository 移行**: `domain::entities::bookmark` と `infrastructure::database::bookmark_repository` を追加し、`PostService`／Tauri コマンドを新 Repository 経由に再配線する。
    - 2025年10月24日: Bookmark ドメイン値オブジェクト・エンティティを追加し、`BookmarkRepository` を通じて `SqliteRepository` にブックマーク CRUD を実装。`20251024093000_update_bookmarks_table` マイグレーションを追加し、`PostService`／`AppState` の DI を新リポジトリ経由へ移行、ユニットテストを整備。
  - [x] **WSA-04 SecureStorage / Encryption 再配線**: SecureStorage debug ユーティリティと暗号トレイトを Infrastructure 層へ統合し、`AppState`・`SecureStorageHandler` の依存を刷新する。
    - 2025年10月25日: `domain::entities::account` を追加してアカウントメタデータ/登録用 VO を定義。`application::ports::secure_storage::SecureAccountStore` を導入し、`DefaultSecureStorage` がポート実装を提供するよう再構成。`SecureStorageHandler`／コマンド群／DI を新ポートに切り替え、Legacy `EncryptionManager` を `infrastructure::crypto::DefaultEncryptionService`（`EncryptionService` trait）へ置換。`cargo fmt` / `cargo clippy -- -D warnings` / `./scripts/test-docker.ps1 rust` を実行し回帰確認。
  - [x] **WSA-05 Legacy Database Connection 廃止**: 全呼び出しを `ConnectionPool` + Repository へ揃え、`.sqlx` を再生成した上で `modules::database::connection` を撤去する。
    - 2025年10月25日: Legacy `DbPool` 依存箇所（`state`／`EventManager`／`EventHandler`／`OfflineManager`）の洗い出しを完了し、`ConnectionPool` 経由に統一するリファクタリング手順を整理。コード着手と同期して `.sqlx` 再生成タイミングを確認中。
    - 2025年10月25日: `state.rs` / `EventManager` / `EventHandler` を `ConnectionPool` ベースの DI へ再配線し、`modules::database::{connection,models}` を削除。Docker Rust テストで回帰を確認。
    - 2025年10月25日: 依存棚卸し関連ドキュメント（`tauri_app_implementation_plan.md` / `refactoring_plan_2025-08-08_v3.md` / `phase5_dependency_inventory_template.md`）を更新し、Legacy Database Connection の完了を明記。
  - 2025年10月24日: `domain/p2p` を新設し、GossipMessage／TopicMesh／P2PEvent を移設。`modules::p2p` はリダイレクト化し、`lib.rs`・`state`・P2P/Gossip サービスなど主要呼び出し元を `domain::p2p` 参照へ切り替えた。`cargo fmt` / `cargo clippy -D warnings` / Docker 経由の `cargo test` を完了（ローカル `cargo test` は Windows 既知の STATUS_ENTRYPOINT_NOT_FOUND のため Docker 実行で代替）。
  - 2025年10月24日: `modules/p2p` 配下を完全撤去し、ドメイン層テストを `domain/p2p/tests` へ移設。`test_support` と統合テストを `domain::p2p` 参照に更新し、古い `modules::p2p` 参照を排除した。`cargo fmt` / `cargo clippy -D warnings` / Docker Rust テストで回帰確認（Windowsネイティブ `cargo test` は STATUS_ENTRYPOINT_NOT_FOUND の既知事象）。
  - 2025年10月24日: `refactoring_plan_2025-08-08_v3.md` に Legacy モジュール棚卸し表（event/offline/bookmark/secure_storage/crypto/database）を追記し、`phase5_dependency_inventory_template.md` に BookmarkManager／Legacy SecureStorage／Legacy EncryptionManager の行を追加。Workstream A ロードマップへ段階移行案を反映。
  - 2025年10月24日: `p2p_mainline_runbook.md`・`iroh-native-dht-plan.md`・`phase5_ci_path_audit.md`・`refactoring_phase34_gap_plan.md` ほか Phase 5 関連ドキュメントのテストコマンドを `tests/p2p_*` バイナリに統一し、旧モジュールパスの記述を除去。
- [x] Phase 5 OfflineService Adapter Stage 1（`docs/01_project/activeContext/artefacts/phase5_offline_adapter_plan.md`）
  - [x] Stage1-1: `application::ports::offline_store` を追加し、DI からポートを注入できるよう準備する。
    - 2025年10月24日: `application::ports::offline_store` に `OfflinePersistence` trait を定義し、既存サービスから `OfflineManager` 直接参照を排除。`cargo fmt` を実行。
  - [x] Stage1-2: `LegacyOfflineManagerAdapter` を実装し、既存 OfflineManager を暫定的にラップしてモックテストを更新する。
    - 2025年10月24日: infrastructure/offline に `LegacyOfflineManagerAdapter` を配置し、`state.rs` / OfflineService テスト双方で `Arc<dyn OfflinePersistence>` 注入を確認。`./scripts/test-docker.ps1 rust` にて回帰テストを完走。
  - [x] Stage1-3: `OfflineService` の公開型を新 VO へ差し替え、変換箇所を整理して Stage 2 で差し替え可能な状態にする。
    - 2025年10月25日: `OfflineService` / `OfflinePersistence` / `LegacyAdapter` / ハンドラーをドメイン値オブジェクト対応へ刷新。UI DTO 変換ヘルパを整備し、TypeScript 側の互換性を確認。Docker Rust テスト（`./scripts/test-docker.ps1 rust`）で回帰検証。
- [x] Phase 5 OfflineService Adapter Stage 2（Stage 1 完了後着手／`docs/01_project/activeContext/artefacts/phase5_offline_adapter_plan.md` Stage 2）
  - [x] Stage2-1: `infrastructure/offline/sqlite_store.rs` を実装し、Legacy OfflineManager の CRUD をポート実装へ移植する。
    - 2025年10月24日: `SqliteOfflinePersistence` へ CRUD ロジックを移設し、`serde_json` 変換・同期キュー処理・楽観的更新系 API を `AppError` 戻り値に統一。
  - [x] Stage2-2: DI を新実装へ切り替え、`modules/offline` を read-only ラッパに縮退させる。
    - 2025年10月24日: `state.rs` と OfflineService テストを新実装に差し替え、`infrastructure/offline/mod.rs` から Legacy 再エクスポートを除去。Docker Rust テストで回帰確認。
  - [x] Stage2-3: OfflineService 結合テストを `tests/integration/offline/*` へ移動し、Docker/CI スクリプトを更新する。
    - 2025年10月25日: `application/services/offline_service.rs` の統合テスト群を `tests/integration/offline/mod.rs` へ移設し、SQLite 初期化と `SqliteOfflinePersistence` の DI を再確認。`cargo test --test offline_integration` を Phase 5 ランブックの手順に織り込み可能な構成にした。
- [x] Phase 5 OfflineService Adapter Stage 3（Legacy 解体／`docs/01_project/activeContext/artefacts/phase5_offline_adapter_plan.md` Stage 3）
  - 2025年10月25日: `modules/offline`（manager/models/reindex/tests）と `infrastructure/offline/legacy_adapter.rs` を削除し、`SqliteOfflinePersistence` へ監視系 API を集約。`OfflineReindexJob` を新モジュールで再実装し、`state.rs` の DI を更新。
  - 2025年10月25日: Rust ユニットテストを `infrastructure/offline/{sqlite_store,reindex_job}.rs` 内へ移設。`phase5_dependency_inventory_template.md` / `phase5_offline_adapter_plan.md` を Stage 3 完了内容で更新。ローカル `cargo test` は `cc` リンクエラーで失敗（要 Docker 実行）だが、ビルド・フォーマットまでは完了。
  - 2025年10月24日: `sync_status` テーブルが行 ID カラムを持たない問題に合わせて `SELECT rowid AS id` を適用し、`list_sync_conflicts` ヘルパーが `SyncStatusRecord` を復元できるよう修正。Stage 3 の QA テストが `cargo test` でグリーンになったことを確認。
- [x] Phase 5 EventGateway Sprint 2（`docs/01_project/activeContext/artefacts/phase5_event_gateway_design.md`）
  - [x] Sprint2-1: `infrastructure/event/event_manager_gateway.rs` を実装し、Legacy EventManager への委譲と mapper 呼び出しを整理する。
  - [x] Sprint2-2: `state.rs` / `application_container.rs` の DI を Gateway 経由に更新する。
  - [x] Sprint2-3: `modules/event/manager` の Presentation 依存を Gateway 側へ閉じ込めるラッパを追加する。
  - [x] Sprint2-4: Mainline DHT / EventService 結合テストを Gateway 経由で実行するよう更新する。
  - 2025年10月25日: `tests/integration/test_event_gateway.rs` を追加し、Gateway → EventManager → SQLite 永続化フローを実データベースで検証。P2P（Mainline）経由で受信した DomainEvent が mapper を通じて `events` / `event_topics` テーブルへ反映されることを確認し、Phase 5 Runbook の統合テスト要件に沿うよう更新済み。
  - 2025年10月24日: `LegacyEventManagerGateway` に `AppHandle` セッタと UI emit 機能を集約し、`EventManager` 本体から Presentation 依存を排除。DI で `Arc<dyn EventGateway>` を注入するよう `state.rs` を更新し、Gateway 単体テストを追加済み。
- [x] Phase 5 EventGateway Sprint 3（SubscriptionInvoker ポート化／mapper 残課題：`phase5_event_gateway_design.md` 99-112行）
  - [x] Sprint3-1: SubscriptionInvoker を `application/ports` へ切り出し、`LegacyEventManagerGateway` と `EventManagerHandle` を新ポート経由で再配線して Gateway からの直接参照を排除する。
  - [x] Sprint3-2: `modules/event/manager::conversions` に残っている Nostr ↔ Domain 変換を `application/shared/mappers/event` へ完全移管し、`phase5_event_gateway_design.md` の更新内容を反映する。
  - [x] Sprint3-3: Gateway のメトリクスフック追加・EventManager ユニットテスト再配置（`application/shared/tests`）・NIP-65 など新 DTO の mapper 対応をまとめて実施し、`docs/03_implementation/p2p_mainline_runbook.md` へ検証手順を追記する。
  - 2025年10月25日: `application::ports::subscription_invoker.rs` を追加し、`EventManagerSubscriptionInvoker` を `infrastructure::event` へ移設。`state.rs`・Unit/Integration テストの import を更新し、`EventService` からはポートのみを参照する構成に刷新。
  - 2025年10月25日: Legacy `modules/event/manager/conversions.rs` を廃止し、`nostr_event_to_domain_event` を `application/shared/mappers/event/nostr_to_domain.rs` へ統合。`p2p.rs` は新 mapper 経由で DomainEvent を生成する。
  - 2025年10月25日: `infrastructure::event::metrics` を新設し、Gateway の主要 API に成功/失敗カウンタを付与。`tests/infrastructure/event/event_manager_gateway.rs` にメトリクス検証ケースを追加し、`LegacyEventManagerGateway` の回帰テストを拡充。
- [x] Phase 5 Legacy KeyManager 移行（参照: `docs/01_project/refactoring_plan_2025-08-08_v3.md` 343行／`docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md` 28行）
  - [x] Stage1: `application::ports::key_manager` を追加し、`AppState`／SecureStorage ハンドラー／Tauri コマンドが `Arc<dyn KeyManager>` の DI で動作するよう更新。`DefaultKeyManager` を Infrastructure 層実装として再配置。
  - [x] Stage2: `LegacyEventManagerGateway` / `EventManagerHandle` / `SubscriptionInvoker` から旧 `modules::auth::key_manager` 依存を排除し、Gateway/Topic/Post 系テストを新ポート経由に切り替え。
  - [x] Stage3: `modules::auth::key_manager` と関連テスト・再エクスポートを削除し、`refactoring_plan_2025-08-08_v3.md`／`phase5_dependency_inventory_template.md` を更新。Rust/TS テスト系は Phase 5 既定フロー（`cargo fmt` / `cargo clippy -- -D warnings` / `./scripts/test-docker.ps1 rust` / `pnpm test`）で回帰確認。
  - 2025年10月25日: `application::ports::key_manager` + `DefaultKeyManager` への統一により、Legacy KeyManager はアーカイブ済み。AppState/Tauri/EventManager/SubscriptionInvoker からの直接依存は解消され、ドキュメント＆タスクも更新完了。
- [x] Phase 5 Legacy SecureStorage デバッグユーティリティ整理（参照: `docs/01_project/refactoring_plan_2025-08-08_v3.md` 342行／`docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md` 31行）
  - [x] Stage1: `DefaultSecureStorage`（または `SecureAccountStore`）に debug 用クリア API を追加し、`clear_all_accounts_for_test` が Legacy `modules::secure_storage` を参照しない構成へ変更。`cargo fmt` / `./scripts/test-docker.ps1 rust` / `pnpm test` を実行して互換性を確認。
  - [x] Stage2: `modules::secure_storage` と関連テストを削除し、`lib.rs`／コマンド登録／TypeScript API（変更なし）を新ユーティリティへ置換。ランブックと依存棚卸しドキュメントを更新し、debug 手順を最新化。
  - 2025年10月25日: `DefaultSecureStorage::clear_all_accounts_for_test` を追加し、Tauri `clear_all_accounts_for_test` コマンドを新実装へ接続。Legacy 依存を排除しつつ Keyring 操作は Infrastructure 層へ集約。
  - 2025年10月25日: `modules/secure_storage`（mod/tests）を削除し、`refactoring_plan_2025-08-08_v3.md` / `phase5_dependency_inventory_template.md` に完了状況を記録。Windows `cargo test` は `STATUS_ENTRYPOINT_NOT_FOUND`（既知）で停止したが、`./scripts/test-docker.ps1 rust` / `pnpm test` / `kukuri-cli cargo test` で回帰を確認。
- [ ] Phase 5 BookmarkManager アーカイブ（参照: `docs/01_project/refactoring_plan_2025-08-08_v3.md` 341行／`docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md` 30行）
  - [ ] Stage1: `modules::bookmark`（manager/tests）を `state.rs` や `presentation::handlers::post_handler` から完全に切り離し、`BookmarkRepository` 経路のみで bookmark API が動作することを `pnpm test` / `./scripts/test-docker.ps1 rust` で検証。
  - [ ] Stage2: Legacy モジュール／テストを削除し、`modules/mod.rs` 再エクスポートと関連ドキュメントを更新。Migration 後に `.sqlx`／Plan／Runbook をメンテナンスし、完了ログを `tasks/completed/YYYY-MM-DD.md` へ記録。
関連: `docs/01_project/activeContext/iroh-native-dht-plan.md`

-メモ/進捗ログ:
- 2025年10月17日: Iroh DHT/Discovery 残タスクを完了し、Mainline DHT 統合フェーズへ移行。Phase 7 の残項目（Mainline DHT/OfflineService/EventService/エラーハンドリング）を次スプリントの主テーマに設定。
- 2025年10月17日: 運用・品質セクションの TODO を見直し、メトリクス更新フローと Windows テスト運用の標準化タスクを切り出した。
- 2025年10月20日: 運用/品質・観測タスク群の実作業を開始。メトリクス更新フロー整備と Windows テスト運用ガイド策定に向けて現状調査を進行中。
- 2025年10月20日: `update_flow.md` と `windows_test_docker_runbook.md` を作成し、メトリクス更新手順と PowerShell 運用ガイドのドラフトを共有。
- 2025年10月20日: メトリクス初回収集で `pnpm test` / `pnpm lint` / `cargo test` 等を実行し、Vitest JSON アーティファクトと `log_2025-10.md` を整備。Windows での Rust テスト失敗と Lint 未解決項目をギャップとして記録。
- 2025年10月20日: `scripts/run-rust-tests.ps1` 経由で Rust テストの Docker 実行を試行。`docker compose` の Docker Hub 認証失敗（503）を確認し、初回実行時のネットワーク要件をドキュメントへ反映。
- 2025年10月20日: Phase 3D チケットとして iroh 統合テスト再編を着手。`modules/p2p/tests/iroh/` にシナリオ別モジュールを作成し、テストユーティリティ/Runbook/計画ドキュメントの更新方針を確定。
- 2025年10月20日: `scripts/test-docker.ps1` に `-Integration` オプションを実装し、`BootstrapPeers`/`IrohBin`/`IntegrationLog` パラメータで Docker 経由の統合テストを再現できるよう調整。
- 2025年10月20日: `./scripts/test-docker.ps1 integration -BootstrapPeers "<node_id@127.0.0.1:11233>"` を実行し、Docker 上で P2P 統合テストが成功したことを確認。`KUKURI_IROH_BIN` 未指定でもホスト環境依存の問題なく完走することを検証。
- 2025年10月17日: `DiscoveryOptions` と `P2PService::builder` を導入し、Mainline DHT 切替対応のためのP2Pスタック組み立てを再構成。
- 2025年10月21日: GitHub Actions `Native Test (Linux)` で発生していた TypeScript 型エラーを解消。Zustand 永続化ヘルパー（`persistHelpers.ts`）と `src/stores/config/persist.ts` をジェネリクス対応へ更新し、`pnpm type-check` と `gh act -j native-test-linux -W .github/workflows/test.yml` の成功を確認。
- 2025年10月21日: Prettier ルールに合わせて `src/stores/config/persist.ts` を整形し、`pnpm format:check` および `gh act -j format-check -W .github/workflows/test.yml` が成功することを確認。
- 2025年10月23日: GitHub Actions の `clippy::new_without_default` 失敗を解消するため、EventPublisher/TestGossipService/KeyManager/EventManager に `Default` を実装し、`cargo fmt` を実行。`gh act -j native-test-linux` と `gh act -j format-check` が成功することを確認。
- 2025年10月23日: Format Check ジョブが Prettier 警告（`src/tests/...` 5件）で失敗していたため、対象テストファイルを `pnpm prettier --write` で整形し、`pnpm format:check` と `gh act -j format-check -W .github/workflows/test.yml` が成功することを確認。
- 2025年10月23日: EventService ユニットテスト群を `tests/unit/application/event_service` へ移設し、`tests/common/mocks` に EventManager スタブとモック群を整理。`test_p2p_mainline.rs` を追加して Mainline DHT 設定の統合シナリオを整備。
- 2025年10月17日: `ApplicationContainer` を導入し、Base64 永続化した iroh シークレットキーからノード ID を再利用する初期化と、`NetworkConfig.bootstrap_peers` を `IrohNetworkService` 初期化時に適用する仕組みを整備。Docker 経由の `cargo test` と `kukuri-cli` のテストまで確認済み。
- 2025年10月17日: Mainline DHT ハンドシェイク/ルーティング統合テストを `mainline_dht_tests.rs` に追加し、Docker スモークテストで DHT/Gossip と並行実行するよう `run-smoke-tests.sh` を更新。
- 2025年10月17日: Mainline DHT の接続・ルーティング・再接続メトリクスを Rust 側で集計し、`get_p2p_metrics`／P2PDebugPanel に反映。Docker 経由で Rust テストと `pnpm test` を通過。
- 2025年10月20日: Phase 4 ドキュメント整備として `.sqlx` 更新手順（`docs/03_implementation/sqlx_best_practices.md`）、P2P ランブック、Tauri 実装計画の各ガイドを更新し、共有モジュール化後のフローと永続化テンプレートを明文化。
- 2025年10月18日: `SubscriptionStateMachine` を `kukuri-tauri/src-tauri/src/application/services/subscription_state.rs` に導入し、`nostr_subscriptions` テーブルで購読対象・状態・再同期時刻を管理。接続断検知で `needs_resync` へ遷移し、再接続時に `EventService::handle_network_connected` から自動復元する流れを実装。
- 2025年10月18日: `list_nostr_subscriptions` コマンドと `useNostrSubscriptions` フックを追加し、`P2PDebugPanel` に購読対象・最終同期時刻・失敗回数を可視化するセクションを組み込み。
- 2025年10月18日: GitHub Actions の Format Check 失敗を確認し、`src/components/P2PDebugPanel.tsx` と `src/stores/offlineStore.ts` を Prettier で整形。`pnpm format:check` が成功することをローカルで確認。
- 2025年10月18日: `gh act` で `format-check`・`native-test-linux` ジョブが成功することを確認。Docker Test Suite は `docker compose -f docker-compose.test.yml run --rm test-runner /app/run-tests.sh` で完走（`gh act` 実行環境ではボリューム制限により確認不可）。
- 2025年10月18日: P2P接続イベントから再索引ジョブをトリガーするウォッチャーと、`offline://reindex_*` イベントに応答してUIストアを更新する処理を実装。
- 2025年10月18日: `IrohNetworkService` の接続イベントを用いた再索引結合テストを追加し、再接続時に同期キューへ再投入されることを検証。
- 2025年10月18日: OfflineService の再索引ジョブ整備タスクに着手。現状の Repository キャッシュ構造と再接続時の課題を洗い出すための調査を開始。
- 2025年10月18日: GitHub Actions ワークフロー失敗（最新 `main` 向け CI）について Codex CLI で調査を開始。`gh` コマンドで失敗ログを確認予定。
- 2025年10月18日: `clippy::useless_conversion` による CI 失敗を特定し、該当コマンド群から冗長な `map_err(AppError::from)` を削除。`cargo clippy -D warnings` と `cargo test` を Docker 経由で再実行し、Rust テストが通ることを確認。
- 2025年10月18日: フロント主要UIとユーティリティの `errorHandler` への統一・ESLint `no-console` 追加、および Rust 側 `AppError`/`ApiResponse` の共通化を実施。`post_commands.rs` と `topic_commands.rs` など主要コマンドを `ApiResponse::from_result` で揃え、ドキュメントを更新。
- 2025年10月19日: 残っていた `p2p`/`event`/`offline`/`secure_storage`/`auth` など全ての Tauri コマンドを `AppError` + `ApiResponse` へ統一。TypeScript 側は共通 `invokeCommand` ヘルパーを追加し、API ラッパー・テストを新シグネチャに追随。
- 2025年10月19日: `EventService::process_received_event` の Phase 2 TODO を EventManager 連携で解消し、`topic_commands.rs` の更新/削除コマンドを TopicService に接続。OfflineService の API 仕様と実装の乖離を調査し、次の対応方針を整理中。
- 2025年10月19日: OfflineService/Handler を OfflineManager ベースの実装に刷新し、Tauri 側 DTO を camelCase スキーマへ統一。`save_offline_action` で entityType/entityId を含む JSON を永続化できるよう調整し、取得・同期・メタデータ更新系のコマンドも更新済み。`cargo fmt` / `cargo test` がWindows環境でも通過することを確認済み。
- 2025年10月19日: EventServiceの削除イベントをEventManager経由の発行に切り替え、OfflineService向けにインメモリSQLiteを用いた単体テスト群を追加。`cargo test` をローカルで完走し、Phase 2 TODO の完了を確認。
- 2025年10月24日: OFF-S0-02 の仮実装としてドメイン変換ヘルパを追加し、OfflineService から新 VO へ変換するプレースホルダを設置。段階移行用の TODO を明示しつつ、Docker Rust テストで回帰確認。
- 2025年10月19日: TopicService/TopicHandler 経由でユーザー単位の参加状態を扱えるようにし、`join_topic`/`leave_topic`/`get_topic_stats` コマンドを Phase 2 仕様へ更新。Tauri/TypeScript 双方を揃え、`cargo test` で後方互換を確認。
- 2025年10月19日: Phase 3/4 ギャップ対応に着手。`sqlite_repository.rs`/`event_service.rs`/`modules/event/manager.rs` を再調査し、700行超ファイルの分割・DRY 方針を `docs/01_project/activeContext/refactoring_phase34_gap_plan.md` に整理。`modules/p2p/tests/iroh_integration_tests.rs`（702行）を新規対象として追加。
- 2025年10月19日: 上記計画ドキュメントに背景・スコープ・ロードマップ・リスク・KPI を追記し、フェーズ別の成果物と検証条件を明文化。
- 2025年10月19日: Phase 3A（SqliteRepository 分割）に着手し、現行実装の棚卸しと移行ステップを `docs/01_project/activeContext/refactoring_phase3a_sqlite_repository_plan.md` にまとめ。Post/Topic/User/Event 各リポジトリの責務・依存・検証手順を洗い出し、`.sqlx/` 更新手順を明示。
- 2025年10月19日: Phase 3A の実装を完了し、`sqlite_repository` を posts/topics/users/events/mapper/queries モジュールへ分割。SQLx オフラインデータを再生成し、`cargo fmt` / `cargo clippy -- -D warnings` / `./scripts/test-docker.ps1 rust` で検証済み。
- 2025年10月19日: GitHub Actions ワークフロー失敗の原因特定に着手。`gh run list` で最新失敗ジョブを洗い出し、`gh act` によるローカル再現準備を開始。
- 2025年10月19日: `src/lib/api/tauri.ts` を Prettier で整形し、`pnpm format:check` と `gh act -j format-check` が成功することを確認。CI フォーマットジョブの失敗を解消。
- 2025年10月19日: Phase 3B（EventService モジュール化）実行計画を `docs/01_project/activeContext/refactoring_phase34_gap_plan.md` に追記し、WBS/検証/リスク/完了条件を明文化。
- 2025年10月19日: EventService を `core`/`subscription`/`distribution`/`factory`/`invoker` 構成へ分割し、テストを `tests/` 配下に再配置。`cargo fmt` / `cargo clippy -- -D warnings` / `pnpm test` は完了、Rust テストは Docker 実行で `offline_service::test_update_cache_metadata_and_cleanup` が既存失敗として継続中。
- 2025年10月20日: P2PServiceのmessage_count統計をTopicMeshベースで提供するよう更新し、Mockテストでカバレッジを追加。OfflineReindexJobにイベントエミッタの抽象化を導入し、完了イベントの監視パスをユニットテストで検証。
- 2025年10月20日: Windows 環境で `cargo test` が `STATUS_ENTRYPOINT_NOT_FOUND` により異常終了。Docker 経由（`./scripts/test-docker.ps1 rust`）で Rust テストを再確認予定。
- 2025年10月20日: Phase 3C（EventManager 分割）実装を開始。DefaultTopicsRegistry 抽出とモジュール構成の整備に着手し、`refactoring_phase34_gap_plan.md` に沿って進行予定。
- 2025年10月20日: P2PDebugPanel テストを調整し、`useNostrSubscriptions` モック化と `import.meta.env.MODE` 設定で `act(...)` 警告を解消。`pnpm test` が警告なしで通過することを確認。
- 2025年10月20日: MarkdownPreview の DOM 構造を見直し、メディア埋め込み時に `<p>` 配下へ `<div>` が入らないよう段落レンダラを調整。`pnpm test` で `validateDOMNesting` 警告が解消されたことを確認。
- 2025年10月20日: Phase 4（DRY適用・Zustand永続化共通化）の実行計画を `docs/01_project/activeContext/refactoring_phase34_gap_plan.md` に第13章として追加し、対象領域・WBS・検証方針を明文化。
- 2025年10月20日: Phase 4 の初期実装として `application/shared` モジュールを追加し、Sqliteマッパーと Nostr ファクトリを共通化。Zustand ストアの永続化設定を `persistHelpers` 経由に統一し、`cargo clippy -- -D warnings` / Docker 経由の `cargo test` / `pnpm lint` / `pnpm test` を完走。
- 2025年10月20日: EventPublisher を `application/shared/nostr` へ移設し、EventManager/Service 双方で共通利用。DefaultTopicsRegistry を shared ユーティリティ化して DI を簡素化し、`cargo fmt` 実行後に Windows ローカル `cargo test` が linker 欠如で失敗したため Docker 経由 (`./scripts/test-docker.ps1 rust`) で Rust テストを完走。
- 2025年10月20日: `modules/event` / `modules/p2p` テスト支援コードを `application/shared/tests` へ集約し、`support` モジュールから再エクスポートして Rust 側の DRY 化を完了。Zustand ストアは `withPersist` + `config/persist.ts` に刷新し、Map を扱うストアで `createMapAwareStorage` を適用。`pnpm test --run src/stores` と `./scripts/test-docker.ps1 rust` を実行し、後方互換を検証。

