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
  [string]$OutputDir
)

$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$assets = Get-ChildItem -LiteralPath $InputDir -File -Recurse |
  Where-Object { $_.Extension -in @(".exe", ".msi", ".zip", ".sig") } |
  Sort-Object Name

if ($assets.Count -eq 0) {
  throw "No Windows release assets found in $InputDir"
}

foreach ($asset in $assets) {
  Copy-Item -LiteralPath $asset.FullName -Destination (Join-Path $OutputDir $asset.Name) -Force
}

$copiedAssets = Get-ChildItem -LiteralPath $OutputDir -File | Sort-Object Name
$updaterBundle = $copiedAssets | Where-Object { $_.Extension -eq ".zip" } | Select-Object -First 1
if (-not $updaterBundle) {
  $updaterBundle = $copiedAssets | Where-Object { $_.Extension -eq ".exe" } | Select-Object -First 1
}
if (-not $updaterBundle) {
  throw "No updater bundle candidate (.zip or .exe) found in $OutputDir"
}

$signatureFile = Get-Item -LiteralPath "$($updaterBundle.FullName).sig" -ErrorAction SilentlyContinue
if (-not $signatureFile) {
  $signatureFile = $copiedAssets |
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

$manifestPath = Join-Path $OutputDir "latest-preview.json"
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

$smokePath = Join-Path $OutputDir "manual-smoke-checklist.md"
@"
# Manual smoke checklist for $Tag

- Install the draft asset on a clean Windows 10 profile.
- Install the draft asset on a clean Windows 11 profile.
- Confirm launch, Community Node readiness, starter topic, public post, reply/thread, private channel, DM when a peer is available, local notification inbox, and diagnostic report export.
- Confirm an installed previous preview updates to $Tag and preserves identity, local DB, Iroh data, Community Node config, private channel capability, and notification inbox state.
- Publish the draft release only after these assets pass without replacement.
"@ | Set-Content -LiteralPath $smokePath -Encoding UTF8

$notesPath = Join-Path $OutputDir "RELEASE_NOTES_DRAFT.md"
@"
# kukuri $Tag

Preview channel: `preview`

This preview is distributed for Windows 10 and Windows 11 through GitHub Releases. Linux remains source-run only.

## Included

- Windows NSIS installer.
- Tauri updater bundle and signature.
- `latest-preview.json` with the embedded `.sig` contents.
- `SHA256SUMS.txt`.
- Manual smoke checklist.

## Known limits

- This is not a general public stable release.
- macOS and Linux binary packages are not included.
- If Windows code signing certificates are not configured for this run, SmartScreen warnings are expected for this unsigned preview.

## Feedback

Use the in-app Release settings diagnostic report and attach it to the preview feedback issue template.
"@ | Set-Content -LiteralPath $notesPath -Encoding UTF8

$checksumPath = Join-Path $OutputDir "SHA256SUMS.txt"
Get-ChildItem -LiteralPath $OutputDir -File |
  Where-Object { $_.Name -ne "SHA256SUMS.txt" } |
  Sort-Object Name |
  ForEach-Object {
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName
    "$($hash.Hash.ToLowerInvariant())  $($_.Name)"
  } | Set-Content -LiteralPath $checksumPath -Encoding UTF8

$assetListPath = Join-Path $OutputDir "release-assets.txt"
Get-ChildItem -LiteralPath $OutputDir -File |
  Sort-Object Name |
  ForEach-Object { $_.Name } |
  Set-Content -LiteralPath $assetListPath -Encoding UTF8

Write-Host "Generated release assets in $OutputDir"
