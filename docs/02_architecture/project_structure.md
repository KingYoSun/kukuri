# kukuri プロジェクト構造

## ドキュメント情報
- **作成日**: 2025年07月26日
- **最終更新**: 2026年02月10日
- **目的**: kukuriプロジェクトのディレクトリ構造とファイル構成の説明

## プロジェクト全体構成

```
kukuri/
├── kukuri-tauri/             # デスクトップアプリ（React + TypeScript + Tauri）
├── kukuri-community-node/    # Community Node（サーバー/運用コンポーネント）
├── docs/                     # プロジェクトドキュメント
├── scripts/                  # セットアップ・テスト・起動用スクリプト
├── docker/                   # Docker関連
└── README.md                 # プロジェクトREADME
```

## kukuri-tauri ディレクトリ構造

### ルートレベル
```
kukuri-tauri/
├── package.json         # Node.js プロジェクト設定
├── pnpm-lock.yaml       # pnpm ロックファイル
├── tsconfig.json        # TypeScript 設定
├── tsconfig.node.json   # Node.js用 TypeScript 設定
├── vite.config.ts       # Vite 設定
├── index.html           # エントリーHTML
├── README.md            # アプリケーションREADME
├── src/                 # フロントエンドソースコード
├── src-tauri/           # Tauriバックエンド（Rust）
├── public/              # 静的ファイル
└── dist/                # ビルド出力（gitignore対象）
```

### フロントエンド構造 (src/)
```
src/
├── api/                 # Tauri/HTTP APIラッパー
├── assets/              # 画像・フォント等のアセット
├── components/          # UIコンポーネント
├── config/              # 設定
├── constants/           # 定数
├── hooks/               # カスタムフック
├── lib/                 # 共通ユーティリティ
├── pages/               # ページコンポーネント
├── routes/              # ルーティング定義
├── services/            # アプリケーションサービス層
├── serviceWorker/       # Service Worker
├── stores/              # Zustand ストア
├── testing/             # テストユーティリティ
├── tests/               # テスト資材（E2E/統合向け）
├── types/               # TypeScript型定義
├── App.css              # アプリケーションスタイル
├── App.tsx              # ルートコンポーネント
├── main.tsx             # アプリケーションエントリーポイント
├── router.tsx           # ルーター設定
├── routeTree.gen.ts     # TanStack Router 生成ファイル
└── vite-env.d.ts        # Vite TypeScript定義
```

### Tauriバックエンド構造 (src-tauri/)
```
src-tauri/
├── Cargo.toml          # Rust プロジェクト設定
├── Cargo.lock          # Cargo ロックファイル
├── tauri.conf.json     # Tauri 設定
├── build.rs            # ビルドスクリプト
├── migrations/         # SQLxマイグレーション
├── .sqlx/              # SQLxオフラインスキーマ
├── tests/              # Rust統合テスト
├── src/                # Rustソースコード
└── bootstrap_nodes.json # ブートストラップノード設定
```

#### Rust 5層構造（src-tauri/src/）
```
src-tauri/src/
├── application/        # ユースケース/アプリサービス
├── domain/             # ドメインモデル（Entity/Value Object）
├── infrastructure/     # iroh/DB/外部I/Oの実装
├── presentation/       # Tauriコマンド/DTO
├── shared/             # 設定/エラー/共通ユーティリティ
├── state/              # AppState/起動時配線
├── bin/                # 追加バイナリ
├── contract_testing.rs # 契約テスト補助
├── lib.rs              # ライブラリルート
├── main.rs             # エントリーポイント
└── state.rs            # 状態初期化
```

## cn-cli（kukuri-community-node 内）ディレクトリ構造（概要）

```
kukuri-community-node/crates/cn-cli/
├── Cargo.toml
└── src/                # bootstrap/relay/p2p 管理CLI
```

## kukuri-community-node ディレクトリ構造（概要）

```
kukuri-community-node/
├── apps/               # アプリケーション層
├── crates/             # 共通ライブラリ
├── migrations/         # DBマイグレーション
├── docker/             # デプロイ/運用支援
└── Cargo.toml
```

## 注意事項

- `node_modules/` と `target/` はgitignore対象
- 生成されたコード（例: `routeTree.gen.ts`, `gen/`）は編集しない
- SQLxの `query!` を使う場合は `.sqlx/` を更新する
