# Windows 開発環境セットアップガイド

## 概要
このガイドは、Windows環境でkukuriプロジェクトの開発環境をセットアップする手順を説明します。

## 前提条件
- Windows 10/11 (64-bit)
- 管理者権限
- インターネット接続

## インストール手順

### 1. Node.js のインストール

1. [Node.js公式サイト](https://nodejs.org/)から最新のLTS版をダウンロード
2. インストーラーを実行し、デフォルト設定でインストール
3. コマンドプロンプトまたはPowerShellで確認：
   ```powershell
   node --version
   npm --version
   ```

### 2. pnpm のインストール

PowerShellを**管理者として実行**し、以下を実行：

```powershell
# PowerShellの実行ポリシーを一時的に変更
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope Process

# pnpmのインストール
iwr https://get.pnpm.io/install.ps1 -useb | iex
```

インストール後、新しいターミナルを開いて確認：
```powershell
pnpm --version
```

### 3. Rust & Cargo のインストール

1. [Rust公式サイト](https://www.rust-lang.org/tools/install)からrustup-init.exeをダウンロード
2. rustup-init.exeを実行
3. デフォルト設定でインストール（1を選択してEnter）
4. インストール完了後、新しいターミナルで確認：
   ```powershell
   rustc --version
   cargo --version
   ```

### 4. Visual Studio C++ Build Tools のインストール（必須）

Rustのコンパイルに必要です：

1. [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)をダウンロード
2. インストーラーを実行
3. 「Desktop development with C++」ワークロードを選択
4. インストール（数GBのダウンロードがあります）

### 5. Tauri CLI のインストール

新しいターミナルを開いて実行：
```powershell
cargo install tauri-cli
```

### 6. sqlx-cli のインストール

データベースマイグレーション用：
```powershell
cargo install sqlx-cli --no-default-features --features native-tls,sqlite
```

## インストール確認

すべてのツールが正しくインストールされたか確認：

```powershell
# バージョン確認コマンド
node --version
pnpm --version
rustc --version
cargo --version
cargo tauri --version
sqlx --version
```

## プロジェクトのセットアップ

1. プロジェクトディレクトリに移動：
   ```powershell
   cd C:\Users\<your-username>\kukuri
   ```

2. 依存関係のインストール：
   ```powershell
   cd kukuri-tauri
   pnpm install
   ```

3. 開発サーバーの起動：
   ```powershell
   pnpm tauri dev
   ```

## トラブルシューティング

### pnpmが認識されない
- 新しいターミナルを開いて再試行
- 環境変数PATHに`%USERPROFILE%\.pnpm`が追加されているか確認

### Rustのコンパイルエラー
- Visual Studio C++ Build Toolsがインストールされているか確認
- `rustup update`でRustを最新版に更新

### Tauriの起動エラー
- WebView2がインストールされているか確認（Windows 11は標準搭載）
- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)から手動インストール

## 環境変数の設定（オプション）

開発効率を上げるための環境変数：

1. システムの詳細設定 → 環境変数
2. ユーザー環境変数に追加：
   - `RUST_BACKTRACE=1` （エラー時の詳細表示）
   - `CARGO_HOME=%USERPROFILE%\.cargo`
   - `RUSTUP_HOME=%USERPROFILE%\.rustup`

## 推奨される開発ツール

- **Visual Studio Code**
  - rust-analyzer拡張機能
  - Tauri拡張機能
  - ESLint拡張機能
  - Prettier拡張機能

- **Windows Terminal**（PowerShellのモダンな代替）
  - Microsoft Storeからインストール

## 次のステップ

1. `CLAUDE.md`を確認して開発ルールを理解
2. `docs/01_project/activeContext/current_tasks.md`で現在のタスクを確認
3. `pnpm test`でテストの実行を確認