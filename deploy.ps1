#Requires -Version 5.1
<#
.SYNOPSIS
    Unicom 一键部署脚本 (Windows)
.DESCRIPTION
    下载并安装 Unicom 联通流量查询系统到 Windows。
    默认安装到 %LOCALAPPDATA%\unicom，无需管理员权限。
.EXAMPLE
    iwr -useb https://raw.githubusercontent.com/amuae/unicom/main/deploy.ps1 | iex
    # 或指定参数：
    iwr -useb https://raw.githubusercontent.com/amuae/unicom/main/deploy.ps1 -OutFile deploy.ps1
    .\deploy.ps1 -Dir "C:\unicom" -Port 8080
#>

param(
    [string]$Dir = "",
    [int]$Port = 0,
    [switch]$ResetPassword
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# ─── 配置 ───
$GITHUB_REPO = "amuae/unicom"
$GH_PROXY = "https://ghfast.top"

# ─── 颜色输出 ───
function Log-Info  { param($msg) Write-Host $msg }
function Log-Ok    { param($msg) Write-Host $msg -ForegroundColor Green }
function Log-Warn  { param($msg) Write-Host $msg -ForegroundColor Yellow }
function Log-Err   { param($msg) Write-Host $msg -ForegroundColor Red }
function Log-Step  { param($msg) Write-Host $msg -ForegroundColor Cyan }

# ─── 横幅 ───
Write-Host ""
Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host "          Unicom 联通流量查询 一键部署 (Windows)"               -ForegroundColor Cyan
Write-Host "       https://github.com/amuae/unicom"                        -ForegroundColor Cyan
Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host ""

# ─── 安装目录 ───
if ([string]::IsNullOrEmpty($Dir)) {
    $Dir = Join-Path $env:LOCALAPPDATA "unicom"
}
Log-Step "环境检测:"
Log-Info "  系统: Windows"
Log-Info "  安装目录: $Dir"
Write-Host ""

# ─── 重置密码模式 ───
if ($ResetPassword) {
    $binPath = Join-Path $Dir "unicom.exe"
    if (!(Test-Path $binPath)) {
        Log-Err "未找到二进制: $binPath"
        exit 1
    }
    $output = & $binPath reset-pass 2>&1
    if ($output -match "已重置") {
        Log-Ok $output
    } else {
        Log-Err "重置失败: $output"
    }
    exit 0
}

# ─── 检测已安装 ───
$isUpdate = $false
$existingPort = 0

if ((Test-Path (Join-Path $Dir "unicom.exe")) -and (Test-Path (Join-Path $Dir "config.toml"))) {
    $isUpdate = $true
    # 读取现有端口
    $configContent = Get-Content (Join-Path $Dir "config.toml") -ErrorAction SilentlyContinue
    foreach ($line in $configContent) {
        if ($line -match '^\s*port\s*=\s*(\d+)') {
            $existingPort = [int]$Matches[1]
            break
        }
    }
    if ($existingPort -gt 0) {
        $Port = $existingPort
        Log-Step "检测到已安装，更新模式"
        Log-Info "  保留现有端口: $Port"
    } else {
        Log-Step "检测到已安装，更新模式"
        Log-Warn "  无法读取端口，使用随机端口"
    }

    # 停止服务
    Log-Info "停止服务..."
    $pidFile = Join-Path $Dir "unicom.pid"
    if (Test-Path $pidFile) {
        $svcPid = Get-Content $pidFile -ErrorAction SilentlyContinue
        if ($svcPid) {
            Stop-Process -Id $svcPid -Force -ErrorAction SilentlyContinue
        }
        Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
    }
    # 也尝试 taskkill
    Get-Process -Name "unicom" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
}

# ─── 端口设置 ───
if ($Port -eq 0) {
    $Port = Get-Random -Minimum 10000 -Maximum 65535
    Log-Info "随机端口: $Port"
} else {
    if ($Port -lt 1 -or $Port -gt 65535) {
        Log-Err "端口无效: $Port (1-65535)"
        exit 1
    }
    Log-Info "指定端口: $Port"
}
Write-Host ""

# ─── 创建安装目录 ───
Log-Step "创建安装目录..."
New-Item -ItemType Directory -Force -Path $Dir | Out-Null

# ─── 下载二进制 ───
$fileName = "unicom_windows-amd64.zip"
$downloadUrl = "$GH_PROXY/https://github.com/$GITHUB_REPO/releases/latest/download/$fileName"
$tmpFile = Join-Path $env:TEMP $fileName

Log-Step "下载 Unicom latest (windows-amd64)..."
Log-Info "  URL: $downloadUrl"

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $downloadUrl -OutFile $tmpFile -UseBasicParsing
} catch {
    Log-Err "下载失败: $_"
    Log-Info "请手动下载: https://github.com/$GITHUB_REPO/releases/latest"
    exit 1
}

Log-Step "解压中..."
Expand-Archive -Path $tmpFile -DestinationPath $Dir -Force
Remove-Item $tmpFile -Force -ErrorAction SilentlyContinue

$binPath = Join-Path $Dir "unicom.exe"
if (!(Test-Path $binPath)) {
    Log-Err "解压后未找到 unicom.exe"
    exit 1
}
Log-Ok "二进制下载完成: $binPath"

# ─── 生成配置文件 ───
if (!$isUpdate) {
    $configPath = Join-Path $Dir "config.toml"
    @"
host = "0.0.0.0"
port = $Port
"@ | Out-File -FilePath $configPath -Encoding utf8 -Force
    Log-Ok "配置文件: $configPath (端口: $Port)"
}

# ─── 启动服务 ───
Log-Step "启动服务..."
$proc = Start-Process -FilePath $binPath -ArgumentList "start" -WorkingDirectory $Dir -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3

# 检查是否启动成功
$running = $false
$pidFile = Join-Path $Dir "unicom.pid"
if (Test-Path $pidFile) {
    $servicePid = Get-Content $pidFile -ErrorAction SilentlyContinue
    if ($servicePid) {
        $running = Get-Process -Id $servicePid -ErrorAction SilentlyContinue
    }
}

if ($running) {
    Log-Ok "服务启动成功 (PID: $servicePid)"
} else {
    Log-Warn "服务可能未启动，请手动检查"
}

# ─── 创建开机自启快捷方式 ───
Log-Step "配置开机自启..."
$startupDir = [Environment]::GetFolderPath("Startup")
$shortcutPath = Join-Path $startupDir "Unicom.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $binPath
$shortcut.Arguments = "start"
$shortcut.WorkingDirectory = $Dir
$shortcut.WindowStyle = 7  # 最小化
$shortcut.Description = "Unicom 联通流量查询"
$shortcut.Save()
Log-Ok "开机自启: $shortcutPath"

# ─── 首次安装：注册管理员 ───
if (!$isUpdate) {
    Log-Step "等待服务就绪..."
    $maxWait = 30
    $ready = $false
    for ($i = 0; $i -lt $maxWait; $i++) {
        try {
            $resp = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/" -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue
            if ($resp.StatusCode -eq 200) {
                $ready = $true
                break
            }
        } catch {
            Start-Sleep -Seconds 1
        }
    }

    if ($ready) {
        Log-Step "注册管理员账号..."
        $username = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 10 | ForEach-Object {[char]$_})
        $password = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 16 | ForEach-Object {[char]$_})

        try {
            $body = @{ username = $username; password = $password } | ConvertTo-Json
            $regResp = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/auth/register" -Method Post -Body $body -ContentType "application/json" -ErrorAction Stop
            if ($regResp -match "注册成功") {
                $adminUser = $username
                $adminPass = $password
            }
        } catch {
            Log-Warn "自动注册失败，请手动访问 Web 页面注册"
        }
    } else {
        Log-Warn "服务启动超时，跳过自动注册"
    }

    # 显示安装信息
    Write-Host ""
    Log-Ok "=========================================="
    Log-Ok "  Unicom 部署完成！"
    Log-Ok "=========================================="
    Write-Host ""
    Log-Info "访问地址: http://127.0.0.1:$Port"
    Log-Info "安装目录: $Dir"
    Log-Info "配置文件: $(Join-Path $Dir 'config.toml')"
    Write-Host ""
    Log-Info "服务管理命令:"
    Log-Info "  启动: unicom.exe start"
    Log-Info "  停止: unicom.exe stop"
    Log-Info "  重启: unicom.exe restart"
    Log-Info "  状态: unicom.exe status"
    Log-Info "  菜单: unicom.exe (无参数)"
    Write-Host ""

    if ($adminUser -and $adminPass) {
        Log-Ok "管理员账号（请妥善保管）:"
        Log-Info "  用户名: $adminUser"
        Log-Info "  密  码: $adminPass"
    } else {
        Log-Warn "自动注册失败，请手动访问 Web 页面注册管理员"
    }
    Write-Host ""
    Log-Info "联通接口和通知设置在 Web 后台配置"
} else {
    Write-Host ""
    Log-Ok "=========================================="
    Log-Ok "  Unicom 更新完成！"
    Log-Ok "=========================================="
    Write-Host ""
    Log-Info "访问地址: http://127.0.0.1:$Port"
    Log-Info "配置文件: $(Join-Path $Dir 'config.toml') (已保留)"
}
