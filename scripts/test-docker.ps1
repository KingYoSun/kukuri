# Docker環境でのテスト実行スクリプト (PowerShell版)

param(
    [Parameter(Position = 0)]
    [ValidateSet("all", "rust", "integration", "ts", "lint", "build", "clean", "cache-clean", "metrics", "contracts")]
    [string]$Command = "all",

    [switch]$Integration,            # Rustテスト時にP2P統合テストのみを実行
    [string]$BootstrapPeers,         # 統合テスト用のブートストラップピア指定
    [string]$IrohBin,                # iroh バイナリのパス
    [string]$IntegrationLog = "info,iroh_tests=debug", # 統合テスト用のRUST_LOG

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
  integration  - P2P統合テスト（Rust）を実行
  ts           - TypeScriptのテストのみ実行
  lint         - リントとフォーマットチェックのみ実行
  metrics      - メトリクス関連のショートテスト（Rust test_get_status / TS P2P UI）
  contracts    - 契約テスト（NIP-10境界ケース）を実行
  build        - Dockerイメージのビルドのみ実行
  clean        - Dockerコンテナとイメージをクリーンアップ
  cache-clean  - キャッシュボリュームも含めて完全クリーンアップ

Options:
  -Integration  - Rustコマンドと併せて P2P 統合テストのみ実行
  -BootstrapPeers <node@host:port,...> - 統合テストで使用するブートストラップピアを指定
  -IrohBin <path> - iroh バイナリの明示パスを指定（Windows で DLL 解決が必要な場合など）
  -IntegrationLog <level> - 統合テスト時の RUST_LOG 設定（既定: info,iroh_tests=debug）
  -NoBuild     - Dockerイメージのビルドをスキップ
  -Help        - このヘルプを表示

Examples:
  .\test-docker.ps1                # すべてのテストを実行
  .\test-docker.ps1 rust           # Rustテストのみ実行
  .\test-docker.ps1 rust -Integration -BootstrapPeers "node@127.0.0.1:11233"
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

$BootstrapDefaultPeer = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233"
$BootstrapContainerName = "kukuri-p2p-bootstrap"

# Docker Composeコマンドの実行
function Invoke-DockerCompose {
    param(
        [string[]]$Arguments,
        [switch]$IgnoreFailure
    )

    & docker compose -f docker-compose.test.yml @Arguments 2>&1 | ForEach-Object { Write-Host $_ }
    $code = $LASTEXITCODE
    if (-not $IgnoreFailure -and $code -ne 0) {
        Write-ErrorMessage "Docker Compose command failed"
    }
    return [int]$code
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
    Invoke-DockerCompose @("run", "--rm", "test-runner", "/app/run-tests.sh")
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
    param(
        [string]$BootstrapPeersParam,
        [string]$IrohBinParam,
        [string]$LogLevel = "info,iroh_tests=debug"
    )

    $previousEnable = $env:ENABLE_P2P_INTEGRATION
    $previousBootstrap = $env:KUKURI_BOOTSTRAP_PEERS
    $previousForceLocal = $env:KUKURI_FORCE_LOCALHOST_ADDRS
    $previousIrohBin = $env:KUKURI_IROH_BIN
    $previousLog = $env:RUST_LOG
    $bootstrapStarted = $false
    $exitCode = 0
    $fatalMessage = $null
    try {
        $env:ENABLE_P2P_INTEGRATION = "1"
        if ([string]::IsNullOrWhiteSpace($BootstrapPeersParam)) {
            if ([string]::IsNullOrWhiteSpace($env:KUKURI_BOOTSTRAP_PEERS)) {
                $env:KUKURI_BOOTSTRAP_PEERS = $BootstrapDefaultPeer
                $bootstrapWasSet = $true
            }
        } else {
            $env:KUKURI_BOOTSTRAP_PEERS = $BootstrapPeersParam
            $bootstrapWasSet = $true
        }
        $env:KUKURI_FORCE_LOCALHOST_ADDRS = "0"
        if ($IrohBinParam) {
            $env:KUKURI_IROH_BIN = $IrohBinParam
        }
        if (-not [string]::IsNullOrWhiteSpace($LogLevel)) {
            $env:RUST_LOG = $LogLevel
        }
        if (-not $NoBuild) {
            Build-TestImage
        }
        Write-Info "Using bootstrap peers: $($env:KUKURI_BOOTSTRAP_PEERS)"
        if ($IrohBinParam) {
            Write-Info "Using custom iroh binary: $IrohBinParam"
        }
        Write-Info "RUST_LOG for integration: $($env:RUST_LOG)"
        Start-P2PBootstrap
        $bootstrapStarted = $true
        Write-Host "Running Rust P2P integration tests in Docker..."
        $exitCode = Invoke-DockerCompose @(
            "run", "--rm", "rust-test",
            "cargo", "test",
            "--package", "kukuri-tauri",
            "--lib", "modules::p2p::tests::iroh",
            "--", "--nocapture", "--test-threads=1"
        ) -IgnoreFailure
        if ($exitCode -eq 0) {
            Write-Success "Rust P2P integration tests passed!"
        } else {
            $fatalMessage = "Rust P2P integration tests exited with code $exitCode"
        }
    }
    catch {
        $fatalMessage = $_.Exception.Message
    }
    finally {
        if ($bootstrapStarted) {
            Stop-P2PBootstrap
        }
        if ($null -eq $previousEnable) {
            Remove-Item Env:ENABLE_P2P_INTEGRATION -ErrorAction SilentlyContinue
        } else {
            $env:ENABLE_P2P_INTEGRATION = $previousEnable
        }
        if ($null -eq $previousBootstrap) {
            Remove-Item Env:KUKURI_BOOTSTRAP_PEERS -ErrorAction SilentlyContinue
        } else {
            $env:KUKURI_BOOTSTRAP_PEERS = $previousBootstrap
        }
        if ($null -eq $previousForceLocal) {
            Remove-Item Env:KUKURI_FORCE_LOCALHOST_ADDRS -ErrorAction SilentlyContinue
        } else {
            $env:KUKURI_FORCE_LOCALHOST_ADDRS = $previousForceLocal
        }
        if ($null -eq $previousIrohBin) {
            Remove-Item Env:KUKURI_IROH_BIN -ErrorAction SilentlyContinue
        } else {
            $env:KUKURI_IROH_BIN = $previousIrohBin
        }
        if ($null -eq $previousLog) {
            Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue
        } else {
            $env:RUST_LOG = $previousLog
        }
        if ($fatalMessage) {
            Write-ErrorMessage $fatalMessage
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

function Invoke-MetricsTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running metrics-focused Rust test (test_get_status)..."
    Invoke-DockerCompose @("run", "--rm", "rust-test", "cargo", "test", "test_get_status")
    Write-Host "Running metrics-focused TypeScript tests (P2PStatus/store/useP2P)..."
    Invoke-DockerCompose @(
        "run",
        "--rm",
        "ts-test",
        "pnpm",
        "vitest",
        "run",
        "src/components/__tests__/P2PStatus.test.tsx",
        "src/stores/__tests__/p2pStore.test.ts",
        "src/hooks/__tests__/useP2P.test.tsx"
    )
    Write-Success "Metrics-focused tests passed!"
}

function Invoke-ContractTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running Rust contract tests (nip10_contract_tests)..."
    Invoke-DockerCompose @("run", "--rm", "rust-test", "cargo", "test", "--test", "nip10_contract_tests")
    Write-Host "Running TypeScript contract tests (nip10.contract)..."
    Invoke-DockerCompose @(
        "run",
        "--rm",
        "ts-test",
        "pnpm",
        "vitest",
        "run",
        "src/lib/__tests__/nip10.contract.test.ts"
    )
    Write-Success "Contract tests passed!"
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

function Wait-BootstrapHealthy {
    param(
        [int]$TimeoutSeconds = 60
    )

    for ($i = 0; $i -lt $TimeoutSeconds; $i++) {
        $status = ""
        try {
            $status = docker inspect --format '{{.State.Health.Status}}' $BootstrapContainerName 2>$null
        } catch {
            $status = ""
        }

        if ($status -eq "healthy") {
            return $true
        }
        Start-Sleep -Seconds 1
    }
    return $false
}

function Start-P2PBootstrap {
    Write-Info "Starting p2p-bootstrap container..."
    $code = Invoke-DockerCompose @("up", "-d", "p2p-bootstrap") -IgnoreFailure
    if ($code -ne 0) {
        throw "Failed to start p2p-bootstrap (exit code $code)"
    }
    if (-not (Wait-BootstrapHealthy)) {
        throw "p2p-bootstrap health check failed"
    }
    Write-Success "p2p-bootstrap is healthy"
}

function Stop-P2PBootstrap {
    Invoke-DockerCompose @("down", "--remove-orphans") -IgnoreFailure | Out-Null
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
        if ($Integration) {
            Invoke-IntegrationTests -BootstrapPeersParam $BootstrapPeers -IrohBinParam $IrohBin -LogLevel $IntegrationLog
        } else {
            Invoke-RustTests
        }
        Show-CacheStatus
    }
    "integration" {
        Invoke-IntegrationTests -BootstrapPeersParam $BootstrapPeers -IrohBinParam $IrohBin -LogLevel $IntegrationLog
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
    "metrics" {
        Invoke-MetricsTests
        Show-CacheStatus
    }
    "contracts" {
        Invoke-ContractTests
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
