# コードベース構造

## プロジェクトルート構造
```
kukuri/
├── .github/              # GitHub Actions設定
├── .vscode/              # VSCode設定
├── docs/                 # すべてのドキュメント
│   ├── 01_project/       # プロジェクト管理
│   │   ├── activeContext/    # 現在の状況
│   │   └── progressReports/  # 進捗レポート
│   ├── 02_architecture/  # 設計ドキュメント
│   ├── 03_implementation/# 実装ガイドライン
│   ├── nips/            # Nostr Implementation Possibilities
│   └── SUMMARY.md       # ドキュメント全体の概要
├── kukuri-tauri/        # Tauriアプリケーション本体
├── workers/             # Cloudflare Workers（今後実装）
├── scripts/             # 便利なスクリプト集
├── .gitignore
├── CLAUDE.md           # ClaudeCodeのルール
└── README.md
```

## フロントエンド構造（kukuri-tauri/src/）
```
src/
├── assets/             # 静的アセット
├── components/         # UIコンポーネント
│   ├── auth/          # 認証関連
│   ├── layout/        # レイアウト（Header, Sidebar等）
│   ├── posts/         # 投稿関連
│   ├── topics/        # トピック関連
│   └── ui/            # shadcn/uiコンポーネント
├── hooks/             # カスタムフック
│   ├── useAuth.ts
│   ├── useNostr.ts
│   ├── useP2P.ts
│   └── useTopics.ts
├── lib/               # ユーティリティ
│   ├── errorHandler.ts
│   └── utils.ts
├── routes/            # ページコンポーネント（Tanstack Router）
│   ├── __root.tsx
│   ├── index.tsx
│   ├── topics/
│   └── settings/
├── stores/            # Zustandストア
│   ├── authStore.ts
│   ├── topicStore.ts
│   └── p2pStore.ts
├── types/             # 型定義
│   ├── nostr.ts
│   └── p2p.ts
├── test/              # テスト関連
│   └── integration/   # 統合テスト
├── App.tsx
├── main.tsx
└── router.tsx
```

## バックエンド構造（kukuri-tauri/src-tauri/）
```
src/
├── modules/           # 機能モジュール
│   ├── auth/         # 認証・鍵管理
│   │   ├── mod.rs
│   │   ├── commands.rs
│   │   └── key_manager.rs
│   ├── crypto/       # 暗号化
│   │   ├── mod.rs
│   │   └── encryption_manager.rs
│   ├── database/     # DB操作
│   │   ├── mod.rs
│   │   ├── connection.rs
│   │   └── migrations/
│   ├── event/        # Nostrイベント
│   │   ├── mod.rs
│   │   └── handler.rs
│   ├── p2p/          # P2P通信
│   │   ├── mod.rs
│   │   ├── manager.rs
│   │   ├── gossip.rs
│   │   └── sync.rs
│   ├── post/         # 投稿管理
│   ├── secure_storage/ # セキュアストレージ
│   ├── storage/      # 一般ストレージ
│   ├── topic/        # トピック管理
│   └── mod.rs
├── lib.rs            # ライブラリエントリー
├── main.rs           # メインエントリー
└── state.rs          # アプリケーション状態
```

## 重要なファイル

### 設定ファイル
- `kukuri-tauri/package.json` - フロントエンド依存関係
- `kukuri-tauri/src-tauri/Cargo.toml` - Rust依存関係
- `kukuri-tauri/src-tauri/tauri.conf.json` - Tauri設定
- `.mcp.json` - MCP設定

### テスト関連
- `kukuri-tauri/vitest.config.ts` - Vitestテスト設定
- `kukuri-tauri/tests/` - E2Eテスト

### ビルド関連
- `kukuri-tauri/vite.config.ts` - Viteビルド設定
- `kukuri-tauri/tsconfig.json` - TypeScript設定
- `kukuri-tauri/tailwind.config.ts` - Tailwind CSS設定

## ドキュメント配置ルール
- **禁止**: サブプロジェクト内にdocsディレクトリを作成しない
- **必須**: すべてのドキュメントは`./docs/`以下に配置
- **進捗レポート**: `./docs/01_project/progressReports/`
- **実装ガイド**: `./docs/03_implementation/`
- **アーキテクチャ**: `./docs/02_architecture/`