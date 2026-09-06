param(
  [Parameter(Mandatory = $true)]
  [string]$Tag,
  [Parameter(Mandatory = $true)]
  [string]$Repository,
  [Parameter(Mandatory = $true)]
  [string]$Version,
  [Parameter(Mandatory = $true)]
  [string]$InputDir,
  [Parameter(Mandatory = $true)]
  [string]$OutputDir,
  [string]$ChangelogSectionPath,
  [switch]$IncludeLinux,
  [string]$SourceCommit
)

$ErrorActionPreference = "Stop"

$packagePlan = $null
if ($Tag -notmatch "^v$([regex]::Escape($Version))-preview\.[0-9]+$" -or
    $Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$' -or
    $Repository -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') {
  throw 'Invalid release tag, version or repository'
}
if ($IncludeLinux) {
  $planJson = & python (Join-Path $PSScriptRoot 'release_assets.py') plan `
    --input $InputDir --tag $Tag --repository $Repository --version $Version --source $SourceCommit
  if ($LASTEXITCODE -ne 0) { throw 'Incomplete or inconsistent release packages' }
  $packagePlan = $planJson | ConvertFrom-Json
  $assets = @($packagePlan.assets | ForEach-Object { Get-Item -LiteralPath $_.path })
} else {
  $assets = @(Get-ChildItem -LiteralPath $InputDir -File -Recurse |
  Where-Object { $_.Extension -in @(".exe", ".msi", ".zip", ".sig") } |
  Sort-Object Name)
}

if ($assets.Count -eq 0) {
  throw "No Windows release assets found in $InputDir"
}

if (($assets | Group-Object Name | Where-Object Count -gt 1).Count -gt 0) {
  throw 'Duplicate release asset filenames'
}

$updaterBundle = $assets | Where-Object { $_.Extension -eq ".zip" } | Select-Object -First 1
if (-not $updaterBundle) {
  $updaterBundle = $assets | Where-Object { $_.Extension -eq ".exe" } | Select-Object -First 1
}
if (-not $updaterBundle) {
  throw "No updater bundle candidate (.zip or .exe) found in $OutputDir"
}

$signatureFile = Get-Item -LiteralPath "$($updaterBundle.FullName).sig" -ErrorAction SilentlyContinue
if (-not $signatureFile) {
  $signatureFile = $assets |
    Where-Object { $_.Extension -eq ".sig" -and $_.BaseName -eq $updaterBundle.Name } |
    Select-Object -First 1
}
if (-not $signatureFile) {
  throw "No signature file found for updater bundle $($updaterBundle.Name)"
}

$assetBaseUrl = "https://github.com/$Repository/releases/download/$Tag"
$signature = (Get-Content -LiteralPath $signatureFile.FullName -Raw -Encoding UTF8).Trim()
if ([string]::IsNullOrWhiteSpace($signature)) {
  throw "Signature file $($signatureFile.Name) is empty"
}

$manifest = [ordered]@{
  version = $Version
  notes = "kukuri preview $Tag"
  pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = $signature
      url = "$assetBaseUrl/$($updaterBundle.Name)"
    }
  }
}
if ($IncludeLinux) { $manifest.platforms = $packagePlan.platforms }

$noticesSource = Join-Path $PSScriptRoot "..\..\docs\THIRD_PARTY_NOTICES.md"
if (-not (Test-Path -LiteralPath $noticesSource)) { throw 'Third-party notices are missing' }
if ((Test-Path -LiteralPath $OutputDir) -and @(Get-ChildItem -LiteralPath $OutputDir -Force).Count -gt 0) {
  throw 'Output directory is not empty; use a fresh staging directory'
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
foreach ($asset in $assets) {
  Copy-Item -LiteralPath $asset.FullName -Destination (Join-Path $OutputDir $asset.Name)
}
if ($IncludeLinux) {
  $provenance = [ordered]@{ source_commit = $SourceCommit; tag = $Tag; packages = $packagePlan.packages }
  [IO.File]::WriteAllText((Join-Path $OutputDir 'release-provenance.json'),
    ($provenance | ConvertTo-Json -Depth 12), [Text.UTF8Encoding]::new($false))
}

$manifestPath = Join-Path $OutputDir "latest-preview.json"
$manifestJson = $manifest | ConvertTo-Json -Depth 8
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText(
  $manifestPath,
  $manifestJson + [System.Environment]::NewLine,
  $utf8WithoutBom
)

$smokePath = Join-Path $OutputDir "manual-smoke-checklist.md"
@"
# Manual smoke checklist for $Tag

- Install the draft asset on a clean Windows 10 profile.
- Install the draft asset on a clean Windows 11 profile.
- Confirm launch, Community Node readiness, starter topic, public post, reply/thread, private channel, DM when a peer is available, local notification inbox, and diagnostic report export.
- Confirm an installed previous preview updates to $Tag and preserves identity, local DB, Iroh data, Community Node config, private channel capability, and notification inbox state.
- Publish the draft release only after these assets pass without replacement.
"@ | Set-Content -LiteralPath $smokePath -Encoding UTF8
if ($IncludeLinux) {
  @"
# Release verification for $Tag

- Match source $SourceCommit, version, target, checksums and embedded signatures to the final assets.
- Require the existing CI gates and x86_64/aarch64 CLI archive smoke; reuse matching #889 GUI/update evidence.
- Include reviewed native notices and corresponding source delivery material before publishing.
- Verify the stable public URLs after publication. Do not replace assets during verification.
- Additional manual desktop checks are only for concrete gaps that automated evidence cannot resolve.
"@ | Set-Content -LiteralPath $smokePath -Encoding UTF8
}

# Embed the changelog section for this tag (generated by update-changelog.ps1)
# so the release notes list the included changes with their pull request links.
$changesSection = ""
if ($ChangelogSectionPath) {
  if (Test-Path -LiteralPath $ChangelogSectionPath) {
    $changesContent = (Get-Content -LiteralPath $ChangelogSectionPath -Raw -Encoding UTF8).Trim()
    if ($changesContent) {
      $changesLines = $changesContent -split "`n"
      if ($changesLines.Length -gt 0 -and $changesLines[0] -match '^##\s+\[') {
        $changesLines[0] = "## Changes"
      }
      $changesSection = (($changesLines -join "`n").Trim()) + "`n`n"
    }
    else {
      Write-Warning "Changelog section file $ChangelogSectionPath is empty; release notes will omit the Changes section."
    }
  }
  else {
    Write-Warning "Changelog section file not found: $ChangelogSectionPath; release notes will omit the Changes section."
  }
}

$notesPath = Join-Path $OutputDir "RELEASE_NOTES_DRAFT.md"
@"
# kukuri $Tag

Preview channel: `preview`

This preview is distributed for Windows 10 and Windows 11 through GitHub Releases. Linux remains source-run only.

$changesSection## Included

- Windows NSIS installer.
- Tauri updater bundle and signature.
- `latest-preview.json` with the embedded `.sig` contents.
- `SHA256SUMS.txt`.
- `THIRD_PARTY_NOTICES.md`.
- Manual smoke checklist.

## Known limits

- This is not a general public stable release.
- macOS and Linux binary packages are not included.
- If Windows code signing certificates are not configured for this run, SmartScreen warnings are expected for this unsigned preview.

## Feedback

Use the in-app Release settings diagnostic report and attach it to the preview feedback issue template.
"@ | Set-Content -LiteralPath $notesPath -Encoding UTF8
if ($IncludeLinux) {
  $notes = Get-Content -LiteralPath $notesPath -Raw -Encoding UTF8
  $notes = $notes.Replace('This preview is distributed for Windows 10 and Windows 11 through GitHub Releases. Linux remains source-run only.',
    'This preview includes Windows NSIS, Linux x86_64 AppImage, and Linux x86_64/aarch64 CLI archives.')
  $notes = $notes.Replace('- macOS and Linux binary packages are not included.',
    '- macOS and aarch64 GUI packages are not included. Additional Linux desktop/device environments are unverified.')
  $notes = $notes.Replace('- Manual smoke checklist.', '- Release verification checklist and native notice/source material.')
  $notes | Set-Content -LiteralPath $notesPath -Encoding UTF8
}

$noticesSource = Join-Path $PSScriptRoot "..\..\docs\THIRD_PARTY_NOTICES.md"
if (-not (Test-Path -LiteralPath $noticesSource)) {
  throw "Third-party notices file not found: $noticesSource"
}
Copy-Item -LiteralPath $noticesSource -Destination (Join-Path $OutputDir "THIRD_PARTY_NOTICES.md") -Force

$assetListPath = Join-Path $OutputDir "release-assets.txt"
(@(Get-ChildItem -LiteralPath $OutputDir -File | ForEach-Object Name) + @('SHA256SUMS.txt', 'release-assets.txt')) |
  Sort-Object -Unique | Set-Content -LiteralPath $assetListPath -Encoding UTF8
$checksumPath = Join-Path $OutputDir "SHA256SUMS.txt"
Get-ChildItem -LiteralPath $OutputDir -File |
  Where-Object { $_.Name -ne "SHA256SUMS.txt" } |
  Sort-Object Name |
  ForEach-Object {
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName
    "$($hash.Hash.ToLowerInvariant())  $($_.Name)"
  } | Set-Content -LiteralPath $checksumPath -Encoding UTF8

Write-Host "Generated release assets in $OutputDir"
