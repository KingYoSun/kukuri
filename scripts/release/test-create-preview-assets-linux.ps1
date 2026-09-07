$ErrorActionPreference = 'Stop'
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$workDir = Join-Path $tempRoot ("kukuri-release-linux-test-" + [Guid]::NewGuid().ToString('N'))
$inputDir = Join-Path $workDir 'input'
$outputDir = Join-Path $workDir 'output'
$source = 'a' * 40
$version = '0.1.8'
$utf8 = [Text.UTF8Encoding]::new($false)
try {
  $fixtures = @{
    'windows-x86_64' = 'kukuri_0.1.8_x64-setup.exe'
    'linux-x86_64' = 'kukuri_0.1.8_amd64.AppImage'
    'cli-linux-x86_64' = 'kukuri-cli_0.1.8_x86_64-unknown-linux-gnu.tar.gz'
    'cli-linux-aarch64' = 'kukuri-cli_0.1.8_aarch64-unknown-linux-gnu.tar.gz'
  }
  foreach ($target in $fixtures.Keys) {
    $directory = Join-Path $inputDir $target
    [IO.Directory]::CreateDirectory($directory) | Out-Null
    $name = $fixtures[$target]
    [IO.File]::WriteAllText((Join-Path $directory $name), 'fixture', $utf8)
    $arguments = @((Join-Path $PSScriptRoot 'release_assets.py'), 'package', '--directory', $directory,
      '--target', $target, '--version', $version, '--source', $source, '--file', $name)
    if (-not $target.StartsWith('cli-')) {
      $key = "$target-updater-key.pub"
      [IO.File]::WriteAllText((Join-Path $directory $key), 'public-key', $utf8)
      [IO.File]::WriteAllText((Join-Path $directory "$name.sig"), 'signature', $utf8)
      $arguments += @('--updater', $name, '--public-key-file', $key, '--signing', 'distribution', '--file', "$name.sig", '--file', $key)
    }
    if ($target -eq 'linux-x86_64') {
      $debName = "kukuri_${version}_amd64.deb"
      $payloadName = "kukuri_${version}_deb-payload.json"
      [IO.File]::WriteAllText((Join-Path $directory $debName), 'deb fixture', $utf8)
      [IO.File]::WriteAllText((Join-Path $directory "$debName.sig"), 'deb-signature', $utf8)
      $debHash = (Get-FileHash -LiteralPath (Join-Path $directory $debName)).Hash.ToLowerInvariant()
      $payload = @{ deb_sha256 = $debHash; source_commit = $source; package = 'kukuri'; version = $version;
        architecture = 'amd64'; maintainer_scripts = @(); elf_paths = @('usr/bin/kukuri-desktop-tauri');
        payload = @('usr/bin/kukuri-desktop-tauri', 'usr/share/doc/kukuri/copyright', 'usr/share/doc/kukuri/THIRD_PARTY_NOTICES.md') | ForEach-Object { @{path = $_} } }
      [IO.File]::WriteAllText((Join-Path $directory $payloadName), ($payload | ConvertTo-Json -Depth 6), $utf8)
      $arguments += @('--deb-updater', $debName, '--file', $debName, '--file', "$debName.sig", '--file', $payloadName)
      $spec = Get-Content -Raw (Join-Path $PSScriptRoot 'native-runtime-sources.json') | ConvertFrom-Json
      $material = @()
      foreach ($kind in 'sources', 'notices') {
        $materialName = "kukuri_${version}_linux-native-$kind.tar.gz"
        $materialPath = Join-Path $directory $materialName
        [IO.File]::WriteAllText($materialPath, 'source/notice fixture', $utf8)
        $material += @{ name = $materialName; sha256 = (Get-FileHash -LiteralPath $materialPath -Algorithm SHA256).Hash.ToLowerInvariant() }
        $arguments += @('--file', $materialName)
      }
      $complianceName = "kukuri_${version}_linux-native-compliance.json"
      $compliance = @{ source_material_complete = $true; ubuntu_source_count = 1; static_source_count = $spec.sources.Count;
        runtime_source = $spec.runtime.source_commit; runtime_normalized_sha256 = $spec.runtime.normalized_prefix_sha256;
        appimage_sha256 = (Get-FileHash -LiteralPath (Join-Path $directory $name) -Algorithm SHA256).Hash.ToLowerInvariant(); material = $material }
      [IO.File]::WriteAllText((Join-Path $directory $complianceName), ($compliance | ConvertTo-Json -Depth 6), $utf8)
      $compliance.deb_sha256 = $debHash
      $compliance.deb_payload_sha256 = (Get-FileHash -LiteralPath (Join-Path $directory $payloadName)).Hash.ToLowerInvariant()
      $compliance.deb_native_scope = 'first-party-elf-system-shared-libraries'
      [IO.File]::WriteAllText((Join-Path $directory $complianceName), ($compliance | ConvertTo-Json -Depth 6), $utf8)
      $arguments += @('--file', $complianceName)
    }
    & python @arguments
    if ($LASTEXITCODE -ne 0) { throw 'Fixture package failed' }
  }
  $create = Join-Path $PSScriptRoot 'create-preview-assets.ps1'
  & $create -Tag 'v0.1.8-preview.2' -Repository 'KingYoSun/kukuri' -Version $version `
    -InputDir $inputDir -OutputDir $outputDir -IncludeLinux -SourceCommit $source
  & python (Join-Path $PSScriptRoot 'release_assets.py') validate-output --input $outputDir `
    --tag 'v0.1.8-preview.2' --repository 'KingYoSun/kukuri' --version $version --source $source
  if ($LASTEXITCODE -ne 0) { throw 'Final output verification failed' }
  $manifest = Get-Content -Raw (Join-Path $outputDir 'latest-preview.json') | ConvertFrom-Json
  if (@($manifest.platforms.PSObject.Properties).Count -ne 3) { throw 'All three updater platforms are required' }
  if ($manifest.platforms.'linux-x86_64-deb'.signature -ne 'deb-signature' -or $manifest.platforms.'linux-x86_64-deb'.url -notlike '*/kukuri_0.1.8_amd64.deb') { throw 'Wrong Deb updater entry' }
  if ($manifest.platforms.'linux-x86_64'.signature -ne 'signature') { throw 'Linux embedded signature mismatch' }
  if ($manifest.platforms.'linux-x86_64'.url -notlike '*/kukuri_0.1.8_amd64.AppImage') { throw 'Wrong Linux updater URL' }
  $provenance = Get-Content -Raw (Join-Path $outputDir 'release-provenance.json') | ConvertFrom-Json
  if ($provenance.source_commit -ne $source) { throw 'Source provenance lost' }
  $sums = Get-Content (Join-Path $outputDir 'SHA256SUMS.txt')
  foreach ($file in Get-ChildItem -LiteralPath $outputDir -File | Where-Object Name -ne 'SHA256SUMS.txt') {
    $expected = "{0}  {1}" -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $file.Name
    if ($sums -notcontains $expected) { throw "Checksum missing or wrong: $($file.Name)" }
  }
  $before = [IO.File]::ReadAllText((Join-Path $outputDir 'latest-preview.json'))
  [IO.File]::WriteAllText((Join-Path $inputDir 'linux-x86_64/kukuri_0.1.8_amd64.AppImage'), 'changed', $utf8)
  $rejected = $false
  try {
    & $create -Tag 'v0.1.8-preview.2' -Repository 'KingYoSun/kukuri' -Version $version `
      -InputDir $inputDir -OutputDir $outputDir -IncludeLinux -SourceCommit $source
  } catch { $rejected = $true }
  if (-not $rejected -or [IO.File]::ReadAllText((Join-Path $outputDir 'latest-preview.json')) -ne $before) {
    throw 'Invalid input changed existing output'
  }
  Write-Host 'Linux + Windows assembly contracts passed'
} finally {
  $resolved = [IO.Path]::GetFullPath($workDir)
  if (-not $resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe cleanup path' }
  if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}

# The expected native-command rejection leaves LASTEXITCODE=1. Report success
# only after every assertion and cleanup completed, including when called by CI.
exit 0
