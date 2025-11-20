# Docker���ł̃e�X�g���s�X�N���v�g (PowerShell��)

param(
    [Parameter(Position = 0)]
    [ValidateSet("all", "rust", "integration", "ts", "lint", "coverage", "build", "clean", "cache-clean", "metrics", "performance", "contracts", "e2e")]
    [string]$Command = "all",

    [switch]$Integration,            # Rust�e�X�g����P2P�����e�X�g�݂̂���s
    [Alias("Test", "tests")]
    [string]$TestTarget,             # Rust�e�X�g���ɓ���o�C�i���̂ݎ��s
    [string]$Scenario,               # TypeScript�e�X�g�p�̃V�i���I�w��
    [string]$Fixture,                # �V�i���I�p�t�B�N�X�`���p�X
    [switch]$ServiceWorker,          # profile-avatar-sync �V�i���I�� Service Worker �g������s
    [string]$OfflineCategory,        # offline-sync �V�i���I�p�J�e�S���[
    [string]$BootstrapPeers,         # �����e�X�g�p�̃u�[�g�X�g���b�v�s�A�w��
    [string]$IrohBin,                # iroh �o�C�i���̃p�X
    [string]$IntegrationLog = "info,iroh_tests=debug", # �����e�X�g�p��RUST_LOG

    [switch]$NoBuild,  # �r���h��X�L�b�v����I�v�V����
    [switch]$Help
)

$scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = Split-Path $scriptDirectory -Parent
$ResultsDir = Join-Path $repositoryRoot "test-results"
$NewBinMainlineTarget = if (![string]::IsNullOrWhiteSpace($env:P2P_MAINLINE_TEST_TARGET)) { $env:P2P_MAINLINE_TEST_TARGET } else { "p2p_mainline_smoke" }
$NewBinGossipTarget = if (![string]::IsNullOrWhiteSpace($env:P2P_GOSSIP_TEST_TARGET)) { $env:P2P_GOSSIP_TEST_TARGET } else { "p2p_gossip_smoke" }
$PrometheusMetricsUrl = if (![string]::IsNullOrWhiteSpace($env:PROMETHEUS_METRICS_URL)) { $env:PROMETHEUS_METRICS_URL } else { "http://127.0.0.1:9898/metrics" }

# �J���[�֐�
function Write-Success {
    param([string]$Message)
    Write-Host "? $Message" -ForegroundColor Green
}

function Write-ErrorMessage {
    param([string]$Message)
    Write-Host "Error: $Message" -ForegroundColor Red
    exit 1
}

function Write-Warning {
    param([string]$Message)
    Write-Host "? $Message" -ForegroundColor Yellow
}

function Write-Info {
    param([string]$Message)
    Write-Host "? $Message" -ForegroundColor Cyan
}

if ($Integration -and $TestTarget) {
    Write-ErrorMessage "-Integration �� -Test �͓����ɂ͎w��ł��܂���B"
}

if ($TestTarget -and $Command -ne "rust") {
    Write-ErrorMessage "-Test �� rust �R�}���h�݂̂Ɏw��ł��܂��B"
}

if ($Scenario -and $Command -ne "ts") {
    Write-ErrorMessage "-Scenario �� ts �R�}���h�ł̂ݎg�p�ł��܂��B"
}

if ($OfflineCategory -and $Scenario -ne "offline-sync") {
    Write-ErrorMessage "-OfflineCategory �� ts -Scenario offline-sync �ł̂ݎg�p�ł��܂��B"
}

if ($Fixture -and $Command -ne "ts") {
    Write-ErrorMessage "-Fixture �� ts �R�}���h�ł̂ݎg�p�ł��܂��B"
}

if ($ServiceWorker -and $Command -ne "ts") {
    Write-ErrorMessage "-ServiceWorker �� ts �R�}���h�ł̂ݎg�p�ł��܂��B"
}

# �w���v�\��
function Show-Help {
    Write-Host @"
Usage: .\test-docker.ps1 [Command] [Options]

Commands:
  all          - ���ׂẴe�X�g����s�i�f�t�H���g�j
  rust         - Rust�̃e�X�g�̂ݎ��s
  integration  - P2P�����e�X�g�iRust�j����s
  ts           - TypeScript�̃e�X�g�̂ݎ��s�i-Scenario �ŃV�i���I�w��j
  lint         - �����g�ƃt�H�[�}�b�g�`�F�b�N�̂ݎ��s
  coverage     - Rust�J�o���b�W�icargo tarpaulin�j����s�����ʕ���ۑ�
  metrics      - ���g���N�X�֘A�̃V���[�g�e�X�g�iRust test_get_status / TS P2P UI�j
  performance  - �p�t�H�[�}���X�n�[�l�X�iRust ignored �e�X�g�j����s�����ʕ��𐶐�
  contracts    - �_��e�X�g�iNIP-10���E�P�[�X�j����s
  e2e          - Desktop E2E テスト（tauri-driver + WebDriverIO）を実行`n  build        - Docker�C���[�W�̃r���h�̂ݎ��s
  clean        - Docker�R���e�i�ƃC���[�W��N���[���A�b�v
  cache-clean  - �L���b�V���{�����[����܂߂Ċ��S�N���[���A�b�v

Options:
  -Integration  - Rust�R�}���h�ƕ����� P2P �����e�X�g�̂ݎ��s
  -Test <target> - Rust�R�}���h���Ɏw��e�X�g�o�C�i���̂ݎ��s�i��: event_manager_integration�j
  -Scenario <name> - TypeScript�e�X�g���ɃV�i���I��w��i��: trending-feed, profile-avatar-sync, direct-message, user-search-pagination, topic-create, post-delete-cache, offline-sync�j
  -OfflineCategory <name> - offline-sync �V�i���I�p�J�e�S���[ (topic/post/follow/dm)
  -Fixture <path>  - �V�i���I�p�t�B�N�X�`���p�X��㏑���i����: tests/fixtures/trending/default.json�j
  -ServiceWorker   - `ts -Scenario profile-avatar-sync` ���s���� Service Worker �g���e�X�g�� Stage4 ���O��L����
  -BootstrapPeers <node@host:port,...> - �����e�X�g�Ŏg�p����u�[�g�X�g���b�v�s�A��w��
  -IrohBin <path> - iroh �o�C�i���̖����p�X��w��iWindows �� DLL ������K�v�ȏꍇ�Ȃǁj
  -IntegrationLog <level> - �����e�X�g���� RUST_LOG �ݒ�i����: info,iroh_tests=debug�j
  -NoBuild     - Docker�C���[�W�̃r���h��X�L�b�v
  -Help        - ���̃w���v��\��
  �� P2P�����e�X�g�� `p2p_gossip_smoke` / `p2p_mainline_smoke` ��������s���܂��B`P2P_GOSSIP_TEST_TARGET` �� `P2P_MAINLINE_TEST_TARGET` �ŔC�ӂ̃^�[�Q�b�g�ɏ㏑���\�ł��B

Examples:
  .\test-docker.ps1                # ���ׂẴe�X�g����s
  .\test-docker.ps1 rust           # Rust�e�X�g�̂ݎ��s
  .\test-docker.ps1 rust -Test event_manager_integration
  .\test-docker.ps1 rust -Integration -BootstrapPeers "node@127.0.0.1:11233"
  .\test-docker.ps1 rust -NoBuild  # �r���h��X�L�b�v����Rust�e�X�g����s
  .\test-docker.ps1 ts -Scenario trending-feed
  .\test-docker.ps1 ts -Scenario profile-avatar-sync
  .\test-docker.ps1 ts -Scenario user-search-pagination
  .\test-docker.ps1 performance    # �p�t�H�[�}���X�v���p�e�X�g�o�C�i������s
  .\test-docker.ps1 cache-clean    # �L���b�V����܂߂Ċ��S�N���[���A�b�v
  .\test-docker.ps1 -Help          # �w���v��\��

Performance Tips:
  - ������s���͈ˑ��֌W�̃_�E�����[�h�̂��ߎ��Ԃ�������܂�
  - 2��ڈȍ~��Docker�{�����[���ɃL���b�V������邽�ߍ����ɂȂ�܂�
  - �L���b�V����N���A�������ꍇ�� 'cache-clean' �R�}���h��g�p���Ă�������
"@
    exit 0
}

function Assert-CorepackPnpmReady {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepoRoot
    )

    Write-Info "Checking Corepack/pnpm initialization..."
    if ($IsWindows) {
        & cmd.exe /c "corepack pnpm --version" 1>$null 2>$null
    }
    else {
        & corepack pnpm --version 1>$null 2>$null
    }
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorMessage "Corepack �̓��l pnpm shim �����s���Ă��Ȃ��A�܂��̓C���X�g�[�������Ă��܂��Ȃ��ł��Bcmd.exe /c ""corepack enable pnpm"" ����s���A������ cmd.exe /c ""corepack pnpm install --frozen-lockfile"" �i macOS / Linux: corepack enable pnpm && corepack pnpm install --frozen-lockfile �j�Ō\�̋@�\����̋�E���Ɠ�����݂��Ă��������Bdocs/01_project/setup_guide.md ����Q�Ƃ��Ă��������B"
    }

    $tauriRoot = Join-Path $RepoRoot "kukuri-tauri"
    $modulesFile = Join-Path (Join-Path $tauriRoot "node_modules") ".modules.yaml"
    if (-not (Test-Path $modulesFile)) {
        Write-ErrorMessage "kukuri-tauri/node_modules/.modules.yaml ���݂��Ȃ��̂ŁA corepack pnpm install --frozen-lockfile ���s���Ă��Ȃ��ƌ�����܂��Bcmd.exe /c ""cd kukuri-tauri && corepack pnpm install --frozen-lockfile"" �܂��� cd kukuri-tauri && corepack pnpm install --frozen-lockfile ����s���A��̍����Ń_�C�A�g�����ɗ��������Բ����Ă��������B�s���ɗ� scripts/test-docker.ps1 $Command ����s���Ă��������B"
    }
}

# Docker Buildkit ��L����
$env:DOCKER_BUILDKIT = "1"
$env:COMPOSE_DOCKER_CLI_BUILD = "1"

$BootstrapDefaultPeer = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233"
$BootstrapContainerName = "kukuri-p2p-bootstrap"

# Docker Compose�R�}���h�̎��s
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

# Docker�C���[�W�̑��݊m�F
function Test-DockerImageExists {
    $runnerImage = docker images -q "kukuri_test-runner" 2>$null
    $tsImage = docker images -q "kukuri_ts-test" 2>$null
    return (![string]::IsNullOrEmpty($runnerImage) -and -not [string]::IsNullOrEmpty($tsImage))
}

# Docker�C���[�W�̃r���h
function Build-TestImage {
    param([switch]$Force)
    
    if (-not $Force -and (Test-DockerImageExists)) {
        Write-Info "Docker image already exists. Use 'build' command to rebuild."
        return
    }
    
    Write-Host "Building Docker test image (with cache optimization)..."
    Invoke-DockerCompose @("build", "--build-arg", "DOCKER_BUILDKIT=1", "test-runner", "ts-test")
    Write-Success "Docker image built successfully"
}

# ���ׂẴe�X�g����s
function Invoke-AllTests {
    if (-not $NoBuild) {
        Build-TestImage
    }
    Write-Host "Running all tests in Docker..."
    Invoke-DockerCompose @("run", "--rm", "test-runner", "/app/run-tests.sh")
    Write-Success "All tests passed!"
}

# Rust�e�X�g�̂ݎ��s
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

# TypeScript�e�X�g�̂ݎ��s
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

    $scenarioDir = Join-Path $repositoryRoot "test-results/trending-feed/reports"
    if (-not (Test-Path $scenarioDir)) {
        New-Item -ItemType Directory -Path $scenarioDir | Out-Null
    }

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $logDir = Join-Path $repositoryRoot "tmp/logs/trending-feed"
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
    $logRelPath = "tmp/logs/trending-feed/$timestamp.log"
    $latestRelPath = "tmp/logs/trending-feed/latest.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $latestHostPath = Join-Path $repositoryRoot $latestRelPath
    $header = @(
        "=== trending-feed scenario ===",
        "timestamp: $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))",
        "fixture: $fixturePath",
        ""
    )
    Set-Content -Path $logHostPath -Value $header -Encoding UTF8

    $vitestTargets = @(
        'src/tests/unit/routes/trending.test.tsx',
        'src/tests/unit/routes/following.test.tsx',
        'src/tests/unit/hooks/useTrendingFeeds.test.tsx'
    )

    $promStarted = Start-PrometheusTrending
    $vitestStatus = 0

    try {
        foreach ($target in $vitestTargets) {
            $slug = $target.Replace('/', '_').Replace('.', '_')
            $reportRelPath = "test-results/trending-feed/reports/$timestamp-$slug.json"
            $reportContainerPath = "/app/$reportRelPath"
            Add-Content -Path $logHostPath -Value @("`n--- Running target: $target ---", "report: $reportRelPath") -Encoding UTF8

            $dockerArgs = @(
                "compose", "-f", "docker-compose.test.yml",
                "run", "--rm",
                "-e", "VITE_TRENDING_FIXTURE_PATH=$fixturePath",
                "ts-test", "bash", "/app/scripts/docker/run-vitest-target.sh", $target, $reportContainerPath
            )

            $dockerOutput = & docker $dockerArgs 2>&1
            $exitCode = $LASTEXITCODE
            if ($dockerOutput) {
                foreach ($line in $dockerOutput) {
                    Write-Host $line
                }
                Add-Content -Path $logHostPath -Value ($dockerOutput -join [Environment]::NewLine) -Encoding UTF8
                Add-Content -Path $logHostPath -Value "" -Encoding UTF8
            }
            if ($exitCode -ne 0) {
                Write-Warning "Vitest target $target failed with exit code $exitCode"
                $vitestStatus = $exitCode
                break
            }

            $reportHostPath = Join-Path $repositoryRoot $reportRelPath
            if (Test-Path $reportHostPath) {
                Write-Info "Scenario report saved to $reportRelPath"
            } else {
                Write-Warning "Scenario report not found at $reportRelPath"
            }
        }
    }
    finally {
        if ($promStarted) {
            Collect-TrendingMetricsSnapshot -Timestamp $timestamp -RunState "active"
            Stop-PrometheusTrending
        } else {
            Collect-TrendingMetricsSnapshot -Timestamp $timestamp -RunState "skipped"
        }
    }

    if (Test-Path $logHostPath) {
        Copy-Item -Path $logHostPath -Destination $latestHostPath -Force
        Write-Success "Scenario log saved to $logRelPath"
        Write-Info "Latest scenario log updated at $latestRelPath"
    } else {
        Write-Warning "Scenario log was not generated at $logRelPath"
    }

    Add-Content -Path $logHostPath -Value @('', '--- Exporting trending metrics snapshot (scripts/metrics/export-p2p.ps1 -Job trending -Pretty) ---') -Encoding UTF8
    $metricsScript = Join-Path $scriptDirectory 'metrics/export-p2p.ps1'
    try {
        $exportOutput = & $metricsScript -Job trending -Pretty
        if ($exportOutput) {
            Add-Content -Path $logHostPath -Value $exportOutput -Encoding UTF8
        }
        Write-Success "Trending metrics JSON exported to test-results/trending-feed/metrics"
    }
    catch {
        Write-Warning "Trending metrics export failed: $_"
        Add-Content -Path $logHostPath -Value "[WARN] Trending metrics export failed: $_" -Encoding UTF8
    }
    if ($vitestStatus -ne 0) {
        throw "Scenario 'trending-feed' failed. See $logRelPath for details."
    }

    Write-Success "Scenario reports stored under test-results/trending-feed/reports/ (prefix $timestamp)"
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
    $reportsDir = Join-Path $resultsDir "reports"
    $logsDir = Join-Path $resultsDir "logs"
    $searchErrorDir = Join-Path $resultsDir "search-error"
    foreach ($dir in @($resultsDir, $reportsDir, $logsDir, $searchErrorDir)) {
        if (-not (Test-Path $dir)) {
            New-Item -ItemType Directory -Path $dir | Out-Null
        }
    }

    Write-Host "Running TypeScript scenario 'user-search-pagination'..."
    $vitestTargets = @(
        "src/tests/unit/hooks/useUserSearchQuery.test.tsx",
        "src/tests/unit/components/search/UserSearchResults.test.tsx",
        "src/tests/unit/scenario/userSearchPaginationArtefact.test.tsx"
    )

    $vitestStatus = 0
    foreach ($target in $vitestTargets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/user-search-pagination/reports/${timestamp}-${slug}.json"

        $commandLines = @(
            "set -euo pipefail",
            "cd /app/kukuri-tauri",
            "if [ ! -f node_modules/.bin/vitest ]; then",
            "  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'",
            "  pnpm install --frozen-lockfile --ignore-workspace",
            "fi",
            "pnpm vitest run '$target' --testTimeout 15000 --reporter=default --reporter=json --outputFile '/app/$reportRelPath'"
        )
        $command = [string]::Join("`n", $commandLines)




        $dockerArgs = @(
            "compose", "-f", "docker-compose.test.yml",
            "run", "--rm",
            "--env", "USER_SEARCH_SCENARIO_TIMESTAMP=$timestamp",
            "ts-test", "bash", "-lc", $command
        )
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

    if (Test-Path $logHostPath) {
        Write-Success "Scenario log saved to $logRelPath"
        $logArchiveRelPath = "test-results/user-search-pagination/logs/${timestamp}.log"
        $logArchiveHostPath = Join-Path $repositoryRoot $logArchiveRelPath
        Copy-Item -Path $logHostPath -Destination $logArchiveHostPath -Force
        Write-Success "Scenario log archived to $logArchiveRelPath"
    } else {
        Write-Warning "Scenario log was not generated at $logRelPath"
    }

    if ($vitestStatus -ne 0) {
        Write-ErrorMessage "Scenario 'user-search-pagination' failed. See $logRelPath for details."
    }
}





function Invoke-TypeScriptDirectMessageScenario {

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

    $logRelPath = "tmp/logs/vitest_direct_message_$timestamp.log"

    $logHostPath = Join-Path $repositoryRoot $logRelPath

    $logDir = Split-Path $logHostPath -Parent

    if (-not (Test-Path $logDir)) {

        New-Item -ItemType Directory -Path $logDir | Out-Null

    }

    Set-Content -Path $logHostPath -Value @()



    $resultsDir = Join-Path $repositoryRoot "test-results/direct-message"

    if (-not (Test-Path $resultsDir)) {

        New-Item -ItemType Directory -Path $resultsDir | Out-Null

    }



    Write-Host "Running TypeScript scenario 'direct-message'..."

    $vitestTargets = @(

        "src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx",

        "src/tests/unit/components/directMessages/DirectMessageInbox.test.tsx",

        "src/tests/unit/components/layout/Header.test.tsx",

        "src/tests/unit/hooks/useDirectMessageBadge.test.tsx"

    )



    $vitestStatus = 0

    foreach ($target in $vitestTargets) {

        $slug = $target.Replace("/", "_").Replace(".", "_")

        $reportRelPath = "test-results/direct-message/${timestamp}-${slug}.json"



        $commandLines = @(
            "set -euo pipefail",
            "cd /app/kukuri-tauri",
            "if [ ! -f node_modules/.bin/vitest ]; then",
            "  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'",
            "  pnpm install --frozen-lockfile --ignore-workspace",
            "fi",
            "pnpm vitest run '$target' --reporter=default --reporter=json --outputFile '/app/$reportRelPath'"
        )
        $command = [string]::Join("`n", $commandLines)




        $dockerArgs = @('compose', '-f', 'docker-compose.test.yml', 'run', '--rm', 'ts-test', 'bash', '-lc', $command)

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

            Write-Warning "Scenario report was not generated at $reportRelPath"

        }

    }



    if ($vitestStatus -ne 0) {

        throw "Scenario 'direct-message' failed. See $logRelPath for details."

    }



    Write-Success "Scenario log saved to $logRelPath"

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
        "src/tests/unit/stores/postStore.test.ts",
        "src/tests/unit/components/posts/PostCard.test.tsx",
        "src/tests/unit/components/posts/PostCard.deleteOffline.test.tsx"
    )

    $vitestStatus = 0
    foreach ($target in $vitestTargets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/post-delete-cache/${timestamp}-${slug}.json"

        $reportContainerPath = "/app/$reportRelPath"

        $dockerArgs = @(
            "compose", "-f", "docker-compose.test.yml",
            "run", "--rm",
            "ts-test", "bash", "/app/scripts/docker/run-vitest-target.sh", $target, $reportContainerPath
        )
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
        "src/tests/unit/components/layout/Sidebar.test.tsx",
        "src/tests/unit/scenarios/topicCreateOffline.test.tsx"
    )

    $vitestStatus = 0
    foreach ($target in $vitestTargets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/topic-create/${timestamp}-${slug}.json"

        $reportContainerPath = "/app/$reportRelPath"

        $dockerArgs = @(
            "compose", "-f", "docker-compose.test.yml",
            "run", "--rm",
            "ts-test", "bash", "/app/scripts/docker/run-vitest-target.sh", $target, $reportContainerPath
        )
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
    param(
        [string]$Category
    )
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $suffix = ""
    if ($Category) {
        $suffix = "_$Category"
    }
    $logRelPath = "tmp/logs/sync_status_indicator_stage4${suffix}_$timestamp.log"
    $logHostPath = Join-Path $repositoryRoot $logRelPath
    $logDir = Split-Path $logHostPath -Parent
    $reportsDir = Join-Path $ResultsDir "offline-sync"
    if ($Category) {
        $reportsDir = Join-Path $reportsDir $Category
    }
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
    if (-not (Test-Path $reportsDir)) {
        New-Item -ItemType Directory -Path $reportsDir | Out-Null
    }
    Set-Content -Path $logHostPath -Value @()

    if ($Category) {
        Write-Host "Running TypeScript scenario 'offline-sync' (category: $Category)..."
    } else {
        Write-Host "Running TypeScript scenario 'offline-sync'..."
    }

    $targets = @()
    if ($Category) {
        $targets = @("src/tests/unit/scenarios/offlineSyncTelemetry.test.tsx")
    } else {
        $targets = @(
            "src/tests/unit/hooks/useSyncManager.test.tsx",
            "src/tests/unit/components/SyncStatusIndicator.test.tsx",
            "src/tests/unit/components/OfflineIndicator.test.tsx"
        )
    }
    $vitestStatus = 0
    foreach ($target in $targets) {
        $slug = $target.Replace("/", "_").Replace(".", "_")
        $reportRelPath = "test-results/offline-sync"
        if ($Category) {
            $reportRelPath = "$reportRelPath/$Category"
        }
        $reportRelPath = "$reportRelPath/${timestamp}-${slug}.json"

        $commandLines = @(
            "set -euo pipefail",
            "cd /app/kukuri-tauri",
            "if [ ! -f node_modules/.bin/vitest ]; then",
            "  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'",
            "  pnpm install --frozen-lockfile --ignore-workspace",
            "fi"
        )
        if ($Category) {
            $commandLines += "export OFFLINE_SYNC_CATEGORY='$Category'"
        }
        $commandLines += "pnpm vitest run '$target' --reporter=default --reporter=json --outputFile '/app/$reportRelPath'"
        $command = [string]::Join("`n", $commandLines)

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
        Write-ErrorMessage "Scenario 'offline-sync' failed. See $logRelPath for details."
    } else {
        Write-Success "Scenario log saved to $logRelPath"
    }
}

function Invoke-DesktopE2EScenario {
    if (-not $NoBuild) {
        Build-TestImage
    }

    $logDir = Join-Path $repositoryRoot "tmp/logs/desktop-e2e"
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }

    Write-Host "Running desktop E2E scenario via Docker..."
    $previousScenario = $env:SCENARIO
    $env:SCENARIO = "desktop-e2e"
    try {
        Invoke-DockerCompose @("run", "--rm", "test-runner")
    }
    finally {
        if ($null -ne $previousScenario) {
            $env:SCENARIO = $previousScenario
        } else {
            Remove-Item Env:SCENARIO -ErrorAction SilentlyContinue
        }
    }

    Write-Success "Desktop E2E scenario finished. Check tmp/logs/desktop-e2e/ and test-results/desktop-e2e/ for artefacts."
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

            "direct-message" {

                Invoke-TypeScriptDirectMessageScenario

            }
            "post-delete-cache" {
                Invoke-TypeScriptPostDeleteCacheScenario
            }
            "topic-create" {
                Invoke-TypeScriptTopicCreateScenario
            }
            "offline-sync" {
                Invoke-TypeScriptOfflineSyncScenario -Category $OfflineCategory
            }
            default {
                Write-ErrorMessage "Unknown TypeScript scenario: $Scenario"
            }
        }
    }
}

# �����g�ƃt�H�[�}�b�g�`�F�b�N
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

# �N���[���A�b�v
function Invoke-Cleanup {
    Write-Host "Cleaning up Docker containers and images..."
    Invoke-DockerCompose @("down", "--rmi", "local", "--remove-orphans")
    Write-Success "Cleanup completed"
}

# ���S�N���[���A�b�v�i�L���b�V���{�����[����폜�j
function Invoke-CacheCleanup {
    Write-Host "Performing complete cleanup including cache volumes..."
    
    # �R���e�i�ƃC���[�W�̍폜
    Invoke-DockerCompose @("down", "--rmi", "local", "--volumes", "--remove-orphans")
    
    # ���O�t���{�����[���̍폜
    Write-Host "Removing cache volumes..."
    docker volume rm kukuri-cargo-registry kukuri-cargo-git kukuri-cargo-target kukuri-pnpm-store 2>$null
    
    Write-Success "Complete cleanup finished"
    Write-Info "Next build will take longer as all caches have been cleared"
}

# �L���b�V���󋵂̕\��
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

# ���C������
if ($Help) {
    Show-Help
}

# �e�X�g���ʃf�B���N�g���̍쐬
if (-not (Test-Path "test-results")) {
    New-Item -ItemType Directory -Path "test-results" | Out-Null
}

$pnpmRequiredCommands = @("all", "ts", "lint", "metrics", "performance", "contracts", "e2e")
if (-not $Help -and $pnpmRequiredCommands -contains $Command) {
    Assert-CorepackPnpmReady -RepoRoot $repositoryRoot
}

# �R�}���h�̎��s
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
    "e2e" {
        Invoke-DesktopE2EScenario
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
