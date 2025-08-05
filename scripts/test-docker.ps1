# Docker環境でのテスト実行スクリプト (PowerShell版)

param(
    [Parameter(Position = 0)]
    [ValidateSet("all", "rust", "ts", "lint", "build", "clean")]
    [string]$Command = "all",

    [switch]$Help
)

# カラー関数
function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-ErrorMessage {
    param([string]$Message)
    Write-Host "Error: $Message" -ForegroundColor Red
    exit 1
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

# ヘルプ表示
function Show-Help {
    Write-Host @"
Usage: .\test-docker.ps1 [Command]

Commands:
  all          - すべてのテストを実行（デフォルト）
  rust         - Rustのテストのみ実行
  ts           - TypeScriptのテストのみ実行
  lint         - リントとフォーマットチェックのみ実行
  build        - Dockerイメージのビルドのみ実行
  clean        - Dockerコンテナとイメージをクリーンアップ

Options:
  -Help        - このヘルプを表示

Examples:
  .\test-docker.ps1                # すべてのテストを実行
  .\test-docker.ps1 rust           # Rustテストのみ実行
  .\test-docker.ps1 -Help          # ヘルプを表示
"@
    exit 0
}

# Docker Composeコマンドの実行
function Invoke-DockerCompose {
    param(
        [string[]]$Arguments
    )

    & docker-compose -f docker-compose.test.yml @Arguments
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorMessage "Docker Compose command failed"
    }
}

# Dockerイメージのビルド
function Build-TestImage {
    Write-Host "Building Docker test image..."
    Invoke-DockerCompose @("build", "test-runner")
    Write-Success "Docker image built successfully"
}

# すべてのテストを実行
function Invoke-AllTests {
    Write-Host "Running all tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "test-runner")
    Write-Success "All tests passed!"
}

# Rustテストのみ実行
function Invoke-RustTests {
    Write-Host "Running Rust tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "rust-test")
    Write-Success "Rust tests passed!"
}

# TypeScriptテストのみ実行
function Invoke-TypeScriptTests {
    Write-Host "Running TypeScript tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "ts-test")
    Write-Success "TypeScript tests passed!"
}

# リントとフォーマットチェック
function Invoke-LintCheck {
    Write-Host "Running lint and format checks in Docker..."
    Invoke-DockerCompose @("run", "--rm", "lint-check")
    Write-Success "Lint and format checks passed!"
}

# クリーンアップ
function Invoke-Cleanup {
    Write-Host "Cleaning up Docker containers and images..."
    Invoke-DockerCompose @("down", "--rmi", "local", "--volumes", "--remove-orphans")
    Write-Success "Cleanup completed"
}

# メイン処理
if ($Help) {
    Show-Help
}

# テスト結果ディレクトリの作成
if (-not (Test-Path "test-results")) {
    New-Item -ItemType Directory -Path "test-results" | Out-Null
}

# コマンドの実行
switch ($Command) {
    "all" {
        Build-TestImage
        Invoke-AllTests
    }
    "rust" {
        Build-TestImage
        Invoke-RustTests
    }
    "ts" {
        Build-TestImage
        Invoke-TypeScriptTests
    }
    "lint" {
        Build-TestImage
        Invoke-LintCheck
    }
    "build" {
        Build-TestImage
    }
    "clean" {
        Invoke-Cleanup
    }
}

Write-Host "Done!" -ForegroundColor Green
