#!/usr/bin/env pwsh

<#
.SYNOPSIS
    Kukuri P2P bootstrap/relay ノードを起動します
.DESCRIPTION
    ルートの docker-compose.yml を利用して cn-cli (cn p2p) ベースの
    ブートストラップノード/リレーノードを起動します。
.PARAMETER Mode
    起動モード: all, bootstrap, relay
.PARAMETER Profile
    relay モード時の選択: default=relay1のみ / full=relay1+relay-n0
.EXAMPLE
    .\start-bootstrap-nodes.ps1
    .\start-bootstrap-nodes.ps1 -Mode bootstrap
    .\start-bootstrap-nodes.ps1 -Mode relay -Profile full
#>

param(
    [ValidateSet('all', 'bootstrap', 'relay')]
    [string]$Mode = 'all',

    [ValidateSet('default', 'full')]
    [string]$Profile = 'default'
)

$ErrorActionPreference = 'Stop'

function Write-ColorOutput {
    param([string]$Message, [ConsoleColor]$Color = [ConsoleColor]::White)
    Write-Host $Message -ForegroundColor $Color
}

function Test-DockerAvailable {
    try {
        $null = docker version 2>&1
        return $true
    }
    catch {
        return $false
    }
}

function Invoke-Compose {
    param(
        [string[]]$Arguments
    )

    & docker compose -f docker-compose.yml @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "docker compose failed (exit code $LASTEXITCODE): $($Arguments -join ' ')"
    }
}

function Start-BootstrapNodes {
    Write-ColorOutput '========================================' -Color Cyan
    Write-ColorOutput '  Kukuri P2P Nodes Launcher (cn-cli)   ' -Color Cyan
    Write-ColorOutput '========================================' -Color Cyan
    Write-ColorOutput ''

    if (-not (Test-DockerAvailable)) {
        Write-ColorOutput 'エラー: Dockerが見つかりません。Dockerをインストールしてください。' -Color Red
        exit 1
    }

    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $projectRoot = Split-Path -Parent $scriptDir
    $composeFile = Join-Path $projectRoot 'docker-compose.yml'

    if (-not (Test-Path $composeFile)) {
        Write-ColorOutput "エラー: docker-compose.yml が見つかりません: $composeFile" -Color Red
        exit 1
    }

    Set-Location $projectRoot
    Write-ColorOutput "作業ディレクトリ: $projectRoot" -Color Gray
    Write-ColorOutput ''

    $services = @()
    switch ($Mode) {
        'all' {
            $services = @('kukuri-bootstrap', 'kukuri-relay', 'kukuri-relay-n0')
            Write-ColorOutput 'モード: ブートストラップ + リレーを起動' -Color Green
        }
        'bootstrap' {
            $services = @('kukuri-bootstrap')
            Write-ColorOutput 'モード: ブートストラップノードのみ起動' -Color Green
        }
        'relay' {
            $services = @('kukuri-relay')
            if ($Profile -eq 'full') {
                $services += 'kukuri-relay-n0'
            }
            Write-ColorOutput 'モード: リレーノードのみ起動' -Color Green
        }
    }

    if ($services.Count -eq 0) {
        throw '起動対象サービスが空です。'
    }

    Write-ColorOutput ''
    Write-ColorOutput "対象サービス: $($services -join ', ')" -Color Yellow

    Write-ColorOutput ''
    Write-ColorOutput 'Dockerイメージをビルド中...' -Color Yellow
    Invoke-Compose -Arguments (@('build') + $services)

    Write-ColorOutput ''
    Write-ColorOutput 'コンテナを起動中...' -Color Yellow
    Invoke-Compose -Arguments (@('up', '-d') + $services)

    Write-ColorOutput ''
    Write-ColorOutput '起動状態を確認中...' -Color Yellow
    Start-Sleep -Seconds 2
    Invoke-Compose -Arguments (@('ps') + $services)

    Write-ColorOutput ''
    Write-ColorOutput '========================================' -Color Green
    Write-ColorOutput '  ノードが正常に起動しました' -Color Green
    Write-ColorOutput '========================================' -Color Green
    Write-ColorOutput ''
    Write-ColorOutput '接続情報:' -Color Cyan
    if ($services -contains 'kukuri-bootstrap') {
        Write-ColorOutput '  Bootstrap:  localhost:11223' -Color White
    }
    if ($services -contains 'kukuri-relay') {
        Write-ColorOutput '  Relay:      localhost:11225' -Color White
    }
    if ($services -contains 'kukuri-relay-n0') {
        Write-ColorOutput '  Relay (n0): localhost:11226' -Color White
    }
    Write-ColorOutput ''
    Write-ColorOutput 'ログを確認: docker compose -f docker-compose.yml logs -f' -Color Gray
    Write-ColorOutput '停止:       docker compose -f docker-compose.yml down' -Color Gray
    Write-ColorOutput ''
}

try {
    Start-BootstrapNodes
}
catch {
    Write-ColorOutput "エラーが発生しました: $_" -Color Red
    exit 1
}
