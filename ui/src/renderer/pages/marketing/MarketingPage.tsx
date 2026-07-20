/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MarketingPage — 营销页主入口（spec §三 12.5.1 / 第二十一波 sub-step C + 12.5.2 / 第二十二波 sub-agent B）
 *
 * 营销策略：声明式优势描述（不直接竞品对比）+ 数据主权强调
 *
 * 5 个 Section：
 *   1. HeroSection            — 首屏（中文多跳 SOTA 卖点）
 *   2. BenchmarkSection       — 4 策略 Benchmark 数据展示
 *   3. DataSovereigntySection — 数据主权 slogan + 三大支柱
 *   4. ReasoningChainSection  — 推理链可视化 + 8 步多跳推理流程
 *   5. VideoDemoSection       — 推理链 GIF 演示区块（12.5.2 新增，2 个 GIF 占位）
 *
 * 路由（不修改 route 文件，仅组件导出）：
 *   由调用方按需引入 MarketingPage，本组件不直接挂载到 Router
 */

import React from 'react';
import HeroSection from './sections/HeroSection';
import BenchmarkSection from './sections/BenchmarkSection';
import DataSovereigntySection from './sections/DataSovereigntySection';
import ReasoningChainSection from './sections/ReasoningChainSection';
import VideoDemoSection from './sections/VideoDemoSection';

/**
 * MarketingPage — 营销页主入口
 *
 * 由 5 个 Section 组合而成，全页遵循「声明式优势描述」策略：
 * 采用 SparkFox 自身能力 + Benchmark 数据 + 数据主权 slogan 阐述价值主张，
 * 不与外部产品直接对比。
 *
 * 12.5.2 新增：VideoDemoSection 通过 2 个 GIF 演示直观展示推理链 8 步流程 +
 * MULTI_ES 多跳路径 + 超边激活，强化卖点感知。
 */
export function MarketingPage() {
  return (
    <div className="marketing-page">
      <HeroSection />
      <BenchmarkSection />
      <DataSovereigntySection />
      <ReasoningChainSection />
      <VideoDemoSection />
    </div>
  );
}

export default MarketingPage;
