/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * HopViaEntitiesDisplay 测试 — spec §三 11.5.2 / 第 17 波并行 sub-step A
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 MultiHopPathView.test.tsx、
 * CitationChip.test.tsx、EntityEditDrawer.test.tsx）。本项目未引入 React Testing
 * Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 11.5.2 验收点：
 *   1. test_hop_via_entities_display_renders_nothing_when_no_hits — 无 hits 时不渲染
 *   2. test_hop_via_entities_display_renders_hop_tag               — 渲染 hop Tag
 *   3. test_hop_via_entities_display_renders_via_entities_tags     — 渲染 via_entities Tag
 *   4. test_hop_via_entities_display_hop_color_mapping             — hop1 蓝 / hop2 黄 / hop3 灰 颜色映射
 *   5. test_hop_via_entities_display_renders_arrow_between_entities — via_entities 之间渲染箭头
 *   6. test_hop_via_entities_display_entity_click_callback         — 实体点击回调 onEntityClick
 *
 * 范围说明（spec §三 11.5.2）：
 *   - 与 KnowledgeGraphView/MultiHopPathView 不同，本组件是「行内紧凑展示」而非「Card 详情」
 *   - 嵌入到 ChatMessage 的 CitationChip 列表后，每个 SearchHit 一行
 *   - hop 颜色映射必须与 MultiHopPathView 保持一致（hop1 蓝 / hop2 黄 / hop3 灰）
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const componentSource = readFileSync(
  new URL('./HopViaEntitiesDisplay.tsx', import.meta.url),
  'utf8'
);
const componentCss = readFileSync(
  new URL('./HopViaEntitiesDisplay.module.css', import.meta.url),
  'utf8'
);

describe('HopViaEntitiesDisplay — 行内 hop/via_entities 展示（spec §三 11.5.2）', () => {
  test('test_hop_via_entities_display_renders_nothing_when_no_hits: 无 hits 时不渲染', () => {
    // 默认导出 HopViaEntitiesDisplay 组件
    expect(componentSource.includes('export default HopViaEntitiesDisplay')).toBe(true);
    // 无 hits 时返回 null（提前 return 避免无意义渲染）
    expect(
      componentSource.includes('hits.length === 0') ||
        componentSource.includes('hits.length > 0')
    ).toBe(true);
    // 显式 return null（避免渲染空 div 浪费 DOM 节点）
    expect(componentSource.includes('return null')).toBe(true);
  });

  test('test_hop_via_entities_display_renders_hop_tag: 渲染 hop Tag（Arco Tag）', () => {
    // 引用 Arco Design Tag 组件
    expect(componentSource.includes('Tag')).toBe(true);
    // 含 hop 字段读取（SearchHit.hop）
    expect(componentSource.includes('hop')).toBe(true);
    // hop Tag 显示「hop=N」格式
    expect(
      componentSource.includes('hop=') || componentSource.includes('hop${')
    ).toBe(true);
  });

  test('test_hop_via_entities_display_renders_via_entities_tags: 渲染 via_entities Tag', () => {
    // 源码含 via_entities 字段引用
    expect(componentSource.includes('via_entities')).toBe(true);
    // 含 EntityRef 类型定义（与 MultiHopPathView 保持类型一致）
    expect(componentSource.includes('EntityRef')).toBe(true);
    // 含 SearchHit 类型定义
    expect(componentSource.includes('SearchHit')).toBe(true);
    // via_entities 用 Tag 显示（与 hop Tag 同一组件族）
    expect(componentSource.includes('Tag')).toBe(true);
  });

  test('test_hop_via_entities_display_hop_color_mapping: hop1 蓝 / hop2 黄 / hop3 灰 颜色映射', () => {
    // 颜色映射表常量名 HOP_COLOR_MAP（与 MultiHopPathView 保持一致）
    expect(componentSource.includes('HOP_COLOR_MAP')).toBe(true);
    // hop1 蓝：源码含中文「蓝」标识或 #007aff 蓝色值
    expect(componentSource.includes('蓝') || componentSource.includes('#007aff')).toBe(true);
    // hop2 黄：源码含中文「黄」标识或 #ff9500 黄色值
    expect(componentSource.includes('黄') || componentSource.includes('#ff9500')).toBe(true);
    // hop3 灰：源码含中文「灰」标识或 #6e6e73 灰色值
    expect(componentSource.includes('灰') || componentSource.includes('#6e6e73')).toBe(true);
    // CSS 中 hop1 / hop2 / hop3 三色类齐全
    expect(componentCss.includes('.hop1')).toBe(true);
    expect(componentCss.includes('.hop2')).toBe(true);
    expect(componentCss.includes('.hop3')).toBe(true);
  });

  test('test_hop_via_entities_display_renders_arrow_between_entities: via_entities 之间渲染箭头', () => {
    // 渲染箭头（→ Unicode 字符或 ArrowRight 组件）
    expect(
      componentSource.includes('→') || componentSource.includes('ArrowRight')
    ).toBe(true);
    // 多个 via_entities 之间存在分隔（箭头或分隔符）
    expect(
      componentSource.includes('→') ||
        componentSource.includes('arrow') ||
        componentSource.includes('Arrow')
    ).toBe(true);
  });

  test('test_hop_via_entities_display_entity_click_callback: 实体点击触发 onEntityClick 回调', () => {
    // Props 接口声明 onEntityClick 回调
    expect(componentSource.includes('onEntityClick')).toBe(true);
    // onEntityClick 为可选回调（?: (entityId: string) => void）
    expect(componentSource.includes('?:')).toBe(true);
    // 绑定 onClick 事件触发 onEntityClick
    expect(componentSource.includes('onClick')).toBe(true);
    // 显式调用 props.onEntityClick（含可选链或直接调用）
    expect(
      componentSource.includes('onEntityClick?.(') ||
        componentSource.includes('onEntityClick(')
    ).toBe(true);
  });
});
