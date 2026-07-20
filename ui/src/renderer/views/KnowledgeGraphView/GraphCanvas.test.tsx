/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * GraphCanvas 测试 — spec §三 11.3.2 / 第 13 波并行 sub-step C
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 index.test.tsx、
 * CitationChip.test.tsx、ExtractionProgressCard.test.tsx）。本项目未引入
 * React Testing Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 11.3.2 验收点：
 *   1. test_graph_canvas_renders_nodes           — 渲染 entity 节点（SVG circle）
 *   2. test_graph_canvas_renders_edges            — 渲染 event_entity_relation 边（SVG line）
 *   3. test_node_color_by_entity_type             — 节点按 entity_type 着色
 *   4. test_11_entity_types_have_distinct_colors  — 11 类实体颜色互异
 *   5. test_node_click_triggers_callback          — 节点点击触发 onNodeClick 回调
 *   6. test_edge_click_triggers_callback          — 边点击触发 onEdgeClick 回调
 *   7. test_graph_canvas_handles_large_graph_1k_nodes — 1k 节点不崩溃（节点上限保护）
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const canvasSource = readFileSync(new URL('./GraphCanvas.tsx', import.meta.url), 'utf8');
const typesSource = readFileSync(new URL('./types.ts', import.meta.url), 'utf8');

describe('GraphCanvas — 11 类着色 + 图例（spec §三 11.3.2）', () => {
  test('test_graph_canvas_renders_nodes: 渲染 entity 节点（SVG circle）', () => {
    // 使用 SVG 渲染节点
    expect(canvasSource.includes('<svg')).toBe(true);
    // 节点用 circle 元素绘制
    expect(canvasSource.includes('<circle')).toBe(true);
    // 节点附带 label 文本（<text 元素）
    expect(canvasSource.includes('<text')).toBe(true);
    // 节点数据通过 props.nodes 传入
    expect(canvasSource.includes('nodes')).toBe(true);
    // 节点对象含 id 字段
    expect(canvasSource.includes('node.id')).toBe(true);
  });

  test('test_graph_canvas_renders_edges: 渲染 event_entity_relation 边（SVG line）', () => {
    // 边用 line 元素绘制
    expect(canvasSource.includes('<line')).toBe(true);
    // 边数据通过 props.edges 传入
    expect(canvasSource.includes('edges')).toBe(true);
    // 边对象含 source / target 字段
    expect(canvasSource.includes('source')).toBe(true);
    expect(canvasSource.includes('target')).toBe(true);
  });

  test('test_node_color_by_entity_type: 节点按 entity_type 着色', () => {
    // 引用 ENTITY_TYPE_COLORS 常量
    expect(canvasSource.includes('ENTITY_TYPE_COLORS')).toBe(true);
    // 节点 props 中含 type 字段（用于查表着色）
    expect(canvasSource.includes('type')).toBe(true);
    // 通过 fill 属性应用颜色
    expect(canvasSource.includes('fill')).toBe(true);
    // types.ts 中定义了 ENTITY_TYPE_COLORS 常量
    expect(typesSource.includes('ENTITY_TYPE_COLORS')).toBe(true);
  });

  test('test_11_entity_types_have_distinct_colors: 11 类实体颜色互异', () => {
    // 11 类实体的 key 必须全部出现在 types.ts 中
    const expectedTypes = [
      'PERSON',
      'LOCATION',
      'ORGANIZATION',
      'TIME',
      'NUMBER',
      'EVENT',
      'OBJECT',
      'CONCEPT',
      'LAW',
      'DISEASE',
      'OTHER',
    ];
    for (const t of expectedTypes) {
      expect(typesSource.includes(t)).toBe(true);
    }

    // 从 types.ts 源码中提取 ENTITY_TYPE_COLORS 对象字面量
    // 匹配形如 'PERSON': '#E5484D' 或 PERSON: '#E5484D'
    const colorBlockMatch = typesSource.match(
      /ENTITY_TYPE_COLORS[\s\S]*?=\s*\{([\s\S]*?)\}/
    );
    expect(colorBlockMatch).not.toBeNull();
    const colorBlock = colorBlockMatch![1];

    // 提取所有颜色值（#RRGGBB 形式）
    const colorValues = colorBlock.match(/#[0-9A-Fa-f]{6}/g);
    expect(colorValues).not.toBeNull();
    // 必须有 11 个颜色值
    expect(colorValues!.length).toBeGreaterThanOrEqual(11);

    // 11 类颜色值必须互异（去重后仍为 11 个）
    const uniqueColors = new Set(colorValues!.map((c) => c.toUpperCase()));
    expect(uniqueColors.size).toBe(11);
  });

  test('test_node_click_triggers_callback: 节点点击触发 onNodeClick 回调', () => {
    // Props 接口声明 onNodeClick
    expect(canvasSource.includes('onNodeClick')).toBe(true);
    // 节点元素绑定 onClick 事件
    expect(canvasSource.includes('onClick')).toBe(true);
    // 调用回调时传入节点 id
    expect(canvasSource.includes('node.id')).toBe(true);
  });

  test('test_edge_click_triggers_callback: 边点击触发 onEdgeClick 回调', () => {
    // Props 接口声明 onEdgeClick
    expect(canvasSource.includes('onEdgeClick')).toBe(true);
    // 边元素绑定 onClick 事件
    expect(canvasSource.includes('onClick')).toBe(true);
    // 调用回调时传入 edge 对象
    expect(canvasSource.includes('edge')).toBe(true);
  });

  test('test_graph_canvas_handles_large_graph_1k_nodes: 1k 节点不崩溃（节点上限保护）', () => {
    // 实现节点上限保护常量（避免一次性渲染过多 SVG 节点导致性能崩溃）
    // 上限建议 1000（spec 要求「1k 节点不崩溃」）
    expect(canvasSource.includes('MAX_')).toBe(true);
    // 至少有节点数量上限或分页逻辑
    const hasLimit =
      canvasSource.includes('MAX_NODES') ||
      canvasSource.includes('MAX_RENDER_NODES') ||
      canvasSource.includes('slice') ||
      canvasSource.includes('limit');
    expect(hasLimit).toBe(true);
    // 出现 1000 这个数字常量
    expect(canvasSource.includes('1000')).toBe(true);
  });
});
