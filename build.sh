#!/bin/bash
# ═══════════════════════════════════════════════
# Unicom 一站式构建脚本
# 用法: ./build.sh [目标平台]
#   无参数    - 本机编译
#   arm64     - 交叉编译 Linux ARM64
#   android   - 交叉编译 Android ARM64
# ═══════════════════════════════════════════════
set -e
cd "$(dirname "$0")"

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

step()  { echo -e "\n${CYAN}▶ $1${NC}"; }
ok()    { echo -e "${GREEN}✓ $1${NC}"; }
die()   { echo -e "${RED}✗ $1${NC}"; exit 1; }

TARGET="${1:-}"

# ── 1. 检查依赖 ──
step "检查环境"
command -v cargo  >/dev/null || die "未找到 cargo (需要 Rust 1.70+)"
command -v node   >/dev/null || die "未找到 node (需要 Node.js 18+)"
command -v npm    >/dev/null || die "未找到 npm"
command -v python3 >/dev/null || die "未找到 python3"

echo "  Rust:    $(rustc --version | awk '{print $2}')"
echo "  Node:    $(node --version)"
echo "  Python:  $(python3 --version | awk '{print $2}')"

# ── 2. 设置编译目标 ──
CROSS_TARGET=""
case "$TARGET" in
  arm64)
    CROSS_TARGET="aarch64-unknown-linux-gnu"
    ;;
  android)
    CROSS_TARGET="aarch64-linux-android"
    ;;
  "")
    ;;
  *)
    die "未知目标: $TARGET (可选: arm64, android)"
    ;;
esac

if [ -n "$CROSS_TARGET" ]; then
  echo "  目标:    $CROSS_TARGET (交叉编译)"
  command -v cross >/dev/null || die "交叉编译需要 cross (cargo install cross)"
  BUILD_CMD="cross"
else
  echo "  目标:    $(uname -m) (本机)"
  BUILD_CMD="cargo"
fi

# ── 3. 构建前端 ──
step "构建前端"
cd web
npm install --silent
npm run build
cd ..
[ -f dist/index.html ] || die "前端构建失败，未找到 dist/index.html"
ok "前端构建完成"

# ── 4. 生成静态文件嵌入代码 ──
step "生成 src/static_files.rs"

python3 << 'PYEOF'
import os, glob

assets = sorted(glob.glob('dist/assets/*'))
lines = []
lines.append("// 自动生成，请勿手动编辑")
lines.append("use std::collections::HashMap;")
lines.append("")
lines.append("pub struct StaticFile {")
lines.append("    pub content: &'static [u8],")
lines.append("    pub content_type: &'static str,")
lines.append("}")
lines.append("")
lines.append("pub fn load_static_files() -> HashMap<String, StaticFile> {")
lines.append("    let mut files = HashMap::new();")
lines.append("")

count = 0
for f in assets:
    name = os.path.basename(f)
    if name.endswith('.js'):
        ct = 'application/javascript'
    elif name.endswith('.css'):
        ct = 'text/css'
    else:
        continue
    lines.append('    files.insert(')
    lines.append(f'        "assets/{name}".to_string(),')
    lines.append('        StaticFile {')
    lines.append(f'            content: include_bytes!("../dist/assets/{name}"),')
    lines.append(f'            content_type: "{ct}",')
    lines.append('        },')
    lines.append('    );')
    lines.append('')
    count += 1

lines.append('    files.insert(')
lines.append('        "index.html".to_string(),')
lines.append('        StaticFile {')
lines.append('            content: include_bytes!("../dist/index.html"),')
lines.append('            content_type: "text/html; charset=utf-8",')
lines.append('        },')
lines.append('    );')
lines.append('')
lines.append('    files')
lines.append('}')
lines.append('')

with open('src/static_files.rs', 'w') as f:
    f.write('\n'.join(lines))
print(f"  嵌入 {count} 个资源文件 + index.html")
PYEOF

ok "静态文件代码生成完成"

# ── 5. 构建后端 ──
step "构建后端"

export CARGO_BUILD_JOBS=$(nproc 2>/dev/null || echo 4)

if [ -n "$CROSS_TARGET" ]; then
  $BUILD_CMD build --release --target "$CROSS_TARGET"
  BINARY="target/${CROSS_TARGET}/release/unicom"
else
  $BUILD_CMD build --release
  BINARY="target/release/unicom"
fi

[ -f "$BINARY" ] || die "后端构建失败"

# 复制到项目根目录
cp "$BINARY" ./unicom
chmod +x ./unicom

SIZE=$(du -h ./unicom | cut -f1)
ok "构建完成: ./unicom ($SIZE)"
