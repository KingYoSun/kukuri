<#
.SYNOPSIS
  Collects repository-wide code quality metrics (TODO / any / allow(dead_code) counts).

.DESCRIPTION
  Runs ripgrep-based searches to count occurrences of key maintenance indicators for
  TypeScript and Rust code. The script outputs a JSON summary to stdout by default,
  or writes it to the specified path when -OutputPath is provided.

.EXAMPLE
  ./scripts/metrics/collect-metrics.ps1

.EXAMPLE
  ./scripts/metrics/collect-metrics.ps1 -OutputPath docs/01_project/activeContext/tasks/metrics/latest.json
#>
[CmdletBinding()]
param(
  [Parameter()]
  [string]
  $OutputPath
)

function Invoke-RipgrepCount {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory)]
    [string] $Pattern,
    [Parameter()]
    [string[]] $Globs = @(),
    [Parameter()]
    [string[]] $Paths = @('.'),
    [switch] $UsePCRE2
  )

  $arguments = @('--json', '--no-heading')
  if ($UsePCRE2) {
    $arguments += '--pcre2'
  }

  foreach ($glob in $Globs) {
    $arguments += @('-g', $glob)
  }

  $arguments += $Pattern
  $arguments += $Paths

  $rawOutput = & rg @arguments 2>$null
  $exitCode = $LASTEXITCODE

  if ($exitCode -gt 1) {
    throw "ripgrep failed (exit code $exitCode) while searching for pattern '$Pattern'"
  }

  if (-not $rawOutput) {
    return 0
  }

  $count = 0
  foreach ($line in $rawOutput) {
    try {
      $json = $line | ConvertFrom-Json -ErrorAction Stop
    } catch {
      continue
    }

    if ($json.type -eq 'match') {
      $count++
    }
  }

  return $count
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$originalLocation = Get-Location

try {
  Set-Location -Path $repoRoot

  $typescriptPaths = @('kukuri-tauri/src')
  $rustPaths = @('kukuri-tauri/src-tauri')

  $metrics = [ordered]@{
    timestamp  = [DateTime]::UtcNow.ToString('o')
    typescript = [ordered]@{
      todo = Invoke-RipgrepCount -Pattern 'TODO' -Globs @('*.ts', '*.tsx') -Paths $typescriptPaths
      any  = Invoke-RipgrepCount -Pattern '\bany\b' -Globs @('*.ts', '*.tsx') -Paths $typescriptPaths -UsePCRE2
    }
    rust = [ordered]@{
      todo           = Invoke-RipgrepCount -Pattern 'TODO' -Globs @('*.rs') -Paths $rustPaths
      allow_dead_code = Invoke-RipgrepCount -Pattern '#\[allow\(dead_code\)\]' -Globs @('*.rs') -Paths $rustPaths
    }
  }

  $json = $metrics | ConvertTo-Json -Depth 4

  if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    Write-Output $json
  } else {
    if ([System.IO.Path]::IsPathRooted($OutputPath)) {
      $resolvedPath = [System.IO.Path]::GetFullPath($OutputPath)
    } else {
      $resolvedPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputPath))
    }
    $resolvedDirectory = [System.IO.Path]::GetDirectoryName($resolvedPath)
    if (-not [string]::IsNullOrWhiteSpace($resolvedDirectory) -and -not (Test-Path -Path $resolvedDirectory)) {
      New-Item -ItemType Directory -Path $resolvedDirectory -Force | Out-Null
    }
    Set-Content -Path $resolvedPath -Value $json -Encoding utf8
    Write-Output "Metrics written to $resolvedPath"
  }
}
finally {
  Set-Location -Path $originalLocation
}
