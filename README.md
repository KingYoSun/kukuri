# kukuri

Nostrプロトコルベースの分散型トピック中心ソーシャルアプリケーション

## 概要

kukuriは、Nostrプロトコルを基盤とした分散型ソーシャルアプリケーションです。トピックベースのタイムラインでユーザーが情報を共有・発見できる、検閲耐性を持つP2Pソーシャルプラットフォームを提供します。

### 主な特徴

- 🌐 **完全分散型**: 中央サーバーに依存しないP2P通信
- 🔐 **暗号化通信**: エンドツーエンドの暗号化によるプライバシー保護
- 📝 **トピックベース**: 興味のあるトピックに基づいた情報共有
- ⚡ **高速同期**: iroh-gossipによる効率的なイベント配信
- 🖥️ **デスクトップアプリ**: Tauri v2による軽量で高速なネイティブアプリ

## 技術スタック

- **フロントエンド**: React 19 + TypeScript + Vite
- **デスクトップフレームワーク**: Tauri v2
- **UI**: shadcn/ui (Radix UI + Tailwind CSS)
- **状態管理**: Zustand
- **P2P通信**: iroh + iroh-gossip
- **プロトコル**: Nostr (NIP準拠)
- **データベース**: SQLite

## クイックスタート

### 前提条件

- Node.js v20以上
- pnpm
- Rust 1.70以上
- Git

### インストール

```bash
# リポジトリのクローン
git clone https://github.com/yourusername/kukuri.git
cd kukuri

# 開発ツールのインストール（初回のみ）
./scripts/install-dev-tools.sh

# 依存関係のインストール
pnpm install

# 開発サーバーの起動
pnpm tauri dev
```

## 開発

### 利用可能なコマンド

```bash
# 開発サーバー起動
pnpm tauri dev

# ビルド
pnpm tauri build

# リント
pnpm lint

# フォーマット
pnpm format

# テスト実行
pnpm test
cargo test
```

### プロジェクト構造

```
kukuri/
├── src/                    # Reactフロントエンド
│   ├── components/         # UIコンポーネント
│   ├── hooks/              # カスタムフック
│   ├── stores/             # Zustandストア
│   └── pages/              # ページコンポーネント
├── src-tauri/              # Rustバックエンド
│   ├── src/
│   │   ├── commands/       # Tauriコマンド
│   │   ├── nostr/          # Nostr実装
│   │   ├── p2p/            # P2P通信
│   │   └── db/             # データベース
│   └── Cargo.toml
├── docs/                   # ドキュメント
├── scripts/                # ユーティリティスクリプト
└── workers/                # Cloudflare Workers
```

## ドキュメント

詳細なドキュメントは`docs/`ディレクトリを参照してください：

- [プロジェクト設計書](docs/01_project/design_doc.md)
- [システム設計書](docs/02_architecture/system_design.md)
- [実装計画](docs/05_implementation/implementation_plan.md)
- [開発環境セットアップガイド](docs/01_project/setup_guide.md)

## コントリビューション

プルリクエストを歓迎します！コントリビューションの際は以下をご確認ください：

1. Issueを作成して変更内容を説明
2. フィーチャーブランチを作成（`git checkout -b feature/amazing-feature`）
3. 変更をコミット（`git commit -m 'Add amazing feature'`）
4. ブランチをプッシュ（`git push origin feature/amazing-feature`）
5. プルリクエストを作成

## ライセンス

このプロジェクトはMITライセンスの下で公開されています。詳細は[LICENSE](LICENSE)ファイルを参照してください。

## お問い合わせ

- Issue: [GitHub Issues](https://github.com/yourusername/kukuri/issues)
- Discussion: [GitHub Discussions](https://github.com/yourusername/kukuri/discussions)