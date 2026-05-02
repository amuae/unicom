# Unicom 联通流量查询系统

单文件部署的联通流量查询服务。一个二进制 + 一个配置文件即可运行。

## 一键部署

### Linux (amd64 / ARM64)

```bash
curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sudo bash
```

自动检测架构 + 服务管理器（systemd / openrc / procd / sysvinit），设置开机自启。

### Android（root）

```bash
curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sh
```

自动安装到 `/data/adb/unicom`，通过 `/data/adb/service.d/` 实现开机自启。

### Windows

```powershell
$dir = "$env:LOCALAPPDATA\unicom"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Invoke-WebRequest -Uri "https://ghfast.top/https://github.com/amuae/unicom/releases/latest/download/unicom_windows-amd64.zip" -OutFile "$dir\unicom.zip"
Expand-Archive -Path "$dir\unicom.zip" -DestinationPath $dir -Force
Remove-Item "$dir\unicom.zip"
cd $dir
.\unicom.exe
```

## 快捷命令

```bash
unicom start       # 启动服务
unicom stop        # 停止服务
unicom restart     # 重启服务
unicom status      # 查看状态（PID、内存、端口）
unicom reset-pass  # 重置管理员账密（随机生成）
unicom version     # 显示版本
unicom help        # 显示帮助
```

## 交互菜单

直接运行 `unicom`（无参数）进入交互菜单：

```
┌───────────────────────────────────┐
│     Unicom 联通流量查询系统       │
│            v1.0.0                 │
├───────────────────────────────────┤
│  状态: 运行中 (PID: 12345)        │
├───────────────────────────────────┤
│  1. 启动服务                      │
│  2. 重启服务                      │
│  3. 停止服务                      │
│  4. 运行状态                      │
│  5. 管理账密                      │
│  6. 检查更新                      │
│  7. 卸载程序                      │
│  0. 退出                          │
└───────────────────────────────────┘
```

## 配置

最小配置 `config.toml`：

```toml
host = "0.0.0.0"
port = 8080
```

完整配置参考 `config.toml.example`。联通接口、通知渠道等在 Web 后台配置。

## 通知模板

**基础占位符：** `[套餐]` `[时间]` `[日期]` `[时长]`

**9 大流量桶 × 5 属性：**

| 占位符 | 含义 | 数据来源 |
|--------|------|----------|
| `[桶名.总量]` | 流量总量 | 当前查询 |
| `[桶名.已用]` | 已用量 | 当前查询 |
| `[桶名.剩余]` | 剩余量 | 当前查询 |
| `[桶名.用量]` | 本次消耗 | 上次通知 → 本次查询 |
| `[桶名.今日用量]` | 今日累计 | 今日首次 → 本次查询 |

**桶名：** 所有通用 / 所有免流 / 所有流量 / 通用有限 / 通用不限 / 区域有限 / 区域不限 / 免流有限 / 免流不限

示例模板：
```
[套餐] [时长]
跳: [所有通用.用量] 钉: [免流不限.用量]
钉钉: [免流不限.已用] 剩余: [免流不限.剩余]
通用剩余: [所有通用.剩余]  [时间]
```

## 从源码构建

需要：Rust 1.70+、Node.js 18+、Python 3

```bash
git clone https://github.com/amuae/unicom.git
cd unicom

# 本机编译
./build.sh

# 交叉编译 Linux ARM64
./build.sh arm64

# 交叉编译 Android ARM64
./build.sh android
```

产物：`./unicom`（项目根目录）

## 支持平台

| 平台 | 架构 | 说明 |
|------|------|------|
| Linux | amd64 / ARM64 | systemd / openrc / procd / sysvinit |
| Android | ARM64 | 需要 root |
| Windows | amd64 | %LOCALAPPDATA%\unicom |

## License

MIT
