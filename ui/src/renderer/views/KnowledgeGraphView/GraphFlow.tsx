/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * GraphFlow — @xyflow/react v12 渲染组件（spec §三 11.4.1 / 第 15 波并行 sub-step C）
 *
 * 本组件使用 @xyflow/react v12（ReactFlow）替代 GraphCanvas 的 SVG 简单渲染：
 *   - 接收 GraphData DTO（graphContract.ts）作为输入
 *   - 通过 dtoToFlowNode / dtoToFlowEdge 转换为 ReactFlow Node / Edge
 *   - 渲染 Background（背景网格）+ Controls（缩放/平移控件）+ MiniMap（缩略图）
 *   - 节点 / 边点击触发 onNodeClick / onEdgeClick 回调
 *
 * 与 GraphCanvas 的关系：
 *   - GraphCanvas：SVG 简单渲染（11.3.2 阶段实现，作为 fallback 保留）
 *   - GraphFlow：@xyflow/react v12 完整渲染（11.4.1 阶段实现，作为主推荐模式）
 *   - 两种模式由 KnowledgeGraphView/index.tsx 的 renderMode state 切换
 *
 * 范围说明（spec §三 11.4.1）：
 *   - PoC 阶段使用 dtoToFlowNode 中的 Math.random 随机布局
 *   - 11.4.x 阶段替换为力导布局（d3-force / elkjs）
 *   - 节点 / 边的自定义样式（如按 entity_type 着色）通过 style 字段注入
 */

import React, { useCallback, useMemo } from 'react';
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  type GraphData,
  dtoToFlowEdge,
  dtoToFlowNode,
} from './graphContract';

/**
 * GraphFlow 组件 Props（spec §三 11.4.1）。
 *
 * - data：图谱数据（GraphData DTO，含 nodes / edges / meta）
 * - onNodeClick：节点点击回调（参数为 nodeId）
 * - onEdgeClick：边点击回调（参数为 edgeId）
 */
export interface GraphFlowProps {
  /** 图谱数据 DTO（nodes + edges + meta） */
  data: GraphData;
  /** 节点点击回调（参数为 nodeId，触发父组件 EntityEditDrawer 等） */
  onNodeClick?: (nodeId: string) => void;
  /** 边点击回调（参数为 edgeId，PoC 阶段仅 console.log） */
  onEdgeClick?: (edgeId: string) => void;
}

/**
 * GraphFlow 主组件。
 *
 * 实现要点：
 *   - 用 useMemo 派生 ReactFlow nodes / edges，依赖 data 变化时重新计算
 *   - 用 useCallback 包装点击回调，避免子组件无谓重渲染
 *   - ReactFlow 启用 fitView 自动适配视口（首次加载即居中显示所有节点）
 *   - 内置 Background / Controls / MiniMap 三个组件
 */
const GraphFlow: React.FC<GraphFlowProps> = ({ data, onNodeClick, onEdgeClick }) => {
  // 派生 ReactFlow nodes：data.nodes → Node[]（依赖 data 变化时重算）
  const nodes: Node[] = useMemo(
    () => data.nodes.map(dtoToFlowNode),
    [data]
  );

  // 派生 ReactFlow edges：data.edges → Edge[]（依赖 data 变化时重算）
  const edges: Edge[] = useMemo(
    () => data.edges.map(dtoToFlowEdge),
    [data]
  );

  // 节点点击回调：转发给父组件，参数为 node.id
  const handleNodeClick = useCallback(
    (event: React.MouseEvent, node: Node) => {
      void event; // ReactFlow 签名要求 event 参数，PoC 阶段未使用
      onNodeClick?.(node.id);
    },
    [onNodeClick]
  );

  // 边点击回调：转发给父组件，参数为 edge.id
  const handleEdgeClick = useCallback(
    (event: React.MouseEvent, edge: Edge) => {
      void event; // ReactFlow 签名要求 event 参数，PoC 阶段未使用
      onEdgeClick?.(edge.id);
    },
    [onEdgeClick]
  );

  return (
    <div className='sparkfox-graph-flow' style={{ width: '100%', height: '600px' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodeClick={handleNodeClick}
        onEdgeClick={handleEdgeClick}
        fitView
      >
        <Background />
        <Controls />
        <MiniMap />
      </ReactFlow>
    </div>
  );
};

export default GraphFlow;
