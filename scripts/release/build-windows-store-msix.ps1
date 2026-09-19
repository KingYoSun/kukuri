param(
    [switch]$SkipBuild,
    [switch]$AllowDirty,
    [switch]$SignForLocalTest,
    [switch]$PromptForCertificatePassword,
    [string]$CertificatePath = "code_sign_certificate.pfx",
    [string]$OutputDirectory = "dist/microsoft-store"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))
$desktopDir = Join-Path $repoRoot "apps/desktop"
$manifestPath = Join-Path $desktopDir "src-tauri/windows/store/Package.appxmanifest"
$targetBinary = Join-Path $desktopDir "src-tauri/target/x86_64-pc-windows-msvc/release/kukuri-desktop-tauri.exe"
$requiredWinAppVersion = "0.6.1"

function Resolve-WorkspacePath([string]$Path) {
    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }
    return [IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Assert-WorkspaceChild([string]$Path, [string]$Label) {
    $rootPrefix = $repoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $Path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label must stay inside the repository workspace"
    }
}

function Invoke-Native([string]$Executable, [string[]]$Arguments, [string]$Label) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

function Find-SignTool {
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "$env:ProgramFiles\Windows Kits\10\bin"
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
    $candidates = foreach ($root in $roots) {
        Get-ChildItem -LiteralPath $root -Filter signtool.exe -File -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.DirectoryName -match '[\\/]x64$' }
    }
    $tool = $candidates | Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $tool) {
        throw "SignTool.exe was not found in the Windows SDK"
    }
    return $tool.FullName
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Store manifest is missing: $manifestPath"
}

$winappCommand = Get-Command winapp -ErrorAction Stop
$winappVersionText = (& $winappCommand.Source --version 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "winapp --version failed"
}
$versionMatches = [regex]::Matches($winappVersionText, '(?m)^([0-9]+\.[0-9]+\.[0-9]+)\s*$')
if ($versionMatches.Count -eq 0) {
    throw "Could not parse the WinApp CLI version"
}
$winappVersion = $versionMatches[$versionMatches.Count - 1].Groups[1].Value
if ($winappVersion -ne $requiredWinAppVersion) {
    throw "WinApp CLI $requiredWinAppVersion is required; found $winappVersion"
}

$osBuild = [Environment]::OSVersion.Version.Build
if ($osBuild -lt 19041) {
    throw "Windows build 19041 or newer is required for the Store package target; found $osBuild"
}

[xml]$manifest = Get-Content -LiteralPath $manifestPath -Raw
$namespace = New-Object Xml.XmlNamespaceManager($manifest.NameTable)
$namespace.AddNamespace("f", "http://schemas.microsoft.com/appx/manifest/foundation/windows10")
$identity = $manifest.SelectSingleNode("/f:Package/f:Identity", $namespace)
$publisherDisplayName = $manifest.SelectSingleNode("/f:Package/f:Properties/f:PublisherDisplayName", $namespace).InnerText
if (-not $identity) {
    throw "Store manifest Identity is missing"
}
$packageName = [string]$identity.Name
$publisher = [string]$identity.Publisher
$storeVersion = [string]$identity.Version
$architecture = [string]$identity.ProcessorArchitecture
if ($packageName -ne "KingYoSun.kukuri" -or
    $publisher -ne "CN=33EB763C-4859-4E44-886F-1784E16DD6D5" -or
    $publisherDisplayName -ne "KingYoSun" -or
    $storeVersion -ne "1.0.0.0" -or
    $architecture -ne "x64") {
    throw "Store manifest identity does not match the approved Partner Center product"
}

$gitStatus = (& git -C $repoRoot status --porcelain | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "git status failed"
}
if ($gitStatus -and -not $AllowDirty) {
    throw "Store packages require a clean worktree (use -AllowDirty only for local development validation)"
}
$sourceCommit = (& git -C $repoRoot rev-parse HEAD | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') {
    throw "Could not resolve the source commit"
}

$outputDir = Resolve-WorkspacePath $OutputDirectory
Assert-WorkspaceChild $outputDir "OutputDirectory"
if (Test-Path -LiteralPath $outputDir) {
    Remove-Item -LiteralPath $outputDir -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDir | Out-Null
$stagingDir = Join-Path $outputDir "staging"
$assetDir = Join-Path $stagingDir "Assets"
New-Item -ItemType Directory -Path $assetDir -Force | Out-Null

if (-not $SkipBuild) {
    $previousDistribution = $env:VITE_KUKURI_DISTRIBUTION
    $previousTelemetry = $env:WINAPP_CLI_TELEMETRY_OPTOUT
    try {
        $env:VITE_KUKURI_DISTRIBUTION = "microsoft-store"
        $env:WINAPP_CLI_TELEMETRY_OPTOUT = "1"
        Push-Location $desktopDir
        try {
            Invoke-Native "npx" @(
                "pnpm@10.16.1", "tauri", "build",
                "--target", "x86_64-pc-windows-msvc",
                "--features", "microsoft-store",
                "--no-bundle",
                "--config", "src-tauri/tauri.microsoft-store.conf.json",
                "--ci"
            ) "Tauri Microsoft Store build"
        }
        finally {
            Pop-Location
        }
    }
    finally {
        $env:VITE_KUKURI_DISTRIBUTION = $previousDistribution
        $env:WINAPP_CLI_TELEMETRY_OPTOUT = $previousTelemetry
    }
}

if (-not (Test-Path -LiteralPath $targetBinary -PathType Leaf)) {
    throw "Store build binary is missing: $targetBinary"
}
Copy-Item -LiteralPath $targetBinary -Destination (Join-Path $stagingDir "kukuri.exe")
foreach ($asset in @("StoreLogo.png", "Square44x44Logo.png", "Square150x150Logo.png")) {
    Copy-Item -LiteralPath (Join-Path $desktopDir "src-tauri/icons/$asset") -Destination (Join-Path $assetDir $asset)
}

$unsignedName = "${packageName}_${storeVersion}_${architecture}.msix"
$unsignedPath = Join-Path $outputDir $unsignedName
$previousTelemetry = $env:WINAPP_CLI_TELEMETRY_OPTOUT
try {
    $env:WINAPP_CLI_TELEMETRY_OPTOUT = "1"
    Invoke-Native $winappCommand.Source @(
        "pack", $stagingDir,
        "--manifest", $manifestPath,
        "--executable", "kukuri.exe",
        "--output", $unsignedPath,
        "--quiet"
    ) "WinApp CLI packaging"
}
finally {
    $env:WINAPP_CLI_TELEMETRY_OPTOUT = $previousTelemetry
}
if (-not (Test-Path -LiteralPath $unsignedPath -PathType Leaf)) {
    throw "WinApp CLI did not create the expected MSIX: $unsignedPath"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($unsignedPath)
try {
    $actualEntries = @($archive.Entries | ForEach-Object { $_.FullName } | Sort-Object)
    $expectedEntries = @(
        "[Content_Types].xml",
        "AppxBlockMap.xml",
        "AppxManifest.xml",
        "Assets/Square150x150Logo.png",
        "Assets/Square44x44Logo.png",
        "Assets/StoreLogo.png",
        "kukuri.exe",
        "pri.resfiles",
        "priconfig.xml",
        "resources.pri"
    ) | Sort-Object
    if (Compare-Object $expectedEntries $actualEntries) {
        throw "MSIX payload does not match the fixed allowlist"
    }
    $manifestEntry = $archive.GetEntry("AppxManifest.xml")
    $blockMapEntry = $archive.GetEntry("AppxBlockMap.xml")
    if (-not $manifestEntry -or -not $blockMapEntry) {
        throw "MSIX metadata is incomplete"
    }
    $reader = New-Object IO.StreamReader($manifestEntry.Open())
    try { [xml]$packedManifest = $reader.ReadToEnd() } finally { $reader.Dispose() }
    $packedNs = New-Object Xml.XmlNamespaceManager($packedManifest.NameTable)
    $packedNs.AddNamespace("f", "http://schemas.microsoft.com/appx/manifest/foundation/windows10")
    $packedIdentity = $packedManifest.SelectSingleNode("/f:Package/f:Identity", $packedNs)
    if ($packedIdentity.Name -ne $packageName -or
        $packedIdentity.Publisher -ne $publisher -or
        $packedIdentity.Version -ne $storeVersion -or
        $packedIdentity.ProcessorArchitecture -ne $architecture) {
        throw "Packed MSIX identity differs from the approved manifest"
    }
    $reader = New-Object IO.StreamReader($blockMapEntry.Open())
    try { [xml]$blockMap = $reader.ReadToEnd() } finally { $reader.Dispose() }
    if ($blockMap.BlockMap.HashMethod -ne "http://www.w3.org/2001/04/xmlenc#sha256") {
        throw "MSIX block map must use SHA-256"
    }
}
finally {
    $archive.Dispose()
}

$appPackage = Get-Content -LiteralPath (Join-Path $desktopDir "package.json") -Raw | ConvertFrom-Json
$unsignedHash = (Get-FileHash -LiteralPath $unsignedPath -Algorithm SHA256).Hash.ToLowerInvariant()
$provenance = [ordered]@{
    schema_version = 1
    source_commit = $sourceCommit
    dirty = [bool]$gitStatus
    app_version = [string]$appPackage.version
    store_version = $storeVersion
    package_name = $packageName
    publisher = $publisher
    publisher_display_name = $publisherDisplayName
    package_family_name = "KingYoSun.kukuri_p8fpcaf1kx88g"
    store_id = "9NQ18HML4GS3"
    architecture = $architecture
    winapp_cli_version = $winappVersion
    unsigned = [ordered]@{
        file = $unsignedName
        sha256 = $unsignedHash
        bytes = (Get-Item -LiteralPath $unsignedPath).Length
    }
    signed_local_test = $null
}
$provenancePath = Join-Path $outputDir "store-package.json"
$json = $provenance | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText($provenancePath, $json + "`n", [Text.UTF8Encoding]::new($false))
$signedPath = $null
$certificateOutputPath = $null

if ($SignForLocalTest) {
    $resolvedCertificate = Resolve-WorkspacePath $CertificatePath
    Assert-WorkspaceChild $resolvedCertificate "CertificatePath"
    if (-not (Test-Path -LiteralPath $resolvedCertificate -PathType Leaf)) {
        throw "Local-test signing certificate is missing"
    }
    $password = if ($PromptForCertificatePassword) {
        Read-Host "PFX password" -AsSecureString
    }
    elseif ($null -eq $env:KUKURI_MSIX_CERT_PASSWORD) {
        [Security.SecureString]::new()
    }
    else {
        ConvertTo-SecureString -String $env:KUKURI_MSIX_CERT_PASSWORD -AsPlainText -Force
    }
    $beforeThumbprints = @(Get-ChildItem Cert:\CurrentUser\My | ForEach-Object { $_.Thumbprint })
    $imported = $null
    $importedCertificates = @()
    $signingSucceeded = $false
    try {
        $importedCertificates = @(Import-PfxCertificate -FilePath $resolvedCertificate -CertStoreLocation Cert:\CurrentUser\My -Password $password -Exportable:$false)
        $privateKeyCertificates = @($importedCertificates | Where-Object { $_.HasPrivateKey })
        if ($privateKeyCertificates.Count -ne 1) {
            throw "The local-test PFX must contain exactly one certificate with a private key"
        }
        $imported = $privateKeyCertificates[0]
        if ($imported.Subject -ne $publisher) {
            throw "The certificate subject does not match the Store manifest Publisher"
        }
        $now = Get-Date
        if ($imported.NotBefore -gt $now -or $imported.NotAfter -le $now) {
            throw "The local-test certificate is outside its validity period"
        }
        $eku = @($imported.EnhancedKeyUsageList | ForEach-Object {
            if ($_.ObjectId -is [Security.Cryptography.Oid]) {
                $_.ObjectId.Value
            }
            else {
                [string]$_.ObjectId
            }
        })
        if ($eku.Count -gt 0 -and $eku -notcontains "1.3.6.1.5.5.7.3.3") {
            throw "The local-test certificate is not valid for code signing"
        }
        $signedName = "${packageName}_${storeVersion}_${architecture}_local-test-signed.msix"
        $signedPath = Join-Path $outputDir $signedName
        Copy-Item -LiteralPath $unsignedPath -Destination $signedPath
        $signTool = Find-SignTool
        Invoke-Native $signTool @(
            "sign", "/fd", "SHA256", "/sha1", $imported.Thumbprint, "/s", "My", $signedPath
        ) "MSIX local-test signing"
        Invoke-Native $signTool @("verify", "/pa", "/all", "/v", $signedPath) "MSIX signature verification"
        $certificateName = "${packageName}_local-test.cer"
        $certificateOutputPath = Join-Path $outputDir $certificateName
        Export-Certificate -Cert $imported -FilePath $certificateOutputPath -Force | Out-Null
        $provenance.signed_local_test = [ordered]@{
            file = $signedName
            sha256 = (Get-FileHash -LiteralPath $signedPath -Algorithm SHA256).Hash.ToLowerInvariant()
            certificate_file = $certificateName
            certificate_thumbprint = $imported.Thumbprint
            certificate_not_after = $imported.NotAfter.ToUniversalTime().ToString("o")
        }
        $signingSucceeded = $true
    }
    finally {
        if (-not $signingSucceeded -and $signedPath -and (Test-Path -LiteralPath $signedPath)) {
            Remove-Item -LiteralPath $signedPath -Force
        }
        foreach ($certificate in $importedCertificates) {
            if ($beforeThumbprints -notcontains $certificate.Thumbprint) {
                & certutil.exe -user -delstore My $certificate.Thumbprint | Out-Null
                if ($LASTEXITCODE -ne 0 -or
                    (Test-Path -LiteralPath "Cert:\CurrentUser\My\$($certificate.Thumbprint)")) {
                    throw "Failed to remove the temporary local-test certificate"
                }
            }
        }
        $password.Dispose()
    }
}
elseif ($PromptForCertificatePassword) {
    throw "-PromptForCertificatePassword requires -SignForLocalTest"
}

$json = $provenance | ConvertTo-Json -Depth 8
try {
    [IO.File]::WriteAllText($provenancePath, $json + "`n", [Text.UTF8Encoding]::new($false))
}
catch {
    foreach ($localArtifact in @($signedPath, $certificateOutputPath)) {
        if ($localArtifact -and (Test-Path -LiteralPath $localArtifact)) {
            Remove-Item -LiteralPath $localArtifact -Force
        }
    }
    throw
}
Write-Output "Store package: $unsignedPath"
Write-Output "SHA-256: $unsignedHash"
Write-Output "Provenance: $provenancePath"
