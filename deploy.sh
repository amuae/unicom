#!/bin/bash

# Unicom 一键部署脚本
# Usage: curl -fsSL https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sudo bash
# Options: curl -fsSL ... | sudo bash -s -- --dir /opt/unicom --port 8080
# Reset password: curl -fsSL ... | sudo bash -s -- --reset-password

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
GITHUB_REPO="amuae/unicom"
GH_PROXY="https://ghfast.top"

# ---- 参数解析 ----
INSTALL_DIR=""
LISTEN_PORT=""
RESET_PASSWORD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dir)  INSTALL_DIR="$2"; shift 2 ;;
        --port) LISTEN_PORT="$2"; shift 2 ;;
        --reset-password) RESET_PASSWORD=true; shift ;;
        *)      log_err "未知参数: $1"; exit 1 ;;
    esac
done

# ---- 环境检测 ----
IS_ANDROID=false

detect_environment() {
    if [ -f /system/build.prop ] || [ -d /data/adb ]; then
        IS_ANDROID=true
    fi
}

# ---- 前置检查 ----
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        log_err "请使用 root 权限运行此脚本"
        exit 1
    fi
}

# ---- 架构检测 ----
detect_arch() {
    local arch=$(uname -m)
    case $arch in
        x86_64)          echo "amd64" ;;
        aarch64|arm64)   echo "arm64" ;;
        armv7l|armhf)    echo "armv7" ;;
        i386|i686)       echo "386" ;;
        *)
            log_err "不支持的架构: $arch"
            exit 1
            ;;
    esac
}

# ---- 服务管理器检测 ----
detect_init_system() {
    # Android 优先检测
    if $IS_ANDROID; then
        if [ -d /data/adb/modules ] || [ -d /data/adb/ksu ] || [ -d /data/adb/magisk ]; then
            echo "android"
            return
        fi
    fi

    # systemd
    if command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
        echo "systemd"
        return
    fi

    # OpenWrt procd
    if isOpenWrt; then
        echo "procd"
        return
    fi

    # OpenRC
    if command -v rc-service >/dev/null 2>&1 && command -v rc-update >/dev/null 2>&1; then
        echo "openrc"
        return
    fi

    # SysVinit
    if [ -d /etc/init.d ]; then
        if command -v update-rc.d >/dev/null 2>&1 || command -v chkconfig >/dev/null 2>&1; then
            echo "sysvinit"
            return
        fi
    fi

    echo "unknown"
}

# ---- OpenWrt 检测 ----
isOpenWrt() {
    [ -f /etc/openwrt_release ] && return 0
    [ -f /etc/openwrt_version ] && return 0
    [ -x /sbin/procd ] && [ -f /lib/functions/procd.sh ] && return 0
    return 1
}

# ---- 端口生成 ----
generate_random_port() {
    echo $((RANDOM % 55000 + 10000))
}

# ---- 随机字符串生成 ----
generate_random_string() {
    local len=${1:-10}
    cat /dev/urandom | tr -dc 'a-zA-Z0-9' | head -c "$len" 2>/dev/null || \
    head -c 256 /dev/urandom | md5sum | head -c "$len"
}

# ---- 等待服务就绪 + 注册管理员 ----
wait_and_register() {
    local port="$1"
    local max_wait=30
    local count=0

    log_step "等待服务启动..." >&2
    while [ $count -lt $max_wait ]; do
        if curl -sf "http://127.0.0.1:${port}/" >/dev/null 2>&1; then
            break
        fi
        sleep 1
        count=$((count + 1))
    done

    if [ $count -ge $max_wait ]; then
        log_warn "服务启动超时，跳过自动注册" >&2
        return 1
    fi

    # 直接用 reset-pass 写数据库（WAL 模式，不需要停服务）
    log_step "注册管理员账号..." >&2
    local output
    output=$("${INSTALL_DIR}/unicom" reset-pass 2>&1)

    if echo "$output" | grep -q "已重置"; then
        local username=$(echo "$output" | grep "用户名:" | sed 's/.*用户名:\s*//')
        local password=$(echo "$output" | grep "密码:" | sed 's/.*密码:\s*//')
        echo "$username"
        echo "$password"
        return 0
    else
        log_warn "自动注册失败: ${output}" >&2
        return 1
    fi
}

# ---- 安装依赖 ----
install_deps() {
    log_step "检查依赖..."
    for cmd in curl tar; do
        if ! command -v $cmd >/dev/null 2>&1; then
            log_info "安装 $cmd ..."
            if command -v apt >/dev/null 2>&1; then
                apt update -qq && apt install -y -qq $cmd
            elif command -v yum >/dev/null 2>&1; then
                yum install -y -q $cmd
            elif command -v apk >/dev/null 2>&1; then
                apk add --no-cache $cmd
            elif command -v pacman >/dev/null 2>&1; then
                pacman -Sy --noconfirm $cmd
            elif command -v opkg >/dev/null 2>&1; then
                opkg install $cmd
            elif command -v pkg >/dev/null 2>&1; then
                # Termux
                pkg install -y $cmd
            else
                log_err "无法安装 $cmd，请手动安装"
                exit 1
            fi
        fi
    done
}

# ---- 下载二进制 ----
download_binary() {
    local arch="$1"
    local target="$2"

    # Android 使用 android-arm64，其他使用 linux-*
    local platform_name
    if $IS_ANDROID; then
        platform_name="android-${arch}"
    else
        platform_name="linux-${arch}"
    fi

    local file_name="unicom_${platform_name}.tar.gz"
    local download_url="${GH_PROXY}/https://github.com/${GITHUB_REPO}/releases/latest/download/${file_name}"

    log_step "下载 Unicom latest (${platform_name})..."
    log_info "URL: ${download_url}"

    local tmp_file="/tmp/unicom_${platform_name}.tar.gz"

    if ! curl -fSL -o "$tmp_file" "$download_url"; then
        log_err "下载失败"
        exit 1
    fi

    tar -xzf "$tmp_file" -C "$(dirname "$target")"
    chmod +x "$target"
    rm -f "$tmp_file"

    log_ok "二进制下载完成: $target"
}

# ---- 生成配置文件 ----
generate_config() {
    local port="$1"
    local config_file="$2"

    cat > "$config_file" << EOF
host = "0.0.0.0"
port = ${port}
EOF

    log_ok "配置文件: $config_file (端口: $port)"
}

# ---- systemd 服务 ----
create_systemd_service() {
    local service_file="/etc/systemd/system/${SERVICE_NAME}.service"

    cat > "$service_file" << EOF
[Unit]
Description=Unicom Data Query Service
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/unicom
WorkingDirectory=${INSTALL_DIR}
Restart=always
RestartSec=5
User=root

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable ${SERVICE_NAME}.service
    systemctl start ${SERVICE_NAME}.service

    if systemctl is-active --quiet ${SERVICE_NAME}.service; then
        log_ok "systemd 服务启动成功"
    else
        log_err "服务启动失败: journalctl -u ${SERVICE_NAME} -f"
        return 1
    fi
}

# ---- openrc 服务 ----
create_openrc_service() {
    local service_file="/etc/init.d/${SERVICE_NAME}"

    cat > "$service_file" << EOF
#!/sbin/openrc-run

name="${SERVICE_NAME}"
description="Unicom Data Query Service"

command="${INSTALL_DIR}/unicom"
command_background=true
pidfile="/run/\${RC_SVCNAME}.pid"
directory="${INSTALL_DIR}"

depend() {
    need net
    after firewall
}
EOF

    chmod +x "$service_file"
    rc-update add ${SERVICE_NAME} default
    rc-service ${SERVICE_NAME} start

    log_ok "openrc 服务启动成功"
}

# ---- procd 服务 (OpenWrt) ----
create_procd_service() {
    local service_file="/etc/init.d/${SERVICE_NAME}"

    cat > "$service_file" << EOF
#!/bin/sh /etc/rc.common

START=99
STOP=10

USE_PROCD=1

start_service() {
    procd_open_instance
    procd_set_param command ${INSTALL_DIR}/unicom
    procd_set_param respawn 3600 5 5
    procd_set_param stdout 1
    procd_set_param stderr 1
    procd_close_instance
}
EOF

    chmod +x "$service_file"
    /etc/init.d/${SERVICE_NAME} enable
    /etc/init.d/${SERVICE_NAME} start

    log_ok "procd 服务启动成功"
}

# ---- sysvinit 服务 ----
create_sysvinit_service() {
    local service_file="/etc/init.d/${SERVICE_NAME}"

    cat > "$service_file" << EOF
#!/bin/sh
### BEGIN INIT INFO
# Provides:          ${SERVICE_NAME}
# Required-Start:    \$network \$remote_fs
# Required-Stop:     \$network \$remote_fs
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: Unicom Data Query Service
# Description:       Unicom data query service
### END INIT INFO

DAEMON="${INSTALL_DIR}/unicom"
DAEMON_DIR="${INSTALL_DIR}"
PIDFILE="/var/run/${SERVICE_NAME}.pid"
LOGFILE="/var/log/${SERVICE_NAME}.log"

case "\$1" in
    start)
        echo "Starting ${SERVICE_NAME}..."
        cd \$DAEMON_DIR
        nohup \$DAEMON >> \$LOGFILE 2>&1 &
        echo \$! > \$PIDFILE
        echo "Started (PID: \$(cat \$PIDFILE))"
        ;;
    stop)
        echo "Stopping ${SERVICE_NAME}..."
        if [ -f \$PIDFILE ]; then
            kill \$(cat \$PIDFILE) 2>/dev/null
            rm -f \$PIDFILE
        fi
        echo "Stopped"
        ;;
    restart)
        \$0 stop
        sleep 1
        \$0 start
        ;;
    status)
        if [ -f \$PIDFILE ] && kill -0 \$(cat \$PIDFILE) 2>/dev/null; then
            echo "${SERVICE_NAME} is running (PID: \$(cat \$PIDFILE))"
        else
            echo "${SERVICE_NAME} is not running"
            exit 1
        fi
        ;;
    *)
        echo "Usage: \$0 {start|stop|restart|status}"
        exit 1
        ;;
esac
exit 0
EOF

    chmod +x "$service_file"

    if command -v update-rc.d >/dev/null 2>&1; then
        update-rc.d ${SERVICE_NAME} defaults
    elif command -v chkconfig >/dev/null 2>&1; then
        chkconfig --add ${SERVICE_NAME}
        chkconfig ${SERVICE_NAME} on
    fi

    /etc/init.d/${SERVICE_NAME} start
    log_ok "sysvinit 服务启动成功"
}

# ---- Android（root）服务 ----
create_android_service() {
    # Android root 通用 service.d 目录
    local service_d="/data/adb/service.d"
    local service_script="${service_d}/${SERVICE_NAME}.sh"

    mkdir -p "$service_d"

    cat > "$service_script" << 'SCRIPT_EOF'
#!/system/bin/sh
# Unicom - Android（root）开机自启脚本
# 在系统启动完成后执行 (late_start)

MODDIR="${0%/*}"
UNICOM_DIR="INSTALL_DIR_PLACEHOLDER"
UNICOM_BIN="${UNICOM_DIR}/unicom"
UNICOM_CONFIG="${UNICOM_DIR}/config.toml"
LOGFILE="${UNICOM_DIR}/unicom.log"
PIDFILE="${UNICOM_DIR}/unicom.pid"

# 等待网络就绪
wait_for_network() {
    local count=0
    while [ $count -lt 60 ]; do
        if ping -c 1 -W 2 223.5.5.5 >/dev/null 2>&1; then
            return 0
        fi
        # 也检查是否有网络接口获得 IP
        if ip addr show 2>/dev/null | grep -q "inet.*scope global"; then
            sleep 3  # 多等几秒让路由稳定
            return 0
        fi
        sleep 2
        count=$((count + 1))
    done
    return 1
}

# 检查是否已在运行
is_running() {
    if [ -f "$PIDFILE" ]; then
        local pid=$(cat "$PIDFILE")
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi
    # 也检查进程表
    if pgrep -f "$UNICOM_BIN" >/dev/null 2>&1; then
        return 0
    fi
    return 1
}

# 设置 SELinux 上下文
setup_selinux() {
    if command -v chcon >/dev/null 2>&1; then
        chcon u:object_r:system_file:s0 "$UNICOM_BIN" 2>/dev/null
    fi
    # 允许网络访问
    if command -v supolicy >/dev/null 2>&1; then
        supolicy --live "allow unicom unicom tcp_socket { create connect bind listen accept }" 2>/dev/null
    fi
}

start_unicom() {
    if is_running; then
        echo "[unicom] 已在运行，跳过"
        return 0
    fi

    echo "[unicom] 等待网络..."
    if ! wait_for_network; then
        echo "[unicom] 网络等待超时，仍然尝试启动"
    fi

    echo "[unicom] 启动中..."
    cd "$UNICOM_DIR"
    nohup "$UNICOM_BIN" >> "$LOGFILE" 2>&1 &
    local pid=$!
    echo "$pid" > "$PIDFILE"

    sleep 2
    if kill -0 "$pid" 2>/dev/null; then
        echo "[unicom] 启动成功 (PID: $pid)"
        return 0
    else
        echo "[unicom] 启动失败，查看日志: $LOGFILE"
        return 1
    fi
}

# 主逻辑
case "${1}" in
    start)
        setup_selinux
        start_unicom
        ;;
    stop)
        if [ -f "$PIDFILE" ]; then
            kill $(cat "$PIDFILE") 2>/dev/null
            rm -f "$PIDFILE"
        fi
        pkill -f "$UNICOM_BIN" 2>/dev/null
        echo "[unicom] 已停止"
        ;;
    restart)
        "$0" stop
        sleep 2
        "$0" start
        ;;
    *)
        setup_selinux
        start_unicom
        ;;
esac
SCRIPT_EOF

    # 替换安装路径
    sed -i "s|INSTALL_DIR_PLACEHOLDER|${INSTALL_DIR}|g" "$service_script"
    chmod 755 "$service_script"

    # 设置 SELinux (如果存在)
    if command -v chcon >/dev/null 2>&1; then
        chcon u:object_r:system_file:s0 "${INSTALL_DIR}/unicom" 2>/dev/null || true
        chcon u:object_r:system_file:s0 "$service_script" 2>/dev/null || true
    fi

    log_ok "Android 服务配置完成"
}

# ---- 显示访问信息 ----
show_info() {
    local port="$1"
    local init_system="$2"
    local admin_user="$3"
    local admin_pass="$4"

    # Android 直接用 127.0.0.1（移动数据无公网 IP）
    local ip
    if $IS_ANDROID; then
        ip="127.0.0.1"
    else
        ip=$(hostname -I 2>/dev/null | awk '{print $1}')
    fi
    [ -z "$ip" ] && ip="<服务器IP>"

    echo
    log_ok "=========================================="
    log_ok "  Unicom 部署完成！"
    log_ok "=========================================="
    echo
    log_info "访问地址: http://${ip}:${port}"
    log_info "安装目录: ${INSTALL_DIR}"
    log_info "配置文件: ${INSTALL_DIR}/config.toml"
    echo
    log_info "服务管理命令:"
    case "$init_system" in
        systemd)
            log_info "  状态: systemctl status ${SERVICE_NAME}"
            log_info "  启动: systemctl start ${SERVICE_NAME}"
            log_info "  停止: systemctl stop ${SERVICE_NAME}"
            log_info "  重启: systemctl restart ${SERVICE_NAME}"
            log_info "  日志: journalctl -u ${SERVICE_NAME} -f"
            ;;
        openrc)
            log_info "  状态: rc-service ${SERVICE_NAME} status"
            log_info "  启动: rc-service ${SERVICE_NAME} start"
            log_info "  停止: rc-service ${SERVICE_NAME} stop"
            log_info "  重启: rc-service ${SERVICE_NAME} restart"
            ;;
        procd|sysvinit)
            log_info "  状态: /etc/init.d/${SERVICE_NAME} status"
            log_info "  启动: /etc/init.d/${SERVICE_NAME} start"
            log_info "  停止: /etc/init.d/${SERVICE_NAME} stop"
            log_info "  重启: /etc/init.d/${SERVICE_NAME} restart"
            ;;
        android)
            log_info "  脚本: /data/adb/service.d/${SERVICE_NAME}.sh"
            log_info "  启动: sh /data/adb/service.d/${SERVICE_NAME}.sh start"
            log_info "  停止: sh /data/adb/service.d/${SERVICE_NAME}.sh stop"
            log_info "  重启: sh /data/adb/service.d/${SERVICE_NAME}.sh restart"
            log_info "  日志: cat ${INSTALL_DIR}/unicom.log"
            ;;
        *)
            log_warn "  手动运行: cd ${INSTALL_DIR} && ./unicom"
            ;;
    esac
    echo
    if [ -n "$admin_user" ] && [ -n "$admin_pass" ]; then
        echo
        log_ok "管理员账号（请妥善保管）:"
        log_info "  用户名: ${admin_user}"
        log_info "  密  码: ${admin_pass}"
    else
        log_warn "自动注册失败，请手动访问 Web 页面注册管理员"
    fi
    echo
    log_info "联通接口和通知设置在 Web 后台配置"
}

# ---- 读取现有配置端口 ----
get_existing_port() {
    local config_file="$1"
    if [ -f "$config_file" ]; then
        grep -E '^port\s*=' "$config_file" | head -1 | sed 's/.*=\s*//' | tr -d ' '
    fi
}

# ---- 停止服务 ----
stop_service() {
    local init_system="$1"
    case "$init_system" in
        systemd)
            systemctl stop "$SERVICE_NAME" 2>/dev/null
            ;;
        openrc)
            rc-service "$SERVICE_NAME" stop 2>/dev/null
            ;;
        procd|sysvinit)
            /etc/init.d/"$SERVICE_NAME" stop 2>/dev/null
            ;;
        android)
            local pidfile="${INSTALL_DIR}/unicom.pid"
            if [ -f "$pidfile" ]; then
                kill "$(cat "$pidfile")" 2>/dev/null
                rm -f "$pidfile"
            fi
            pkill -f "${INSTALL_DIR}/unicom" 2>/dev/null
            ;;
    esac
}

# ---- 启动服务 ----
start_service() {
    local init_system="$1"
    case "$init_system" in
        systemd)
            systemctl start "$SERVICE_NAME" 2>/dev/null
            ;;
        openrc)
            rc-service "$SERVICE_NAME" start 2>/dev/null
            ;;
        procd|sysvinit)
            /etc/init.d/"$SERVICE_NAME" start 2>/dev/null
            ;;
        android)
            sh /data/adb/service.d/${SERVICE_NAME}.sh start 2>/dev/null
            ;;
    esac
}

# ---- 重置管理员密码 ----
reset_admin_password() {
    local bin="${INSTALL_DIR}/unicom"
    if [ ! -x "$bin" ]; then
        log_err "未找到二进制: $bin"
        return 1
    fi

    # 使用二进制自带的 reset-pass 命令（bcrypt 加密，兼容所有平台）
    local output
    output=$("$bin" reset-pass 2>&1)

    if echo "$output" | grep -q "已重置"; then
        echo "$output" | sed 's/^/  /'
        return 0
    else
        log_err "重置失败: $output"
        return 1
    fi
}

# ---- 主流程 ----
main() {
    echo "=============================================================="
    echo "          Unicom 联通流量查询 一键部署"
    echo "       https://github.com/amuae/unicom"
    echo "=============================================================="
    echo

    check_root
    detect_environment

    local arch=$(detect_arch)
    local init_system=$(detect_init_system)

    # Android 默认安装目录
    if [ -z "$INSTALL_DIR" ]; then
        if $IS_ANDROID; then
            INSTALL_DIR="/data/adb/unicom"
        else
            INSTALL_DIR="/opt/unicom"
        fi
    fi

    log_step "环境检测:"
    log_info "  系统: $($IS_ANDROID && echo "Android（root）" || echo "Linux")"
    log_info "  架构: ${arch}"
    log_info "  服务管理器: ${init_system}"
    log_info "  安装目录: ${INSTALL_DIR}"
    echo

    # 重置密码模式
    if $RESET_PASSWORD; then
        reset_admin_password
        exit $?
    fi

    # 端口设置
    if [ -z "$LISTEN_PORT" ]; then
        LISTEN_PORT=$(generate_random_port)
        log_info "随机端口: ${LISTEN_PORT}"
    else
        if ! [[ "$LISTEN_PORT" =~ ^[0-9]+$ ]] || [ "$LISTEN_PORT" -lt 1 ] || [ "$LISTEN_PORT" -gt 65535 ]; then
            log_err "端口无效: $LISTEN_PORT (1-65535)"
            exit 1
        fi
        log_info "指定端口: ${LISTEN_PORT}"
    fi
    echo

    # 检测是否已安装
    local is_update=false
    if [ -f "${INSTALL_DIR}/unicom" ] && [ -f "${INSTALL_DIR}/config.toml" ]; then
        is_update=true
        local existing_port=$(get_existing_port "${INSTALL_DIR}/config.toml")
        if [ -n "$existing_port" ]; then
            LISTEN_PORT="$existing_port"
            log_step "检测到已安装，更新模式"
            log_info "  保留现有端口: ${LISTEN_PORT}"
        else
            log_step "检测到已安装，更新模式"
            log_warn "  无法读取端口，使用: ${LISTEN_PORT}"
        fi

        # 停止服务
        log_info "停止服务..."
        stop_service "$init_system"
        sleep 1
    fi

    # 安装
    install_deps

    log_step "创建安装目录..."
    mkdir -p "$INSTALL_DIR"

    download_binary "$arch" "${INSTALL_DIR}/unicom"

    # 首次安装才生成配置
    if ! $is_update; then
        generate_config "$LISTEN_PORT" "${INSTALL_DIR}/config.toml"
    fi

    # 创建服务
    log_step "配置自启动服务 (${init_system})..."
    local ADMIN_USER=""
    local ADMIN_PASS=""
    case "$init_system" in
        systemd)   create_systemd_service ;;
        openrc)    create_openrc_service ;;
        procd)     create_procd_service ;;
        sysvinit)  create_sysvinit_service ;;
        android)   create_android_service ;;
        *)
            log_warn "未检测到支持的服务管理器"
            log_warn "请手动运行: cd ${INSTALL_DIR} && ./unicom"
            ;;
    esac

    # 启动服务
    log_step "启动服务..."
    start_service "$init_system"
    sleep 2

    if $is_update; then
        echo
        log_ok "=========================================="
        log_ok "  Unicom 更新完成！"
        log_ok "=========================================="
        echo
        log_info "访问地址: http://127.0.0.1:${LISTEN_PORT}"
        log_info "配置文件: ${INSTALL_DIR}/config.toml (已保留)"
    else
        # 首次安装，注册管理员
        local creds
        creds=$(wait_and_register "$LISTEN_PORT")
        if [ $? -eq 0 ] && [ -n "$creds" ]; then
            ADMIN_USER=$(echo "$creds" | sed -n '1p')
            ADMIN_PASS=$(echo "$creds" | sed -n '2p')
        fi
        show_info "$LISTEN_PORT" "$init_system" "$ADMIN_USER" "$ADMIN_PASS"
    fi
}

main "$@"
