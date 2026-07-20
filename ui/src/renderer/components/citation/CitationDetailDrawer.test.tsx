/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * CitationDetailDrawer 三级溯源抽屉测试
 *
 * 测试模式：源码审查测试（与项目其他 .test.ts 一致，使用 bun:test + readFileSync）
 * 覆盖 spec §三 10.10.1 + 10.10.2 的 TDD-RED 用例：
 *   1. test_renders_three_levels                      — 渲染 L1/L2/L3 三级
 *   2. test_l1_shows_entity_id_name_type              — L1 显示 entity_id + name + type
 *   3. test_l2_shows_event_subject_predicate_object   — L2 显示 subject + predicate + object
 *   4. test_l3_shows_chunk_id_span_text               — L3 显示 chunk_id + span + 原文片段
 *   5. test_drawer_open_close                         — 抽屉打开/关闭交互（Arco Drawer）
 *   6. test_empty_citation_renders_placeholder        — 空 citation 显示「暂无溯源信息」占位
 *
 * U-03 修复：三级溯源缺失问题（spec §三 10.10.1）
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const readSource = (url: URL): string => readFileSync(url, 'utf8');

const drawerSource = readSource(new URL('./CitationDetailDrawer.tsx', import.meta.url));
const entitySource = readSource(new URL('./EntityLevel.tsx', import.meta.url));
const eventSource = readSource(new URL('./EventLevel.tsx', import.meta.url));
const chunkSource = readSource(new URL('./ChunkLevel.tsx', import.meta.url));
const typesSource = readSource(new URL('./types.ts', import.meta.url));

describe('CitationDetailDrawer — 三级溯源抽屉', () => {
  test('test_renders_three_levels: 渲染 L1 实体 / L2 事件 / L3 chunk 三级', () => {
    // 主组件导入三级子组件
    expect(drawerSource.includes('import EntityLevel')).toBe(true);
    expect(drawerSource.includes('import EventLevel')).toBe(true);
    expect(drawerSource.includes('import ChunkLevel')).toBe(true);

    // 主组件在 JSX 中渲染三级
    expect(drawerSource.includes('<EntityLevel')).toBe(true);
    expect(drawerSource.includes('<EventLevel')).toBe(true);
    expect(drawerSource.includes('<ChunkLevel')).toBe(true);

    // 三级层级标题存在（L1 实体 / L2 事件 / L3 chunk）
    expect(drawerSource.includes('L1') || entitySource.includes('L1')).toBe(true);
    expect(drawerSource.includes('L2') || eventSource.includes('L2')).toBe(true);
    expect(drawerSource.includes('L3') || chunkSource.includes('L3')).toBe(true);
  });

  test('test_l1_shows_entity_id_name_type: L1 显示 entity_id + name + type', () => {
    // EntityLevel 接收 EntityRef 类型
    expect(entitySource.includes('EntityRef')).toBe(true);

    // 渲染 entity_id / name / entity_type 三个字段
    expect(entitySource.includes('entity_id')).toBe(true);
    expect(entitySource.includes('entity_type')).toBe(true);
    expect(entitySource.includes('name')).toBe(true);

    // 类型定义包含 EntityRef
    expect(typesSource.includes('export interface EntityRef')).toBe(true);
    expect(typesSource.includes('entity_id: string')).toBe(true);
    expect(typesSource.includes('entity_type: string')).toBe(true);
    expect(typesSource.includes('name: string')).toBe(true);
  });

  test('test_l2_shows_event_subject_predicate_object: L2 显示 subject + predicate + object', () => {
    // EventLevel 接收 EventRef 类型
    expect(eventSource.includes('EventRef')).toBe(true);

    // 渲染 event_id / subject / predicate / object 字段
    expect(eventSource.includes('event_id')).toBe(true);
    expect(eventSource.includes('subject')).toBe(true);
    expect(eventSource.includes('predicate')).toBe(true);
    expect(eventSource.includes('object')).toBe(true);

    // 类型定义包含 EventRef
    expect(typesSource.includes('export interface EventRef')).toBe(true);
    expect(typesSource.includes('event_id: string')).toBe(true);
    expect(typesSource.includes('subject: string')).toBe(true);
    expect(typesSource.includes('predicate: string')).toBe(true);
    expect(typesSource.includes('object: string')).toBe(true);
  });

  test('test_l3_shows_chunk_id_span_text: L3 显示 chunk_id + span + 原文片段', () => {
    // ChunkLevel 接收 ChunkRef 类型
    expect(chunkSource.includes('ChunkRef')).toBe(true);

    // 渲染 chunk_id / span / text 字段
    expect(chunkSource.includes('chunk_id')).toBe(true);
    expect(chunkSource.includes('span')).toBe(true);
    expect(chunkSource.includes('text')).toBe(true);

    // 类型定义包含 ChunkRef（span 为二元组）
    expect(typesSource.includes('export interface ChunkRef')).toBe(true);
    expect(typesSource.includes('chunk_id: string')).toBe(true);
    expect(typesSource.includes('span: [number, number]')).toBe(true);
    expect(typesSource.includes('text: string')).toBe(true);
  });

  test('test_drawer_open_close: 抽屉打开/关闭交互（用 Arco Drawer）', () => {
    // 使用 Arco Design Drawer 组件
    expect(drawerSource.includes("from '@arco-design/web-react'")).toBe(true);
    expect(drawerSource.includes('Drawer')).toBe(true);

    // Props 包含 visible + onClose
    expect(drawerSource.includes('visible')).toBe(true);
    expect(drawerSource.includes('onClose')).toBe(true);

    // visible 透传给 Arco Drawer
    expect(drawerSource.includes('visible={visible}')).toBe(true);

    // onClose 透传给 Arco Drawer onCancel
    expect(drawerSource.includes('onCancel={onClose}')).toBe(true);
  });

  test('test_empty_citation_renders_placeholder: 空 citation 显示占位「暂无溯源信息」', () => {
    // 当 citation 为 null 时显示占位文本
    expect(drawerSource.includes('暂无溯源信息')).toBe(true);

    // 通过判断 citation 是否为 null 决定渲染内容
    expect(drawerSource.includes('citation')).toBe(true);
    expect(drawerSource.includes('!citation') || drawerSource.includes('citation === null')).toBe(true);
  });
});
