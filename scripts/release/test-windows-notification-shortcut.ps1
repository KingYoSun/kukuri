param(
    [string]$MakensisPath = (Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe')
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$contractDirectory = Join-Path $repositoryRoot ('test-results\kukuri\notification-shortcut-contract.' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $contractDirectory | Out-Null
$hookPath = Join-Path $repositoryRoot 'apps\desktop\src-tauri\windows\notification-hooks.nsh'
& $MakensisPath /V2 "/DTEST_OUTPUT=$contractDirectory" "/DHOOK_PATH=$hookPath" (Join-Path $PSScriptRoot 'test-windows-notification-shortcut.nsi')
if ($LASTEXITCODE -ne 0) { throw "NSIS contract compilation failed: $LASTEXITCODE" }

# The fixture only writes/reads notification.lnk beside itself. No global install.
$contractProcess = Start-Process -FilePath (Join-Path $contractDirectory 'shortcut-contract.exe') -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
if ($contractProcess.ExitCode -ne 0) { throw "Notification shortcut contract failed: $($contractProcess.ExitCode)" }
if (Test-Path -LiteralPath (Join-Path $contractDirectory 'not-created.lnk')) {
    throw 'The no-shortcut case created a shortcut unexpectedly.'
}
Write-Output "Notification shortcut contract passed: $contractDirectory"
