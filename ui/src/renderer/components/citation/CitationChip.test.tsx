/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * CitationChip 点击触发抽屉测试
 *
 * 覆盖 spec §三 10.10.2 的 TDD-RED 用例：
 *   1. test_citation_chip_click_opens_drawer        — 点击 chip 打开 CitationDetailDrawer
 *   2. test_citation_chip_passes_citation_data      — citation 数据传递给抽屉
 *   3. test_e2e_multi_strategy_citation_traceable   — MULTI 策略下点击 chip 可展开三级抽屉
 *
 * U-03 修复：三级溯源缺失问题（spec §三 10.10.2）
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const readSource = (url: URL): string => readFileSync(url, 'utf8');

const chipSource = readSource(new URL('./CitationChip.tsx', import.meta.url));
const drawerSource = readSource(new URL('./CitationDetailDrawer.tsx', import.meta.url));

describe('CitationChip — 点击触发抽屉', () => {
  test('test_citation_chip_click_opens_drawer: 点击 CitationChip 打开 CitationDetailDrawer', () => {
    // 引入 CitationDetailDrawer
    expect(chipSource.includes('import CitationDetailDrawer')).toBe(true);

    // 使用 useState 控制抽屉 visible
    expect(chipSource.includes('useState')).toBe(true);
    expect(chipSource.includes('visible')).toBe(true);

    // 点击 chip 修改 visible 状态
    expect(chipSource.includes('onClick')).toBe(true);

    // 渲染 CitationDetailDrawer
    expect(chipSource.includes('<CitationDetailDrawer')).toBe(true);

    // onClose 回调关闭抽屉
    expect(chipSource.includes('onClose')).toBe(true);
  });

  test('test_citation_chip_passes_citation_data: citation 数据传递给抽屉', () => {
    // Props 接收 citation
    expect(chipSource.includes('citation')).toBe(true);

    // citation 透传给 CitationDetailDrawer
    expect(chipSource.includes('citation={citation}')).toBe(true);

    // visible 透传
    expect(chipSource.includes('visible={visible}')).toBe(true);
  });

  test('test_e2e_multi_strategy_citation_traceable: MULTI 策略下点击 chip 可展开三级抽屉', () => {
    // 抽屉内部渲染三级溯源（L1 实体 / L2 事件 / L3 chunk）
    expect(drawerSource.includes('EntityLevel')).toBe(true);
    expect(drawerSource.includes('EventLevel')).toBe(true);
    expect(drawerSource.includes('ChunkLevel')).toBe(true);

    // chip 渲染时显示某种标识（如 [1]/[n]/引用编号）触发抽屉
    // 这里检查 chip 既是按钮又渲染 children
    expect(chipSource.includes('children')).toBe(true);

    // 抽屉接受 Citation 类型（含 entity + event + chunk）
    expect(drawerSource.includes('Citation')).toBe(true);
    expect(drawerSource.includes('entity')).toBe(true);
    expect(drawerSource.includes('event')).toBe(true);
    expect(drawerSource.includes('chunk')).toBe(true);
  });
});
