#!/bin/bash

# 色付き出力の定義
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# エラーハンドリング
set -e

echo -e "${GREEN}Starting Kukuri Integration Tests${NC}"

# 環境チェック
check_requirements() {
    echo -e "${YELLOW}Checking requirements...${NC}"
    
    # Node.jsチェック
    if ! command -v node &> /dev/null; then
        echo -e "${RED}Node.js is not installed${NC}"
        exit 1
    fi
    
    # pnpmチェック
    if ! command -v pnpm &> /dev/null; then
        echo -e "${RED}pnpm is not installed${NC}"
        exit 1
    fi
    
    # Rustチェック
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Rust is not installed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}All requirements met${NC}"
}

# 依存関係のインストール
install_deps() {
    echo -e "${YELLOW}Installing dependencies...${NC}"
    pnpm install
}

# ユニットテストの実行
run_unit_tests() {
    echo -e "${YELLOW}Running unit tests...${NC}"
    
    # フロントエンドのテスト
    echo -e "${YELLOW}Running frontend tests...${NC}"
    pnpm test
    
    # Rustのテスト
    echo -e "${YELLOW}Running Rust tests...${NC}"
    cd src-tauri
    cargo test --all-features
    cd ..
}

# インテグレーションテストの実行
run_integration_tests() {
    echo -e "${YELLOW}Running integration tests...${NC}"
    
    # フロントエンドのインテグレーションテスト
    echo -e "${YELLOW}Running frontend integration tests...${NC}"
    pnpm test:integration
    
    # Rustのインテグレーションテスト
    echo -e "${YELLOW}Running Rust integration tests...${NC}"
    cd src-tauri
    cargo test --test '*' -- --test-threads=1
    cd ..
}

# E2Eテストの実行
run_e2e_tests() {
    echo -e "${YELLOW}Running E2E tests...${NC}"
    
    # Tauri Driverのインストール確認
    if ! command -v tauri-driver &> /dev/null; then
        echo -e "${YELLOW}Installing tauri-driver...${NC}"
        cargo install tauri-driver
    fi
    
    # アプリケーションのビルド
    echo -e "${YELLOW}Building application for E2E tests...${NC}"
    pnpm tauri build --debug
    
    # E2Eテストの実行
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linuxの場合はxvfbを使用
        if command -v xvfb-run &> /dev/null; then
            xvfb-run -a pnpm test:e2e
        else
            echo -e "${YELLOW}xvfb not found, running without virtual display${NC}"
            pnpm test:e2e
        fi
    else
        pnpm test:e2e
    fi
}

# リントとフォーマットチェック
run_lint_checks() {
    echo -e "${YELLOW}Running lint checks...${NC}"
    
    # ESLint
    echo -e "${YELLOW}Running ESLint...${NC}"
    pnpm lint
    
    # TypeScriptの型チェック
    echo -e "${YELLOW}Checking TypeScript types...${NC}"
    pnpm type-check
    
    # Rustのフォーマットチェック
    echo -e "${YELLOW}Checking Rust formatting...${NC}"
    cd src-tauri
    cargo fmt -- --check
    
    # Clippy
    echo -e "${YELLOW}Running Clippy...${NC}"
    cargo clippy -- -D warnings
    cd ..
}

# メイン処理
main() {
    # オプション解析
    RUN_UNIT=true
    RUN_INTEGRATION=true
    RUN_E2E=false
    RUN_LINT=true
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --unit-only)
                RUN_INTEGRATION=false
                RUN_E2E=false
                RUN_LINT=false
                shift
                ;;
            --integration-only)
                RUN_UNIT=false
                RUN_E2E=false
                RUN_LINT=false
                shift
                ;;
            --e2e-only)
                RUN_UNIT=false
                RUN_INTEGRATION=false
                RUN_LINT=false
                RUN_E2E=true
                shift
                ;;
            --all)
                RUN_E2E=true
                shift
                ;;
            --no-lint)
                RUN_LINT=false
                shift
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                echo "Usage: $0 [--unit-only|--integration-only|--e2e-only|--all|--no-lint]"
                exit 1
                ;;
        esac
    done
    
    check_requirements
    install_deps
    
    if [ "$RUN_LINT" = true ]; then
        run_lint_checks
    fi
    
    if [ "$RUN_UNIT" = true ]; then
        run_unit_tests
    fi
    
    if [ "$RUN_INTEGRATION" = true ]; then
        run_integration_tests
    fi
    
    if [ "$RUN_E2E" = true ]; then
        run_e2e_tests
    fi
    
    echo -e "${GREEN}All tests completed successfully!${NC}"
}

# スクリプトの実行
main "$@"