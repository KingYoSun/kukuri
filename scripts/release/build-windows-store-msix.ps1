[CmdletBinding()]
param(
    [switch]$AllowDirty,
    [string]$OutputDirectory = "dist/microsoft-store"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))
$desktopDir = Join-Path $repoRoot "apps/desktop"
$manifestPath = Join-Path $desktopDir "src-tauri/windows/store/Package.appxmanifest"
$storeTargetDir = Join-Path $repoRoot "target/microsoft-store"
$targetBinary = Join-Path $storeTargetDir "x86_64-pc-windows-msvc/release/kukuri-desktop-tauri.exe"
$requiredWinAppVersion = "0.6.1"

function Resolve-WorkspacePath([string]$Path) {
    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }
    return [IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Assert-WorkspaceChild([string]$Path, [string]$Label) {
    $rootPrefix = (Join-Path $repoRoot 'dist') + [IO.Path]::DirectorySeparatorChar
    if (-not $Path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label must be a new directory below the repository dist directory"
    }
    if (Test-Path -LiteralPath $Path) {
        throw "$Label already exists; choose a new output directory"
    }
    $ancestor = [IO.Path]::GetDirectoryName($Path)
    while ($ancestor -and $ancestor.Length -ge $repoRoot.Length) {
        if (Test-Path -LiteralPath $ancestor) {
            if ((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "$Label cannot pass through a reparse point"
            }
        }
        $ancestor = [IO.Path]::GetDirectoryName($ancestor)
    }
}

# Refuse an occupied output before invoking tools or building. Never delete it.
$outputDir = Resolve-WorkspacePath $OutputDirectory
Assert-WorkspaceChild $outputDir "OutputDirectory"

function Invoke-Native([string]$Executable, [string[]]$Arguments, [string]$Label) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
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
$appPackage = Get-Content -LiteralPath (Join-Path $desktopDir 'package.json') -Raw | ConvertFrom-Json
$tauriConfig = Get-Content -LiteralPath (Join-Path $desktopDir 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
if ([string]$tauriConfig.version -cne [string]$appPackage.version) {
    throw 'Tauri and app package versions must match before Store packaging'
}
. (Join-Path $PSScriptRoot 'windows-store-version.ps1')
$storeVersion = ConvertTo-StoreVersion ([string]$appPackage.version)
if ([string]$identity.Version -ne '0.0.0.0') { throw 'Store manifest template version must remain 0.0.0.0; version is generated' }
$identity.SetAttribute('Version', $storeVersion)
$architecture = [string]$identity.ProcessorArchitecture
if ($packageName -ne "KingYoSun.kukuri" -or
    $publisher -ne "CN=33EB763C-4859-4E44-886F-1784E16DD6D5" -or
    $publisherDisplayName -ne "KingYoSun" -or
    $architecture -ne "x64") {
    throw "Store manifest identity does not match the approved Partner Center product"
}
$versionParts = $storeVersion.Split('.')
if ($versionParts.Count -ne 4 -or $versionParts.Where({ $_ -notmatch '^\d+$' -or [long]$_ -gt 65535 }).Count -gt 0 -or
    [long]$versionParts[0] -lt 1 -or [long]$versionParts[3] -ne 0) {
    throw 'Store version must contain four 16-bit integers, start above zero, and end in zero'
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

New-Item -ItemType Directory -Path $outputDir | Out-Null
$stagingDir = Join-Path $outputDir "staging"
$assetDir = Join-Path $stagingDir "Assets"
New-Item -ItemType Directory -Path $assetDir -Force | Out-Null
$manifestPath = Join-Path $outputDir 'AppxManifest.xml'
$manifest.Save($manifestPath)

& {
    $previousDistribution = $env:VITE_KUKURI_DISTRIBUTION
    $previousTelemetry = $env:WINAPP_CLI_TELEMETRY_OPTOUT
    $previousTargetDir = $env:CARGO_TARGET_DIR
    try {
        $env:VITE_KUKURI_DISTRIBUTION = "microsoft-store"
        $env:WINAPP_CLI_TELEMETRY_OPTOUT = "1"
        $env:CARGO_TARGET_DIR = $storeTargetDir
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
        $env:CARGO_TARGET_DIR = $previousTargetDir
    }
}

if (-not (Test-Path -LiteralPath $targetBinary -PathType Leaf)) {
    throw "Store build binary is missing: $targetBinary"
}
Copy-Item -LiteralPath $targetBinary -Destination (Join-Path $stagingDir "kukuri.exe")
foreach ($asset in @("StoreLogo.png", "Square44x44Logo.png", "Square150x150Logo.png")) {
    Copy-Item -LiteralPath (Join-Path $desktopDir "src-tauri/icons/$asset") -Destination (Join-Path $assetDir $asset)
}
. (Join-Path $PSScriptRoot 'windows-store-assets.ps1')
$shellIcons = @(New-StoreShellIcons (Join-Path $desktopDir 'src-tauri/icons/icon.png') $assetDir)

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
    ) + @($shellIcons | ForEach-Object { "Assets/$_" }) | Sort-Object
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

$finalCommit = (& git -C $repoRoot rev-parse HEAD | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $finalCommit -ne $sourceCommit) { throw "Source commit changed during build" }
$finalStatus = (& git -C $repoRoot status --porcelain | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not verify final source state" }
if ($finalStatus -and -not $AllowDirty) { throw "Worktree changed during build:`n$finalStatus" }
$unsignedHash = (Get-FileHash -LiteralPath $unsignedPath -Algorithm SHA256).Hash.ToLowerInvariant()
$provenance = [ordered]@{
    schema_version = 1
    source_commit = $sourceCommit
    dirty = [bool]($gitStatus -or $finalStatus)
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
}
$provenancePath = Join-Path $outputDir "store-package.json"
$json = $provenance | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText($provenancePath, $json + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "Store package: $unsignedPath"
Write-Output "SHA-256: $unsignedHash"
Write-Output "Provenance: $provenancePath"
