$ErrorActionPreference = "Stop"

$scriptPath = Join-Path $PSScriptRoot "generate-third-party-notices.ps1"
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("kukuri-third-party-notices-test-" + [System.Guid]::NewGuid())
$rustMetadataPath = Join-Path $workDir "cargo-metadata.json"
$npmLicensesPath = Join-Path $workDir "pnpm-licenses.json"
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

  & $scriptPath `
    -RustMetadataPath $rustMetadataPath `
    -NpmLicensesPath $npmLicensesPath `
    -OutputPath $outputPath

  & $scriptPath `
    -RustMetadataPath $rustMetadataPath `
    -NpmLicensesPath $npmLicensesPath `
    -OutputPath $outputPath `
    -Check

  $content = Get-Content -LiteralPath $outputPath -Raw -Encoding UTF8
  foreach ($requiredText in @(
      "# Third-party notices",
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
} finally {
  if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
  }
}
