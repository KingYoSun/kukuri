$ErrorActionPreference = "Stop"

param(
    [string]$Output,
    [switch]$Pretty,
    [ValidateSet("p2p", "trending")]
    [string]$Job = "p2p",
    [string]$DatabaseUrl,
    [int]$Limit
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..")

if (-not $Output) {
    $timestamp = [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssZ")
    if ($Job -eq "trending") {
        $trendingDir = Join-Path $repoRoot "test-results/trending-feed/metrics"
        if (-not (Test-Path $trendingDir)) {
            New-Item -ItemType Directory -Path $trendingDir | Out-Null
        }
        $Output = Join-Path $trendingDir "$timestamp-trending-metrics.json"
    }
    else {
        $Output = Join-Path $repoRoot "docs/01_project/activeContext/artefacts/metrics/$timestamp-p2p-metrics.json"
    }
}

$argsList = @(
    "run",
    "--manifest-path", "kukuri-tauri/src-tauri/Cargo.toml",
    "--bin", "p2p_metrics_export",
    "--",
    "--job", $Job,
    "--output", $Output
)

if ($Pretty) {
    $argsList += "--pretty"
}

if ($DatabaseUrl) {
    $argsList += @("--database-url", $DatabaseUrl)
}

if ($Limit -gt 0) {
    $argsList += @("--limit", $Limit)
}

Push-Location $repoRoot
try {
    cargo @argsList
} finally {
    Pop-Location
}
