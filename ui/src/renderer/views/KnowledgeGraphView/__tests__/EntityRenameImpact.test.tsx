/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EntityRenameImpact 测试 — spec §三 12.4.2 / 第二十波 sub-step C
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 11.5.1 MultiHopPathView.test.tsx、
 * 11.5.2 HopViaEntitiesDisplay.test.tsx）。本项目未引入 React Testing Library / jsdom，
 * 因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 12.4.2 验收点（重命名全局影响预览）：
 *   1. test_rename_preview_shows_affected_events   — 预览显示受影响 events 数量
 *   2. test_rename_preview_shows_affected_relations — 预览显示受影响 event_entity_relation 数量
 *   3. test_rename_preview_shows_affected_chunks    — 预览显示受影响 chunks（文本块）数量
 *   4. test_rename_preview_invokes_ipc_command      — 预览调用 IPC 命令 preview_entity_rename_impact
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

// 读取 EntityEditDrawer.tsx 源码（影响预览面板在重命名 tab 中实现）
const drawerSource = readFileSync(
  new URL('../EntityEditDrawer.tsx', import.meta.url),
  'utf8'
);
// 读取 KnowledgeGraphView/index.tsx 源码（IPC 调用可能在父组件或子组件中触发）
const kgViewSource = readFileSync(
  new URL('../index.tsx', import.meta.url),
  'utf8'
);

describe('EntityRenameImpact — 重命名全局影响预览（spec §三 12.4.2）', () => {
  test('test_rename_preview_shows_affected_events: 预览显示受影响 events 数量', () => {
    // 组件源码含 affectedEvents 字段（IPC 返回结构）或「受影响事件」中文文案
    const hasField = drawerSource.includes('affectedEvents');
    const hasLabel =
      drawerSource.includes('受影响事件') ||
      drawerSource.includes('受影响 Events');
    expect(hasField || hasLabel).toBe(true);
  });

  test('test_rename_preview_shows_affected_relations: 预览显示受影响 event_entity_relation 数量', () => {
    // 组件源码含 affectedRelations 字段或「受影响关系」中文文案
    const hasField = drawerSource.includes('affectedRelations');
    const hasLabel =
      drawerSource.includes('受影响关系') ||
      drawerSource.includes('受影响 Relations');
    expect(hasField || hasLabel).toBe(true);
  });

  test('test_rename_preview_shows_affected_chunks: 预览显示受影响 chunks 数量', () => {
    // 组件源码含 affectedChunks 字段或「受影响文本块」中文文案
    const hasField = drawerSource.includes('affectedChunks');
    const hasLabel =
      drawerSource.includes('受影响文本块') ||
      drawerSource.includes('受影响 Chunks');
    expect(hasField || hasLabel).toBe(true);
  });

  test('test_rename_preview_invokes_ipc_command: 预览调用 IPC 命令 preview_entity_rename_impact', () => {
    // 组件源码（EntityEditDrawer 或 KnowledgeGraphView）含 invoke('preview_entity_rename_impact') 调用
    // 兼容单引号 / 双引号两种写法
    const hasInvokeInDrawer =
      drawerSource.includes("invoke('preview_entity_rename_impact'") ||
      drawerSource.includes('invoke("preview_entity_rename_impact"');
    const hasInvokeInKgView =
      kgViewSource.includes("invoke('preview_entity_rename_impact'") ||
      kgViewSource.includes('invoke("preview_entity_rename_impact"');
    expect(hasInvokeInDrawer || hasInvokeInKgView).toBe(true);
  });
});
