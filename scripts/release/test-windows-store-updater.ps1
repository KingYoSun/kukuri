[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$manifestTool = Get-ChildItem -LiteralPath "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Filter mt.exe -File -Recurse |
    Where-Object { $_.DirectoryName -match '[\\/]x64$' } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $manifestTool) { throw 'Windows SDK mt.exe is required' }
$outputDir = Join-Path $repoRoot ('test-results/kukuri/store-updater-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $outputDir | Out-Null
Push-Location $repoRoot
try {
    foreach ($flavor in @('direct', 'microsoft-store')) {
        $arguments = @('test', '--manifest-path', 'apps/desktop/src-tauri/Cargo.toml', '--lib', '--no-run', '--message-format=json')
        if ($flavor -eq 'microsoft-store') { $arguments += @('--features', 'microsoft-store') }
        $messages = & cargo @arguments
        if ($LASTEXITCODE -ne 0) { throw "$flavor test compilation failed" }
        $artifacts = @($messages | ForEach-Object { try { $_ | ConvertFrom-Json } catch {} } |
            Where-Object { $_.reason -eq 'compiler-artifact' -and $_.executable -and $_.target.name -eq 'kukuri_desktop_tauri_lib' })
        if ($artifacts.Count -ne 1) { throw 'Expected one Tauri library test executable' }
        $copy = Join-Path $outputDir "$flavor.exe"
        Copy-Item -LiteralPath $artifacts[0].executable -Destination $copy
        # Cargo library-test binaries do not inherit the application's Windows manifest.
        & $manifestTool -manifest (Join-Path $PSScriptRoot 'windows-test-runtime.manifest') "-outputresource:$copy;#1"
        if ($LASTEXITCODE -ne 0) { throw 'Test Common Controls manifest embedding failed' }
        & $copy 'app_update::' '--nocapture'
        if ($LASTEXITCODE -ne 0) { throw "$flavor updater boundary tests failed" }
    }
}
finally { Pop-Location }
