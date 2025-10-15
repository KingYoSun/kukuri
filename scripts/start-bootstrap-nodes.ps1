#!/usr/bin/env pwsh

<#
.SYNOPSIS
    Kukuri DHTブートストラップノードを起動します
.DESCRIPTION
    Docker Composeを使用してローカル開発環境用のDHTブートストラップノードとリレーノードを起動します
.PARAMETER Mode
    起動モード: all, bootstrap, relay
.PARAMETER Profile
    Dockerプロファイル: default, full
.EXAMPLE
    .\start-bootstrap-nodes.ps1
    .\start-bootstrap-nodes.ps1 -Mode bootstrap
    .\start-bootstrap-nodes.ps1 -Profile full
#>

param(
    [ValidateSet("all", "bootstrap", "relay")]
    [string]$Mode = "all",
    
    [ValidateSet("default", "full")]
    [string]$Profile = "default"
)

$ErrorActionPreference = "Stop"

# カラー出力用関数
function Write-ColorOutput {
    param([string]$Message, [ConsoleColor]$Color = [ConsoleColor]::White)
    Write-Host $Message -ForegroundColor $Color
}

# Dockerが利用可能か確認
function Test-DockerAvailable {
    try {
        $null = docker version 2>&1
        return $true
    }
    catch {
        return $false
    }
}

# メイン処理
function Start-BootstrapNodes {
    Write-ColorOutput "========================================" -Color Cyan
    Write-ColorOutput "  Kukuri DHT Bootstrap Nodes Launcher  " -Color Cyan
    Write-ColorOutput "========================================" -Color Cyan
    Write-ColorOutput ""

    # Dockerチェック
    if (-not (Test-DockerAvailable)) {
        Write-ColorOutput "エラー: Dockerが見つかりません。Dockerをインストールしてください。" -Color Red
        exit 1
    }

    # kukuri-cliディレクトリに移動
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $projectRoot = Split-Path -Parent $scriptDir
    $cliDir = Join-Path $projectRoot "kukuri-cli"
    
    if (-not (Test-Path $cliDir)) {
        Write-ColorOutput "エラー: kukuri-cliディレクトリが見つかりません: $cliDir" -Color Red
        exit 1
    }
    
    Set-Location $cliDir
    Write-ColorOutput "作業ディレクトリ: $cliDir" -Color Gray
    Write-ColorOutput ""

    # サービスリストを作成
    $services = @()
    switch ($Mode) {
        "all" {
            $services = @()  # docker composeに任せる
            Write-ColorOutput "モード: すべてのノードを起動" -Color Green
        }
        "bootstrap" {
            $services = @("bootstrap-node-1", "bootstrap-node-2")
            Write-ColorOutput "モード: ブートストラップノードのみ起動" -Color Green
        }
        "relay" {
            $services = @("relay-node-1")
            if ($Profile -eq "full") {
                $services += "relay-node-2"
            }
            Write-ColorOutput "モード: リレーノードのみ起動" -Color Green
        }
    }

    # Docker Composeコマンドを構築
    $composeCmd = "docker compose"
    if ($Profile -eq "full") {
        $composeCmd += " --profile full"
        Write-ColorOutput "プロファイル: フル（すべてのオプショナルサービスを含む）" -Color Yellow
    }
    
    # ビルドと起動
    Write-ColorOutput ""
    Write-ColorOutput "Dockerイメージをビルド中..." -Color Yellow
    $buildCmd = "$composeCmd build"
    Write-ColorOutput "実行: $buildCmd" -Color Gray
    Invoke-Expression $buildCmd
    
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "ビルドに失敗しました" -Color Red
        exit 1
    }
    
    Write-ColorOutput ""
    Write-ColorOutput "コンテナを起動中..." -Color Yellow
    
    $upCmd = "$composeCmd up -d"
    if ($services.Count -gt 0) {
        $upCmd += " " + ($services -join " ")
    }
    
    Write-ColorOutput "実行: $upCmd" -Color Gray
    Invoke-Expression $upCmd
    
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "起動に失敗しました" -Color Red
        exit 1
    }
    
    # 起動状態を確認
    Write-ColorOutput ""
    Write-ColorOutput "起動状態を確認中..." -Color Yellow
    Start-Sleep -Seconds 2
    
    Invoke-Expression "$composeCmd ps"
    
    Write-ColorOutput ""
    Write-ColorOutput "========================================" -Color Green
    Write-ColorOutput "  ノードが正常に起動しました！" -Color Green
    Write-ColorOutput "========================================" -Color Green
    Write-ColorOutput ""
    Write-ColorOutput "接続情報:" -Color Cyan
    Write-ColorOutput "  Bootstrap Node 1: localhost:11223" -Color White
    Write-ColorOutput "  Bootstrap Node 2: localhost:11224" -Color White
    Write-ColorOutput "  Relay Node 1:     localhost:11225" -Color White
    if ($Profile -eq "full") {
        Write-ColorOutput "  Relay Node 2:     localhost:11226" -Color White
    }
    Write-ColorOutput ""
    Write-ColorOutput "ログを確認: docker compose logs -f" -Color Gray
    Write-ColorOutput "停止:       docker compose down" -Color Gray
    Write-ColorOutput ""
}

# スクリプト実行
try {
    Start-BootstrapNodes
}
catch {
    Write-ColorOutput "エラーが発生しました: $_" -Color Red
    exit 1
}
