#Requires -Version 5.1
<#
.SYNOPSIS
    Unicom 卸载脚本 (Windows)
.EXAMPLE
    irm https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/uninstall.ps1 | iex
#>

param(
    [string]$Dir = "",
    [switch]$Force
)
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Log-Ok   { param($msg) Write-Host $msg -ForegroundColor Green }
function Log-Warn { param($msg) Write-Host $msg -ForegroundColor Yellow }
function Log-Err  { param($msg) Write-Host $msg -ForegroundColor Red }
function Log-Step { param($msg) Write-Host $msg -ForegroundColor Cyan }

if ([string]::IsNullOrEmpty($Dir)) {
    $Dir = Join-Path $env:LOCALAPPDATA "unicom"
}

Write-Host ""
Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host "          Unicom 卸载 (Windows)"                               -ForegroundColor Cyan
Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host ""

if (!(Test-Path (Join-Path $Dir "unicom.exe")) -and !(Test-Path (Join-Path $Dir "config.toml"))) {
    Log-Err "未找到 Unicom 安装: $Dir"
    exit 1
}

Write-Host "此操作将："
Write-Host "  - 停止服务"
Write-Host "  - 删除 $Dir 下所有文件（包括数据库）"
Write-Host "  - 删除开机自启快捷方式"
Write-Host ""
if (!$Force) {
    $confirm = Read-Host "确认卸载？(输入 yes)"
    if ($confirm -ne "yes") {
        Write-Host "已取消"
        exit 0
    }
}

# 停止服务
Log-Step "停止服务..."
$pidFile = Join-Path $Dir "unicom.pid"
if (Test-Path $pidFile) {
    $svcPid = Get-Content $pidFile -ErrorAction SilentlyContinue
    if ($svcPid) {
        Stop-Process -Id $svcPid -Force -ErrorAction SilentlyContinue
    }
}
Get-Process -Name "unicom" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
Log-Ok "服务已停止"

# 删除开机自启
Log-Step "删除开机自启..."
$shortcutPath = Join-Path ([Environment]::GetFolderPath("Startup")) "Unicom.lnk"
if (Test-Path $shortcutPath) {
    Remove-Item $shortcutPath -Force
    Log-Ok "已删除快捷方式"
}

# 删除安装目录
Log-Step "删除安装目录..."
Remove-Item $Dir -Recurse -Force -ErrorAction SilentlyContinue

if (Test-Path $Dir) {
    # 二进制可能没删掉（刚停的进程文件句柄未释放），重试一次
    Start-Sleep -Seconds 2
    Remove-Item $Dir -Recurse -Force -ErrorAction SilentlyContinue
}

if (Test-Path $Dir) {
    Log-Err "删除失败，可能有进程仍在运行"
    Log-Info "请手动停止 unicom 进程后删除: $Dir"
    exit 1
}

Log-Ok "安装目录已删除"

Write-Host ""
Log-Ok "=========================================="
Log-Ok "  Unicom 卸载完成"
Log-Ok "=========================================="
