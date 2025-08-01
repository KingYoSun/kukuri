# kukuri Development Environment Setup Script (Windows)
# Please run with administrator privileges

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "kukuri Development Environment Setup" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Stop on error
$ErrorActionPreference = "Stop"

# Function: Check if command exists
function Test-CommandExists {
    param($Command)
    try {
        if (Get-Command $Command -ErrorAction SilentlyContinue) {
            return $true
        }
    } catch {
        return $false
    }
    return $false
}

# Function: Get program version
function Get-ProgramVersion {
    param($Command, $VersionArg = "--version")
    try {
        $version = & $Command $VersionArg 2>&1 | Select-String -Pattern "[\d\.]+" | Select-Object -First 1
        return $version.Matches[0].Value
    } catch {
        return "Unknown"
    }
}

# 1. Check Node.js
Write-Host "1. Checking Node.js..." -ForegroundColor Yellow
if (Test-CommandExists "node") {
    $nodeVersion = Get-ProgramVersion "node" "-v"
    Write-Host "OK Node.js is already installed (v$nodeVersion)" -ForegroundColor Green
} else {
    Write-Host "ERROR Node.js is not installed" -ForegroundColor Red
    Write-Host "   Please download and install from:" -ForegroundColor White
    Write-Host "   https://nodejs.org/" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Please run this script again after installation." -ForegroundColor Yellow
    exit 1
}

Write-Host ""

# 2. Install pnpm
Write-Host "2. Installing pnpm..." -ForegroundColor Yellow
if (Test-CommandExists "pnpm") {
    $pnpmVersion = Get-ProgramVersion "pnpm"
    Write-Host "OK pnpm is already installed (v$pnpmVersion)" -ForegroundColor Green
} else {
    try {
        Write-Host "   Installing pnpm..." -ForegroundColor White
        Invoke-WebRequest https://get.pnpm.io/install.ps1 -UseBasicParsing | Invoke-Expression
        
        # Update environment variables immediately
        $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
        $machinePath = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
        $env:Path = $userPath + ";" + $machinePath
        
        Write-Host "OK pnpm has been installed" -ForegroundColor Green
        Write-Host "WARNING It will be available in a new terminal session" -ForegroundColor Yellow
    } catch {
        Write-Host "ERROR Failed to install pnpm: $_" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""

# 3. Check and install Rust
Write-Host "3. Checking Rust & Cargo..." -ForegroundColor Yellow
if (Test-CommandExists "rustc") {
    $rustVersion = Get-ProgramVersion "rustc"
    Write-Host "OK Rust is already installed (v$rustVersion)" -ForegroundColor Green
} else {
    Write-Host "ERROR Rust is not installed" -ForegroundColor Red
    Write-Host "   Do you want to install Rust? (Y/N): " -ForegroundColor White -NoNewline
    $response = Read-Host
    
    if ($response -eq 'Y' -or $response -eq 'y') {
        Write-Host "   Downloading rustup-init.exe..." -ForegroundColor White
        $rustupUrl = "https://win.rustup.rs/x86_64"
        $rustupPath = "$env:TEMP\rustup-init.exe"
        
        try {
            Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath
            Write-Host "   Starting Rust installer..." -ForegroundColor White
            Start-Process -FilePath $rustupPath -Wait
            
            # Update environment variables
            $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
            $machinePath = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
            $env:Path = $userPath + ";" + $machinePath
            
            Write-Host "OK Rust has been installed" -ForegroundColor Green
        } catch {
            Write-Host "ERROR Failed to install Rust: $_" -ForegroundColor Red
            Write-Host "   Please install manually from: https://www.rust-lang.org/tools/install" -ForegroundColor Cyan
        }
    } else {
        Write-Host "   Skipped. Please install manually later." -ForegroundColor Yellow
    }
}

Write-Host ""

# 4. Check Visual Studio Build Tools
Write-Host "4. Checking Visual Studio Build Tools..." -ForegroundColor Yellow
$vswhereExe = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasBuildTools = $false

if (Test-Path $vswhereExe) {
    $vsInstances = & $vswhereExe -products * -property installationPath 2>$null
    if ($vsInstances) {
        $hasBuildTools = $true
    }
}

if ($hasBuildTools) {
    Write-Host "OK Visual Studio Build Tools is already installed" -ForegroundColor Green
} else {
    Write-Host "WARNING Visual Studio Build Tools is not installed" -ForegroundColor Yellow
    Write-Host "   Required for Rust compilation." -ForegroundColor White
    Write-Host "   Please download and install from:" -ForegroundColor White
    Write-Host "   https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022" -ForegroundColor Cyan
    Write-Host "   Select 'Desktop development with C++' during installation." -ForegroundColor Yellow
}

Write-Host ""

# Only install additional tools if Rust is installed
if (Test-CommandExists "cargo") {
    # 5. Install Tauri CLI
    Write-Host "5. Checking Tauri CLI..." -ForegroundColor Yellow
    if (Test-CommandExists "cargo-tauri") {
        Write-Host "OK Tauri CLI is already installed" -ForegroundColor Green
    } else {
        Write-Host "   Do you want to install Tauri CLI? (Y/N): " -ForegroundColor White -NoNewline
        $response = Read-Host
        
        if ($response -eq 'Y' -or $response -eq 'y') {
            Write-Host "   Installing... (this may take several minutes)" -ForegroundColor White
            try {
                & cargo install tauri-cli
                Write-Host "OK Tauri CLI has been installed" -ForegroundColor Green
            } catch {
                Write-Host "ERROR Failed to install Tauri CLI: $_" -ForegroundColor Red
            }
        }
    }
    
    Write-Host ""
    
    # 6. Install sqlx-cli
    Write-Host "6. Checking sqlx-cli..." -ForegroundColor Yellow
    if (Test-CommandExists "sqlx") {
        Write-Host "OK sqlx-cli is already installed" -ForegroundColor Green
    } else {
        Write-Host "   Do you want to install sqlx-cli? (Y/N): " -ForegroundColor White -NoNewline
        $response = Read-Host
        
        if ($response -eq 'Y' -or $response -eq 'y') {
            Write-Host "   Installing... (this may take several minutes)" -ForegroundColor White
            try {
                & cargo install sqlx-cli --no-default-features --features native-tls,sqlite
                Write-Host "OK sqlx-cli has been installed" -ForegroundColor Green
            } catch {
                Write-Host "ERROR Failed to install sqlx-cli: $_" -ForegroundColor Red
            }
        }
    }
}

Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "Setup Status" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Installation status summary
$tools = @(
    @{Name="Node.js"; Command="node"; Version="-v"},
    @{Name="pnpm"; Command="pnpm"; Version="--version"},
    @{Name="Rust"; Command="rustc"; Version="--version"},
    @{Name="Cargo"; Command="cargo"; Version="--version"},
    @{Name="Tauri CLI"; Command="cargo-tauri"; Version="--version"},
    @{Name="sqlx-cli"; Command="sqlx"; Version="--version"}
)

foreach ($tool in $tools) {
    if (Test-CommandExists $tool.Command) {
        $version = Get-ProgramVersion $tool.Command $tool.Version
        Write-Host "OK $($tool.Name): v$version" -ForegroundColor Green
    } else {
        Write-Host "ERROR $($tool.Name): Not installed" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Open a new terminal to update environment variables" -ForegroundColor White
Write-Host "  2. Run 'pnpm install' in the project directory" -ForegroundColor White
Write-Host "  3. Start the development server with 'pnpm tauri dev'" -ForegroundColor White
Write-Host ""

# Check WebView2
Write-Host "Additional information:" -ForegroundColor Yellow
$webView2Path = "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application"
if (Test-Path $webView2Path) {
    Write-Host "OK WebView2 is already installed" -ForegroundColor Green
} else {
    Write-Host "WARNING WebView2 not found. Required for Tauri apps." -ForegroundColor Yellow
    Write-Host "   Windows 11 includes it by default. If needed, install from:" -ForegroundColor White
    Write-Host "   https://developer.microsoft.com/microsoft-edge/webview2/" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host "If you encounter any issues, please refer to docs/01_project/windows_setup_guide.md" -ForegroundColor Cyan