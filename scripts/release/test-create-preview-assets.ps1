$ErrorActionPreference = "Stop"

$scriptPath = Join-Path $PSScriptRoot "create-preview-assets.ps1"
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("kukuri-release-assets-test-" + [System.Guid]::NewGuid())
$inputDir = Join-Path $workDir "input"
$outputDir = Join-Path $workDir "output"

try {
  New-Item -ItemType Directory -Force -Path $inputDir | Out-Null
  Set-Content -LiteralPath (Join-Path $inputDir "kukuri_0.1.0_x64-setup.exe") -Value "installer" -Encoding UTF8
  Set-Content -LiteralPath (Join-Path $inputDir "kukuri_0.1.0_x64.msi") -Value "msi" -Encoding UTF8
  Set-Content -LiteralPath (Join-Path $inputDir "kukuri_0.1.0_x64.zip") -Value "updater" -Encoding UTF8
  Set-Content -LiteralPath (Join-Path $inputDir "kukuri_0.1.0_x64.zip.sig") -Value "test-signature" -Encoding UTF8

  $sectionPath = Join-Path $workDir "CHANGELOG_SECTION.md"
  @"
## [v0.1.0-preview.1] - 2026-06-15

### Features

- topic一覧にsearch/filter/sort機能を追加 ([#340](https://github.com/KingYoSun/kukuri/pull/340))
"@ | Set-Content -LiteralPath $sectionPath -Encoding UTF8

  & $scriptPath `
    -Tag "v0.1.0-preview.1" `
    -Repository "KingYoSun/kukuri" `
    -Version "0.1.0" `
    -InputDir $inputDir `
    -OutputDir $outputDir `
    -ChangelogSectionPath $sectionPath

  $manifestPath = Join-Path $outputDir "latest-preview.json"
  $checksumPath = Join-Path $outputDir "SHA256SUMS.txt"
  $assetListPath = Join-Path $outputDir "release-assets.txt"
  $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $platform = $manifest.platforms.'windows-x86_64'

  if ($manifest.version -ne "0.1.0") {
    throw "Unexpected manifest version: $($manifest.version)"
  }
  if ($platform.signature -ne "test-signature") {
    throw "Manifest signature did not embed .sig contents"
  }
  if ($platform.url -ne "https://github.com/KingYoSun/kukuri/releases/download/v0.1.0-preview.1/kukuri_0.1.0_x64.zip") {
    throw "Unexpected updater URL: $($platform.url)"
  }

  $checksums = Get-Content -LiteralPath $checksumPath -Encoding UTF8
  foreach ($requiredName in @(
      "kukuri_0.1.0_x64-setup.exe",
      "kukuri_0.1.0_x64.zip",
      "latest-preview.json"
    )) {
    if (-not ($checksums | Where-Object { $_ -like "*  $requiredName" })) {
      throw "Missing checksum for $requiredName"
    }
  }

  $assetNames = Get-Content -LiteralPath $assetListPath -Encoding UTF8
  foreach ($requiredName in @(
      "RELEASE_NOTES_DRAFT.md",
      "manual-smoke-checklist.md",
      "SHA256SUMS.txt",
      "THIRD_PARTY_NOTICES.md"
    )) {
    if ($assetNames -notcontains $requiredName) {
      throw "Missing release asset list entry for $requiredName"
    }
  }

  $notes = Get-Content -LiteralPath (Join-Path $outputDir "RELEASE_NOTES_DRAFT.md") -Raw -Encoding UTF8
  if ($notes -notmatch '## Changes') {
    throw "Release notes are missing the Changes section"
  }
  if ($notes -notmatch '\[#340\]\(https://github\.com/KingYoSun/kukuri/pull/340\)') {
    throw "Release notes did not embed the changelog PR link"
  }
  if ($notes -notmatch '## Included') {
    throw "Release notes lost the Included section after embedding changes"
  }

  Write-Host "create-preview-assets smoke test passed"
} finally {
  if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
  }
}
