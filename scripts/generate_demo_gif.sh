#!/bin/bash
# =============================================================================
# generate_demo_gif.sh — 推理链可视化 GIF 生成脚本
# spec §三 12.5.2 / 第二十二波 sub-agent B
#
# 用途：
#   在 dev server 运行时录制 ReasoningChainPanel + KnowledgeGraphView 演示画面，
#   生成两个 GIF 文件，用于营销页 VideoDemoSection：
#     1. reasoning_chain_demo.gif — Step1..Step8 完整 8 步多跳推理流程（30s 循环）
#     2. multihop_demo.gif        — MULTI_ES 多跳路径 + 超边激活（20s 循环）
#
# 前置条件：
#   - dev server 已启动（http://localhost:5173）
#   - 已安装 ffmpeg + gifsicle
#     Windows 安装：winget install ffmpeg gifsicle
#     macOS 安装：brew install ffmpeg gifsicle
#     Linux 安装：apt install ffmpeg gifsicle
#
# 录制规范（验收标准）：
#   - 时长：reasoning_chain 30s / multihop 20s（循环播放）
#   - 帧率：10 fps（平衡流畅度与文件大小）
#   - 分辨率：800px 宽，高度按比例缩放
#   - 压缩：gifsicle --optimize=3 --colors=128
#   - 体积上限：< 5MB（验收标准，由 VideoDemoSection.test.tsx 强制检查）
#
# 用法：
#   cd SparkFox
#   bash scripts/generate_demo_gif.sh
#
# 输出：
#   ui/src/renderer/pages/marketing/assets/reasoning_chain_demo.gif
#   ui/src/renderer/pages/marketing/assets/multihop_demo.gif
# =============================================================================
set -euo pipefail

# 项目根目录（脚本所在目录的上一级）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# 输出目录（营销页 assets 目录）
ASSETS_DIR="$PROJECT_ROOT/ui/src/renderer/pages/marketing/assets"
mkdir -p "$ASSETS_DIR"

# 录制窗口区域（左上角 + 宽高，按 1920x1080 屏幕配置，按需调整）
# 录制 ReasoningChainPanel 区域
REASONING_CHAIN_REGION="100,100,1280,800"
# 录制 KnowledgeGraphView 区域
MULTIHOP_REGION="100,100,1280,800"

# 输出文件路径
REASONING_CHAIN_GIF="$ASSETS_DIR/reasoning_chain_demo.gif"
MULTIHOP_GIF="$ASSETS_DIR/multihop_demo.gif"

echo "=========================================="
echo "  SparkFox 推理链 GIF 演示生成脚本"
echo "=========================================="
echo ""
echo "前置检查：ffmpeg + gifsicle + dev server (http://localhost:5173)"
echo ""

# 检查 ffmpeg 是否安装
if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ERROR: ffmpeg 未安装。"
  echo "  Windows: winget install ffmpeg"
  echo "  macOS:   brew install ffmpeg"
  echo "  Linux:   sudo apt install ffmpeg"
  exit 1
fi

# 检查 gifsicle 是否安装
if ! command -v gifsicle >/dev/null 2>&1; then
  echo "ERROR: gifsicle 未安装。"
  echo "  Windows: winget install gifsicle"
  echo "  macOS:   brew install gifsicle"
  echo "  Linux:   sudo apt install gifsicle"
  exit 1
fi

echo "✓ ffmpeg + gifsicle 已安装"
echo ""

# -----------------------------------------------------------------------------
# 1. 生成 reasoning_chain_demo.gif（30s 循环，Step1..Step8 推理链流程）
# -----------------------------------------------------------------------------
echo "[1/2] 生成 reasoning_chain_demo.gif（30s 循环，Step1..Step8 推理链流程）..."

# Step 1：使用 ffmpeg 屏幕录制（gdigrab 用于 Windows，x11grab 用于 Linux，avfoundation 用于 macOS）
# 这里以 gdigrab（Windows）为例，跨平台时请按需切换
ffmpeg -y \
  -f gdigrab \
  -framerate 10 \
  -i desktop \
  -t 30 \
  -vf "fps=10,scale=800:-1" \
  -pix_fmt rgb24 \
  "$REASONING_CHAIN_GIF.tmp.gif"

# Step 2：使用 gifsicle 压缩（--optimize=3 最高压缩 + --colors=128 减色）
gifsicle --optimize=3 --colors=128 \
  "$REASONING_CHAIN_GIF.tmp.gif" \
  -o "$REASONING_CHAIN_GIF"

# 清理临时文件
rm -f "$REASONING_CHAIN_GIF.tmp.gif"

# 检查文件大小（必须 < 5MB = 5242880 字节）
RC_SIZE=$(stat -c%s "$REASONING_CHAIN_GIF" 2>/dev/null || stat -f%z "$REASONING_CHAIN_GIF")
echo "  reasoning_chain_demo.gif 大小: $RC_SIZE 字节"
if [ "$RC_SIZE" -ge 5242880 ]; then
  echo "  WARN: GIF 体积 ≥ 5MB，请降低帧率或缩短时长后重试"
fi
echo ""

# -----------------------------------------------------------------------------
# 2. 生成 multihop_demo.gif（20s 循环，MULTI_ES 多跳路径 + 超边激活）
# -----------------------------------------------------------------------------
echo "[2/2] 生成 multihop_demo.gif（20s 循环，MULTI_ES 多跳路径 + 超边激活）..."

ffmpeg -y \
  -f gdigrab \
  -framerate 10 \
  -i desktop \
  -t 20 \
  -vf "fps=10,scale=800:-1" \
  -pix_fmt rgb24 \
  "$MULTIHOP_GIF.tmp.gif"

gifsicle --optimize=3 --colors=128 \
  "$MULTIHOP_GIF.tmp.gif" \
  -o "$MULTIHOP_GIF"

rm -f "$MULTIHOP_GIF.tmp.gif"

MH_SIZE=$(stat -c%s "$MULTIHOP_GIF" 2>/dev/null || stat -f%z "$MULTIHOP_GIF")
echo "  multihop_demo.gif 大小: $MH_SIZE 字节"
if [ "$MH_SIZE" -ge 5242880 ]; then
  echo "  WARN: GIF 体积 ≥ 5MB，请降低帧率或缩短时长后重试"
fi
echo ""

# -----------------------------------------------------------------------------
# 汇总输出
# -----------------------------------------------------------------------------
echo "=========================================="
echo "  GIF 生成完成"
echo "=========================================="
echo "  reasoning_chain_demo.gif: $REASONING_CHAIN_GIF ($RC_SIZE 字节)"
echo "  multihop_demo.gif:        $MULTIHOP_GIF ($MH_SIZE 字节)"
echo ""
echo "  验收标准：每个 GIF < 5MB（5242880 字节）"
echo "  验收测试：cd ui && bun test src/renderer/pages/marketing/__tests__/VideoDemoSection.test.tsx"
echo "=========================================="
