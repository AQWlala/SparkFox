#!/bin/bash
# =============================================================================
# SparkFox 合规检查脚本（v1.1.0）
# =============================================================================
# 用途：自动化验证 NOTICE / LICENSE 合规性
# 运行：bash scripts/compliance_check.sh
# CI：GitHub Actions 兼容（Windows 通过 git bash 运行）
# 检查项：5 项（AGPL 声明 / 上游致谢 / SAG 引用 / 无 MIT 残留 / Apache 致谢）
# =============================================================================
set -e

echo "=== SparkFox 合规检查 v1.1.0 ==="

# 检查 1: 全局 NOTICE 含 AGPL-3.0 声明
# 合规要求：AGPL-3.0-only 是项目主许可证，全局 NOTICE 必须明确声明
check_global_notice_agpl() {
  if grep -q "AGPL-3.0-only" NOTICE; then
    echo "✓ 检查 1 通过: 全局 NOTICE 含 AGPL-3.0 声明"
    return 0
  else
    echo "✗ 检查 1 失败: 全局 NOTICE 缺少 AGPL-3.0 声明"
    return 1
  fi
}

# 检查 2: 每个 crate 的 NOTICE 含上游致谢
# 合规要求：Apache-2.0 §4(d) / MIT 许可证均要求保留上游致谢
check_per_crate_attribution() {
  local crates=("sparkfox-knowledge" "sparkfox-graph" "sparkfox-llm")
  for crate in "${crates[@]}"; do
    local notice="crates/sparkfox/$crate/NOTICE"
    if [ -f "$notice" ] && grep -q "致谢\|Attribution\|上游" "$notice"; then
      echo "✓ 检查 2 通过: $crate NOTICE 含上游致谢"
    else
      echo "✗ 检查 2 失败: $crate NOTICE 缺少上游致谢"
      return 1
    fi
  done
  return 0
}

# 检查 3: NOTICE 含 SAG 引用声明（XLM-RoBERTa / jieba / DuReader / CMRC2018）
# 合规要求：v1.1.0 引入 SAG 中文检索增强流程，相关数据集与模型必须显式致谢
check_sag_attribution() {
  local sag_deps=("XLM-RoBERTa" "jieba" "DuReader" "CMRC2018")
  for dep in "${sag_deps[@]}"; do
    if grep -q "$dep" NOTICE; then
      echo "✓ SAG 依赖 $dep 已致谢"
    else
      echo "✗ 检查 3 失败: NOTICE 缺少 SAG 依赖 $dep 致谢"
      return 1
    fi
  done
  return 0
}

# 检查 4: AGPL crate 中无 MIT 文件残留
# 合规要求：crate 内不应直接包含 MIT LICENSE 文件（MIT 依赖致谢在 NOTICE 中保留即可）
# 注意：target/ 与 bench/ 目录是构建产物 / POC，不属于 crate 源码
check_no_mit_in_agpl_crates() {
  local mit_files=$(find crates/sparkfox/ -name "LICENSE*" \
    -not -path "*/target/*" \
    -not -path "*/bench/*" \
    -exec grep -l "MIT License" {} \; 2>/dev/null || true)
  if [ -z "$mit_files" ]; then
    echo "✓ 检查 4 通过: AGPL crate 中无 MIT 文件残留"
    return 0
  else
    echo "✗ 检查 4 失败: AGPL crate 中发现 MIT LICENSE 文件: $mit_files"
    return 1
  fi
}

# 检查 5: Apache 依赖致谢完整（hnswlib-rs / candle-core / petgraph 等）
# 合规要求：Apache-2.0 §4(d) 要求 NOTICE 文件保留所有上游致谢
check_apache_attribution() {
  local apache_deps=("hnsw" "petgraph" "candle")
  for dep in "${apache_deps[@]}"; do
    if grep -qi "$dep" NOTICE; then
      echo "✓ Apache 依赖 $dep 已致谢"
    else
      echo "✗ 检查 5 失败: NOTICE 缺少 Apache 依赖 $dep 致谢"
      return 1
    fi
  done
  return 0
}

# 执行所有检查（任一失败即整体失败）
check_global_notice_agpl
check_per_crate_attribution
check_sag_attribution
check_no_mit_in_agpl_crates
check_apache_attribution

echo "=== 所有检查通过 ==="
