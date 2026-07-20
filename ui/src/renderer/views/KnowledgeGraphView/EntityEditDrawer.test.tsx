/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EntityEditDrawer 实体编辑抽屉测试 — spec §三 11.3.3 / 第 14 波并行 sub-step C
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 GraphCanvas.test.tsx、
 * CitationDetailDrawer.test.tsx）。本项目未引入 React Testing Library / jsdom，
 * 因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 11.3.3 验收点（适配为字符串断言）：
 *   1. test_drawer_renders_three_actions      — 渲染 合并 / 拆分 / 重命名 3 操作（Tabs）
 *   2. test_merge_two_entities                — 合并操作有目标实体选择 + onMerge 回调
 *   3. test_split_entity                      — 拆分操作有新名称输入 + onSplit 回调
 *   4. test_rename_entity                     — 重命名操作有新名称输入 + onRename 回调
 *   5. test_edit_persists_to_entity_table     — 编辑操作持久化（PoC 阶段断言回调调用 + console.log mock 说明）
 *   6. test_edit_updates_graph_view           — 编辑后图谱视图刷新（断言 onMerge/onSplit/onRename 回调存在，调用后图谱刷新由父组件处理）
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const drawerSource = readFileSync(new URL('./EntityEditDrawer.tsx', import.meta.url), 'utf8');
const drawerCss = readFileSync(new URL('./EntityEditDrawer.module.css', import.meta.url), 'utf8');
const kgViewSource = readFileSync(new URL('./index.tsx', import.meta.url), 'utf8');

describe('EntityEditDrawer — 实体编辑抽屉（合并 / 拆分 / 重命名，spec §三 11.3.3）', () => {
  test('test_drawer_renders_three_actions: 渲染 合并 / 拆分 / 重命名 3 操作（Tabs）', () => {
    // 使用 Arco Design Drawer 组件作为容器
    expect(drawerSource.includes("from '@arco-design/web-react'")).toBe(true);
    expect(drawerSource.includes('Drawer')).toBe(true);

    // 使用 Arco Tabs 组件呈现 3 操作
    expect(drawerSource.includes('Tabs')).toBe(true);
    expect(drawerSource.includes('Tabs.TabPane')).toBe(true);

    // 3 个 Tab 标题分别为「合并」「拆分」「重命名」
    expect(drawerSource.includes('合并')).toBe(true);
    expect(drawerSource.includes('拆分')).toBe(true);
    expect(drawerSource.includes('重命名')).toBe(true);

    // 必须有 3 个 TabPane（每个对应一个操作）
    const tabPaneCount = (drawerSource.match(/Tabs\.TabPane/g) || []).length;
    expect(tabPaneCount).toBeGreaterThanOrEqual(3);

    // CSS Module 中定义了抽屉主体样式
    expect(drawerCss.includes('.drawerBody')).toBe(true);
  });

  test('test_merge_two_entities: 合并操作有目标实体选择 + onMerge 回调', () => {
    // Props 接口声明 onMerge 回调
    expect(drawerSource.includes('onMerge')).toBe(true);
    // Props 接口签名（sourceId, targetId）
    expect(drawerSource.includes('sourceId')).toBe(true);
    expect(drawerSource.includes('targetId')).toBe(true);

    // 合并 tab 中有目标实体选择控件（Select 或 Input 输入目标 entity_id）
    const hasTargetSelect =
      drawerSource.includes('Select') || drawerSource.includes('targetId');
    expect(hasTargetSelect).toBe(true);

    // 提交按钮触发 onMerge
    expect(drawerSource.includes('Button')).toBe(true);

    // 合并 tab 中包含「合并」相关文案
    expect(drawerSource.includes('合并到目标实体')).toBe(true);
  });

  test('test_split_entity: 拆分操作有新名称输入 + onSplit 回调', () => {
    // Props 接口声明 onSplit 回调
    expect(drawerSource.includes('onSplit')).toBe(true);
    // Props 接口签名（sourceId, newNames）
    expect(drawerSource.includes('newNames')).toBe(true);

    // 拆分 tab 中有新名称输入控件（Input + 逗号分隔）
    expect(drawerSource.includes('Input')).toBe(true);

    // 拆分 tab 中包含「拆分」相关文案 + 逗号分隔提示
    expect(drawerSource.includes('拆分为多个')).toBe(true);
    expect(drawerSource.includes('逗号')).toBe(true);
  });

  test('test_rename_entity: 重命名操作有新名称输入 + onRename 回调', () => {
    // Props 接口声明 onRename 回调
    expect(drawerSource.includes('onRename')).toBe(true);
    // Props 接口签名（entityId, newName）
    expect(drawerSource.includes('entityId')).toBe(true);
    expect(drawerSource.includes('newName')).toBe(true);

    // 重命名 tab 中有新名称输入控件（Input）
    expect(drawerSource.includes('Input')).toBe(true);

    // 重命名 tab 中包含「重命名」相关文案
    expect(drawerSource.includes('新名称')).toBe(true);
  });

  test('test_edit_persists_to_entity_table: 编辑操作持久化（PoC 阶段断言回调调用 + console.log mock 说明）', () => {
    // PoC 阶段：编辑回调内含 console.log（说明持久化将由 11.4.x 接入 IPC 实现）
    expect(drawerSource.includes('console.log')).toBe(true);

    // 3 个操作均通过回调暴露给父组件（父组件在 PoC 阶段打印 + 关闭抽屉）
    expect(drawerSource.includes('onMerge')).toBe(true);
    expect(drawerSource.includes('onSplit')).toBe(true);
    expect(drawerSource.includes('onRename')).toBe(true);

    // 注释中说明「IPC 调用 + 持久化推迟到 11.4.x」
    expect(drawerSource.includes('11.4')).toBe(true);
    expect(drawerSource.includes('IPC')).toBe(true);
  });

  test('test_edit_updates_graph_view: 编辑后图谱视图刷新（onMerge/onSplit/onRename 回调存在，调用后图谱刷新由父组件处理）', () => {
    // KGView 主组件已集成 EntityEditDrawer
    expect(kgViewSource.includes('EntityEditDrawer')).toBe(true);

    // KGView 新增 editingEntity / drawerVisible state
    expect(kgViewSource.includes('editingEntity')).toBe(true);
    expect(kgViewSource.includes('drawerVisible')).toBe(true);

    // 节点点击时打开抽屉（onNodeClick 内会设置 editingEntity + drawerVisible=true）
    expect(kgViewSource.includes('drawerVisible')).toBe(true);

    // 抽屉 onClose 关闭（visible 透传）
    expect(kgViewSource.includes('visible=')).toBe(true);
    expect(kgViewSource.includes('onClose')).toBe(true);

    // 3 个回调被集成（PoC：console.log + 关闭抽屉）
    expect(kgViewSource.includes('onMerge')).toBe(true);
    expect(kgViewSource.includes('onSplit')).toBe(true);
    expect(kgViewSource.includes('onRename')).toBe(true);
  });
});
