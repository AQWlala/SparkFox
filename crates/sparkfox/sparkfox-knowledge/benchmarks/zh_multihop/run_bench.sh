#!/usr/bin/env bash
# Sub-Step 12.3.2 — 4 策略对比 Benchmark 运行脚本（spec §三 12.3.2）
#
# ## 用途
# 一键运行 4 策略对比 benchmark，输出重定向到 results.json（基于 results_template.json 填充）。
#
# ## 使用方式
# ```bash
# cd crates/sparkfox/sparkfox-knowledge
# bash benchmarks/zh_multihop/run_bench.sh
# ```
#
# ## 输出
# - STDOUT：4 策略对比表（Markdown 格式）+ 6 个测试的运行结果
# - benchmarks/zh_multihop/results.json：完整测试输出（stdout + stderr）
#
# ## 依赖
# - Rust toolchain（cargo）
# - 测试 binary 已编译（cargo test 会自动处理）
#
# ## License
# AGPL-3.0-only

set -euo pipefail

# 切换到 crate 根目录（脚本所在目录的上两级）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${CRATE_DIR}"

echo "========================================================"
echo "SparkFox Sub-Step 12.3.2 — 4 策略对比 Benchmark"
echo "========================================================"
echo "工作目录: $(pwd)"
echo "时间: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""

# 输出文件（覆盖上次结果）
RESULTS_FILE="benchmarks/zh_multihop/results.json"

# 初始化 results.json（基于 template）
if [ -f "benchmarks/zh_multihop/results_template.json" ]; then
    cp benchmarks/zh_multihop/results_template.json "${RESULTS_FILE}"
fi

# 运行 benchmark 测试（--ignored 触发 #[ignore] 测试，--nocapture 输出 println!）
# 输出重定向到 results.json + stdout
echo "运行 cargo test --test bench_compare_4_strategies -- --ignored --nocapture ..."
echo ""

# 使用 tee 同时输出到 stdout 和 results.json
cargo test -p sparkfox-knowledge --test bench_compare_4_strategies -- --ignored --nocapture 2>&1 | tee "${RESULTS_FILE}"

echo ""
echo "========================================================"
echo "Benchmark 完成"
echo "结果已保存到: ${RESULTS_FILE}"
echo "========================================================"
