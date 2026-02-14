# kukuri 開発環境セットアップガイド

## ドキュメント情報
- **作成日**: 2025年07月25日
- **最終更新**: 2026年02月14日
- **目的**: kukuriプロジェクトの開発環境構築手順

## 目次
1. [前提条件](#前提条件)
2. [開発ツールのインストール](#開発ツールのインストール)
3. [プロジェクトのセットアップ](#プロジェクトのセットアップ)
4. [開発環境の確認](#開発環境の確認)
5. [トラブルシューティング](#トラブルシューティング)

## 前提条件

### オペレーティングシステム
- Windows 10/11 (WSL2推奨)
- macOS 11以降
- Linux (Ubuntu 20.04以降推奨)

### ハードウェア要件
- RAM: 8GB以上（16GB推奨）
- ストレージ: 10GB以上の空き容量
- CPU: 64ビットプロセッサ

## 開発ツールのインストール

### 方法1: 自動インストールスクリプト（推奨）

```bash
# スクリプトに実行権限を付与
chmod +x scripts/install-dev-tools.sh

# スクリプトを実行
./scripts/install-dev-tools.sh
```

### 方法2: 手動インストール

#### 1. Node.js (v20以降)
```bash
# nvm経由でのインストール（推奨）
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc
nvm install 20
nvm use 20
```

#### 2. pnpm
```bash
# 公式インストーラー
curl -fsSL https://get.pnpm.io/install.sh | sh -

# または npm経由
npm install -g pnpm
```

##### Ops / CI Onboarding: corepack + pnpm 初期化
Node.js 20 以降は Corepack が同梱されているため、pnpm 実行前に Corepack 側で shim を有効化する。`cmd.exe` から実行することで Windows 環境でも確実に反映され、GitHub Actions/Nightly と同じバージョンが保証される。

```powershell
# Windows (PowerShell 経由で cmd を呼び出す)
cmd.exe /c "corepack enable pnpm"
cmd.exe /c "corepack pnpm --version"

# macOS / Linux
corepack enable pnpm
corepack pnpm --version
```

上記を実行したら、pnpm コマンドも Corepack 経由で呼び出す（例: `corepack pnpm install --frozen-lockfile`, `corepack pnpm vitest run …`）。Ops / CI Onboarding のチェックリストでは、`tmp/logs/*.log` に残す各種テストログと併せて本手順を完了済みであることを記録する。Windows の Nightly/Vitest 再現では `cmd.exe /c "corepack enable pnpm"` → `cmd.exe /c "corepack pnpm install --frozen-lockfile"` の結果が `scripts/test-docker.ps1` の前提チェック（Corepack shim＋`node_modules/.modules.yaml`）として扱われる。

#### 3. Rust & Cargo
```bash
# 公式インストーラー
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 安定版に設定
rustup default stable
```

#### 4. Tauri CLI
```bash
cargo install tauri-cli
```

#### 5. 追加ツール
```bash
# SQLx CLI（データベースマイグレーション用）
cargo install sqlx-cli --no-default-features --features native-tls,sqlite

# Git（未インストールの場合）
# Ubuntu/Debian
sudo apt update && sudo apt install git

# macOS
brew install git
```

## プロジェクトのセットアップ

### 1. リポジトリのクローン
```bash
git clone https://github.com/yourusername/kukuri.git
cd kukuri
```

### 2. 依存関係のインストール

#### Node.js依存関係（Corepack + pnpm）
Windows ホストでは `cmd.exe` 経由で Corepack を初期化してから pnpm を固定バージョンで実行する。
```powershell
cmd.exe /c "corepack enable pnpm"
cmd.exe /c "corepack pnpm install --frozen-lockfile"
```
macOS / Linux では同じ手順をシェルから直接実行する。
```bash
corepack enable pnpm
corepack pnpm install --frozen-lockfile
```
> `scripts/test-docker.ps1 all` / `ts` / `lint` などの Nightly 再現系コマンドは、上記初期化と `node_modules/.modules.yaml` の存在を確認してから処理を進める。未実施の場合はスクリプトがエラーで停止するため、Runbook やログに初期化済みであることを残しておく。

#### Rust依存関係（自動的にインストールされます）
```bash
cargo build
```

### 3. 環境変数の設定
```bash
# .env.localファイルを作成
cp .env.example .env.local

# 必要に応じて編集
vim .env.local
```

### 4. データベースのセットアップ
```bash
# SQLiteデータベースの初期化
cd src-tauri
sqlx database create
sqlx migrate run
cd ..
```

#### P2P ブートストラップの準備（手動検証時）
手動検証では Mainline DHT ブートストラップノードをローカルで起動する。
```bash
# Docker でブートストラップノードを起動
docker compose -f docker-compose.test.yml up -d p2p-bootstrap

# 既存ノード一覧を `cn p2p bootstrap` から適用する場合
cn p2p bootstrap --export-path tmp/p2p_bootstrap_nodes.json
```
検証後は `docker compose -f docker-compose.test.yml down --remove-orphans` で停止する。

## 開発環境の確認

### ツールのバージョン確認
```bash
# すべてのツールが正しくインストールされているか確認
node --version  # v20.0.0以上
pnpm --version  # 8.0.0以上
rustc --version # 1.70.0以上
cargo --version
cargo tauri --version
```

### 開発サーバーの起動
```bash
# Tauriアプリケーションの起動
pnpm tauri dev
```

正常に起動すると：
- Viteの開発サーバーが起動
- Rustのビルドが開始
- Tauriウィンドウが表示される

### ビルドテスト
```bash
# プロダクションビルド
pnpm tauri build
```

## IDE設定

### Visual Studio Code（推奨）
1. VSCodeを開く
2. 推奨拡張機能をインストール（自動的にプロンプトが表示される）
3. 設定は`.vscode/settings.json`に自動適用

### 推奨拡張機能
- Tauri
- rust-analyzer
- ESLint
- Prettier
- Tailwind CSS IntelliSense

## よく使うコマンド

```bash
# 開発
pnpm tauri dev          # 開発サーバー起動
pnpm dev                # フロントエンドのみ起動

# ビルド
pnpm tauri build        # プロダクションビルド
pnpm build              # フロントエンドのみビルド

# テスト
./scripts/test-docker.ps1 all   # Windows は必ず Docker 経由で実行（ts/rust/lint をまとめて実行）
./scripts/test-docker.ps1 ts    # Windows の TypeScript テスト
./scripts/test-docker.ps1 rust  # Windows の Rust テスト
pnpm test               # Linux/macOS/WSL2 でのフロントエンドテスト
cargo test              # Linux/macOS/WSL2 での Rust テスト
docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch
docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "${PWD}:/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli" # community-node テスト/ビルド（全OS既定）

# リント/フォーマット
pnpm lint               # ESLint実行
pnpm format             # Prettier実行
cargo fmt               # Rustフォーマット
cargo clippy            # Rust linter

# 依存関係
pnpm install            # Node.js依存関係インストール
pnpm update             # 依存関係の更新
cargo update            # Rust依存関係の更新
```

## トラブルシューティング

### pnpmが見つからない
Corepack 未初期化または PATH が更新されていない可能性がある。
```powershell
# Windows
cmd.exe /c "corepack enable pnpm"
cmd.exe /c "corepack pnpm --version"
```
```bash
# macOS / Linux
corepack enable pnpm
corepack pnpm --version

# パスを再読み込み（必要に応じて）
source ~/.bashrc
# または
source ~/.zshrc
```

### Rustコンパイルエラー
```bash
# ツールチェーンを更新
rustup update

# キャッシュをクリア
cargo clean
```

### Tauriビルドエラー
```bash
# 依存関係の再インストール
rm -rf node_modules pnpm-lock.yaml
pnpm install

# Rust依存関係の再ビルド
cd src-tauri
cargo clean
cargo build
```

### WebView2関連のエラー（Windows）
- Microsoft Edge WebView2をインストール
- https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Linux依存関係
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

## 追加リソース

- [Tauri公式ドキュメント](https://tauri.app/v1/guides/)
- [Rust公式ドキュメント](https://doc.rust-lang.org/book/)
- [React公式ドキュメント](https://react.dev/)
- [pnpm公式ドキュメント](https://pnpm.io/)

## サポート

問題が解決しない場合：
1. [GitHub Issues](https://github.com/yourusername/kukuri/issues)で既存の問題を検索
2. 新しいIssueを作成して詳細を報告
3. [Discord](https://discord.gg/kukuri)コミュニティで質問
