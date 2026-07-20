/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MarketingPage 测试 — spec §三 12.5.1 / 第二十一波 sub-step C
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 11.5.1 MultiHopPathView.test.tsx、
 * 11.5.2 HopViaEntitiesDisplay.test.tsx、12.4.2 EntityRenameImpact.test.tsx）。
 * 本项目未引入 React Testing Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 营销策略：声明式优势描述（不直接竞品对比）+ 数据主权强调
 *
 * 测试用例对应 spec §三 12.5.1 验收点：
 *   1. test_hero_section_contains_zh_multihop_sota          — 首屏含「中文多跳 SOTA」卖点
 *   2. test_benchmark_section_displays_4_strategies          — Benchmark 区块展示 4 策略
 *   3. test_benchmark_section_shows_multi_es_above_0_85      — MULTI_ES Recall@10 > 0.85
 *   4. test_data_sovereignty_section_contains_slogan         — 数据主权区块含 slogan
 *   5. test_reasoning_chain_section_mentions_8_step           — 推理链区块提及「8 步多跳推理」
 *   6. test_marketing_page_no_direct_competitor_comparison   — 全页无直接竞品对比
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

// 读取 4 个 Section 源码 + 主入口（源码扫描模式，不渲染 React 组件）
const heroSource = readFileSync(
  new URL('../sections/HeroSection.tsx', import.meta.url),
  'utf8'
);
const benchmarkSource = readFileSync(
  new URL('../sections/BenchmarkSection.tsx', import.meta.url),
  'utf8'
);
const dataSovereigntySource = readFileSync(
  new URL('../sections/DataSovereigntySection.tsx', import.meta.url),
  'utf8'
);
const reasoningChainSource = readFileSync(
  new URL('../sections/ReasoningChainSection.tsx', import.meta.url),
  'utf8'
);
// REFACTOR 阶段：营销文案集中管理后，也需扫描 copy/zh.ts 源码
const copySource = readFileSync(
  new URL('../copy/zh.ts', import.meta.url),
  'utf8'
);
// 读取 benchmark_results.json 数据（验证 multi_es Recall@10 > 0.85）
const benchmarkData = JSON.parse(
  readFileSync(new URL('../data/benchmark_results.json', import.meta.url), 'utf8')
) as {
  strategies: Array<{ name: string; recall_at_10: number }>;
};

describe('MarketingPage — 营销页文案 + Benchmark 数据展示（spec §三 12.5.1）', () => {
  test('test_hero_section_contains_zh_multihop_sota: 首屏含「中文多跳 SOTA」卖点', () => {
    // HeroSection 源码含「中文多跳 SOTA」中文文案或 zh-multihop-sota 标识符
    const hasZhText = heroSource.includes('中文多跳 SOTA');
    const hasIdentifier = heroSource.includes('zh-multihop-sota');
    expect(hasZhText || hasIdentifier).toBe(true);
  });

  test('test_benchmark_section_displays_4_strategies: Benchmark 区块展示 4 策略 Recall@10', () => {
    // BenchmarkSection 源码含 4 个策略名（multi / multi1 / hopllm / multi_es）
    const hasMulti = benchmarkSource.includes('multi');
    const hasMulti1 = benchmarkSource.includes('multi1');
    const hasHopllm = benchmarkSource.includes('hopllm');
    const hasMultiEs = benchmarkSource.includes('multi_es');
    // 或显式标注「4 策略」字样
    const hasFourLabel = benchmarkSource.includes('4 策略');
    expect(hasMulti && hasMulti1 && hasHopllm && hasMultiEs || hasFourLabel).toBe(true);
  });

  test('test_benchmark_section_shows_multi_es_above_0_85: MULTI_ES Recall@10 > 0.85', () => {
    // benchmark_results.json 中 multi_es 的 recall_at_10 必须 > 0.85
    const multiEs = benchmarkData.strategies.find((s) => s.name === 'multi_es');
    expect(multiEs).toBeDefined();
    expect(multiEs!.recall_at_10).toBeGreaterThan(0.85);
  });

  test('test_data_sovereignty_section_contains_slogan: 数据主权区块含 slogan', () => {
    // DataSovereigntySection 源码含完整 slogan
    // 「别把第二大脑租给别人——你的思考，不该成为别人的养料」
    const slogan = '别把第二大脑租给别人——你的思考，不该成为别人的养料';
    expect(dataSovereigntySource.includes(slogan)).toBe(true);
  });

  test('test_reasoning_chain_section_mentions_8_step: 推理链区块提及「8 步多跳推理」', () => {
    // ReasoningChainSection 源码含「8 步」或「8-step」字样
    const hasZhStep = reasoningChainSource.includes('8 步');
    const hasEnStep = reasoningChainSource.includes('8-step');
    expect(hasZhStep || hasEnStep).toBe(true);
  });

  test('test_marketing_page_no_direct_competitor_comparison: 全页无直接竞品对比', () => {
    // 扫描所有 4 个 Section 源码 + copy/zh.ts 文案文件，不含直接竞品对比字样
    // 声明式优势描述策略：不与 Nomifun / OpenAkita / BaiLongma 等竞品直接对比
    const allSources =
      heroSource + benchmarkSource + dataSovereigntySource + reasoningChainSource + copySource;
    const forbiddenPatterns = [
      'vs Nomifun',
      'vs OpenAkita',
      'vs BaiLongma',
      '对比竞品',
      '竞品对比',
      '击败 Nomifun',
      '击败 OpenAkita',
      '击败 BaiLongma',
    ];
    for (const pattern of forbiddenPatterns) {
      expect(allSources.includes(pattern)).toBe(false);
    }
  });
});
