# 実装ドキュメント概要

**最終更新**: 2026年02月02日

このディレクトリには、Kukuriプロジェクトの実装に関する詳細なドキュメントを集約しています。
最新の実装状況は `docs/01_project/activeContext/tasks/` と進捗レポートを参照してください。

## ドキュメント一覧

### 実装計画・方針
- [implementation_plan.md](./implementation_plan.md) - 段階的な実装計画
- [error_handling_guidelines.md](./error_handling_guidelines.md) - エラーハンドリング指針
- [send_sync_trait_bounds_implementation.md](./send_sync_trait_bounds_implementation.md) - Rust制約整理

### P2P / iroh / Gossip
- [dht_integration_guide.md](./dht_integration_guide.md) - DHT統合ガイド
- [p2p_mainline_runbook.md](./p2p_mainline_runbook.md) - Mainline DHT運用手順
- [p2p_event_routing_design.md](./p2p_event_routing_design.md) - イベントルーティング設計
- [p2p_dht_test_strategy.md](./p2p_dht_test_strategy.md) - DHTテスト戦略
- [iroh_gossip_integration_design.md](./iroh_gossip_integration_design.md) - iroh-gossip統合設計
- [iroh_gossip_implementation_plan.md](./iroh_gossip_implementation_plan.md) - 実装計画
- [iroh_gossip_implementation_status.md](./iroh_gossip_implementation_status.md) - 実装状況の整理
- [iroh_gossip_api_v090.md](./iroh_gossip_api_v090.md) / [iroh_v090_specification.md](./iroh_v090_specification.md) - 旧API仕様（参照用）

### Nostr
- [nostr_event_validation.md](./nostr_event_validation.md) - Nostrイベント検証
- [nostr_reactions_implementation.md](./nostr_reactions_implementation.md) - リアクション実装

### ストレージ / SQLx
- [storage_implementation_guide.md](./storage_implementation_guide.md) - ストレージ実装
- [sqlx_best_practices.md](./sqlx_best_practices.md) - SQLx運用ベストプラクティス
- [sqlx_offline_mode_guide.md](./sqlx_offline_mode_guide.md) - SQLxオフライン準備

### テスト / QA
- [testing_guide.md](./testing_guide.md) - テスト戦略
- [docker_test_environment.md](./docker_test_environment.md) - Dockerテスト環境
- [e2e_test_setup.md](./e2e_test_setup.md) - E2Eテスト環境セットアップ
- [e2e_test_implementation_plan.md](./e2e_test_implementation_plan.md) - E2E導入計画
- [e2e_test_stabilization.md](./e2e_test_stabilization.md) - E2E安定化メモ
- [windows_test_docker_runbook.md](./windows_test_docker_runbook.md) - Windows環境のテスト運用
- [zustand_testing_best_practices.md](./zustand_testing_best_practices.md) - Zustandテスト

### 追加ガイド
- [trending_metrics_job.md](./trending_metrics_job.md) - トレンドメトリクスジョブ

## 関連ドキュメント
- [プロジェクト構造](../02_architecture/project_structure.md)
- [システム設計](../02_architecture/system_design.md)
- [activeContext 概要](../01_project/activeContext/summary.md)
- [進行中タスク](../01_project/activeContext/tasks/status/in_progress.md)
- [進捗レポート](../01_project/progressReports/)
