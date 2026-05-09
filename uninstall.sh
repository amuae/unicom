#!/bin/bash

# Unicom 卸载脚本
# Usage: curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/uninstall.sh | sudo bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()  { echo -e "$1"; }
log_ok()    { echo -e "${GREEN}$1${NC}"; }
log_warn()  { echo -e "${YELLOW}$1${NC}"; }
log_err()   { echo -e "${RED}$1${NC}"; }
log_step()  { echo -e "${CYAN}$1${NC}"; }

SERVICE_NAME="unicom"
INSTALL_DIR="${1:-/opt/unicom}"

echo "=============================================================="
echo "          Unicom 卸载"
echo "=============================================================="
echo

if [ "$(id -u)" -ne 0 ]; then
    log_err "请使用 root 权限运行此脚本"
    exit 1
fi

if [ ! -f "${INSTALL_DIR}/unicom" ] && [ ! -f "${INSTALL_DIR}/config.toml" ]; then
    log_err "未找到 Unicom 安装: ${INSTALL_DIR}"
    exit 1
fi

echo "⚠️  此操作将："
echo "  - 停止服务"
echo "  - 删除 ${INSTALL_DIR} 下所有文件（包括数据库）"
echo "  - 删除 systemd / openrc / procd / sysvinit 服务"
echo "  - 删除 Android service.d 脚本"
echo

# 管道执行时 stdin 不是终端，从 /dev/tty 读取确认
if [ -t 0 ]; then
    read -p "确认卸载？(输入 yes): " confirm
else
    read -p "确认卸载？(输入 yes): " confirm < /dev/tty
fi
if [ "$confirm" != "yes" ]; then
    echo "已取消"
    exit 0
fi

# ---- 停止服务 ----
log_step "停止服务..."

# systemd
if command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
    systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    log_ok "已停止 systemd 服务"
fi

# openrc
if command -v rc-service >/dev/null 2>&1 && rc-service "$SERVICE_NAME" status 2>/dev/null; then
    rc-service "$SERVICE_NAME" stop 2>/dev/null || true
    rc-update delete "$SERVICE_NAME" 2>/dev/null || true
    log_ok "已停止 openrc 服务"
fi

# procd / sysvinit
if [ -x "/etc/init.d/${SERVICE_NAME}" ]; then
    "/etc/init.d/${SERVICE_NAME}" stop 2>/dev/null || true
    log_ok "已停止 init.d 服务"
fi

# PID file fallback
if [ -f "${INSTALL_DIR}/unicom.pid" ]; then
    local_pid=$(cat "${INSTALL_DIR}/unicom.pid" 2>/dev/null)
    if [ -n "$local_pid" ]; then
        kill "$local_pid" 2>/dev/null || true
        sleep 1
        kill -9 "$local_pid" 2>/dev/null || true
    fi
fi

# 等待进程退出
sleep 2

# ---- 删除服务文件 ----
log_step "删除服务文件..."

# systemd
rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
systemctl daemon-reload 2>/dev/null || true

# openrc / procd / sysvinit
rm -f "/etc/init.d/${SERVICE_NAME}"

# Android
rm -f "/data/adb/service.d/${SERVICE_NAME}.sh"

log_ok "服务文件已清理"

# ---- 删除安装目录 ----
log_step "删除安装目录..."

rm -rf "$INSTALL_DIR"

if [ -d "$INSTALL_DIR" ]; then
    log_err "删除失败，可能有进程仍在运行"
    log_info "请手动停止进程后执行: rm -rf $INSTALL_DIR"
    exit 1
fi

log_ok "安装目录已删除"

echo
log_ok "=========================================="
log_ok "  Unicom 卸载完成"
log_ok "=========================================="
