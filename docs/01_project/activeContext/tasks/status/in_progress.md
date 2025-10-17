[title] 作業中タスク（in_progress）

最終更新日: 2025年10月17日

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

- [ ] DHT購読状態を永続化するステートマシンを設計し、`EventService` に実装。
- [ ] 再接続時の購読復元シーケンスをテストで検証（離脱→再接続→履歴同期）。
- [ ] UI 側で購読状態と同期状況を可視化するフックを追加（P2PDebugPanel 連携）。

## エラーハンドリング統一タスク

- [ ] フロントエンドの主要フローを `errorHandler` ベースに移行し、`console.error` を廃止。
- [ ] Rust/Tauri 側のドメインエラーを `thiserror` でラップし、コマンド境界の共通レスポンスを整理。
- [ ] `docs/03_implementation/error_handling_guidelines.md` を更新し、統一フローと実装例を追記。

## 運用/品質・観測

- [ ] `tasks/metrics/{build_status,code_quality,test_results}.md` の更新フローを確立し、定期レビュー体制を文書化。
- [ ] Windows での `./scripts/test-docker.ps1` 実行を基本ラインとする運用ガイドを策定し、CI とローカルの手順差異を吸収。
- [ ] ドキュメントの日付表記を `YYYY年MM月DD日` に統一するルールを整理し、主要ドキュメントの棚卸しを行う。

## リファクタリング計画フォローアップ

- [ ] Phase 2 TODO 解消: `event_service` の未実装処理（Post変換・メタデータ更新・Reaction/Repost処理）を完了し、`EventManager` との連携を整備する（`kukuri-tauri/src-tauri/src/application/services/event_service.rs:122`）。
- [ ] Phase 2 TODO 解消: `offline_service` の Repository 統合タスクを実装し、同期/キャッシュ関連の TODO を解消する（`kukuri-tauri/src-tauri/src/application/services/offline_service.rs:134`）。
- [ ] Phase 2 TODO 解消: トピック更新・削除コマンドの未実装部分を実装し、フロントからの操作を完了させる（`kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs:99`）。
- [ ] Phase 3/4 ギャップ対応: 700行超のファイル（`kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs:1003`, `kukuri-tauri/src-tauri/src/application/services/event_service.rs:341`, `kukuri-tauri/src-tauri/src/modules/event/manager.rs:240`）の分割計画を策定し、リファクタリングタスクへ落とし込む。
- [ ] Phase 4 DRY 適用: Zustand ストアで `persistHelpers.ts`（`kukuri-tauri/src/stores/utils/persistHelpers.ts:6`）を採用し、永続化設定の重複を解消する。
- [ ] Phase 5 成果測定: dead code 数やテストカバレッジといった指標を `tasks/metrics/` 配下で定期記録する運用を整備する。

関連: `docs/01_project/activeContext/iroh-native-dht-plan.md`

メモ/進捗ログ:
- 2025年10月18日: GitHub Actions の Format Check 失敗を確認し、`src/components/P2PDebugPanel.tsx` と `src/stores/offlineStore.ts` を Prettier で整形。`pnpm format:check` が成功することをローカルで確認。
- 2025年10月18日: P2P接続イベントから再索引ジョブをトリガーするウォッチャーと、`offline://reindex_*` イベントに応答してUIストアを更新する処理を実装。
- 2025年10月18日: `IrohNetworkService` の接続イベントを用いた再索引結合テストを追加し、再接続時に同期キューへ再投入されることを検証。
- 2025年10月18日: OfflineService の再索引ジョブ整備タスクに着手。現状の Repository キャッシュ構造と再接続時の課題を洗い出すための調査を開始。
- 2025年10月17日: Iroh DHT/Discovery 残タスクを完了し、Mainline DHT 統合フェーズへ移行。Phase 7 の残項目（Mainline DHT/OfflineService/EventService/エラーハンドリング）を次スプリントの主テーマに設定。
- 2025年10月17日: 運用・品質セクションの TODO を見直し、メトリクス更新フローと Windows テスト運用の標準化タスクを切り出した。
- 2025年10月17日: `DiscoveryOptions` と `P2PService::builder` を導入し、Mainline DHT 切替対応のためのP2Pスタック組み立てを再構成。
- 2025年10月17日: `ApplicationContainer` を導入し、Base64 永続化した iroh シークレットキーからノード ID を再利用する初期化と、`NetworkConfig.bootstrap_peers` を `IrohNetworkService` 初期化時に適用する仕組みを整備。Docker 経由の `cargo test` と `kukuri-cli` のテストまで確認済み。
- 2025年10月17日: Mainline DHT ハンドシェイク/ルーティング統合テストを `mainline_dht_tests.rs` に追加し、Docker スモークテストで DHT/Gossip と並行実行するよう `run-smoke-tests.sh` を更新。
- 2025年10月17日: Mainline DHT の接続・ルーティング・再接続メトリクスを Rust 側で集計し、`get_p2p_metrics`／P2PDebugPanel に反映。Docker 経由で Rust テストと `pnpm test` を通過。
