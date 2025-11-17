# 進捗レポート一覧

このディレクトリには、kukuriプロジェクトの開発進捗レポートを格納しています。

## Nightly Runbook

- [Nightly テストID / Artefact マッピング](./nightly.index.md) - Nightly ジョブと `*-logs` / `*-reports` artefact の対応表
- [Nightly `trending-feed` Runbook](./nightly.trending-feed.md) - `/trending` `/following` Docker シナリオのトリアージ手順

## レポート一覧

### 2025年07月28日
- [Phase 1 認証フローテスト実装](./2025-07-28_phase1_auth_tests.md) - Tauriアプリケーション Phase 1の包括的テスト作成（37テスト）
- [Tauriアプリケーション Phase 1 実装](./2025-07-28_tauri_app_phase1_implementation.md) - 認証フロー（ウェルカム、ログイン、プロフィール設定）実装
- [Tauriアプリケーション体験設計](./2025-07-28_tauri_app_experience_design.md) - ユーザー体験改善の設計と実装計画
- [実装優先順位の決定](./2025-07-28_implementation_priority_decision.md) - Tauriアプリ優先、手動P2P接続機能追加
- [ドキュメント全体更新](./2025-07-28_documentation_update.md) - プロジェクトドキュメントの整理と更新

### 2025年07月27日
- [フロントエンドテスト非同期初期化問題の修正](./2025-07-27_frontend_test_async_fix.md) - Zustand非同期初期化、Radix UIテスト問題の解決
- [バックエンドのテスト・型・リント修正](./2025-07-27_backend_test_lint_fix.md) - 未使用コード対応、P2P統合テスト修正
- [P2Pトピック管理テスト](./2025-07-27_p2p_topic_management_tests.md) - 包括的なテスト実装（19件追加）
- [Nostr統合 Day 6 実装](./2025-07-27_nostr_integration_day6.md) - イベント変換機能実装
- [Nostr統合 Day 8 実装](./2025-07-27_nostr_integration_day8.md) - ハイブリッド配信機能実装
- [ドキュメント更新](./2025-07-27_documentation_update.md) - プロジェクト全体のドキュメント整備

### 2025年07月26日
- [開発環境準備](./2025-07-26_開発環境準備.md) - 開発ツール自動インストールスクリプト、プロジェクト設定ファイル作成
- [UIコンポーネント基盤実装](./2025-07-26_UIコンポーネント基盤実装.md) - shadcn/ui導入、基本レイアウト実装、テスト環境構築
- [状態管理とルーティング基盤実装](./2025-07-26_state_management_implementation.md) - Zustand、Tanstack Router/Query設定
- [Rust基盤実装](./2025-07-26_rust_foundation_implementation.md) - 認証、暗号化、データベースモジュール実装
- [テスト・リント・型チェックエラー解消](./2025-07-26_test_lint_fix.md) - zustand v5対応、全エラー修正
- [Tauriコマンド実装](./2025-07-26_tauri_commands_implementation.md) - フロントエンド・バックエンド統合
- [Nostr SDK統合](./2025-07-26_nostr_sdk_integration.md) - Nostrイベント処理基盤実装
- [Nostrリレー接続とイベント送受信](./2025-07-26_nostr_relay_connection.md) - リレー接続、ヘルスチェック、テストパネル実装
- [包括的なテスト実装](./2025-07-26_comprehensive_test_implementation.md) - Rust/TypeScript全コンポーネントのテスト作成（158件）

## レポート作成ガイドライン

### ファイル名規則
```
YYYY-MM-DD_作業内容.md
```

### レポート構成
1. **概要** - 作業の要約
2. **実施内容** - 詳細な作業内容
3. **技術的詳細** - 実装の技術的な詳細
4. **次のステップ** - 今後の作業予定
5. **課題・懸念事項** - 発見した問題や懸念点
6. **まとめ** - 作業の総括

### 作成タイミング
- 大きな機能の実装完了時
- マイルストーンの達成時
- 重要な技術的決定を行った時
- 週次・月次の定期レポート（必要に応じて）
