$ErrorActionPreference = 'Stop'
$work = Join-Path ([IO.Path]::GetTempPath()) ('kukuri-wrapper-test-' + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($work) | Out-Null
$prior = [Environment]::GetEnvironmentVariable('KUKURI_UPDATER_PUBLIC_KEY_FILE')
$global:KukuriWrapperFixture = @{ calls = 0; passed = 1 }
function Invoke-FixtureVerifier {
    if ($env:KUKURI_UPDATER_PUBLIC_KEY_FILE) { throw 'Stale public key override leaked' }
    if (-not (Test-Path -LiteralPath $env:KUKURI_UPDATER_BUNDLE)) { throw 'Missing bundle' }
    if ([IO.File]::ReadAllText($env:KUKURI_UPDATER_SIGNATURE) -ne 'embedded') { throw 'Missing signature' }
    $global:KukuriWrapperFixture.calls++
    $global:LASTEXITCODE = 0
    "test result: ok. $($global:KukuriWrapperFixture.passed) passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;"
}
try {
    $env:KUKURI_UPDATER_PUBLIC_KEY_FILE = 'original-value'
    $manifest = @{ version = '0.1.8'; platforms = @{} }
    $sums = @()
    foreach ($target in 'windows-x86_64', 'linux-x86_64') {
        $name = "$target.bundle"
        [IO.File]::WriteAllText((Join-Path $work $name), $target)
        $hash = (Get-FileHash (Join-Path $work $name)).Hash.ToLowerInvariant()
        $sums += "$hash  $name"
        $manifest.platforms[$target] = @{ signature = 'embedded'; url = "https://github.com/KingYoSun/kukuri/releases/download/v0.1.8-preview.2/$name" }
    }
    [IO.File]::WriteAllLines((Join-Path $work 'SHA256SUMS.txt'), $sums)
    function Write-Fixture {
        [IO.File]::WriteAllText((Join-Path $work 'latest-preview.json'), ($manifest | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
    }
    function Run-Wrapper {
        & (Join-Path $PSScriptRoot 'test-published-updater-signature.ps1') -Tag v0.1.8-preview.2 -InputDir $work -Platforms windows-x86_64,linux-x86_64 -VerifierExecutable Invoke-FixtureVerifier
    }
    function Expect-Rejection([string]$message, [int]$expectedCalls) {
        $global:KukuriWrapperFixture.calls = 0
        try { Run-Wrapper; throw 'Unexpected acceptance' } catch {
            if ($_.Exception.Message -notlike "*$message*") { throw }
        }
        if ($global:KukuriWrapperFixture.calls -ne $expectedCalls) { throw 'Unexpected verifier calls' }
        if ($env:KUKURI_UPDATER_PUBLIC_KEY_FILE -ne 'original-value') { throw 'Environment not restored' }
    }
    Write-Fixture
    Run-Wrapper
    if ($global:KukuriWrapperFixture.calls -ne 2 -or $env:KUKURI_UPDATER_PUBLIC_KEY_FILE -ne 'original-value') { throw 'Two-platform verification failed' }
    $global:KukuriWrapperFixture.passed = 0
    Expect-Rejection 'did not pass exactly one test' 1
    $global:KukuriWrapperFixture.passed = 1
    $originalUrl = $manifest.platforms.'windows-x86_64'.url
    foreach ($url in @($originalUrl.Replace('KingYoSun/kukuri', 'other/repo'), "$originalUrl`?foreign=1", "$originalUrl#fragment")) {
        $manifest.platforms.'windows-x86_64'.url = $url
        Write-Fixture
        Expect-Rejection 'must belong to the selected release' 0
    }
    $manifest.platforms.'windows-x86_64'.url = $originalUrl
    Write-Fixture
    [IO.File]::WriteAllText((Join-Path $work 'windows-x86_64.bundle'), 'changed')
    Expect-Rejection 'checksum mismatch' 0
    Write-Host 'Updater signature wrapper contracts passed (stub routing only; real cryptography is tested separately).'
} finally {
    Remove-Variable -Name KukuriWrapperFixture -Scope Global
    [Environment]::SetEnvironmentVariable('KUKURI_UPDATER_PUBLIC_KEY_FILE', $prior)
    $resolved = [IO.Path]::GetFullPath($work)
    $temp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (-not $resolved.StartsWith($temp, [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($resolved) -notlike 'kukuri-wrapper-test-*') { throw 'Unsafe fixture cleanup' }
    [IO.Directory]::Delete($resolved, $true)
}
