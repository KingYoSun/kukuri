# Docker環境でのテスト実行スクリプト (PowerShell版)

param(
    [Parameter(Position = 0)]
    [ValidateSet("all", "rust", "integration", "ts", "lint", "build", "clean", "cache-clean")]
    [string]$Command = "all",

    [switch]$NoBuild,  # ビルドをスキップするオプション
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

function Write-Info {
    param([string]$Message)
    Write-Host "ℹ $Message" -ForegroundColor Cyan
}

# ヘルプ表示
function Show-Help {
    Write-Host @"
Usage: .\test-docker.ps1 [Command] [Options]

Commands:
  all          - すべてのテストを実行（デフォルト）
  rust         - Rustのテストのみ実行
  ts           - TypeScriptのテストのみ実行
  lint         - リントとフォーマットチェックのみ実行
  build        - Dockerイメージのビルドのみ実行
  clean        - Dockerコンテナとイメージをクリーンアップ
  cache-clean  - キャッシュボリュームも含めて完全クリーンアップ

Options:
  -NoBuild     - Dockerイメージのビルドをスキップ
  -Help        - このヘルプを表示

Examples:
  .\test-docker.ps1                # すべてのテストを実行
  .\test-docker.ps1 rust           # Rustテストのみ実行
  .\test-docker.ps1 rust -NoBuild  # ビルドをスキップしてRustテストを実行
  .\test-docker.ps1 cache-clean    # キャッシュを含めて完全クリーンアップ
  .\test-docker.ps1 -Help          # ヘルプを表示

Performance Tips:
  - 初回実行時は依存関係のダウンロードのため時間がかかります
  - 2回目以降はDockerボリュームにキャッシュされるため高速になります
  - キャッシュをクリアしたい場合は 'cache-clean' コマンドを使用してください
"@
    exit 0
}

# Docker Buildkit を有効化
$env:DOCKER_BUILDKIT = "1"
$env:COMPOSE_DOCKER_CLI_BUILD = "1"

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

# Dockerイメージの存在確認
function Test-DockerImageExists {
    $image = docker images -q "kukuri_test-runner" 2>$null
    return ![string]::IsNullOrEmpty($image)
}

# Dockerイメージのビルド
function Build-TestImage {
    param([switch]$Force)
    
    if (-not $Force -and (Test-DockerImageExists)) {
        Write-Info "Docker image already exists. Use 'build' command to rebuild."
        return
    }
    
    Write-Host "Building Docker test image (with cache optimization)..."
    Invoke-DockerCompose @("build", "--build-arg", "DOCKER_BUILDKIT=1", "test-runner")
    Write-Success "Docker image built successfully"
}

# すべてのテストを実行
function Invoke-AllTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running all tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "test-runner")
    Write-Success "All tests passed!"
}

# Rustテストのみ実行
function Invoke-RustTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running Rust tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "rust-test")
    Write-Success "Rust tests passed!"
}

function Invoke-IntegrationTests {
    $previousValue = $env:ENABLE_P2P_INTEGRATION
    try {
        $env:ENABLE_P2P_INTEGRATION = "1"
        if (-not $NoBuild) {
            Build-TestImage
        }
        Write-Host "Running Rust P2P integration tests in Docker..."
        Invoke-DockerCompose @("run", "--rm", "rust-test")
        Write-Success "Rust P2P integration tests passed!"
    }
    finally {
        if ($null -eq $previousValue) {
            Remove-Item Env:ENABLE_P2P_INTEGRATION -ErrorAction SilentlyContinue
        } else {
            $env:ENABLE_P2P_INTEGRATION = $previousValue
        }
    }
}

# TypeScriptテストのみ実行
function Invoke-TypeScriptTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running TypeScript tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "ts-test")
    Write-Success "TypeScript tests passed!"
}

# リントとフォーマットチェック
function Invoke-LintCheck {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running lint and format checks in Docker..."
    Invoke-DockerCompose @("run", "--rm", "lint-check")
    Write-Success "Lint and format checks passed!"
}

# クリーンアップ
function Invoke-Cleanup {
    Write-Host "Cleaning up Docker containers and images..."
    Invoke-DockerCompose @("down", "--rmi", "local", "--remove-orphans")
    Write-Success "Cleanup completed"
}

# 完全クリーンアップ（キャッシュボリュームも削除）
function Invoke-CacheCleanup {
    Write-Host "Performing complete cleanup including cache volumes..."
    
    # コンテナとイメージの削除
    Invoke-DockerCompose @("down", "--rmi", "local", "--volumes", "--remove-orphans")
    
    # 名前付きボリュームの削除
    Write-Host "Removing cache volumes..."
    docker volume rm kukuri-cargo-registry kukuri-cargo-git kukuri-cargo-target kukuri-pnpm-store 2>$null
    
    Write-Success "Complete cleanup finished"
    Write-Info "Next build will take longer as all caches have been cleared"
}

# キャッシュ状況の表示
function Show-CacheStatus {
    Write-Host "`nCache Volume Status:" -ForegroundColor Yellow
    Write-Host "-------------------"
    
    $volumes = @("kukuri-cargo-registry", "kukuri-cargo-git", "kukuri-cargo-target", "kukuri-pnpm-store")
    foreach ($vol in $volumes) {
        $exists = docker volume ls --quiet --filter "name=$vol" 2>$null
        if ($exists) {
            $size = docker run --rm -v "${vol}:/data" alpine du -sh /data 2>$null | Select-Object -First 1
            Write-Host "  $vol : $size"
        } else {
            Write-Host "  $vol : Not created yet" -ForegroundColor Gray
        }
    }
    Write-Host ""
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
        Invoke-AllTests
        Show-CacheStatus
    }
    "rust" {
        Invoke-RustTests
        Show-CacheStatus
    }
    "integration" {
        Invoke-IntegrationTests
        Show-CacheStatus
    }
    "ts" {
        Invoke-TypeScriptTests
        Show-CacheStatus
    }
    "lint" {
        Invoke-LintCheck
        Show-CacheStatus
    }
    "build" {
        Build-TestImage -Force
        Show-CacheStatus
    }
    "clean" {
        Invoke-Cleanup
    }
    "cache-clean" {
        Invoke-CacheCleanup
    }
}

Write-Host "Done!" -ForegroundColor Green
