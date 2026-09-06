$ErrorActionPreference = "Stop"

$scriptPath = Join-Path $PSScriptRoot "generate-third-party-notices.ps1"
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("kukuri-third-party-notices-test-" + [System.Guid]::NewGuid())
$rustMetadataPath = Join-Path $workDir "cargo-metadata.json"
$tauriMetadataPath = Join-Path $workDir "tauri-metadata.json"
$npmLicensesPath = Join-Path $workDir "pnpm-licenses.json"
$assetManifestPath = Join-Path $workDir "asset-manifest.json"
$outputPath = Join-Path $workDir "THIRD_PARTY_NOTICES.md"

try {
  New-Item -ItemType Directory -Force -Path $workDir | Out-Null

  @"
{
  "packages": [
    {
      "name": "kukuri-core",
      "version": "0.1.1",
      "license": "MIT",
      "source": null
    },
    {
      "name": "anyhow",
      "version": "1.0.102",
      "license": "MIT OR Apache-2.0",
      "source": "registry+https://github.com/rust-lang/crates.io-index"
    },
    {
      "name": "serde",
      "version": "1.0.228",
      "license": "MIT OR Apache-2.0",
      "source": "registry+https://github.com/rust-lang/crates.io-index"
    }
  ]
}
"@ | Set-Content -LiteralPath $rustMetadataPath -Encoding UTF8

  @"
{"packages":[
  {"name":"kukuri-desktop-tauri","version":"0.1.8","license":"MIT","source":null},
  {"name":"serde","version":"1.0.228","license":"MIT OR Apache-2.0","source":"registry+https://github.com/rust-lang/crates.io-index"},
  {"name":"ashpd","version":"0.13.8","license":"MIT","source":"registry+https://github.com/rust-lang/crates.io-index"}
]}
"@ | Set-Content -LiteralPath $tauriMetadataPath -Encoding UTF8

  @"
{
  "MIT": [
    {
      "name": "react",
      "versions": ["19.2.5"],
      "license": "MIT",
      "homepage": "https://react.dev/"
    }
  ],
  "Apache-2.0": [
    {
      "name": "typescript",
      "versions": ["6.0.3"],
      "license": "Apache-2.0",
      "homepage": "https://www.typescriptlang.org/"
    }
  ]
}
"@ | Set-Content -LiteralPath $npmLicensesPath -Encoding UTF8

  @"
{
  "schema_version": 1,
  "assets": [
    {
      "id": "app-icon",
      "display_name": "kukuri application icon",
      "origin": "authored",
      "author": "KingYoSun",
      "rights_holder": "KingYoSun",
      "source": { "created_on": "2026-03-26" },
      "license": {
        "id": "MIT",
        "text_path": "LICENSE",
        "commercial_use": true,
        "repository_redistribution": true,
        "binary_redistribution": true,
        "attribution_required": true,
        "attribution": "Copyright (c) 2025 KingYoSun"
      },
      "modification": { "modified": true, "description": "Derived platform sizes." },
      "generation": null,
      "files": [
        { "path": "apps/desktop/app-icon.png", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "distribution": "source-only" },
        { "path": "apps/desktop/src-tauri/icons/icon.ico", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "distribution": "bundled-binary" }
      ]
    },
    {
      "id": "idle-loop",
      "display_name": "Idle Loop VRMA",
      "origin": "third_party",
      "author": "Quaternius",
      "rights_holder": "Quaternius",
      "source": { "url": "https://quaternius.com/packs/universalanimationlibrary.html", "acquired_on": "2026-05-29" },
      "license": {
        "id": "CC0-1.0",
        "text_url": "https://creativecommons.org/publicdomain/zero/1.0/",
        "commercial_use": true,
        "repository_redistribution": true,
        "binary_redistribution": true,
        "attribution_required": false,
        "attribution": "Universal Animation Library by Quaternius (courtesy credit)"
      },
      "modification": { "modified": false },
      "generation": null,
      "files": [
        { "path": "apps/desktop/public/animation/Idle_Loop.vrma", "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "distribution": "bundled-binary" }
      ]
    }
  ]
}
"@ | Set-Content -LiteralPath $assetManifestPath -Encoding UTF8

  & $scriptPath `
    -RustMetadataPath $rustMetadataPath `
    -NpmLicensesPath $npmLicensesPath `
    -AssetManifestPath $assetManifestPath `
    -OutputPath $outputPath

  & $scriptPath `
    -RustMetadataPath $rustMetadataPath `
    -NpmLicensesPath $npmLicensesPath `
    -AssetManifestPath $assetManifestPath `
    -OutputPath $outputPath `
    -Check

  $content = Get-Content -LiteralPath $outputPath -Raw -Encoding UTF8
  foreach ($requiredText in @(
      "# Third-party notices",
      "## Non-code asset notices",
      "### Bundled first-party assets",
      "kukuri application icon",
      "source-only: apps/desktop/app-icon.png; bundled-binary: apps/desktop/src-tauri/icons/icon.ico",
      "### Bundled third-party assets",
      "Idle Loop VRMA",
      "CC0-1.0 (https://creativecommons.org/publicdomain/zero/1.0/)",
      "Not required; Universal Animation Library by Quaternius (courtesy credit)",
      "### Bundled generated or generation-assisted assets",
      "### Source-only non-code assets",
      "| anyhow | 1.0.102 | MIT OR Apache-2.0 | https://crates.io/crates/anyhow |",
      "| serde | 1.0.228 | MIT OR Apache-2.0 | https://crates.io/crates/serde |",
      "| react | 19.2.5 | MIT | https://react.dev/ |",
      "| typescript | 6.0.3 | Apache-2.0 | https://www.typescriptlang.org/ |"
    )) {
    if (-not $content.Contains($requiredText)) {
      throw "Generated notices missing expected text: $requiredText"
    }
  }
  if ($content.Contains("kukuri-core")) {
    throw "Generated notices should exclude workspace packages"
  }

  Write-Host "generate-third-party-notices smoke test passed"

  & $scriptPath `
    -RustMetadataPath @($rustMetadataPath, $tauriMetadataPath) `
    -NpmLicensesPath $npmLicensesPath `
    -AssetManifestPath $assetManifestPath `
    -OutputPath $outputPath
  $combined = Get-Content -LiteralPath $outputPath -Raw -Encoding UTF8
  if (-not $combined.Contains("| ashpd | 0.13.8 | MIT | https://crates.io/crates/ashpd |")) {
    throw "Generated notices omit the separate Tauri workspace"
  }
  if ([regex]::Matches($combined, '\| serde \| 1\.0\.228 \|').Count -ne 1) {
    throw "Shared packages must appear exactly once"
  }
  if ($combined.Contains("kukuri-desktop-tauri")) {
    throw "Generated notices should exclude the Tauri workspace package"
  }
  & $scriptPath `
    -RustMetadataPath @($tauriMetadataPath, $rustMetadataPath) `
    -NpmLicensesPath $npmLicensesPath `
    -AssetManifestPath $assetManifestPath `
    -OutputPath $outputPath `
    -Check
  Write-Host "Separate-workspace notice test passed"

  # Missing metadata and desktop-only UNKNOWN licenses must fail closed.
  $missingMetadataFailed = $false
  try {
    & $scriptPath -RustMetadataPath @($rustMetadataPath, (Join-Path $workDir "absent.json")) `
      -NpmLicensesPath $npmLicensesPath -AssetManifestPath $assetManifestPath -OutputPath $outputPath
  } catch {
    $missingMetadataFailed = $true
  }
  if (-not $missingMetadataFailed) { throw "Missing workspace metadata was accepted" }
  $unknownMetadata = Get-Content -LiteralPath $tauriMetadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $unknownMetadata.packages[2].license = $null
  $unknownMetadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $tauriMetadataPath -Encoding UTF8
  $unknownFailed = $false
  try {
    & $scriptPath -RustMetadataPath @($rustMetadataPath, $tauriMetadataPath) `
      -NpmLicensesPath $npmLicensesPath -AssetManifestPath $assetManifestPath -OutputPath $outputPath
  } catch {
    if (-not $_.Exception.Message.Contains("UNKNOWN licenses: ashpd@0.13.8")) { throw }
    $unknownFailed = $true
  }
  if (-not $unknownFailed) { throw "Desktop-only UNKNOWN license was accepted" }
  if ((Get-Content -LiteralPath $outputPath -Raw -Encoding UTF8) -cne $combined) {
    throw "Failed inventory generation changed the previous notice"
  }
  Write-Host "Invalid metadata and UNKNOWN license rejection tests passed"
} finally {
  if (Test-Path -LiteralPath $workDir) {
    $resolvedWorkDir = [System.IO.Path]::GetFullPath($workDir)
    $temporaryRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolvedWorkDir.StartsWith($temporaryRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not ([System.IO.Path]::GetFileName($resolvedWorkDir)).StartsWith("kukuri-third-party-notices-test-")) {
      throw "Refusing to remove an unexpected test directory: $resolvedWorkDir"
    }
    Remove-Item -LiteralPath $workDir -Recurse -Force
  }
}
