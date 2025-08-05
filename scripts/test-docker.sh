#!/bin/bash
# Docker環境でのテスト実行スクリプト

set -e

# カラー出力の定義
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 使用方法の表示
usage() {
    echo "Usage: $0 [options]"
    echo "Options:"
    echo "  all          - すべてのテストを実行（デフォルト）"
    echo "  rust         - Rustのテストのみ実行"
    echo "  ts           - TypeScriptのテストのみ実行"
    echo "  lint         - リントとフォーマットチェックのみ実行"
    echo "  build        - Dockerイメージのビルドのみ実行"
    echo "  clean        - Dockerコンテナとイメージをクリーンアップ"
    echo "  -h, --help   - このヘルプを表示"
    exit 0
}

# エラーハンドリング
error_exit() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

# 成功メッセージ
success_msg() {
    echo -e "${GREEN}✓ $1${NC}"
}

# 警告メッセージ
warning_msg() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Dockerイメージのビルド
build_image() {
    echo "Building Docker test image..."
    docker-compose -f docker-compose.test.yml build test-runner || error_exit "Failed to build Docker image"
    success_msg "Docker image built successfully"
}

# すべてのテストを実行
run_all_tests() {
    echo "Running all tests in Docker..."
    docker-compose -f docker-compose.test.yml run --rm test-runner || error_exit "Tests failed"
    success_msg "All tests passed!"
}

# Rustテストのみ実行
run_rust_tests() {
    echo "Running Rust tests in Docker..."
    docker-compose -f docker-compose.test.yml run --rm rust-test || error_exit "Rust tests failed"
    success_msg "Rust tests passed!"
}

# TypeScriptテストのみ実行
run_ts_tests() {
    echo "Running TypeScript tests in Docker..."
    docker-compose -f docker-compose.test.yml run --rm ts-test || error_exit "TypeScript tests failed"
    success_msg "TypeScript tests passed!"
}

# リントとフォーマットチェック
run_lint_check() {
    echo "Running lint and format checks in Docker..."
    docker-compose -f docker-compose.test.yml run --rm lint-check || error_exit "Lint/format checks failed"
    success_msg "Lint and format checks passed!"
}

# クリーンアップ
cleanup() {
    echo "Cleaning up Docker containers and images..."
    docker-compose -f docker-compose.test.yml down --rmi local --volumes --remove-orphans
    success_msg "Cleanup completed"
}

# テスト結果ディレクトリの作成
mkdir -p test-results

# メイン処理
case "${1:-all}" in
    all)
        build_image
        run_all_tests
        ;;
    rust)
        build_image
        run_rust_tests
        ;;
    ts)
        build_image
        run_ts_tests
        ;;
    lint)
        build_image
        run_lint_check
        ;;
    build)
        build_image
        ;;
    clean)
        cleanup
        ;;
    -h|--help)
        usage
        ;;
    *)
        error_exit "Unknown option: $1"
        ;;
esac