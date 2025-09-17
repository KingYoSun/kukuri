param(
    [Parameter(Position = 0)]
    [ValidateSet('probe', 'down')]
    [string]$Command = 'probe',

    [switch]$NoBuild,
    [switch]$KeepEnv,
    [switch]$Help
)

if ($Help) {
    $lines = @(
        'Usage: .\test-docker.ps1 [probe|down] [options]',
        '',
        'Commands:',
        '  probe   Start the docker connectivity probe (default)',
        '  down    Stop containers and remove volumes',
        '',
        'Options:',
        '  -NoBuild  Skip docker image build',
        '  -KeepEnv  Keep generated .env files under tests/ after execution',
        '  -Help     Show this help text'
    )
    Write-Host ($lines -join [Environment]::NewLine)
    exit 0
}

function Test-DockerInstalled {
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Error 'docker command was not found in PATH. Install Docker Desktop or Docker CLI first.'
        exit 1
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$cliRoot = Split-Path -Parent $scriptDir
$testsDir = Join-Path $cliRoot 'tests'
$composeFile = Join-Path $testsDir 'docker-compose.test.yml'
$projectName = 'kukuri_cli_docker_test'

$secretA = 'ERERERERERERERERERERERERERERERERERERERERERE='
$secretB = 'IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI='
$nodeAId = 'd04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c9778737'

function Write-EnvFile {
    param(
        [string]$Path,
        [string]$Content
    )

    $directory = Split-Path -Parent $Path
    if (-not (Test-Path $directory)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }

    Set-Content -Path $Path -Value $Content -Encoding ascii
}

function Ensure-EnvFiles {
    $files = @()

    $envNodeA = Join-Path $testsDir '.env.node_a'
    $envNodeB = Join-Path $testsDir '.env.node_b'

    Write-EnvFile -Path $envNodeA -Content "KUKURI_SECRET_KEY=$secretA`n"
    $peerValue = "$nodeAId@node_a:11223"
    Write-EnvFile -Path $envNodeB -Content "KUKURI_SECRET_KEY=$secretB`nNODE_A_ADDR=$peerValue`nCONNECT_PEER=$peerValue`n"

    $files += $envNodeA
    $files += $envNodeB
    return $files
}

function Remove-EnvFiles {
    param([string[]]$Files)
    if ($KeepEnv) {
        return
    }
    foreach ($file in $Files) {
        if (Test-Path $file) {
            Remove-Item -Path $file -Force -ErrorAction SilentlyContinue
        }
    }
}

function Invoke-DockerCompose {
    param(
        [string[]]$Arguments,
        [switch]$IgnoreFailure
    )

    $previousName = $env:COMPOSE_PROJECT_NAME
    $env:COMPOSE_PROJECT_NAME = $projectName

    Push-Location $testsDir
    try {
        & docker compose -f $composeFile @Arguments
        $code = $LASTEXITCODE
        if (-not $IgnoreFailure -and $code -ne 0) {
            throw "docker compose exited with code $code"
        }
        return $code
    }
    finally {
        Pop-Location
        if ($null -ne $previousName) {
            $env:COMPOSE_PROJECT_NAME = $previousName
        }
        else {
            Remove-Item Env:COMPOSE_PROJECT_NAME -ErrorAction SilentlyContinue
        }
    }
}

switch ($Command) {
    'probe' {
        Test-DockerInstalled
        $envFiles = Ensure-EnvFiles
        try {
            $args = @('up', '--abort-on-container-exit')
            if (-not $NoBuild) {
                $args += '--build'
            }
            Write-Host 'Starting docker connectivity probe...' -ForegroundColor Cyan
            $exitCode = Invoke-DockerCompose -Arguments $args
        }
        catch {
            Write-Error $_
            $exitCode = 1
        }
        finally {
            Invoke-DockerCompose -Arguments @('down', '--volumes', '--remove-orphans') -IgnoreFailure | Out-Null
            Remove-EnvFiles -Files $envFiles
        }
        exit $exitCode
    }
    'down' {
        Invoke-DockerCompose -Arguments @('down', '--volumes', '--remove-orphans') -IgnoreFailure | Out-Null
        Remove-EnvFiles -Files @(Join-Path $testsDir '.env.node_a', Join-Path $testsDir '.env.node_b')
        Write-Host 'Cleaned docker connectivity resources.' -ForegroundColor Green
        exit 0
    }
}
