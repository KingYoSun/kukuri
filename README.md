# kukuri

Nostrプロトコルベースの完全分散型トピック中心ソーシャルアプリケーション

## 概要

kukuriは、Nostrプロトコルを基盤とした完全分散型ソーシャルアプリケーションです。BitTorrent Mainline DHTを活用した分散型ピア発見により、中央サーバー依存を完全に排除し、真の検閲耐性とユーザー主権を実現します。

### 主な特徴

- 🌐 **完全分散型**: BitTorrent DHTによるサーバーレスピア発見
- 🔐 **暗号化通信**: エンドツーエンドの暗号化によるプライバシー保護
- 📝 **トピックベース**: 興味のあるトピックに基づいた情報共有
- ⚡ **高速同期**: iroh-gossipによる効率的なイベント配信
- 🔍 **DHT基盤**: irohビルトインDHTによる自動ブートストラップ
- 🖥️ **デスクトップアプリ**: Tauri v2による軽量で高速なネイティブアプリ
- 👥 **複数アカウント管理**: セキュアストレージによる安全なアカウント切り替え
- 🔑 **自動ログイン**: プラットフォーム固有のキーチェーンによる安全な認証

## 技術スタック

- **フロントエンド**: React 18 + TypeScript + Vite
- **デスクトップフレームワーク**: Tauri v2
- **UI**: shadcn/ui (Radix UI + Tailwind CSS)
- **状態管理**: Zustand
- **P2P通信**: iroh + iroh-gossip
- **ピア発見**: irohビルトインDHT (BitTorrent Mainline DHT)
- **プロトコル**: Nostr (NIP準拠)
- **データベース**: SQLite
- **セキュアストレージ**: keyring (Keychain/Credential Manager/Secret Service)

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

# ビルド(Windows)
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc

# ビルド(Android)
pnpm tauri android build

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
├── workers/                # Cloudflare Workers
└── pkarr/                  # Pkarr submodule (DHTリレーサーバー)
```

## Pkarrリレーサーバー (ローカル開発用)

kukuriは、irohのビルトインDHTディスカバリー機能を通じてPkarrリレーサーバーと連携します。ローカル開発環境では、Docker Composeを使用してPkarrリレーサーバーを起動できます。

### Pkarrリレーサーバーの起動

```bash
# 初回のみ: submoduleの初期化
git submodule update --init --recursive

# Pkarrリレーサーバーの起動
docker-compose up -d

# ログの確認
docker-compose logs -f pkarr

# サーバーの停止
docker-compose down
```

### 設定

Pkarrリレーサーバーの設定は`config.toml`で管理されています：

- **HTTPポート**: 8080 (API)
- **Mainline DHTポート**: 6881 (DHT通信)
- **キャッシュ**: `.pkarr_cache/`ディレクトリに保存

### 動作確認

```bash
# Pkarrリレーサーバーのヘルスチェック
curl http://localhost:8080/health

# ステータス確認
curl http://localhost:8080/stats
```

## ドキュメント

詳細なドキュメントは`docs/`ディレクトリを参照してください：

- [プロジェクト設計書](docs/01_project/design_doc.md)
- [システム設計書](docs/02_architecture/system_design.md)
- [実装計画](docs/03_implementation/implementation_plan.md)
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

## 外部リソース

### 技術ドキュメント
- [iroh公式ドキュメント](https://docs.rs/iroh/latest/iroh/)
- [iroh-gossip公式ドキュメント](https://docs.rs/iroh-gossip/latest/iroh_gossip/)
- [Nostr NIPs](https://github.com/nostr-protocol/nips)
- [Tauri公式ドキュメント](https://tauri.app/)

## お問い合わせ

- Issue: [GitHub Issues](https://github.com/yourusername/kukuri/issues)
- Discussion: [GitHub Discussions](https://github.com/yourusername/kukuri/discussions)