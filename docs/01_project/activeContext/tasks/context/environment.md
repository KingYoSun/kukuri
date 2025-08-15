# 開発環境情報

**最終更新**: 2025年08月16日

## 動作確認済み環境

### Windows環境 ✅
- **OS**: Windows 11
- **Node.js**: v20.x以上
- **pnpm**: v8.x以上
- **Rust**: 1.80.0以上
- **動作確認日**: 2025年08月02日
- **特記事項**: 
  - アプリケーションデータ: `C:\Users\{username}\AppData\Roaming\com.kukuri.app`
  - SQLiteデータベース正常動作
  - Tauri v2環境で正常動作
  - Windows Credential Managerでアカウント永続化
  - keyringライブラリの`windows-native`フィーチャー使用

### WSL環境（Ubuntu on Windows） ✅
- **OS**: Ubuntu 22.04 on WSL2
- **動作確認日**: 2025年08月01日
- **特記事項**:
  - セキュアストレージはフォールバック実装使用（開発環境のみ）
  - データ保存先: `~/.local/share/kukuri-dev/secure_storage/`
  - アカウント永続化機能正常動作

### macOS環境 ❌
- **動作確認**: 未実施

### Linux環境（ネイティブ） ❌
- **動作確認**: 未実施

## 必要な依存関係

### システム要件

#### Windows
- WebView2ランタイム（Windows 10/11標準搭載）
- Visual Studio 2022 Build Tools（C++開発ワークロード）

#### Linux/WSL
- 開発ライブラリ（libssl-dev, libgtk-3-dev等）
- X11/Waylandサポート

### 開発ツール
```bash
node --version  # v20.x以上
pnpm --version  # v8.x以上
rustc --version # 1.80.0以上
cargo --version # 1.80.0以上
cargo tauri --version # 2.x
```

## 主要コマンド

### 開発
```bash
pnpm tauri dev         # 開発サーバー起動
pnpm dev               # フロントエンドのみ
cargo run --manifest-path kukuri-tauri/src-tauri/Cargo.toml  # バックエンドのみ
```

### ビルド
```bash
pnpm tauri build --debug  # デバッグビルド
pnpm tauri build          # リリースビルド
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc  # クロスコンパイル
```

### テスト
```bash
pnpm test           # フロントエンド
cargo test          # バックエンド（Windowsではtest-docker.ps1推奨）
pnpm test:e2e       # E2E
.\scripts\test-docker.ps1  # Windows環境推奨
```

### 品質管理
```bash
pnpm lint          # TypeScriptリント
cargo clippy       # Rustリント
pnpm format        # フォーマット
cargo fmt          # Rustフォーマット
```

## 環境別注意事項

### Windows
- パス処理: `tauri::Manager` traitインポート必要
- SQLite URL: `sqlite:C:/path/to/db`（スラッシュなし）
- バックスラッシュ→スラッシュ変換必要
- keyring: `windows-native`フィーチャー有効化

### WSL
- セキュアストレージ: 標準keyringライブラリ使用
- GUI: WSLgまたはX11サーバー必要
- ファイルシステム権限注意
- フォールバック実装削除済み（セキュリティ理由）

### 共通
- 初回起動時: Rust依存関係DLに時間必要
- 除外ディレクトリ: `node_modules`, `target`
- ビルド成果物: `kukuri-tauri/src-tauri/target/release/bundle/`

## Docker環境（Windows推奨）

```powershell
# 全テスト実行
.\scripts\test-docker.ps1

# Rustのみ
.\scripts\test-docker.ps1 rust

# TypeScriptのみ
.\scripts\test-docker.ps1 ts

# リントとフォーマット
.\scripts\test-docker.ps1 lint
```