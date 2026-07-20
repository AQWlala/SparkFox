/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SearchDegradeBanner 测试（spec §三 10.12.2 / U-06b 修复）
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 InstantHoverTooltip.test.ts、
 * MessageThinking.expansion.test.tsx）。本项目未引入 React Testing Library / jsdom，
 * 因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 10.12.2 验收点：
 *   1. event 表有数据时（is_degraded=false）隐藏横幅
 *   2. 降级到 VECTOR 时（is_degraded=true）显示横幅
 *   3. 横幅文案含「未抽取事件」关键词
 *   4. 横幅可关闭（点击关闭按钮后隐藏）
 */

import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'bun:test';

const source = readFileSync(new URL('./SearchDegradeBanner.tsx', import.meta.url), 'utf8');
const hookSource = readFileSync(new URL('./useDegradeBanner.ts', import.meta.url), 'utf8');
const cssSource = readFileSync(new URL('./SearchDegradeBanner.module.css', import.meta.url), 'utf8');

describe('SearchDegradeBanner', () => {
  test('test_banner_hidden_when_event_table_has_data', () => {
    // event 表有数据时（is_degraded=false）隐藏横幅
    // 组件必须基于 is_degraded prop 做条件渲染，且在 false 时返回 null
    expect(source.includes('is_degraded')).toBe(true);
    expect(source.includes('return null')).toBe(true);
    // 条件渲染：当 is_degraded 为 false 或 visible 为 false 时不渲染
    expect(
      source.includes('!is_degraded') || source.includes('!visible') || source.includes('is_degraded && visible')
    ).toBe(true);
  });

  test('test_banner_shown_when_degraded_to_vector', () => {
    // 降级到 VECTOR 时（is_degraded=true）显示横幅
    expect(source.includes('Alert')).toBe(true);
    expect(source.includes("type='warning'") || source.includes('type="warning"')).toBe(true);
    // 必须使用 Arco Design Alert 组件
    expect(source.includes("@arco-design/web-react")).toBe(true);
  });

  test('test_banner_text_mentions_no_event_extracted', () => {
    // 横幅文案含「未抽取事件」关键词（spec 明确要求）
    expect(source.includes('未抽取事件')).toBe(true);
    // 同时包含「VECTOR 检索」描述降级目标
    expect(source.includes('VECTOR')).toBe(true);
  });

  test('test_banner_dismissible', () => {
    // 横幅可关闭（点击关闭按钮后隐藏）
    // 1. Arco Alert 启用 closable 属性
    expect(source.includes('closable')).toBe(true);
    // 2. 关闭时调用 onDismiss 回调
    expect(source.includes('onDismiss')).toBe(true);
    // 3. useDegradeBanner hook 提供 visible 状态 + dismiss 方法
    expect(hookSource.includes('visible')).toBe(true);
    expect(hookSource.includes('dismiss')).toBe(true);
    // 4. 样式文件存在且定义了 banner 类
    expect(cssSource.includes('.banner')).toBe(true);
  });
});
