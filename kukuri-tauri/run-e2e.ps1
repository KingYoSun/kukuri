# PowerShell script to run E2E tests on Windows

Write-Host "Starting E2E tests for Kukuri..." -ForegroundColor Green

# Check if tauri-driver is installed
if (!(Get-Command tauri-driver -ErrorAction SilentlyContinue)) {
    Write-Host "Error: tauri-driver not found. Please install it with: cargo install tauri-driver" -ForegroundColor Red
    exit 1
}

# Check if msedgedriver.exe exists
if (!(Test-Path "$env:USERPROFILE\.cargo\bin\msedgedriver.exe")) {
    Write-Host "Warning: msedgedriver.exe not found. Running msedgedriver-tool..." -ForegroundColor Yellow
    & msedgedriver-tool
}

# Check if debug build exists
$appPath = "src-tauri\target\debug\kukuri-tauri.exe"
if (!(Test-Path $appPath)) {
    Write-Host "Debug build not found. Building application..." -ForegroundColor Yellow
    pnpm tauri build --debug
}

# Start tauri-driver in background
Write-Host "Starting tauri-driver..." -ForegroundColor Cyan
$tauriDriver = Start-Process -FilePath "tauri-driver" -WindowStyle Hidden -PassThru

# Wait for tauri-driver to start
Start-Sleep -Seconds 2

# Run WebdriverIO tests
Write-Host "Running WebdriverIO tests..." -ForegroundColor Cyan
npm run e2e

# Stop tauri-driver
Write-Host "Stopping tauri-driver..." -ForegroundColor Cyan
Stop-Process -Id $tauriDriver.Id -Force -ErrorAction SilentlyContinue

Write-Host "E2E tests completed!" -ForegroundColor Green