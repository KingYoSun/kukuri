[CmdletBinding()]
param(
    [switch]$Integration,
    [switch]$NoBuild,
    [string]$BootstrapPeers,
    [string]$IrohBin,
    [string]$IntegrationLog
)

# Wrapper for running Rust tests via Docker on Windows.
# Calls scripts/test-docker.ps1 from the repository root and forwards exit codes.

$ErrorActionPreference = 'Stop'

$scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = Split-Path $scriptDirectory -Parent
$testDockerScript = Join-Path $repositoryRoot 'scripts\test-docker.ps1'

if (-not (Test-Path $testDockerScript)) {
    Write-Error 'scripts/test-docker.ps1 was not found. Please run from the repository root.'
}

$arguments = @('rust')

if ($Integration) {
    $arguments += '-Integration'
}

if ($NoBuild) {
    $arguments += '-NoBuild'
}

if ($BootstrapPeers) {
    $arguments += @('-BootstrapPeers', $BootstrapPeers)
}

if ($IrohBin) {
    $arguments += @('-IrohBin', $IrohBin)
}

if ($IntegrationLog) {
    $arguments += @('-IntegrationLog', $IntegrationLog)
}

Push-Location $repositoryRoot
try {
    & $testDockerScript @arguments
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

exit $exitCode
