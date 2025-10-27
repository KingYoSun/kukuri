$ErrorActionPreference = "Stop"

param(
    [string]$Output,
    [switch]$Pretty
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..")

if (-not $Output) {
    $timestamp = [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssZ")
    $Output = Join-Path $repoRoot "docs/01_project/activeContext/artefacts/metrics/$timestamp-p2p-metrics.json"
}

$argsList = @(
    "run",
    "--manifest-path", "kukuri-tauri/src-tauri/Cargo.toml",
    "--bin", "p2p_metrics_export",
    "--",
    "--output", $Output
)

if ($Pretty) {
    $argsList += "--pretty"
}

Push-Location $repoRoot
try {
    cargo @argsList
} finally {
    Pop-Location
}
