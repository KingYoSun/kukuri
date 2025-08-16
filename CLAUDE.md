# CLAUDE.md

## 作業開始時の確認事項
1. `docs/01_project/activeContext/tasks/` で最新タスクを把握
   - `tasks/priority/critical.md` - 最重要タスク（最大3個）
   - `tasks/status/in_progress.md` - 現在進行中のタスク
   - `tasks/README.md` - タスク全体のダッシュボード
2. ドキュメントの最終更新日を確認（古い情報に注意）

## 基本ルール
- **serena mcpを使用**: typescriptの開発ではserena mcpを活用する。Rustの開発では利用しない。
- **ghコマンドを使用**: GitHubに関連した操作はGitHub Cli（ghコマンド）を用いる
- **言語**: 必ず日本語で回答
- **コミット**: ユーザーから明示的に要求されない限り、絶対にコミットしない
- **日付**: ドキュメント作成/更新の前に必ず`date "+%Y年%m月%d日"`コマンドで今日の日付を確認し、出力された日付を使用する
- **DRY原則**: 新しいクラス・新しいメソッド・新しい型を実装する際は同じ機能を持ったものが既にないかを必ず調査する
- **依存最新化**: 依存ライブラリを追加する際は、webから最新バージョンを確認する
- **エラーハンドリング**: フロントエンドでの`console.error`使用は禁止。代わりに`errorHandler`を使用する（`docs/03_implementation/error_handling_guidelines.md`参照）
- **テスト実行**: E2Eテストを除き、Windows環境では必ず`.\scripts\test-docker.ps1`を使用してテストを実行する
- **確認必須**: テスト・型・リントのエラーの修正タスクは、コマンドを実行してエラーが出ないことを確認しない限り「完了」にしてはならない

### 実装のリファレンス
- **zustand**: storeのモックを実装/変更する前にzustand公式のドキュメント( https://zustand.docs.pmnd.rs/guides/testing )を必ず参照する
- **zustandテスト**: Zustandストアのテスト実装時は必ず `docs/03_implementation/zustand_testing_best_practices.md` を参照する
- **iroh-gossip**: iroh-gossipを用いる機能を実装/変更する前にiroh-gossipの公式API情報の入ったJSON ( https://docs.rs/crate/iroh-gossip/latest/json )を必ず参照する
- **iroh-docs**: iroh-docsを用いる機能を実装/変更する前にiroh-docsの公式API情報の入ったJSON（ https://docs.rs/crate/iroh-docs/latest/json ）を必ず参照する
- **iroh-blobs**: iroh-blobsを用いる機能を実装/変更する前にiroh-blobsの公式API情報の入ったJSON（ https://docs.rs/crate/iroh-blobs/latest/json ）を必ず参照する
- **iroh**: irohを用いる機能を実装/変更する前にirohの公式API情報の入ったJSON ( https://docs.rs/crate/iroh/latest/json )を必ず参照する

### Nostr互換性の確認
- **NIP準拠**: Nostr関連の実装を行う際は、`docs/nips/`内の該当するNIPを必ず参照
- **標準仕様の確認**: 新機能実装前に関連するNIPが存在しないか確認
- **互換性チェック**: 実装がNIP仕様に準拠しているか検証
- **拡張時の注意**: 独自拡張を行う場合は、NIPs標準との違いを明確に文書化

### タスク管理ルール
**タスク開始時**:
1. `tasks/priority/critical.md`から最重要タスクを選択
2. 選択したタスクを`tasks/status/in_progress.md`に移動
3. TodoWriteツールと同期

**作業中**:
- `tasks/status/in_progress.md`のみを更新（進捗記録）
- 他のファイルは触らない

**タスク完了時**:
1. `tasks/completed/YYYY-MM-DD.md`に完了内容を追記（新規作成も可）
2. `tasks/status/in_progress.md`から削除
3. TodoWriteツールでタスクリストを更新
4. 重要な変更は進捗レポート作成

**ブロッカー発生時**:
- `tasks/context/blockers.md`に追記
- 解決したら削除

### 作業完了時のチェック
- [ ] `tasks/completed/YYYY-MM-DD.md`に完了タスク追記
- [ ] `tasks/status/in_progress.md`から完了タスク削除
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

## SQLx開発環境

### オフラインモード設定
SQLxのquery!マクロを使用する場合、以下の手順でオフラインモードを設定：

```bash
# データベース作成とマイグレーション
cd kukuri-tauri/src-tauri
DATABASE_URL="sqlite:data/kukuri.db" sqlx database create
DATABASE_URL="sqlite:data/kukuri.db" sqlx migrate run

# オフライン用スキーマファイル生成（.sqlxディレクトリに保存）
DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare
```

**注意**: `.sqlx`ディレクトリはGitにコミットする必要があります

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

### Windows環境での推奨実行方法（Docker使用）
DLLエラーによりネイティブ環境でテストが実行できない場合は、Docker環境を使用してください：
```powershell
# Docker環境で全テスト実行（推奨）
.\scripts\test-docker.ps1

# Rustテストのみ
.\scripts\test-docker.ps1 rust

# TypeScriptテストのみ
.\scripts\test-docker.ps1 ts

# リントとフォーマットチェック
.\scripts\test-docker.ps1 lint
```

### Windows環境（WSLなし）でのネイティブ実行
pnpmコマンドでBashエラーが発生する場合は、npm runを使用してください：
```bash
# テスト実行（TypeScriptのみ）
npm run test

# 型チェック
npx tsc --noEmit

# リント
npm run lint
```
**注意**: Rustテストはネイティブ環境では実行できないため、必ずDockerを使用してください。

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
- **iroh**: https://docs.rs/crate/iroh/latest/json
- **iroh-gossip**: https://docs.rs/crate/iroh-gossip/latest/json
- **iroh-docs**: https://docs.rs/crate/iroh-blobs/latest/json
- **iroh-blobs**: https://docs.rs/crate/iroh-blobs/latest/json
- **Nostr NIPs**: https://github.com/nostr-protocol/nips
