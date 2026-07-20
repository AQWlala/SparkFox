/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * GraphFlow 测试 — spec §三 11.4.1 / 第 15 波并行 sub-step C
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 GraphCanvas.test.tsx、
 * index.test.tsx、EntityEditDrawer.test.tsx）。本项目未引入 React Testing
 * Library / jsdom，因此通过断言源码字符串保证组件行为契约。
 *
 * 测试用例对应 spec §三 11.4.1 验收点：
 *   1. test_graph_contract_defines_graph_node_dto  — graphContract.ts 含 GraphNodeDTO（id / label / entity_type）
 *   2. test_graph_contract_defines_graph_edge_dto  — graphContract.ts 含 GraphEdgeDTO（id / source / target / label）
 *   3. test_graph_contract_defines_graph_data      — graphContract.ts 含 GraphData（nodes / edges / meta）
 *   4. test_graph_flow_uses_xyflow_react           — GraphFlow.tsx 引用 @xyflow/react
 *   5. test_graph_flow_renders_nodes_and_edges     — GraphFlow.tsx 渲染 nodes + edges
 *   6. test_graph_flow_click_triggers_callback     — GraphFlow.tsx 含 onNodeClick / onEdgeClick 回调
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const contractSource = readFileSync(
  new URL('./graphContract.ts', import.meta.url),
  'utf8'
);
const flowSource = readFileSync(
  new URL('./GraphFlow.tsx', import.meta.url),
  'utf8'
);

describe('GraphFlow — 数据契约 + @xyflow/react 渲染（spec §三 11.4.1）', () => {
  test('test_graph_contract_defines_graph_node_dto: graphContract.ts 含 GraphNodeDTO（id / label / entity_type）', () => {
    // 定义 GraphNodeDTO 接口
    expect(contractSource.includes('GraphNodeDTO')).toBe(true);
    // 含 id 字段（实体唯一标识）
    expect(contractSource.includes('id: string')).toBe(true);
    // 含 label 字段（实体显示名称）
    expect(contractSource.includes('label: string')).toBe(true);
    // 含 entity_type 字段（实体类型，对应 ENTITY_TYPE_COLORS 的 key）
    expect(contractSource.includes('entity_type')).toBe(true);
  });

  test('test_graph_contract_defines_graph_edge_dto: graphContract.ts 含 GraphEdgeDTO（id / source / target / label）', () => {
    // 定义 GraphEdgeDTO 接口
    expect(contractSource.includes('GraphEdgeDTO')).toBe(true);
    // 边含 source 字段（起点节点 id）
    expect(contractSource.includes('source: string')).toBe(true);
    // 边含 target 字段（终点节点 id）
    expect(contractSource.includes('target: string')).toBe(true);
    // 边含可选 label 字段（关系标签）
    expect(contractSource.includes('label')).toBe(true);
  });

  test('test_graph_contract_defines_graph_data: graphContract.ts 含 GraphData（nodes / edges / meta）', () => {
    // 定义 GraphData 接口
    expect(contractSource.includes('GraphData')).toBe(true);
    // GraphData.nodes 为节点数组
    expect(contractSource.includes('nodes')).toBe(true);
    // GraphData.edges 为边数组
    expect(contractSource.includes('edges')).toBe(true);
    // GraphData.meta 含截断标志（truncated）
    expect(contractSource.includes('meta')).toBe(true);
    expect(contractSource.includes('truncated')).toBe(true);
  });

  test('test_graph_flow_uses_xyflow_react: GraphFlow.tsx 引用 @xyflow/react', () => {
    // 从 @xyflow/react 导入 ReactFlow 组件
    expect(flowSource.includes("@xyflow/react")).toBe(true);
    // 导入 ReactFlow 主组件
    expect(flowSource.includes('ReactFlow')).toBe(true);
    // 导入 Background / Controls / MiniMap 三个内建组件
    expect(flowSource.includes('Background')).toBe(true);
    expect(flowSource.includes('Controls')).toBe(true);
    expect(flowSource.includes('MiniMap')).toBe(true);
    // 引入 @xyflow/react 的样式表
    expect(flowSource.includes('@xyflow/react/dist/style.css')).toBe(true);
  });

  test('test_graph_flow_renders_nodes_and_edges: GraphFlow.tsx 渲染 nodes + edges', () => {
    // 通过 props.data 接收图谱数据
    expect(flowSource.includes('data')).toBe(true);
    // 使用 useMemo 派生 ReactFlow nodes
    expect(flowSource.includes('nodes')).toBe(true);
    // 使用 useMemo 派生 ReactFlow edges
    expect(flowSource.includes('edges')).toBe(true);
    // 将 nodes 传给 ReactFlow 组件
    expect(flowSource.includes('nodes={nodes}')).toBe(true);
    // 将 edges 传给 ReactFlow 组件
    expect(flowSource.includes('edges={edges}')).toBe(true);
    // 启用 fitView 自动适配视口
    expect(flowSource.includes('fitView')).toBe(true);
  });

  test('test_graph_flow_click_triggers_callback: GraphFlow.tsx 含 onNodeClick / onEdgeClick 回调', () => {
    // Props 接口声明 onNodeClick 回调
    expect(flowSource.includes('onNodeClick')).toBe(true);
    // Props 接口声明 onEdgeClick 回调
    expect(flowSource.includes('onEdgeClick')).toBe(true);
    // 绑定 ReactFlow 的 onNodeClick 事件
    expect(flowSource.includes('onNodeClick=')).toBe(true);
    // 绑定 ReactFlow 的 onEdgeClick 事件
    expect(flowSource.includes('onEdgeClick=')).toBe(true);
    // 使用 useCallback 包装回调以稳定引用
    expect(flowSource.includes('useCallback')).toBe(true);
  });
});
