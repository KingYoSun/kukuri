# kukuri

Nostrプロトコルを基盤とした完全分散型トピック指向ソーシャルアプリケーションです。BitTorrent Mainline DHT を用いたピア発見と iroh-gossip による高速配信により、中央サーバーに依存しないエクスペリエンスを提供します。

## 概要
- 完全分散型: DHT によるサーバーレス構成で検閲耐性を確保
- 暗号化通信: エンドツーエンド暗号化でプライバシーを保護
- トピックベース: 興味のあるトピック単位で情報を共有
- 高速同期: iroh-gossip による効率的なイベント配信
- デスクトップアプリ: Tauri v2 による軽量・高速なネイティブ体験
- 複数アカウント: セキュアストレージで安全に切り替え

## 技術スタック
- フロントエンド: React 18 / TypeScript / Vite / shadcn/ui
- デスクトップ: Tauri v2 (Rust)
- 状態管理: Zustand
- P2P 通信: iroh + iroh-gossip
- ピア発見: iroh 内蔵 DHT (BitTorrent Mainline DHT)
- プロトコル: Nostr (NIP 準拠)
- データベース: SQLite

## クイックスタート
### 前提条件
- Node.js v20 以上
- pnpm
- Rust 1.70 以上
- Git

### インストール & 起動
```bash
# リポジトリのクローン
git clone https://github.com/yourusername/kukuri.git
cd kukuri

# 開発ツールのセットアップ（初回のみ）
./scripts/install-dev-tools.sh

# 依存関係のインストール
pnpm install

# 開発サーバーの起動
pnpm tauri dev
```

#### 手動検証時の P2P セットアップ
`pnpm tauri dev` で手動検証を行う場合は、アプリ起動前に以下を準備してください。

1. DHT ブートストラップノードを起動する  
   例: `docker compose -f docker-compose.test.yml up -d p2p-bootstrap` または `./scripts/test-docker.ps1 integration -NoBuild`。
2. 接続確認用の別ノードを少なくとも 1 つ用意する  
   例: `kukuri-cli bootstrap` / `kukuri-cli relay` を利用する、もしくは別の Tauri インスタンスを起動して送受信相手を用意する。

ブートストラップが存在しない、または単独ノードのみの状態では接続成立を確認できません。検証完了後は `docker compose -f docker-compose.test.yml down --remove-orphans` などでクリーンアップしてください。

## 開発
### 主なコマンド
```bash
# 開発サーバー
pnpm tauri dev

# ビルド
pnpm tauri build
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc  # Windows 向け
pnpm tauri android build                                              # Android 向け

# 品質チェック
pnpm lint
pnpm format

# テスト
pnpm test
cargo test
```

### プロジェクト構造
```
kukuri/
├── src/                    # React フロントエンド
│   ├── components/         # UI コンポーネント
│   ├── hooks/              # カスタムフック
│   ├── stores/             # Zustand ストア
│   └── pages/              # ページコンポーネント
├── src-tauri/              # Rust バックエンド
│   ├── src/
│   │   ├── commands/
│   │   ├── nostr/
│   │   ├── p2p/
│   │   └── db/
│   └── Cargo.toml
├── docs/                   # ドキュメント
├── scripts/                # ユーティリティスクリプト
├── workers/                # Cloudflare Workers
└── pkarr/                  # Pkarr サブモジュール (DHT リレー)
```

## Pkarr リレーサーバー (ローカル開発)
ローカル開発では Docker Compose を使って Pkarr リレーサーバーを起動できます。

```bash
# submodule の初期化（初回のみ）
git submodule update --init --recursive

# リレーサーバー起動
docker-compose up -d

# ログ確認
docker-compose logs -f pkarr

# 停止
docker-compose down
```

設定は `config.toml` で管理されています。
- HTTP ポート: 8080
- DHT ポート: 6881
- キャッシュ: `.pkarr_cache/`

動作確認例:
```bash
curl http://localhost:8080/health
curl http://localhost:8080/stats
```

## ドキュメント
- [プロジェクト設計書](docs/01_project/design_doc.md)
- [システム設計書](docs/02_architecture/system_design.md)
- [実装計画](docs/03_implementation/implementation_plan.md)
- [開発環境セットアップガイド](docs/01_project/setup_guide.md)

## コントリビューション
1. Issue を作成して変更内容を共有
2. フィーチャーブランチ作成: `git checkout -b feature/amazing-feature`
3. 変更をコミット: `git commit -m 'Add amazing feature'`
4. ブランチをプッシュ: `git push origin feature/amazing-feature`
5. プルリクエストを作成

## ライセンス
MIT License。詳細は [LICENSE](LICENSE) を参照してください。

## 外部リソース
- [iroh ドキュメント](https://docs.rs/iroh/latest/iroh/)
- [iroh-gossip ドキュメント](https://docs.rs/iroh-gossip/latest/iroh_gossip/)
- [Nostr NIPs](https://github.com/nostr-protocol/nips)
- [Tauri ドキュメント](https://tauri.app/)

## お問い合わせ
- Issue: [GitHub Issues](https://github.com/yourusername/kukuri/issues)
- Discussion: [GitHub Discussions](https://github.com/yourusername/kukuri/discussions)
