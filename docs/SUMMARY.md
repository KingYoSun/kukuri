# kukuri ドキュメント概要

補足: 最新の作業状況・意思決定は `docs/01_project/activeContext/summary.md` と
`docs/01_project/activeContext/tasks/` を参照してください（最終確認: 2026年02月02日）。

**最終更新**: 2026年02月02日

## プロジェクト概要
kukuriは、Nostrプロトコルをベースとした完全分散型トピック中心ソーシャルアプリケーションです。
BitTorrent Mainline DHT を基盤とした分散型ピア発見により、中央サーバー依存を排除し、
検閲耐性とユーザー主権を実現することを目指します。

## ドキュメント構成

### 01_project/ - プロジェクト管理
- **design_doc.md**: プロジェクトの全体設計書
- **requirements.md**: 機能要件・非機能要件定義
- **roadmap.md**: ロードマップ
- **setup_guide.md** / **windows_setup_guide.md**: 開発環境セットアップ
- **activeContext/**: 現在の作業状況・決定事項・タスク管理
  - `tasks/priority/critical.md`: 最重要タスク
  - `tasks/status/in_progress.md`: 進行中タスク
  - `tasks/completed/YYYY-MM-DD.md`: 完了タスクの記録
- **progressReports/**: 進捗レポート（重要な変更の記録）
- **deprecated/**: 役目を終えた計画・設計の保管庫（参照のみ）

### 02_architecture/ - アーキテクチャ設計
- **system_design.md**: システム設計詳細
- **dht_discovery_architecture.md**: DHT基盤Discovery Layerアーキテクチャ
- **project_structure.md**: プロジェクト構造
- **iroh_gossip_review.md** / **storage_comparison_report.md**: 調査・比較資料

### 03_implementation/ - 実装ガイド
- **summary.md**: 実装ドキュメントの索引
- **dht_integration_guide.md** / **p2p_mainline_runbook.md**: P2P/DHT運用・実装
- **testing_guide.md** / **e2e_test_setup.md**: テストガイド
- **sqlx_* / storage_implementation_guide.md**: ストレージ関連
- **error_handling_guidelines.md**: エラーハンドリング指針

### kips/・nips/ - 仕様・提案
- **kips/**: Kukuri Improvement Proposals
- **nips/**: Nostr Improvement Proposals

### apis/ - ローカルAPI JSON
- `docs/apis/` に iroh 系 API JSON を配置

## 開発状況の参照先
最新の進行状況・完了事項は、以下のドキュメントを起点に確認してください。

- `docs/01_project/activeContext/summary.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/`
