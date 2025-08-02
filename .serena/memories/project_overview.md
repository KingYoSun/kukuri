# kukuri プロジェクト概要

## 基本情報
**プロジェクト名**: kukuri  
**説明**: Nostrプロトコルベースの分散型トピック中心ソーシャルアプリケーション  
**ライセンス**: MIT

## 主な特徴
- 🌐 完全分散型: 中央サーバーに依存しないP2P通信
- 🔐 暗号化通信: エンドツーエンドの暗号化によるプライバシー保護
- 📝 トピックベース: 興味のあるトピックに基づいた情報共有
- ⚡ 高速同期: iroh-gossipによる効率的なイベント配信
- 🖥️ デスクトップアプリ: Tauri v2による軽量で高速なネイティブアプリ
- 👥 複数アカウント管理: セキュアストレージによる安全なアカウント切り替え
- 🔑 自動ログイン: プラットフォーム固有のキーチェーンによる安全な認証

## 技術スタック

### フロントエンド
- **Framework**: React 18 + TypeScript + Vite
- **UI Components**: shadcn/ui (Radix UI + Tailwind CSS)
- **State Management**: Zustand
- **Data Fetching**: Tanstack Query
- **Routing**: Tanstack Router
- **Markdown Editor**: @uiw/react-md-editor

### バックエンド
- **Desktop Framework**: Tauri v2 (Rust)
- **P2P Network**: iroh (QUIC-based) + iroh-gossip
- **Protocol**: Nostr (nostr-sdk)
- **Database**: SQLite (sqlx)
- **Cryptography**: secp256k1, AES-256-GCM
- **Secure Storage**: keyring (Windows-native features enabled)

## アーキテクチャ

### レイヤー構成
1. **Client Layer**: Tauri App (UI + Business Logic)
2. **Discovery Layer**: ピア発見サービス (Workers/Container)
3. **P2P Network**: irohによる直接通信
4. **Marketplace**: 専門機能ノード (検索/推薦)

### P2P実装状況
- ✅ ゴシッププロトコル (iroh-gossip v0.90.0)
- ✅ Nostrイベント同期
- ✅ メッセージ署名検証
- ✅ 重複排除 (LRUキャッシュ)
- ✅ NAT traversal
- ✅ UI統合 (状態表示、トピックメッシュ可視化)

## 現在の実装状況
- Phase 1 (認証フロー): ✅ 完了
- Phase 2 (データ連携): ✅ 完了
- Phase 3.1 (トピック参加・離脱): ✅ 完了
- Phase 3.2 (新規投稿機能拡張): ✅ 完了
- Phase 3.3 (その他のリアクション): 進行中
- Phase 4 (オフラインファースト): 計画済み