param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v\d+\.\d+\.\d+-preview\.\d+$')]
    [string]$Tag
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$workDir = Join-Path $tempRoot ("kukuri-updater-signature-" + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($workDir) | Out-Null

try {
    $manifestPath = Join-Path $workDir 'latest-preview.json'
    $bundlePath = Join-Path $workDir 'updater-bundle.zip'
    $signaturePath = Join-Path $workDir 'updater-bundle.sig'
    $manifestUrl = "https://github.com/KingYoSun/kukuri/releases/download/$Tag/latest-preview.json"

    Invoke-WebRequest -UseBasicParsing -Uri $manifestUrl -OutFile $manifestPath
    $manifest = Get-Content -Raw -Encoding UTF8 $manifestPath | ConvertFrom-Json
    $platform = $manifest.platforms.'windows-x86_64'
    if (-not $platform.url -or -not $platform.signature) {
        throw 'latest-preview.json is missing windows-x86_64 url or embedded signature'
    }

    Invoke-WebRequest -UseBasicParsing -Uri $platform.url -OutFile $bundlePath
    [IO.File]::WriteAllText(
        $signaturePath,
        [string]$platform.signature,
        [Text.UTF8Encoding]::new($false)
    )

    $env:KUKURI_UPDATER_BUNDLE = $bundlePath
    $env:KUKURI_UPDATER_SIGNATURE = $signaturePath
    cargo test `
        --manifest-path (Join-Path $repositoryRoot 'apps\desktop\src-tauri\Cargo.toml') `
        --test updater_signature `
        -- `
        --ignored `
        --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "published updater signature smoke failed with exit code $LASTEXITCODE"
    }
}
finally {
    Remove-Item Env:KUKURI_UPDATER_BUNDLE -ErrorAction SilentlyContinue
    Remove-Item Env:KUKURI_UPDATER_SIGNATURE -ErrorAction SilentlyContinue
    $resolvedWorkDir = [IO.Path]::GetFullPath($workDir)
    if ($resolvedWorkDir.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        [IO.Directory]::Delete($resolvedWorkDir, $true)
    }
}
