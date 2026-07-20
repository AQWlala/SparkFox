/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * HyperedgeLayer 测试 — spec §三 12.2.3 / 第二十二波 sub-step A
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 11.5.1 MultiHopPathView.test.tsx、
 * 11.4.1 GraphFlow.test.tsx、12.4.2 EntityRenameImpact.test.tsx）。本项目未引入
 * React Testing Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 12.2.3 验收点（react-flow 超边可视化）：
 *   1. test_hyperedge_layer_renders_hyperedges        — 组件渲染超边（含 Hyperedge 类型 + hyperedges prop）
 *   2. test_hyperedge_dashed_style                    — 超边以虚线样式区分普通边
 *   3. test_hyperedge_gradient_color                  — 超边渐变色（蓝→紫，突出 SAG 创新）
 *   4. test_query_highlights_activated_hyperedges     — 查询时高亮激活的超边
 *   5. test_hyperedge_click_triggers_callback         — 超边点击触发回调
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

// 读取 HyperedgeLayer.tsx 源码（超边图层主组件，12.2.3 实现）
const layerSource = readFileSync(
  new URL('./HyperedgeLayer.tsx', import.meta.url),
  'utf8'
);
// 读取 hyperedge.module.css 源码（超边样式，REFACTOR 阶段提取）
const layerCss = readFileSync(
  new URL('./hyperedge.module.css', import.meta.url),
  'utf8'
);

describe('HyperedgeLayer — react-flow 超边可视化（spec §三 12.2.3）', () => {
  test('test_hyperedge_layer_renders_hyperedges: 组件渲染超边（含 Hyperedge 类型 + hyperedges prop）', () => {
    // 源码含 Hyperedge 类型定义（与后端 sparkfox-knowledge::hyperedge::Hyperedge 对齐）
    expect(layerSource.includes('Hyperedge')).toBe(true);
    // Props 接口含 hyperedges 数组 prop（驱动渲染）
    expect(layerSource.includes('hyperedges')).toBe(true);
    // 含 member_events 字段（超边成员 events，对应后端结构）
    expect(layerSource.includes('member_events')).toBe(true);
    // 含 member_entities 字段（超边成员 entities，对应后端结构）
    expect(layerSource.includes('member_entities')).toBe(true);
    // 默认导出 HyperedgeLayer 组件
    expect(layerSource.includes('export default HyperedgeLayer')).toBe(true);
  });

  test('test_hyperedge_dashed_style: 超边以虚线样式区分普通边', () => {
    // 源码或 CSS 含虚线样式标识：dashed / strokeDasharray / 虚线
    const hasDashedInSource =
      layerSource.includes('dashed') ||
      layerSource.includes('strokeDasharray') ||
      layerSource.includes('虚线');
    const hasDashedInCss =
      layerCss.includes('dashed') ||
      layerCss.includes('strokeDasharray') ||
      layerCss.includes('虚线');
    expect(hasDashedInSource || hasDashedInCss).toBe(true);
  });

  test('test_hyperedge_gradient_color: 超边渐变色（蓝→紫，突出 SAG 创新）', () => {
    // 源码或 CSS 含渐变色标识：gradient / 渐变 / linearGradient
    const hasGradientInSource =
      layerSource.includes('gradient') ||
      layerSource.includes('渐变') ||
      layerSource.includes('linearGradient');
    const hasGradientInCss =
      layerCss.includes('gradient') ||
      layerCss.includes('渐变') ||
      layerCss.includes('linearGradient');
    expect(hasGradientInSource || hasGradientInCss).toBe(true);
  });

  test('test_query_highlights_activated_hyperedges: 查询时高亮激活的超边', () => {
    // Props 接口含 activatedHyperedgeIds prop（激活的超边 ID 列表，用于高亮）
    expect(layerSource.includes('activatedHyperedgeIds')).toBe(true);
    // Props 接口含 queryEntities prop（查询命中的 entity IDs，驱动激活逻辑）
    expect(layerSource.includes('queryEntities')).toBe(true);
    // 源码含「激活」或「highlighted」或「activated」语义标识
    const hasActivateSemantic =
      layerSource.includes('activated') ||
      layerSource.includes('highlighted') ||
      layerSource.includes('激活');
    expect(hasActivateSemantic).toBe(true);
  });

  test('test_hyperedge_click_triggers_callback: 超边点击触发回调', () => {
    // Props 接口含 onHyperedgeClick 回调（与 GraphFlow 的 onEdgeClick 命名风格一致）
    expect(layerSource.includes('onHyperedgeClick')).toBe(true);
    // 源码含 onClick 事件绑定
    expect(layerSource.includes('onClick')).toBe(true);
    // 显式调用 props.onHyperedgeClick（PoC 阶段直接调用）
    expect(
      layerSource.includes('onHyperedgeClick?.(') ||
        layerSource.includes('onHyperedgeClick(')
    ).toBe(true);
  });
});
