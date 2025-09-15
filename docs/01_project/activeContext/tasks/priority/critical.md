# 最重要タスク（Critical）

最終更新日: 2025年09月15日

目的: 今後直近で着手すべき最重要タスクの一覧。着手時は本ファイルから `tasks/status/in_progress.md` へ移動して進捗を管理します。

移動済みメモ
- Iroh DHT/Discovery 残タスクは `tasks/status/in_progress.md` に移動（2025年09月15日）

方針更新（2025年09月15日）
- 当面は Nostr リレーとは接続しない。まず P2P（iroh + iroh-gossip + DHT）で完結した体験の実現を優先。
- kukuri 内部のイベントは全て NIPs 準拠（Nostr Event スキーマ準拠）。

## 2. v2 アプリ Phase 7（DHT統合の仕上げ）
- [ ] P2PService: Iroh Mainline DHT の統合（Builder/DI の明確化）
- [ ] OfflineService: Repository 再索引ジョブと整合性の担保（再起動/再接続時）
- [ ] EventService: DHT 経由イベント購読・再接続時の復元
- [ ] エラーハンドリング統一（フロントは `errorHandler`、Rust 側は `thiserror`）

## 3. 運用/品質・観測
- [ ] メトリクス運用: `tasks/metrics/{build_status,code_quality,test_results}.md` の更新フロー整備
- [ ] Windows 安定化: `./scripts/test-docker.ps1` を用いた Docker 経由のテスト実行を既定化
- [ ] ドキュメントの最終更新日表記の統一（`YYYY年MM月DD日`）

運用ルール（再掲）
- 新規着手: 本ファイルから対象を選び、`tasks/status/in_progress.md` へ移動
- 完了時: `tasks/completed/YYYY-MM-DD.md` に追記 → `in_progress.md` から削除 → 重要変更は `docs/01_project/progressReports/` にレポート作成

補足
- 既に完了済みの内容は本ファイルから除去済み（詳細は `tasks/completed/2025-09-15.md` を参照）。
