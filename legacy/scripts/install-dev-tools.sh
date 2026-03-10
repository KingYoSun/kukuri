#!/bin/bash
# kukuri開発環境セットアップスクリプト

set -e

echo "======================================"
echo "kukuri開発環境セットアップスクリプト"
echo "======================================"
echo ""

# pnpmのインストール
echo "1. pnpmをインストールしています..."
if ! command -v pnpm &> /dev/null; then
    curl -fsSL https://get.pnpm.io/install.sh | sh -
    echo "✅ pnpmがインストールされました"
    echo "⚠️  新しいターミナルセッションを開くか、以下を実行してください:"
    echo "    source ~/.bashrc"
else
    echo "✅ pnpmは既にインストールされています ($(pnpm --version))"
fi

echo ""

# Rustのインストール
echo "2. Rust & Cargoをインストールしています..."
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "✅ Rust & Cargoがインストールされました"
else
    echo "✅ Rustは既にインストールされています ($(rustc --version))"
fi

echo ""

# Tauri CLIのインストール
echo "3. Tauri CLIをインストールしています..."
if ! command -v cargo-tauri &> /dev/null; then
    cargo install tauri-cli
    echo "✅ Tauri CLIがインストールされました"
else
    echo "✅ Tauri CLIは既にインストールされています"
fi

echo ""

# 追加の開発ツール
echo "4. 追加の開発ツールを確認しています..."

# sqlx-cli (データベースマイグレーション用)
if ! command -v sqlx &> /dev/null; then
    echo "   sqlx-cliをインストールしています..."
    cargo install sqlx-cli --no-default-features --features native-tls,sqlite
    echo "   ✅ sqlx-cliがインストールされました"
else
    echo "   ✅ sqlx-cliは既にインストールされています"
fi

echo ""
echo "======================================"
echo "セットアップが完了しました！"
echo "======================================"
echo ""
echo "インストールされたツール:"
echo "  - Node.js: $(node --version)"
echo "  - pnpm: $(pnpm --version 2>/dev/null || echo '新しいターミナルで確認してください')"
echo "  - Rust: $(rustc --version 2>/dev/null || echo '新しいターミナルで確認してください')"
echo "  - Cargo: $(cargo --version 2>/dev/null || echo '新しいターミナルで確認してください')"
echo ""
echo "次のステップ:"
echo "  1. 新しいターミナルを開いてパスを更新"
echo "  2. 'pnpm tauri dev' でTauriプロジェクトを初期化"