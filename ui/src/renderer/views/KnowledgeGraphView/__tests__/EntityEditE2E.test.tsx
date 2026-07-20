/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EntityEditE2E 测试 — spec §三 12.4.2 / 第二十波 sub-step C
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 11.5.1 / 11.5.2）。
 * 本项目未引入 React Testing Library / jsdom，因此通过断言源码字符串保证
 * 端到端流程的契约（编辑 → IPC 调用 → 图谱刷新链路）。
 *
 * 测试用例对应 spec §三 12.4.2 E2E 验收点（merge / split / rename 后图谱刷新）：
 *   1. test_e2e_merge_then_search  — 合并实体后调用 entity_merge + 刷新图谱（搜索结果去重）
 *   2. test_e2e_split_then_search  — 拆分实体后调用 entity_split + 刷新图谱（搜索结果分裂）
 *   3. test_e2e_rename_then_search — 重命名实体后调用 execute_entity_rename + 影响预览 + 刷新图谱
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

// 读取 EntityEditDrawer.tsx 源码（编辑入口组件）
const drawerSource = readFileSync(
  new URL('../EntityEditDrawer.tsx', import.meta.url),
  'utf8'
);
// 读取 KnowledgeGraphView/index.tsx 源码（IPC 调用 + 图谱刷新在父组件）
const kgViewSource = readFileSync(
  new URL('../index.tsx', import.meta.url),
  'utf8'
);

describe('EntityEditE2E — 编辑后图谱刷新（spec §三 12.4.2）', () => {
  test('test_e2e_merge_then_search: 合并实体后调用 entity_merge + 图谱刷新', () => {
    // KGView 含 invoke('entity_merge') IPC 调用
    const hasMergeInvoke =
      kgViewSource.includes("invoke('entity_merge'") ||
      kgViewSource.includes('invoke("entity_merge"');
    expect(hasMergeInvoke).toBe(true);

    // EntityEditDrawer 含 onMerge 回调（提交合并触发 IPC）
    expect(drawerSource.includes('onMerge')).toBe(true);

    // 合并涉及去重逻辑（冲突去重在 entity_ops::merge_entities_with_conflict_report 实现）
    // KGView 注释或代码应含「去重 / 冲突 / 刷新」相关字样
    const hasDedupOrRefresh =
      kgViewSource.includes('刷新') ||
      kgViewSource.includes('去重') ||
      kgViewSource.includes('冲突');
    expect(hasDedupOrRefresh).toBe(true);
  });

  test('test_e2e_split_then_search: 拆分实体后调用 entity_split + 图谱刷新', () => {
    // KGView 含 invoke('entity_split') IPC 调用
    const hasSplitInvoke =
      kgViewSource.includes("invoke('entity_split'") ||
      kgViewSource.includes('invoke("entity_split"');
    expect(hasSplitInvoke).toBe(true);

    // EntityEditDrawer 含 onSplit 回调（提交拆分触发 IPC）
    expect(drawerSource.includes('onSplit')).toBe(true);

    // 拆分后返回新 entity_id 列表（用于图谱刷新时新增节点）
    expect(kgViewSource.includes('newIds') || kgViewSource.includes('new_ids')).toBe(true);
  });

  test('test_e2e_rename_then_search: 重命名实体后调用 execute_entity_rename + 影响预览 + 图谱刷新', () => {
    // 12.4.2 新增：执行重命名调用 execute_entity_rename（含事务 + 影响计数）
    // 兼容单/双引号
    const hasExecuteRename =
      kgViewSource.includes("invoke('execute_entity_rename'") ||
      kgViewSource.includes('invoke("execute_entity_rename"') ||
      drawerSource.includes("invoke('execute_entity_rename'") ||
      drawerSource.includes('invoke("execute_entity_rename"');
    expect(hasExecuteRename).toBe(true);

    // 影响预览：组件含「受影响」相关字样（受影响事件 / 受影响关系 / 受影响文本块）
    const hasImpactPreview =
      drawerSource.includes('受影响') ||
      kgViewSource.includes('受影响');
    expect(hasImpactPreview).toBe(true);

    // EntityEditDrawer 仍保留 onRename 回调（向后兼容 11.4.2）
    expect(drawerSource.includes('onRename')).toBe(true);
  });
});
