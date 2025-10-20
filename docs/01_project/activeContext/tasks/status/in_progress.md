[title] 作業中タスク（in_progress）

最終更新日: 2025年10月20日

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

## エラーハンドリング統一タスク

- [x] フロントエンドの主要フローを `errorHandler` ベースに移行し、`console.error` を廃止。
- [x] Rust/Tauri 側のドメインエラーを `thiserror` でラップし、コマンド境界の共通レスポンスを整理。
- [x] `docs/03_implementation/error_handling_guidelines.md` を更新し、統一フローと実装例を追記。

## 運用/品質・観測（着手）

- [ ] `tasks/metrics/{build_status,code_quality,test_results}.md` の更新フローを確立し、定期レビュー体制を文書化。
  - 2025年10月20日 着手: 既存メトリクスファイルの内容と更新履歴を調査し、レビュー体制案のドラフトを作成中。
  - 2025年10月20日: `docs/01_project/activeContext/tasks/metrics/update_flow.md` に現状サマリー・課題・更新フロー案（取得タイミング、実行コマンド、レビュー手順）を整理。
  - 2025年10月20日: `update_flow.md` に初回収集トライアル結果を追記し、`log_2025-10.md` と `artefacts/metrics/2025-10-20-vitest-results.json` を作成。`pnpm lint` 失敗・`cargo test` Windows エラーなどのギャップを抽出。
- [ ] Windows での `./scripts/test-docker.ps1` 実行を基本ラインとする運用ガイドを策定し、CI とローカルの手順差異を吸収。
  - 2025年10月20日: `docs/03_implementation/windows_test_docker_runbook.md` を作成し、PowerShell 運用手順と GitHub Actions との主な差分を記録。
  - 2025年10月20日: `windows_test_docker_runbook.md` に Linux/macOS ガイドとの共通化ポイントを整理し、`docker_test_environment.md` との統合方針を検討。
  - 2025年10月20日: `scripts/run-rust-tests.ps1` を追加し、Windows から Docker 経由で Rust テストを呼び出す自動化フローを整備。`docker_test_environment.md` / `windows_test_docker_runbook.md` に運用例を追記。
- [ ] ドキュメントの日付表記を `YYYY年MM月DD日` に統一するルールを整理し、主要ドキュメントの棚卸しを行う。
- [ ] Phase 5 CI/ローカルスクリプトのテストモジュール移行対応
  - [ ] `scripts/docker/run-smoke-tests.sh` を `tests/` 配下の `p2p_mainline_smoke` 等へ切り替え、P2P ランブックを最新手順に更新。
  - [ ] `scripts/test-docker.sh` の `TESTS` 既定値と `cargo --lib` 呼び出しを新しい `tests::integration::p2p::*` 構成へ移行し、ヘルプ出力と `docs/03_implementation/docker_test_environment.md` を修正。
  - [ ] `scripts/test-docker.ps1` の `cargo test` 呼び出しを `--test` ベースに更新し、ログ文言と PowerShell オプション説明を調整。
  - [ ] `docker-compose.test.yml` へ `./kukuri-tauri/src-tauri/tests` マウントを追加し、Rust テスト編集を即時反映できるようにする。
  - [ ] `docs/03_implementation/p2p_mainline_runbook.md` 等 Phase 5 連動ドキュメントから旧 `modules::p2p::tests::*` 参照を除去し、新構成のコマンド例で統一。

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
