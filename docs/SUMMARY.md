# kukuri ドキュメント概要

## プロジェクト概要
kukuriは、Nostrプロトコルをベースとした分散型トピック中心ソーシャルアプリケーションです。検閲耐性を持つP2Pネットワークを通じて、ユーザーがトピックベースで情報を共有・発見できるプラットフォームを提供します。

## ドキュメント構成

### 01_project/ - プロジェクト管理
- **design_doc.md**: プロジェクトの全体設計書
- **requirements.md**: 機能要件・非機能要件定義
- **activeContext/**: 現在の作業状況
  - current_tasks.md: 進行中のタスク
  - current_environment.md: 開発環境情報
  - issuesAndNotes.md: 既知の問題と注意事項
- **progressReports/**: 進捗レポート

### 02_architecture/ - アーキテクチャ設計
- **system_design.md**: システム設計詳細
  - レイヤー構成
  - データモデル
  - API設計
  - セキュリティ設計
- **project_structure.md**: プロジェクト構造
  - ディレクトリ構成
  - ファイル構造詳細
  - 開発ワークフロー

### 03_features/ - 機能仕様
- 各機能の詳細仕様書（作成予定）

### 04_data/ - データ設計
- データベーススキーマ
- Nostrイベント仕様（作成予定）

### 05_implementation/ - 実装ガイド
- **implementation_plan.md**: 段階的実装計画
  - Phase 1: MVP (3ヶ月)
  - Phase 2: ベータ版 (3ヶ月)
  - Phase 3: 正式リリース
- **storage_implementation_guide.md**: ストレージ実装ガイド
- **testing_guide.md**: テスト戦略と実装ガイド

## 技術スタック

### フロントエンド
- React 18 + TypeScript
- Vite (ビルドツール)
- shadcn/ui (UIコンポーネント)
- Zustand (状態管理)
- Tanstack Query/Router

### バックエンド
- Tauri v2 (デスクトップフレームワーク)
- Rust (コアロジック)
- iroh (P2P通信基盤)
- iroh-gossip (トピックベースイベント配信)
- nostr-sdk (Nostrプロトコル)
- SQLite (ローカルDB)

### インフラ
- Cloudflare Workers / Docker (ピア発見)
- 分散マーケットプレイス (検索・推薦)

## 開発フェーズ

1. **Phase 1 (MVP)**: 基本機能実装
   - ユーザー管理
   - トピック作成・参加
   - 基本的なP2P通信

2. **Phase 2 (ベータ)**: 高度な機能
   - 画像・動画対応
   - 検索・サジェスト機能
   - パフォーマンス最適化

3. **Phase 3**: エンタープライズ機能
   - トークンエコノミー
   - 高度なプライバシー機能
   - プラグインシステム

## 主要な設計決定

- **Nostr互換性**: 既存のNostrエコシステムとの相互運用性を維持
- **ハイブリッドP2P**: 純粋なP2Pの課題を回避しつつ分散性を確保
- **プライバシーファースト**: エンドツーエンド暗号化とローカル処理を優先
- **開発者フレンドリー**: OSSファーストでコミュニティ駆動の開発

## 更新履歴
- 2025年7月26日: Prettier導入と開発環境改善
- 2025年7月26日: インテグレーションテスト実装、テストガイド追加
- 2025年7月26日: iroh-gossip採用に伴う技術スタック更新
- 2025年7月25日: 初版作成