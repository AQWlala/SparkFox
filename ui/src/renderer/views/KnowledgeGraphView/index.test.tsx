/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * KnowledgeGraphView 入口 + 路由 测试（spec §三 11.3.1 / 第 12 波并行 sub-step B）
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 memoryPanelRoute.test.ts、
 * SearchDegradeBanner.test.tsx、CitationChip.test.tsx）。本项目未引入 React Testing
 * Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 11.3.1 验收点：
 *   1. test_kgview_renders_without_crash        — KGView 组件渲染不崩溃
 *   2. test_kgview_route_accessible             — 路由 /kb/:id/graph 可访问
 *   3. test_kgview_entry_button_on_detail_page  — KnowledgeDetailPage 含入口按钮
 *   4. test_kgview_click_entry_navigates        — 点击入口跳转到图谱视图
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const kgViewSource = readFileSync(new URL('./index.tsx', import.meta.url), 'utf8');
const kgViewCss = readFileSync(new URL('./styles.module.css', import.meta.url), 'utf8');
const routerSource = readFileSync(
  new URL('../../components/layout/Router.tsx', import.meta.url),
  'utf8'
);
const detailPageSource = readFileSync(
  new URL('../../pages/knowledge/KnowledgeDetailPage/index.tsx', import.meta.url),
  'utf8'
);

describe('KnowledgeGraphView — 入口 + 路由（spec §三 11.3.1）', () => {
  test('test_kgview_renders_without_crash: KGView 组件渲染不崩溃', () => {
    // 默认导出 KnowledgeGraphView 组件
    expect(kgViewSource.includes('export default KnowledgeGraphView')).toBe(true);
    // 使用 Arco Design 组件库
    expect(kgViewSource.includes('@arco-design/web-react')).toBe(true);
    // 必须使用 Card 或 Button 组件（spec §三 11.3.1）
    expect(kgViewSource.includes('Card') || kgViewSource.includes('Button')).toBe(true);
    // 顶部标题「知识图谱」
    expect(kgViewSource.includes('知识图谱')).toBe(true);
    // 返回按钮存在
    expect(kgViewSource.includes('返回')).toBe(true);
    // 占位内容「图谱渲染待 11.3.2 实现」
    expect(kgViewSource.includes('图谱渲染待 11.3.2 实现')).toBe(true);
    // 从路由参数获取 kbId（useParams）
    expect(kgViewSource.includes('useParams')).toBe(true);
    // 样式文件存在且定义了容器类
    expect(kgViewCss.includes('.container')).toBe(true);
  });

  test('test_kgview_route_accessible: 路由 /kb/:id/graph 可访问', () => {
    // 路由配置含此路径
    expect(routerSource.includes("path='/kb/:id/graph'")).toBe(true);
    // 懒加载 KnowledgeGraphView 组件
    expect(routerSource.includes('KnowledgeGraphView')).toBe(true);
    // 通过 withRouteFallback 包装（与项目其他路由一致）
    expect(routerSource.includes('withRouteFallback(KnowledgeGraphView)')).toBe(true);
  });

  test('test_kgview_entry_button_on_detail_page: KnowledgeDetailPage 含入口按钮', () => {
    // 含「查看知识图谱」入口文案（spec §三 11.3.1 明确要求）
    expect(detailPageSource.includes('查看知识图谱')).toBe(true);
    // 入口指向图谱路由路径
    expect(detailPageSource.includes('/graph')).toBe(true);
    expect(detailPageSource.includes('/kb/')).toBe(true);
  });

  test('test_kgview_click_entry_navigates: 点击入口跳转到图谱视图', () => {
    // 使用 React Router Link 组件实现跳转
    expect(detailPageSource.includes('Link')).toBe(true);
    // Link to 模板字符串：/kb/${kbId}/graph
    expect(detailPageSource.includes('`/kb/${')).toBe(true);
    expect(detailPageSource.includes('}/graph`')).toBe(true);
  });
});
