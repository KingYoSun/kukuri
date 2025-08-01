# 現在の開発環境

**最終更新**: 2025年8月1日

## 動作確認済み環境

### Windows環境
- **OS**: Windows 11
- **Node.js**: v20.x以上
- **pnpm**: v8.x以上
- **Rust**: 1.80.0以上
- **動作確認日**: 2025年8月1日
- **特記事項**: 
  - アプリケーションデータは`C:\Users\{username}\AppData\Roaming\com.kukuri.app`に保存
  - SQLiteデータベースは正常に作成・接続可能
  - Tauri v2環境で正常動作

### WSL環境（Ubuntu on Windows）
- **OS**: Ubuntu 22.04 on WSL2
- **Node.js**: v20.x以上
- **pnpm**: v8.x以上
- **Rust**: 1.80.0以上
- **動作確認日**: 2025年8月1日
- **特記事項**:
  - セキュアストレージはフォールバック実装を使用（開発環境のみ）
  - データは`~/.local/share/kukuri-dev/secure_storage/`に保存
  - アカウント永続化機能は正常動作

### macOS環境
- **動作確認**: 未実施
- **予定**: 今後確認予定

### Linux環境（ネイティブ）
- **動作確認**: 未実施
- **予定**: 今後確認予定

## 必要な依存関係

### システム要件
- **Windows**: 
  - WebView2ランタイム（Windows 10/11には標準搭載）
  - Visual Studio 2022 Build Tools（C++開発ワークロード）
  
- **Linux/WSL**:
  - 各種開発ライブラリ（libssl-dev, libgtk-3-dev等）
  - X11/Waylandサポート

### 開発ツール
```bash
# Node.js/pnpm
node --version  # v20.x以上
pnpm --version  # v8.x以上

# Rust
rustc --version  # 1.80.0以上
cargo --version  # 1.80.0以上

# Tauri CLI
cargo tauri --version  # 2.x
```

## ビルド・実行コマンド

### 開発環境
```bash
# 開発サーバー起動
pnpm tauri dev

# フロントエンドのみ
pnpm dev

# バックエンドのみ
cargo run --manifest-path kukuri-tauri/src-tauri/Cargo.toml
```

### ビルド
```bash
# デバッグビルド
pnpm tauri build --debug

# リリースビルド
pnpm tauri build

# Windows向けクロスコンパイル（Linux/macOSから）
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
```

### テスト
```bash
# フロントエンドテスト
pnpm test

# バックエンドテスト
cargo test

# E2Eテスト
pnpm test:e2e
```

### リント・フォーマット
```bash
# リント
pnpm lint
cargo clippy

# フォーマット
pnpm format
cargo fmt

# フォーマットチェック
pnpm format:check
cargo fmt -- --check
```

## 環境別の注意事項

### Windows
- パス処理では`tauri::Manager` traitのインポートが必要
- SQLiteのURL形式は`sqlite:C:/path/to/db`（スラッシュなし）
- バックスラッシュはスラッシュに変換する必要がある

### WSL
- セキュアストレージのフォールバック実装が自動的に有効化
- GUI表示にはWSLgまたはX11サーバーが必要
- ファイルシステムの権限に注意

### 共通
- 初回起動時はRustの依存関係ダウンロードに時間がかかる
- `node_modules`と`target`ディレクトリは`.gitignore`に含まれる
- ビルド成果物は`kukuri-tauri/src-tauri/target/release/bundle/`に生成