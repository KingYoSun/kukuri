# Docker環境でのテスト実行スクリプト (PowerShell版)

param(
    [Parameter(Position = 0)]
    [ValidateSet("all", "rust", "integration", "ts", "lint", "coverage", "build", "clean", "cache-clean", "metrics", "performance", "contracts")]
    [string]$Command = "all",

    [switch]$Integration,            # Rustテスト時にP2P統合テストのみを実行
    [Alias("Test", "tests")]
    [string]$TestTarget,             # Rustテスト時に特定バイナリのみ実行
    [string]$Scenario,               # TypeScriptテスト用のシナリオ指定
    [string]$Fixture,                # シナリオ用フィクスチャパス
    [switch]$ServiceWorker,          # profile-avatar-sync シナリオで Service Worker 拡張を実行
    [string]$BootstrapPeers,         # 統合テスト用のブートストラップピア指定
    [string]$IrohBin,                # iroh バイナリのパス
    [string]$IntegrationLog = "info,iroh_tests=debug", # 統合テスト用のRUST_LOG

    [switch]$NoBuild,  # ビルドをスキップするオプション
    [switch]$Help
)

$scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = Split-Path $scriptDirectory -Parent
$NewBinMainlineTarget = if (![string]::IsNullOrWhiteSpace($env:P2P_MAINLINE_TEST_TARGET)) { $env:P2P_MAINLINE_TEST_TARGET } else { "p2p_mainline_smoke" }
$NewBinGossipTarget = if (![string]::IsNullOrWhiteSpace($env:P2P_GOSSIP_TEST_TARGET)) { $env:P2P_GOSSIP_TEST_TARGET } else { "p2p_gossip_smoke" }
$PrometheusMetricsUrl = if (![string]::IsNullOrWhiteSpace($env:PROMETHEUS_METRICS_URL)) { $env:PROMETHEUS_METRICS_URL } else { "http://127.0.0.1:9898/metrics" }

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

if ($Integration -and $TestTarget) {
    Write-ErrorMessage "-Integration と -Test は同時には指定できません。"
}

if ($TestTarget -and $Command -ne "rust") {
    Write-ErrorMessage "-Test は rust コマンドのみに指定できます。"
}

if ($Scenario -and $Command -ne "ts") {
    Write-ErrorMessage "-Scenario は ts コマンドでのみ使用できます。"
}

if ($Fixture -and $Command -ne "ts") {
    Write-ErrorMessage "-Fixture は ts コマンドでのみ使用できます。"
}

if ($ServiceWorker -and $Command -ne "ts") {
    Write-ErrorMessage "-ServiceWorker は ts コマンドでのみ使用できます。"
}

# ヘルプ表示
function Show-Help {
    Write-Host @"
Usage: .\test-docker.ps1 [Command] [Options]

Commands:
  all          - すべてのテストを実行（デフォルト）
  rust         - Rustのテストのみ実行
  integration  - P2P統合テスト（Rust）を実行
  ts           - TypeScriptのテストのみ実行（-Scenario でシナリオ指定可）
  lint         - リントとフォーマットチェックのみ実行
  coverage     - Rustカバレッジ（cargo tarpaulin）を実行し成果物を保存
  metrics      - メトリクス関連のショートテスト（Rust test_get_status / TS P2P UI）
  performance  - パフォーマンスハーネス（Rust ignored テスト）を実行し成果物を生成
  contracts    - 契約テスト（NIP-10境界ケース）を実行
  build        - Dockerイメージのビルドのみ実行
  clean        - Dockerコンテナとイメージをクリーンアップ
  cache-clean  - キャッシュボリュームも含めて完全クリーンアップ

Options:
  -Integration  - Rustコマンドと併せて P2P 統合テストのみ実行
  -Test <target> - Rustコマンド時に指定テストバイナリのみ実行（例: event_manager_integration）
  -Scenario <name> - TypeScriptテスト時にシナリオを指定（例: trending-feed, profile-avatar-sync, user-search-pagination, topic-create, post-delete-cache, offline-sync）
  -Fixture <path>  - シナリオ用フィクスチャパスを上書き（既定: tests/fixtures/trending/default.json）
  -ServiceWorker   - `ts -Scenario profile-avatar-sync` 実行時に Service Worker 拡張テストと Stage4 ログを有効化
  -BootstrapPeers <node@host:port,...> - 統合テストで使用するブートストラップピアを指定
  -IrohBin <path> - iroh バイナリの明示パスを指定（Windows で DLL 解決が必要な場合など）
  -IntegrationLog <level> - 統合テスト時の RUST_LOG 設定（既定: info,iroh_tests=debug）
  -NoBuild     - Dockerイメージのビルドをスキップ
  -Help        - このヘルプを表示
  ※ P2P統合テストは `p2p_gossip_smoke` / `p2p_mainline_smoke` を順次実行します。`P2P_GOSSIP_TEST_TARGET` や `P2P_MAINLINE_TEST_TARGET` で任意のターゲットに上書き可能です。

Examples:
  .\test-docker.ps1                # すべてのテストを実行
  .\test-docker.ps1 rust           # Rustテストのみ実行
  .\test-docker.ps1 rust -Test event_manager_integration
  .\test-docker.ps1 rust -Integration -BootstrapPeers "node@127.0.0.1:11233"
  .\test-docker.ps1 rust -NoBuild  # ビルドをスキップしてRustテストを実行
  .\test-docker.ps1 ts -Scenario trending-feed
  .\test-docker.ps1 ts -Scenario profile-avatar-sync
  .\test-docker.ps1 ts -Scenario user-search-pagination
  .\test-docker.ps1 performance    # パフォーマンス計測用テストバイナリを実行
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

function Invoke-RustTestTarget {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Target
    )

    if (-not $NoBuild) {
        Build-TestImage
    }

    Write-Host "Running Rust test target '$Target' in Docker..."
    $cargoArgs = @(
        "run",
        "--rm",
        "rust-test",
        "cargo",
        "test",
        "--package",
        "kukuri-tauri",
        "--test",
        $Target,
        "--",
        "--nocapture",
        "--test-threads=1"
    )
    $exitCode = Invoke-DockerCompose $cargoArgs -IgnoreFailure
    if ($exitCode -ne 0) {
        Write-ErrorMessage "Rust test target '$Target' failed (exit code $exitCode)"
    }
    Write-Success "Rust test target '$Target' passed!"
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
        $targets = @(
            @{
                Label = "Rust P2P gossip integration tests"
                Binary = $NewBinGossipTarget
            },
            @{
                Label = "Rust P2P mainline integration tests"
                Binary = $NewBinMainlineTarget
            }
        )

        foreach ($target in $targets) {
            $cargoArgs = @(
                "run",
                "--rm",
                "rust-test",
                "cargo",
                "test",
                "--package",
                "kukuri-tauri",
                "--test",
                $target.Binary,
                "--",
                "--nocapture",
                "--test-threads=1"
            )
            Write-Host "Running $($target.Label) (cargo test target: $($target.Binary))..."
            $exitCode = Invoke-DockerCompose $cargoArgs -IgnoreFailure
            if ($exitCode -ne 0) {
                $fatalMessage = "$($target.Label) exited with code $exitCode (cargo test target: $($target.Binary))"
                break
            }
        }

        if (-not $fatalMessage) {
            Write-Success "Rust P2P integration tests passed!"
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
function Start-PrometheusTrending {
    Write-Host "Starting prometheus-trending service (host network)..."
    $code = Invoke-DockerCompose -Arguments @("up", "-d", "prometheus-trending") -IgnoreFailure
    if ($code -ne 0) {
        Write-Warning "Failed to start prometheus-trending. Metrics scraping will be skipped."
        return $false
    }
    Start-Sleep -Seconds 2
    return $true
}

function Stop-PrometheusTrending {
    Write-Host "Stopping prometheus-trending service..."
    Invoke-DockerCompose -Arguments @("rm", "-sf", "prometheus-trending") -IgnoreFailure | Out-Null
}

function Collect-TrendingMetricsSnapshot {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Timestamp,
        [string]$RunState = "active"
    )

    $logRelPath = "tmp/logs/trending_metrics_job_stage4_$Timestamp.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }

    $header = @(
        "=== trending_metrics_job Prometheus snapshot ===",
        "timestamp: $(Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")",
        "endpoint: $PrometheusMetricsUrl",
        "run_state: $RunState",
        ""
    )
    Set-Content -Path $logHostPath -Value $header -Encoding UTF8

    try {
        $response = Invoke-WebRequest -Uri $PrometheusMetricsUrl -TimeoutSec 10 -UseBasicParsing
        Add-Content -Path $logHostPath -Value $response.Content -Encoding UTF8
    }
    catch {
        Add-Content -Path $logHostPath -Value "[WARN] Failed to fetch metrics: $_" -Encoding UTF8
    }

    Add-Content -Path $logHostPath -Value "" -Encoding UTF8
    Add-Content -Path $logHostPath -Value "--- prometheus-trending logs (tail -n 200) ---" -Encoding UTF8

    Push-Location $repositoryRoot
    try {
        $logOutput = & docker compose -f docker-compose.test.yml logs --tail 200 prometheus-trending 2>&1
        if ($LASTEXITCODE -eq 0) {
            Add-Content -Path $logHostPath -Value $logOutput -Encoding UTF8
        }
        else {
            Add-Content -Path $logHostPath -Value "[WARN] Failed to collect prometheus-trending logs: $logOutput" -Encoding UTF8
        }
    }
    finally {
        Pop-Location
    }

    $promResultsDir = Join-Path $repositoryRoot "test-results/trending-feed/prometheus"
    if (-not (Test-Path $promResultsDir)) {
        New-Item -ItemType Directory -Path $promResultsDir | Out-Null
    }
    $promArchivePath = Join-Path $promResultsDir "trending_metrics_job_stage4_$Timestamp.log"
    Copy-Item -Path $logHostPath -Destination $promArchivePath -Force
    Write-Info "Prometheus metrics log copied to test-results/trending-feed/prometheus/trending_metrics_job_stage4_$Timestamp.log"

    Write-Success "Prometheus metrics log saved to $logRelPath"
}

function Invoke-TypeScriptTrendingFeedScenario {
    $fixturePath = if ($Fixture) {
        $Fixture
    } elseif (-not [string]::IsNullOrWhiteSpace($env:VITE_TRENDING_FIXTURE_PATH)) {
        $env:VITE_TRENDING_FIXTURE_PATH
    } else {
        "tests/fixtures/trending/default.json"
    }

    $scenarioDir = Join-Path $repositoryRoot "test-results/trending-feed"
    if (-not (Test-Path $scenarioDir)) {
        New-Item -ItemType Directory -Path $scenarioDir | Out-Null
    }

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $reportRelPath = "test-results/trending-feed/$timestamp-vitest.json"
    $reportContainerPath = "/app/$reportRelPath"

    $promStarted = Start-PrometheusTrending

    Write-Host "Running TypeScript scenario 'trending-feed' (fixture: $fixturePath)..."
    $args = @(
        "run", "--rm",
        "-e", "VITE_TRENDING_FIXTURE_PATH=$fixturePath",
        "ts-test",
        "pnpm", "vitest", "run",
        "src/tests/unit/routes/trending.test.tsx",
        "src/tests/unit/routes/following.test.tsx",
        "src/tests/unit/hooks/useTrendingFeeds.test.tsx",
        "--reporter=default",
        "--reporter=json",
        "--outputFile=$reportContainerPath"
    )

    try {
        Invoke-DockerCompose $args | Out-Null
    }
    finally {
        if ($promStarted) {
            Collect-TrendingMetricsSnapshot -Timestamp $timestamp -RunState "active"
            Stop-PrometheusTrending
        } else {
            Collect-TrendingMetricsSnapshot -Timestamp $timestamp -RunState "skipped"
        }
    }

    $reportHostPath = Join-Path $repositoryRoot $reportRelPath
    if (Test-Path $reportHostPath) {
        Write-Success "Scenario report saved to $reportRelPath"
    } else {
        Write-Warning "Scenario report not found at $reportRelPath"
    }
}

function Invoke-TypeScriptProfileAvatarScenario {
    param(
        [switch]$IncludeServiceWorker
    )

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    if ($IncludeServiceWorker) {
        $logRelPath = "tmp/logs/profile_avatar_sync_stage4_$timestamp.log"
    } else {
        $logRelPath = "tmp/logs/profile_avatar_sync_$timestamp.log"
    }
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }

    $workerTestBlock = if ($IncludeServiceWorker) {
        "  'src/tests/unit/workers/profileAvatarSyncWorker.test.ts' \"
    } else {
        ""
    }

    Write-Host ("Running TypeScript scenario 'profile-avatar-sync'{0}..." -f ($(if ($IncludeServiceWorker) { " (Service Worker)" } else { "" })))
    $command = @"
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run \
  'src/tests/unit/components/settings/ProfileEditDialog.test.tsx' \
  'src/tests/unit/components/auth/ProfileSetup.test.tsx' \
  'src/tests/unit/hooks/useProfileAvatarSync.test.tsx' \
${workerTestBlock}
  | tee '/app/$logRelPath'
"@

    Invoke-DockerCompose @("run", "--rm", "ts-test", "bash", "-lc", $command) | Out-Null
    if (Test-Path $logHostPath) {
        Write-Success "Scenario log saved to $logRelPath"
    } else {
        Write-Warning "Scenario log was not generated at $logRelPath"
    }
}

function Invoke-TypeScriptUserSearchScenario {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $logRelPath = "tmp/logs/user_search_pagination_$timestamp.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
    Set-Content -Path $logHostPath -Value @()

    $resultsDir = Join-Path $repositoryRoot "test-results/user-search-pagination"
    if (-not (Test-Path $resultsDir)) {
        New-Item -ItemType Directory -Path $resultsDir | Out-Null
    }

    Write-Host "Running TypeScript scenario 'user-search-pagination'..."
    $vitestTargets = @(
        "src/tests/unit/hooks/useUserSearchQuery.test.tsx",
        "src/tests/unit/components/search/UserSearchResults.test.tsx"
    )

    $vitestStatus = 0
    foreach ($target in $vitestTargets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/user-search-pagination/${timestamp}-${slug}.json"

        $command = @"
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '$target' --reporter=default --reporter=json --outputFile '/app/$reportRelPath'
"@

        $dockerArgs = @("compose", "-f", "docker-compose.test.yml", "run", "--rm", "ts-test", "bash", "-lc", $command)
        & docker @dockerArgs 2>&1 | Tee-Object -FilePath $logHostPath -Append | Out-Null
        $exitCode = $LASTEXITCODE

        if ($exitCode -ne 0) {
            $vitestStatus = $exitCode
            Write-Warning "Vitest target $target failed with exit code $exitCode"
            break
        }

        $reportHostPath = Join-Path $repositoryRoot $reportRelPath
        if (Test-Path $reportHostPath) {
            Write-Success "Scenario report saved to $reportRelPath"
        } else {
            Write-Warning "Scenario report not found at $reportRelPath"
        }
    }

    if ($vitestStatus -ne 0) {
        Write-ErrorMessage "Scenario 'user-search-pagination' failed. See $logRelPath for details."
    } else {
        Write-Success "Scenario log saved to $logRelPath"
    }
}

function Invoke-TypeScriptPostDeleteCacheScenario {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $logRelPath = "tmp/logs/post_delete_cache_$timestamp.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
    Set-Content -Path $logHostPath -Value @()

    $resultsDir = Join-Path $repositoryRoot "test-results/post-delete-cache"
    if (-not (Test-Path $resultsDir)) {
        New-Item -ItemType Directory -Path $resultsDir | Out-Null
    }

    Write-Host "Running TypeScript scenario 'post-delete-cache'..."
    $vitestTargets = @(
        "src/tests/unit/hooks/useDeletePost.test.tsx",
        "src/tests/unit/components/posts/PostCard.test.tsx"
    )

    $vitestStatus = 0
    foreach ($target in $vitestTargets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/post-delete-cache/${timestamp}-${slug}.json"

        $command = @"
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '$target' --reporter=default --reporter=json --outputFile '/app/$reportRelPath'
"@

        $dockerArgs = @("compose", "-f", "docker-compose.test.yml", "run", "--rm", "ts-test", "bash", "-lc", $command)
        & docker @dockerArgs 2>&1 | Tee-Object -FilePath $logHostPath -Append | Out-Null
        $exitCode = $LASTEXITCODE

        if ($exitCode -ne 0) {
            $vitestStatus = $exitCode
            Write-Warning "Vitest target $target failed with exit code $exitCode"
            break
        }

        $reportHostPath = Join-Path $repositoryRoot $reportRelPath
        if (Test-Path $reportHostPath) {
            Write-Success "Scenario report saved to $reportRelPath"
        } else {
            Write-Warning "Scenario report not found at $reportRelPath"
        }
    }

    if ($vitestStatus -ne 0) {
        Write-ErrorMessage "Scenario 'post-delete-cache' failed. See $logRelPath for details."
    } else {
        Write-Success "Scenario log saved to $logRelPath"
    }
}

function Invoke-TypeScriptTopicCreateScenario {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $logRelPath = "tmp/logs/topic_create_$timestamp.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
    Set-Content -Path $logHostPath -Value @()

    $resultsDir = Join-Path $repositoryRoot "test-results/topic-create"
    if (-not (Test-Path $resultsDir)) {
        New-Item -ItemType Directory -Path $resultsDir | Out-Null
    }

    Write-Host "Running TypeScript scenario 'topic-create'..."
    $vitestTargets = @(
        "src/tests/unit/components/topics/TopicSelector.test.tsx",
        "src/tests/unit/components/posts/PostComposer.test.tsx",
        "src/tests/unit/components/layout/Sidebar.test.tsx"
    )

    $vitestStatus = 0
    foreach ($target in $vitestTargets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/topic-create/${timestamp}-${slug}.json"

        $command = @"
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '$target' --reporter=default --reporter=json --outputFile '/app/$reportRelPath'
"@

        $dockerArgs = @("compose", "-f", "docker-compose.test.yml", "run", "--rm", "ts-test", "bash", "-lc", $command)
        & docker @dockerArgs 2>&1 | Tee-Object -FilePath $logHostPath -Append | Out-Null
        $exitCode = $LASTEXITCODE

        if ($exitCode -ne 0) {
            $vitestStatus = $exitCode
            Write-Warning "Vitest target $target failed with exit code $exitCode"
            break
        }

        $reportHostPath = Join-Path $repositoryRoot $reportRelPath
        if (Test-Path $reportHostPath) {
            Write-Success "Scenario report saved to $reportRelPath"
        } else {
            Write-Warning "Scenario report not found at $reportRelPath"
        }
    }

    if ($vitestStatus -ne 0) {
        Write-ErrorMessage "Scenario 'topic-create' failed. See $logRelPath for details."
    } else {
        Write-Success "Scenario log saved to $logRelPath"
    }
}

function Invoke-TypeScriptOfflineSyncScenario {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $logRelPath = "tmp/logs/sync_status_indicator_stage4_$timestamp.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }

    Write-Host "Running TypeScript scenario 'offline-sync'..."
    $command = @"
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run \
  'src/tests/unit/hooks/useSyncManager.test.tsx' \
  'src/tests/unit/components/SyncStatusIndicator.test.tsx' \
  'src/tests/unit/components/OfflineIndicator.test.tsx' \
  | tee '/app/$logRelPath'
"@

    Invoke-DockerCompose @("run", "--rm", "ts-test", "bash", "-lc", $command) | Out-Null
    if (Test-Path $logHostPath) {
        Write-Success "Scenario log saved to $logRelPath"
    } else {
        Write-Warning "Scenario log was not generated at $logRelPath"
    }
}

function Invoke-TypeScriptTests {
    if (-not $NoBuild) {
        Build-TestImage
    }

    if ([string]::IsNullOrWhiteSpace($Scenario)) {
        Write-Host "Running TypeScript tests in Docker..."
        Invoke-DockerCompose @("run", "--rm", "ts-test")
        Write-Success "TypeScript tests passed!"
    } else {
        switch ($Scenario.ToLower()) {
            "trending-feed" {
                Invoke-TypeScriptTrendingFeedScenario
            }
        "profile-avatar-sync" {
            Invoke-TypeScriptProfileAvatarScenario -IncludeServiceWorker:$ServiceWorker
        }
            "user-search-pagination" {
                Invoke-TypeScriptUserSearchScenario
            }
            "post-delete-cache" {
                Invoke-TypeScriptPostDeleteCacheScenario
            }
            "topic-create" {
                Invoke-TypeScriptTopicCreateScenario
            }
            "offline-sync" {
                Invoke-TypeScriptOfflineSyncScenario
            }
            default {
                Write-ErrorMessage "Unknown TypeScript scenario: $Scenario"
            }
        }
    }
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

function Invoke-RustCoverage {
    if (-not $NoBuild) {
        Build-TestImage
    }

    $tmpDir = Join-Path $repositoryRoot "test-results/tarpaulin"
    $artefactDir = Join-Path $repositoryRoot "docs/01_project/activeContext/artefacts/metrics"
    if (-not (Test-Path $tmpDir)) {
        New-Item -ItemType Directory -Path $tmpDir | Out-Null
    } else {
        Get-ChildItem -Path $tmpDir -Recurse -Force | Remove-Item -Force
    }
    if (-not (Test-Path $artefactDir)) {
        New-Item -ItemType Directory -Path $artefactDir | Out-Null
    }

    Write-Host "Running cargo tarpaulin (Rust coverage) in Docker..."
    Invoke-DockerCompose @("run", "--rm", "rust-coverage")

    $timestamp = Get-Date -Format "yyyy-MM-dd-HHmmss"
    $jsonSrc = Join-Path $tmpDir "tarpaulin-report.json"
    $lcovSrc = Join-Path $tmpDir "tarpaulin-report.lcov"
    if (Test-Path $jsonSrc) {
        $jsonDest = Join-Path $artefactDir "$timestamp-tarpaulin.json"
        Copy-Item $jsonSrc $jsonDest -Force
        Write-Success "Coverage JSON saved: $jsonDest"
    } else {
        Write-Warning "tarpaulin-report.json was not generated"
    }

    $lcovCandidate = $null
    if (Test-Path $lcovSrc) {
        $lcovCandidate = $lcovSrc
    } else {
        $altLcov = Join-Path $tmpDir "lcov.info"
        if (Test-Path $altLcov) {
            $lcovCandidate = $altLcov
        }
    }

    if ($lcovCandidate) {
        $lcovDest = Join-Path $artefactDir "$timestamp-tarpaulin.lcov"
        Copy-Item $lcovCandidate $lcovDest -Force
        Write-Success "Coverage LCOV saved: $lcovDest"
    } else {
        Write-Warning "LCOV output was not generated"
    }

    if (Test-Path $jsonSrc) {
        try {
            $json = Get-Content $jsonSrc -Raw | ConvertFrom-Json
            if ($json.coverage) {
                Write-Info ("Reported coverage: {0}%" -f [math]::Round([double]$json.coverage, 2))
            }
        } catch {
            Write-Warning ("Failed to parse coverage JSON: {0}" -f $_.Exception.Message)
        }
    }
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

function Invoke-PerformanceTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running Rust performance harness (ignored tests)..."
    Invoke-DockerCompose @(
        "run",
        "--rm",
        "--env",
        "KUKURI_PERFORMANCE_OUTPUT=/app/test-results/performance",
        "rust-test",
        "cargo",
        "test",
        "--test",
        "performance",
        "--",
        "--ignored",
        "--nocapture"
    )
    Write-Success "Performance harness completed. Reports stored in test-results/performance"
}

function Invoke-ContractTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running Rust contract tests (contract/nip10)..."
    Invoke-DockerCompose @("run", "--rm", "rust-test", "cargo", "test", "--test", "contract")
    Write-Host "Running TypeScript contract tests (tests/unit/lib/nip10.contract)..."
    Invoke-DockerCompose @(
        "run",
        "--rm",
        "test-runner",
        "bash",
        "-lc",
        "cd /app/kukuri-tauri && pnpm vitest run src/tests/unit/lib/nip10.contract.test.ts"
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
        if ($TestTarget) {
            Invoke-RustTestTarget -Target $TestTarget
        } elseif ($Integration) {
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
    "coverage" {
        Invoke-RustCoverage
        Show-CacheStatus
    }
    "metrics" {
        Invoke-MetricsTests
        Show-CacheStatus
    }
    "performance" {
        Invoke-PerformanceTests
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
