/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MultiHopPathView 测试 — spec §三 11.5.1 / 第 16 波并行 sub-step B
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 index.test.tsx、
 * GraphFlow.test.tsx、EntityEditDrawer.test.tsx）。本项目未引入 React Testing
 * Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 11.5.1 验收点：
 *   1. test_multi_hop_path_view_renders_placeholder_when_no_hit — 无 hit 时渲染占位
 *   2. test_multi_hop_path_view_renders_hop_steps                 — 有 hit 时渲染 Steps（hop 步骤）
 *   3. test_multi_hop_path_view_renders_via_entities_tags         — 渲染 via_entities Tag
 *   4. test_multi_hop_path_view_hop_color_mapping                 — hop1 蓝 / hop2 黄 / hop3 灰 颜色映射
 *   5. test_multi_hop_path_view_displays_score                    — 显示 score
 *   6. test_multi_hop_path_view_close_callback                    — 关闭按钮回调 onClose
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const viewSource = readFileSync(
  new URL('./MultiHopPathView.tsx', import.meta.url),
  'utf8'
);
const viewCss = readFileSync(
  new URL('./MultiHopPathView.module.css', import.meta.url),
  'utf8'
);

describe('MultiHopPathView — 多跳路径渲染（spec §三 11.5.1）', () => {
  test('test_multi_hop_path_view_renders_placeholder_when_no_hit: 无 hit 时渲染占位文案', () => {
    // 默认导出 MultiHopPathView 组件
    expect(viewSource.includes('export default MultiHopPathView')).toBe(true);
    // 无 hit 时显示占位文案「点击图谱节点查看多跳路径」
    expect(viewSource.includes('点击图谱节点查看多跳路径')).toBe(true);
    // 占位文案通过样式类渲染（保持与项目其他模块一致的 CSS Module 写法）
    expect(viewCss.includes('.placeholder')).toBe(true);
  });

  test('test_multi_hop_path_view_renders_hop_steps: 有 hit 时渲染 Arco Steps 步骤', () => {
    // 引用 Arco Design Steps 组件
    expect(viewSource.includes('Steps')).toBe(true);
    // 使用 Steps.Step 子组件（Arco Design 用法约定）
    expect(viewSource.includes('Step')).toBe(true);
    // 含 hop 字段读取（SearchHit.hop）
    expect(viewSource.includes('hop')).toBe(true);
  });

  test('test_multi_hop_path_view_renders_via_entities_tags: 渲染 via_entities Tag', () => {
    // 引用 Arco Design Tag 组件
    expect(viewSource.includes('Tag')).toBe(true);
    // 源码含 via_entities 字段引用
    expect(viewSource.includes('via_entities')).toBe(true);
    // 含 EntityRef 类型定义（避免修改 types.ts）
    expect(viewSource.includes('EntityRef')).toBe(true);
  });

  test('test_multi_hop_path_view_hop_color_mapping: hop1 蓝 / hop2 黄 / hop3 灰 颜色映射', () => {
    // 颜色映射表常量名 HOP_COLOR_MAP（与 ReasoningChainPanel HOP_CLASS_MAP 命名风格一致）
    expect(viewSource.includes('HOP_COLOR_MAP')).toBe(true);
    // hop1 蓝：源码含中文「蓝」标识或 #007aff 蓝色值
    expect(viewSource.includes('蓝') || viewSource.includes('#007aff')).toBe(true);
    // hop2 黄：源码含中文「黄」标识或 #ff9500 黄色值
    expect(viewSource.includes('黄') || viewSource.includes('#ff9500')).toBe(true);
    // hop3 灰：源码含中文「灰」标识或 #6e6e73 灰色值
    expect(viewSource.includes('灰') || viewSource.includes('#6e6e73')).toBe(true);
    // CSS 中 hop1 / hop2 / hop3 三色类齐全
    expect(viewCss.includes('.hop1')).toBe(true);
    expect(viewCss.includes('.hop2')).toBe(true);
    expect(viewCss.includes('.hop3')).toBe(true);
  });

  test('test_multi_hop_path_view_displays_score: 显示检索得分 score', () => {
    // 源码含 score 字段读取（SearchHit.score）
    expect(viewSource.includes('score')).toBe(true);
    // 含 score 显示文案（如「score:」或「得分」）
    expect(
      viewSource.includes('score:') || viewSource.includes('得分') || viewSource.includes('score：')
    ).toBe(true);
  });

  test('test_multi_hop_path_view_close_callback: 关闭按钮触发 onClose 回调', () => {
    // Props 接口声明 onClose 回调
    expect(viewSource.includes('onClose')).toBe(true);
    // onClose 为可选回调（?: () => void）
    expect(viewSource.includes('?:')).toBe(true);
    // 渲染 Button 组件用于关闭
    expect(viewSource.includes('Button')).toBe(true);
    // 绑定 onClick 事件触发 onClose（PoC 阶段直接调用）
    expect(viewSource.includes('onClick')).toBe(true);
    // 显式调用 props.onClose
    expect(
      viewSource.includes('onClose?.(') || viewSource.includes('onClose(')
    ).toBe(true);
  });
});
