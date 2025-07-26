# Kukuri Integration Tests - PowerShell Script

# 色付き出力の定義
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

Write-ColorOutput Green "Starting Kukuri Integration Tests"

# 環境チェック
function Check-Requirements {
    Write-ColorOutput Yellow "Checking requirements..."
    
    # Node.jsチェック
    if (!(Get-Command node -ErrorAction SilentlyContinue)) {
        Write-ColorOutput Red "Node.js is not installed"
        exit 1
    }
    
    # pnpmチェック
    if (!(Get-Command pnpm -ErrorAction SilentlyContinue)) {
        Write-ColorOutput Red "pnpm is not installed"
        exit 1
    }
    
    # Rustチェック
    if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-ColorOutput Red "Rust is not installed"
        exit 1
    }
    
    Write-ColorOutput Green "All requirements met"
}

# 依存関係のインストール
function Install-Dependencies {
    Write-ColorOutput Yellow "Installing dependencies..."
    pnpm install
}

# ユニットテストの実行
function Run-UnitTests {
    Write-ColorOutput Yellow "Running unit tests..."
    
    # フロントエンドのテスト
    Write-ColorOutput Yellow "Running frontend tests..."
    pnpm test
    
    # Rustのテスト
    Write-ColorOutput Yellow "Running Rust tests..."
    Push-Location src-tauri
    cargo test --all-features
    Pop-Location
}

# インテグレーションテストの実行
function Run-IntegrationTests {
    Write-ColorOutput Yellow "Running integration tests..."
    
    # フロントエンドのインテグレーションテスト
    Write-ColorOutput Yellow "Running frontend integration tests..."
    pnpm test:integration
    
    # Rustのインテグレーションテスト
    Write-ColorOutput Yellow "Running Rust integration tests..."
    Push-Location src-tauri
    cargo test --test '*' -- --test-threads=1
    Pop-Location
}

# E2Eテストの実行
function Run-E2ETests {
    Write-ColorOutput Yellow "Running E2E tests..."
    
    # Tauri Driverのインストール確認
    if (!(Get-Command tauri-driver -ErrorAction SilentlyContinue)) {
        Write-ColorOutput Yellow "Installing tauri-driver..."
        cargo install tauri-driver
    }
    
    # アプリケーションのビルド
    Write-ColorOutput Yellow "Building application for E2E tests..."
    pnpm tauri build --debug
    
    # E2Eテストの実行
    pnpm test:e2e
}

# リントとフォーマットチェック
function Run-LintChecks {
    Write-ColorOutput Yellow "Running lint checks..."
    
    # ESLint
    Write-ColorOutput Yellow "Running ESLint..."
    pnpm lint
    
    # TypeScriptの型チェック
    Write-ColorOutput Yellow "Checking TypeScript types..."
    pnpm type-check
    
    # Rustのフォーマットチェック
    Write-ColorOutput Yellow "Checking Rust formatting..."
    Push-Location src-tauri
    cargo fmt -- --check
    
    # Clippy
    Write-ColorOutput Yellow "Running Clippy..."
    cargo clippy -- -D warnings
    Pop-Location
}

# メイン処理
$RunUnit = $true
$RunIntegration = $true
$RunE2E = $false
$RunLint = $true

# パラメータ処理
for ($i = 0; $i -lt $args.Count; $i++) {
    switch ($args[$i]) {
        "--unit-only" {
            $RunIntegration = $false
            $RunE2E = $false
            $RunLint = $false
        }
        "--integration-only" {
            $RunUnit = $false
            $RunE2E = $false
            $RunLint = $false
        }
        "--e2e-only" {
            $RunUnit = $false
            $RunIntegration = $false
            $RunLint = $false
            $RunE2E = $true
        }
        "--all" {
            $RunE2E = $true
        }
        "--no-lint" {
            $RunLint = $false
        }
        default {
            Write-ColorOutput Red "Unknown option: $($args[$i])"
            Write-Host "Usage: .\run-integration-tests.ps1 [--unit-only|--integration-only|--e2e-only|--all|--no-lint]"
            exit 1
        }
    }
}

try {
    Check-Requirements
    Install-Dependencies
    
    if ($RunLint) {
        Run-LintChecks
    }
    
    if ($RunUnit) {
        Run-UnitTests
    }
    
    if ($RunIntegration) {
        Run-IntegrationTests
    }
    
    if ($RunE2E) {
        Run-E2ETests
    }
    
    Write-ColorOutput Green "All tests completed successfully!"
}
catch {
    Write-ColorOutput Red "Test execution failed: $_"
    exit 1
}