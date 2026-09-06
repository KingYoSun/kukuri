param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v\d+\.\d+\.\d+-preview\.\d+$')]
    [string]$Tag,
    [ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')]
    [string]$Repository = 'KingYoSun/kukuri',
    [ValidateSet('windows-x86_64', 'linux-x86_64')]
    [string[]]$Platforms = @('windows-x86_64'),
    [string]$InputDir,
    [string]$PublicKeyFile,
    [string]$VerifierExecutable
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$workDir = Join-Path $tempRoot ("kukuri-updater-signature-" + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($workDir) | Out-Null
$priorEnvironment = @{}
foreach ($name in 'KUKURI_UPDATER_BUNDLE', 'KUKURI_UPDATER_SIGNATURE', 'KUKURI_UPDATER_PUBLIC_KEY_FILE') {
    $priorEnvironment[$name] = [Environment]::GetEnvironmentVariable($name)
}

try {
    $manifestPath = Join-Path $workDir 'latest-preview.json'
    $assetBase = "https://github.com/$Repository/releases/download/$Tag/"
    $manifestUrl = "${assetBase}latest-preview.json"

    if ($InputDir) {
        Copy-Item -LiteralPath (Join-Path $InputDir 'latest-preview.json') -Destination $manifestPath
    } else {
        Invoke-WebRequest -UseBasicParsing -Uri $manifestUrl -OutFile $manifestPath
    }
    $bytes = [IO.File]::ReadAllBytes($manifestPath)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        throw 'Updater manifest must not contain a UTF-8 BOM'
    }
    $manifest = Get-Content -Raw -Encoding UTF8 $manifestPath | ConvertFrom-Json
    if ($manifest.version -ne ($Tag -replace '^v', '' -replace '-preview\.[0-9]+$', '')) {
        throw 'Updater manifest version does not match tag'
    }
    if ($PublicKeyFile) { $env:KUKURI_UPDATER_PUBLIC_KEY_FILE = (Resolve-Path -LiteralPath $PublicKeyFile).Path }
    else { [Environment]::SetEnvironmentVariable('KUKURI_UPDATER_PUBLIC_KEY_FILE', $null) }
    foreach ($target in $Platforms) {
        $platform = $manifest.platforms.$target
        if (-not $platform.url -or -not $platform.signature) { throw "Missing updater platform: $target" }
        $url = [Uri]$platform.url
        $name = [Uri]::UnescapeDataString($url.Segments[-1])
        if (-not ([string]$platform.url).StartsWith($assetBase, [StringComparison]::Ordinal) -or
            $url.Query -or $url.Fragment -or $name -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') {
            throw 'Updater asset URL must belong to the selected release'
        }
        $bundlePath = Join-Path $workDir "$target.bundle"
        $signaturePath = Join-Path $workDir "$target.sig"
        if ($InputDir) {
            $local = Join-Path $InputDir $name
            $hash = (Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant()
            $sums = Get-Content -LiteralPath (Join-Path $InputDir 'SHA256SUMS.txt')
            if ($sums -notcontains "$hash  $name") { throw 'Staging updater checksum mismatch' }
            Copy-Item -LiteralPath $local -Destination $bundlePath
        } else {
            Invoke-WebRequest -UseBasicParsing -Uri $platform.url -OutFile $bundlePath
        }
        [IO.File]::WriteAllText($signaturePath, [string]$platform.signature, [Text.UTF8Encoding]::new($false))
        $env:KUKURI_UPDATER_BUNDLE = $bundlePath
        $env:KUKURI_UPDATER_SIGNATURE = $signaturePath
        if ($VerifierExecutable) {
            $verification = & $VerifierExecutable --ignored --exact published_bundle_accepts_only_its_valid_signature --color never
        } else {
            $verification = cargo test --manifest-path (Join-Path $repositoryRoot 'apps\desktop\src-tauri\Cargo.toml') `
                --test updater_signature -- --ignored --exact published_bundle_accepts_only_its_valid_signature --color never
        }
        if ($LASTEXITCODE -ne 0 -or ($verification -join "`n") -notmatch 'test result: ok\. 1 passed; 0 failed; 0 ignored;') {
            throw "Updater signature verification did not pass exactly one test: $target"
        }
        Write-Host "Verified updater: $target"
    }

}
finally {
    foreach ($name in $priorEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name, $priorEnvironment[$name]) }
    $resolvedWorkDir = [IO.Path]::GetFullPath($workDir)
    if ($resolvedWorkDir.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        [IO.Directory]::Delete($resolvedWorkDir, $true)
    }
}
