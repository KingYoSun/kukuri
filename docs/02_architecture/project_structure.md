# kukuri プロジェクト構造

## ドキュメント情報
- **作成日**: 2025年7月26日
- **最終更新**: 2025年7月26日
- **目的**: kukuriプロジェクトのディレクトリ構造とファイル構成の説明

## プロジェクト全体構成

```
kukuri/
├── kukuri-tauri/         # Tauriアプリケーション本体
├── docs/                 # プロジェクトドキュメント
├── scripts/              # ユーティリティスクリプト
└── README.md            # プロジェクトREADME
```

## kukuri-tauri ディレクトリ構造

### ルートレベル
```
kukuri-tauri/
├── package.json         # Node.js プロジェクト設定
├── pnpm-lock.yaml      # pnpm ロックファイル
├── tsconfig.json       # TypeScript 設定
├── tsconfig.node.json  # Node.js用 TypeScript 設定
├── vite.config.ts      # Vite 設定
├── index.html          # エントリーHTML
├── README.md           # アプリケーションREADME
├── src/                # フロントエンドソースコード
├── src-tauri/          # Tauriバックエンド（Rust）
├── public/             # 静的ファイル
├── dist/               # ビルド出力（gitignore対象）
└── node_modules/       # Node.js依存関係（gitignore対象）
```

### フロントエンド構造 (src/)
```
src/
├── main.tsx            # アプリケーションエントリーポイント
├── App.tsx             # ルートコンポーネント
├── App.css             # アプリケーションスタイル
├── vite-env.d.ts       # Vite TypeScript定義
└── assets/             # 画像・フォント等のアセット
    └── react.svg       # Reactロゴ
```

#### 今後追加予定の構造
```
src/
├── components/         # UIコンポーネント
│   ├── auth/          # 認証関連
│   ├── common/        # 共通コンポーネント
│   ├── profile/       # プロフィール関連
│   ├── topics/        # トピック関連
│   └── ui/            # shadcn/ui コンポーネント
├── hooks/             # カスタムフック
├── lib/               # ユーティリティ関数
├── pages/             # ページコンポーネント
├── services/          # API・サービス層
├── stores/            # Zustand ストア
├── styles/            # グローバルスタイル
├── types/             # TypeScript型定義
└── utils/             # ヘルパー関数
```

### Tauriバックエンド構造 (src-tauri/)
```
src-tauri/
├── Cargo.toml          # Rust プロジェクト設定
├── Cargo.lock          # Cargo ロックファイル
├── tauri.conf.json     # Tauri 設定
├── build.rs            # ビルドスクリプト
├── src/                # Rustソースコード
│   ├── main.rs         # メインエントリーポイント
│   └── lib.rs          # ライブラリエントリーポイント
├── capabilities/       # Tauri権限設定
│   └── default.json    # デフォルト権限
├── icons/              # アプリケーションアイコン
├── target/             # Rustビルド出力（gitignore対象）
└── gen/                # 生成されたコード
    ├── schemas/        # JSONスキーマ
    └── android/        # Android固有ファイル
```

#### 今後追加予定のRust構造
```
src-tauri/src/
├── main.rs             # アプリケーションエントリー
├── lib.rs              # ライブラリルート
├── commands/           # Tauriコマンド（IPC）
│   ├── mod.rs
│   ├── auth.rs         # 認証コマンド
│   ├── profile.rs      # プロフィールコマンド
│   └── topics.rs       # トピックコマンド
├── crypto/             # 暗号化モジュール
│   ├── mod.rs
│   ├── keys.rs         # 鍵管理
│   └── encryption.rs   # 暗号化処理
├── db/                 # データベース層
│   ├── mod.rs
│   ├── models.rs       # データモデル
│   ├── schema.rs       # スキーマ定義
│   └── migrations/     # マイグレーション
├── nostr/              # Nostrプロトコル層
│   ├── mod.rs
│   ├── events.rs       # イベント処理
│   └── client.rs       # Nostrクライアント
├── p2p/                # P2P通信層
│   ├── mod.rs
│   ├── iroh.rs         # iroh統合
│   └── gossip.rs       # iroh-gossip実装
├── state/              # アプリケーション状態
└── utils/              # ユーティリティ
```

### 生成ファイル (gen/)

#### Android関連ファイル
```
gen/android/
├── app/                        # Androidアプリケーションモジュール
│   ├── build.gradle.kts        # Gradleビルド設定
│   ├── proguard-rules.pro      # ProGuard設定
│   ├── src/main/
│   │   ├── AndroidManifest.xml # アプリマニフェスト
│   │   ├── java/com/kukuri/    # Kotlinソースコード
│   │   ├── jniLibs/            # ネイティブライブラリ
│   │   └── res/                # Androidリソース
│   └── tauri.properties        # Tauri設定
├── build.gradle.kts            # ルートGradle設定
├── settings.gradle             # Gradle設定
└── gradle/                     # Gradleラッパー
```

### 設定ファイル詳細

#### tauri.conf.json
Tauriアプリケーションの主要設定ファイル：
- ウィンドウ設定
- セキュリティ設定
- ビルド設定
- バンドル設定

#### capabilities/default.json
Tauriのセキュリティ権限設定：
- APIアクセス権限
- ファイルシステムアクセス
- ネットワークアクセス

## 開発ワークフロー

### 新機能追加時のファイル作成パターン

1. **UIコンポーネント追加**
   ```
   src/components/[機能名]/
   ├── index.ts            # エクスポート
   ├── [Component].tsx     # コンポーネント本体
   └── [Component].test.tsx # テストファイル
   ```

2. **Tauriコマンド追加**
   ```
   src-tauri/src/commands/[機能名].rs
   → main.rsでモジュール登録
   → tauri::Builder に .invoke_handler() で追加
   ```

3. **状態管理追加**
   ```
   src/stores/[機能名]Store.ts
   ```

## ビルド成果物

### 開発ビルド
- `dist/` - Viteビルド出力
- `src-tauri/target/debug/` - Rustデバッグビルド

### プロダクションビルド
- `src-tauri/target/release/` - Rustリリースビルド
- `src-tauri/target/release/bundle/` - OS別パッケージ
  - Windows: `.msi`, `.exe`
  - macOS: `.dmg`, `.app`
  - Linux: `.deb`, `.AppImage`

## 注意事項

- `node_modules/` と `target/` はgitignore対象
- 生成されたコード（`gen/`）は編集しない
- アイコンファイルは複数サイズを用意（Tauri要件）
- Android関連ファイルは`pnpm tauri android init`で生成