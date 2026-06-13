param(
  [string]$OutputPath,
  [string]$RustMetadataPath,
  [string]$NpmLicensesPath,
  [switch]$Check
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$desktopDir = Join-Path $repoRoot "apps\desktop"
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $OutputPath = Join-Path $repoRoot "docs\THIRD_PARTY_NOTICES.md"
}

function Escape-MarkdownCell {
  param([AllowNull()][string]$Value)

  if ([string]::IsNullOrWhiteSpace($Value)) {
    return "-"
  }
  return ($Value.Trim() -replace "\|", "\|" -replace "`r?`n", " ")
}

function Get-CargoMetadata {
  if (-not [string]::IsNullOrWhiteSpace($RustMetadataPath)) {
    return Get-Content -LiteralPath $RustMetadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
  }

  Push-Location $repoRoot
  try {
    $metadataJson = (& cargo metadata --locked --format-version 1 | Out-String)
    if ($LASTEXITCODE -ne 0) {
      throw "cargo metadata failed with exit code $LASTEXITCODE"
    }
    return $metadataJson | ConvertFrom-Json
  } finally {
    Pop-Location
  }
}

function Get-NpmLicenses {
  if (-not [string]::IsNullOrWhiteSpace($NpmLicensesPath)) {
    return Get-Content -LiteralPath $NpmLicensesPath -Raw -Encoding UTF8 | ConvertFrom-Json
  }

  $licensesJson = (& npx pnpm@10.16.1 --dir $desktopDir licenses list --prod --json | Out-String)
  if ($LASTEXITCODE -ne 0) {
    throw "pnpm licenses list failed with exit code $LASTEXITCODE"
  }
  return $licensesJson | ConvertFrom-Json
}

function Get-RustInventory {
  $metadata = Get-CargoMetadata
  $packages = @($metadata.packages) |
    Where-Object { $null -ne $_.source } |
    ForEach-Object {
      [pscustomobject]@{
        Name = [string]$_.name
        Version = [string]$_.version
        License = if ([string]::IsNullOrWhiteSpace($_.license)) { "UNKNOWN" } else { [string]$_.license }
        Source = "https://crates.io/crates/$($_.name)"
      }
    } |
    Sort-Object Name, Version, License -Unique

  return @($packages)
}

function Get-NpmInventory {
  $licenses = Get-NpmLicenses
  $packages = foreach ($licenseGroup in $licenses.PSObject.Properties) {
    foreach ($package in @($licenseGroup.Value)) {
      foreach ($version in @($package.versions)) {
        [pscustomobject]@{
          Name = [string]$package.name
          Version = [string]$version
          License = if ([string]::IsNullOrWhiteSpace($package.license)) { [string]$licenseGroup.Name } else { [string]$package.license }
          Source = if ([string]::IsNullOrWhiteSpace($package.homepage)) { "-" } else { [string]$package.homepage }
        }
      }
    }
  }

  return @($packages | Sort-Object Name, Version, License -Unique)
}

function Add-InventorySection {
  param(
    [System.Collections.Generic.List[string]]$Lines,
    [string]$Title,
    [object[]]$Items
  )

  $Lines.Add("## $Title") | Out-Null
  $Lines.Add("") | Out-Null
  $Lines.Add("Total packages: $($Items.Count)") | Out-Null
  $Lines.Add("") | Out-Null
  $Lines.Add("| Package | Version | License | Source |") | Out-Null
  $Lines.Add("| --- | --- | --- | --- |") | Out-Null
  foreach ($item in $Items) {
    $Lines.Add(
      "| $(Escape-MarkdownCell $item.Name) | $(Escape-MarkdownCell $item.Version) | $(Escape-MarkdownCell $item.License) | $(Escape-MarkdownCell $item.Source) |"
    ) | Out-Null
  }
  $Lines.Add("") | Out-Null
}

$rustInventory = Get-RustInventory
$npmInventory = Get-NpmInventory
$unknown = @($rustInventory + $npmInventory | Where-Object { $_.License -eq "UNKNOWN" })
if ($unknown.Count -gt 0) {
  $names = ($unknown | ForEach-Object { "$($_.Name)@$($_.Version)" }) -join ", "
  throw "Third-party license inventory contains UNKNOWN licenses: $names"
}

$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("# Third-party notices") | Out-Null
$lines.Add("") | Out-Null
$lines.Add("kukuri preview builds include Rust crates, npm packages, and Tauri runtime components from third-party authors.") | Out-Null
$lines.Add("") | Out-Null
$lines.Add("This file is generated from the locked Rust and desktop npm dependency inventories.") | Out-Null
$lines.Add("") | Out-Null
$lines.Add("Regenerate it from the repository root with:") | Out-Null
$lines.Add("") | Out-Null
$lines.Add('```powershell') | Out-Null
$lines.Add("./scripts/release/generate-third-party-notices.ps1") | Out-Null
$lines.Add('```') | Out-Null
$lines.Add("") | Out-Null
$lines.Add("Release owners must review these inventories before publishing a preview build and update this generator if a dependency requires attribution text beyond the package-level license inventory.") | Out-Null
$lines.Add("") | Out-Null
$lines.Add("## Current distribution note") | Out-Null
$lines.Add("") | Out-Null
$lines.Add("The first preview targets Windows installer distribution through GitHub Releases. Linux remains source-run only for this preview scope. If Windows code signing is not configured, the release notes must state that the preview is unsigned and that SmartScreen warnings are expected.") | Out-Null
$lines.Add("") | Out-Null

Add-InventorySection -Lines $lines -Title "Rust crates" -Items $rustInventory
Add-InventorySection -Lines $lines -Title "Desktop npm packages" -Items $npmInventory

$content = ($lines -join "`n").TrimEnd() + "`n"

if ($Check) {
  if (-not (Test-Path -LiteralPath $OutputPath)) {
    throw "Third-party notices file does not exist: $OutputPath"
  }
  $existing = Get-Content -LiteralPath $OutputPath -Raw -Encoding UTF8
  $normalizedExisting = ($existing -replace "`r`n", "`n").TrimEnd()
  $normalizedContent = ($content -replace "`r`n", "`n").TrimEnd()
  if ($normalizedExisting -ne $normalizedContent) {
    throw "Third-party notices are out of date. Run ./scripts/release/generate-third-party-notices.ps1"
  }
  Write-Host "Third-party notices are up to date"
  exit 0
}

$outputDirectory = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
Set-Content -LiteralPath $OutputPath -Value $content -Encoding UTF8 -NoNewline
Write-Host "Wrote third-party notices to $OutputPath"
