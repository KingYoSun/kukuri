# CLAUDE.md

## 作業開始時の確認事項
1. `docs/01_project/activeContext/current_tasks.md` で最新タスクを把握
2. ドキュメントの最終更新日を確認（古い情報に注意）

## 基本ルール
- **serena mcpを使用**: ユーザーから明示的に指示されない限り、serena mcpを活用する
- **言語**: 必ず日本語で回答
- **コミット**: ユーザーから明示的に要求されない限り、絶対にコミットしない
- **日付**: ドキュメント作成/更新の前に必ず`date "+%Y年%m月%d日"`コマンドで今日の日付を確認
- **DRY原則**: 新しいクラス・新しいメソッド・新しい型を実装する際は同じ機能を持ったものが既にないかを必ず調査する
- **依存最新化**: 依存ライブラリを追加する際は、webから最新バージョンを確認する
- **エラーハンドリング**: フロントエンドでの`console.error`使用は禁止。代わりに`errorHandler`を使用する（`docs/03_implementation/error_handling_guidelines.md`参照）

### 実装のリファレンス
- **zustand**: storeのモックを実装/変更する前にzustand公式のドキュメント( https://zustand.docs.pmnd.rs/guides/testing )を必ず参照する
- **zustandテスト**: Zustandストアのテスト実装時は必ず `docs/03_implementation/zustand_testing_best_practices.md` を参照する
- **iroh-gossip**: iroh-gossipを用いる機能を実装/変更する前にiroh-gossipの公式ドキュメント ( https://docs.rs/iroh-gossip/latest/iroh_gossip/ )を必ず参照する
- **iroh**: irohを用いる機能を実装/変更する前にirohの公式ドキュメント ( https://docs.rs/iroh/latest/iroh/)を必ず参照する

### Nostr互換性の確認
- **NIP準拠**: Nostr関連の実装を行う際は、`docs/nips/`内の該当するNIPを必ず参照
- **標準仕様の確認**: 新機能実装前に関連するNIPが存在しないか確認
- **互換性チェック**: 実装がNIP仕様に準拠しているか検証
- **拡張時の注意**: 独自拡張を行う場合は、NIPs標準との違いを明確に文書化

### 作業完了時のチェック
- [ ] current_tasks.mdの更新（完了タスクを反映）
- [ ] TodoWriteツールでタスクリストを更新
- [ ] 重要な変更は進捗レポート作成

## プロジェクト概要

**kukuri** - Nostrプロトコルベースの分散型トピック中心ソーシャルアプリケーション

トピックベースのタイムラインでユーザーが情報を共有・発見できる、検閲耐性を持つP2Pソーシャルプラットフォーム。Nostrの分散性とハイブリッドP2Pアプローチを組み合わせ、優れたユーザー体験を提供。

### 技術スタック

#### フロントエンド
- **Framework**: React 18 + TypeScript
- **Build Tool**: Vite
- **UI Components**: shadcn/ui (Radix UI + Tailwind CSS)
- **State Management**: Zustand
- **Data Fetching**: Tanstack Query
- **Routing**: Tanstack Router

#### バックエンド
- **Desktop Framework**: Tauri v2 (Rust)
- **P2P Network**: iroh (QUIC-based)
- **Event Distribution**: iroh-gossip (トピックベース配信)
- **Protocol**: Nostr (nostr-sdk)
- **Database**: SQLite (sqlx)
- **Cryptography**: secp256k1, AES-256-GCM

#### インフラ
- **Discovery Service**: Cloudflare Workers (OSS) / Docker
- **Marketplace**: 分散ノード（検索・サジェスト）

## 必須コマンド

### 開発
```bash
# 開発サーバー起動
pnpm tauri dev

# ビルド
pnpm tauri build

# ビルド(Windows)
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc

# ビルド(Android)
pnpm tauri android build

# テスト実行
pnpm test
cargo test

# リント
pnpm lint
cargo clippy

# フォーマット
pnpm format
cargo fmt

# フォーマットチェック
pnpm format:check
```

### Windows環境（WSLなし）での実行
pnpmコマンドでBashエラーが発生する場合は、npm runを使用してください：
```bash
# テスト実行
npm run test

# 型チェック
npx tsc --noEmit

# リント
npm run lint
```

## アーキテクチャ

### レイヤー構成
1. **Client Layer**: Tauri App (UI + Business Logic)
2. **Discovery Layer**: ピア発見サービス (Workers/Container)
3. **P2P Network**: irohによる直接通信
4. **Marketplace**: 専門機能ノード (検索/推薦)

## ドキュメント構成

### 優先参照順
1. `docs/SUMMARY.md` - 全体概要
2. `docs/01_project/activeContext/` - 現在の状況
3. 各ディレクトリのsummary.md - カテゴリー概要
4. 詳細ドキュメント - 実装時のみ

## プロジェクトディレクトリ構造

### 重要：ドキュメントは必ず`./docs`以下に配置
```
kukuri/
│── .claude
│── .github
│── .vscode
├── docs/                     # すべてのドキュメントはここに配置
│   ├── 01_project/           # プロジェクト管理
│   ├── 02_architecture/      # 設計ドキュメント
│   ├── 03_implementation/    # 実装ガイドライン
│   ├── nips/                 # Nostr Implementation Possibilities
│   └── SUMMARY.md            # ドキュメント全体の概要
├── kukuri-tauri/             # Tauriアプリケーション本体
│   ├── src/                  # フロントエンドソースコード
│   ├── src-tauri/            # Rustバックエンドコード
│   ├── tests/                # E2Eテスト
│   └── ※ docsディレクトリは作成しない
│── workers/                  # Cloudflare Workers（今後実装）
│── scripts/                  # 便利なスクリプト集
│── .gitignore
│── CLAUDE.md                 # ClaudeCodeのルール
└── README.md
```

### ドキュメント配置ルール
- **禁止事項**: `kukuri-tauri/docs/`のようなサブディレクトリ内にドキュメントを作成しない
- **すべてのドキュメント**: `./docs/`以下の適切なカテゴリに配置
- **進捗レポート**: `./docs/01_project/progressReports/`に作成
- **実装ガイド**: `./docs/03_implementation/`に作成
- **アーキテクチャ文書**: `./docs/02_architecture/`に作成

## 詳細参照先
- 環境情報: `docs/01_project/activeContext/current_environment.md`
- 既知の問題: `docs/01_project/activeContext/issuesAndNotes.md`
- 開発進捗: `docs/01_project/progressReports/`

## 外部ドキュメント
- **iroh**: https://docs.rs/iroh/latest/iroh/
- **iroh-gossip**: https://docs.rs/iroh-gossip/latest/iroh_gossip/
- **Nostr NIPs**: https://github.com/nostr-protocol/nips
