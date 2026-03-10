# Refactoring Plan Phase 1-5 実装状況レビュー（2025年10月17日）

## サマリー
- Phase 1 の主要アクション（`manager_old.rs` の削除と dead code 削減）は完了済みで、現在の `#[allow(dead_code)]` は 10 ファイル・20 箇所に圧縮されています（例: `kukuri-tauri/src-tauri/src/modules/offline/mod.rs:1`, `kukuri-tauri/src-tauri/src/modules/crypto/encryption.rs:9`）。
- Phase 2 で着手された同期周りの TODO はフロント側で大幅に実装が進んだ一方、サーバー側では `event_service` や `offline_service` などに未処理の TODO が残存しています（例: `kukuri-tauri/src-tauri/src/application/services/event_service.rs:122`, `kukuri-tauri/src-tauri/src/application/services/offline_service.rs:134`）。
- Phase 3 で掲げた「700行超のファイル抑制」は未達で、`sqlite_repository.rs`（1003行）や `event_service.rs`（732行）など新たに長大化したファイルが発生しています（`kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs:1003`, `kukuri-tauri/src-tauri/src/application/services/event_service.rs:341`）。
- Phase 4 のエラーハンドリング統一は errorHandler ベースで広範に適用されており、`console.error` 直接呼び出しは解消されていますが、Zustand ストアの Persist 設定共通化ヘルパーはまだ各ストアで未利用です（`kukuri-tauri/src/stores/utils/persistHelpers.ts:6`, `kukuri-tauri/src/stores/offlineStore.ts:46`）。
- Phase 5 のドメイン分割とテスト階層整理は概ね完了しており、`domain/`, `application/`, `infrastructure/`, `presentation/` の4層構成と `tests/{unit,integration,common}` の配置が実現しています（`kukuri-tauri/src-tauri/src/domain/entities/event.rs:1`, `kukuri-tauri/src-tauri/tests/common/mod.rs:1`, `kukuri-tauri/src-tauri/tests/integration/mod.rs:1`）。

## Phase 1: Dead Code削除
- 完了: `manager_old.rs` が削除され、モジュール入口は `OfflineManager` のみを再エクスポートする構成になりました（`kukuri-tauri/src-tauri/src/modules/offline/mod.rs:1`）。
- 完了: `#[allow(dead_code)]` は 10 ファイルに減少し、大半がインフラ層のテスト補助的な残置のみです（例: `kukuri-tauri/src-tauri/src/modules/crypto/encryption.rs:9`, `kukuri-tauri/src-tauri/src/modules/event/manager.rs:46`, `kukuri-tauri/src-tauri/src/modules/p2p/topic_mesh.rs:11`）。
- 残課題: TopicMesh など一部で `#[allow(dead_code)]` と TODO が併存しており、今後の mainline DHT 実装に合わせた整理が必要です（`kukuri-tauri/src-tauri/src/modules/p2p/topic_mesh.rs:11`）。

## Phase 2: TODO実装
- 進捗: フロント側の同期フローは `useSyncManager` と `SyncEngine` が差分同期・競合解決まで実装済みで、Plan の高優先度 TODO を大きく前進させています（`kukuri-tauri/src/hooks/useSyncManager.ts:18`, `kukuri-tauri/src/hooks/useSyncManager.ts:44`, `kukuri-tauri/src/lib/sync/syncEngine.ts:40`）。
- 未完: Rust 側の高優先度 TODO は引き続き残っており、`event_service` のイベント反映や `offline_service` の永続化処理、`topic_commands` の更新/削除などは未実装です（`kukuri-tauri/src-tauri/src/application/services/event_service.rs:122`, `kukuri-tauri/src-tauri/src/application/services/event_service.rs:341`, `kukuri-tauri/src-tauri/src/application/services/offline_service.rs:134`, `kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs:99`）。
- 残課題: TypeScript 側でも未読カウントが TODO のままであり（`kukuri-tauri/src/components/layout/Sidebar.tsx:47`）、Phase 2 で掲げた UI 側の改善が一部継続タスクになっています。

## Phase 3: ファイル分割
- 未完: DDD 化の進行と同時にサービス層が肥大化し、`sqlite_repository.rs`（1003行）や `event_service.rs`（732行）、`modules/event/manager.rs`（751行）など 700 行超のファイルが複数存在しています（`kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs:1003`, `kukuri-tauri/src-tauri/src/application/services/event_service.rs:341`, `kukuri-tauri/src-tauri/src/modules/event/manager.rs:240`）。
- 対応策の検討: Phase 3 で予定していた分割ポリシーが未適用のため、責務ごとの分離やテスト専用モジュール分離のガイドライン策定が必要です。

## Phase 4: DRY原則適用
- 完了: エラーハンドリングは `errorHandler` に集約され、同期・投稿・認証など広範なフローで同一 API を利用しています（`kukuri-tauri/src/lib/errorHandler.ts:32`, `kukuri-tauri/src/hooks/useSyncManager.ts:100`, `kukuri-tauri/src/lib/sync/syncEngine.ts:76`）。`console.error` の直接利用はコメントを除き排除されています。
- 部分完了: Zustand の persist 設定共通化ヘルパー `persistHelpers.ts` は実装済みですが（`kukuri-tauri/src/stores/utils/persistHelpers.ts:6`）、各ストアでは依然として `persist(...)` を個別定義しており未統合です（`kukuri-tauri/src/stores/offlineStore.ts:46`）。
- 次の一手: 共通ヘルパーの適用範囲拡大と、既存ストアの重複定義除去を Phase 4 継続タスクとして扱う必要があります。

## Phase 5: アーキテクチャ改善
- 完了: `src-tauri/src` 直下は `domain/`, `application/`, `infrastructure/`, `presentation/`, `shared/` のレイヤ構成に移行済みで、ドメインエンティティやアプリケーションサービスが分離されています（`kukuri-tauri/src-tauri/src/domain/entities/event.rs:1`）。
- 完了: テストは `tests/common` にモック・フィクスチャを集約し、`unit`, `integration` ディレクトリで階層分けされています（`kukuri-tauri/src-tauri/tests/common/mod.rs:1`, `kukuri-tauri/src-tauri/tests/common/mocks/mod.rs:1`, `kukuri-tauri/src-tauri/tests/integration/mod.rs:1`）。
- 残課題: 大型ファイルの分割やリポジトリ実装の肥大化（Phase 3 由来）により、Phase 5 完了条件のうち「保守性向上」の定量評価（例: コード重複率低減）が未検証です。後続フェーズでメトリクス収集とドキュメント更新が必要です。

## 今後の提案
1. Phase 2 残タスクとして、`event_service` と `offline_service` の TODO 群を優先的に実装し、Topic 更新系コマンドを完結させる。
2. Phase 3/4 のギャップを埋めるため、700 行超のファイル分割計画と Persist ヘルパー適用ガイドラインを策定し、タスク化する。
3. Phase 5 の成果を定量評価できるよう、テストカバレッジや dead code 許容数の指標を定期計測し、`tasks/metrics/` 配下に反映する。
