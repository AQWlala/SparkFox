/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox KnowledgeGraphView — 知识图谱可视化
 *
 * spec 1.0 第 2992-3019 行（Task 8.15）
 *
 * 实现说明：
 * - spec 示例使用 `reactflow`（v11），但本项目已统一使用 `@xyflow/react`（React Flow v12，
 *   reactflow 的官方继任者，API 兼容）。为避免引入新依赖，这里沿用项目既定方案。
 * - v1.0.0：MVP 可视化，接收外部传入的 nodes/edges，渲染交互式图谱。
 * - 节点 type 默认 'default'（React Flow 内置），由调用方按需自定义。
 */

import { useMemo } from 'react';
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

export type { Edge as KGEdge, Node as KGNode } from '@xyflow/react';

export interface KnowledgeGraphViewProps {
  /** 图谱节点（位置由调用方计算或使用自动布局） */
  nodes: Node[];
  /** 图谱边（关系） */
  edges: Edge[];
  /** 容器高度（默认 600px） */
  height?: number | string;
  /** 是否显示小地图（默认 true） */
  showMiniMap?: boolean;
  /** 节点是否可拖拽（默认 true） */
  nodesDraggable?: boolean;
  /** 是否允许连接节点（默认 false，v1.0.0 只读） */
  nodesConnectable?: boolean;
}

/**
 * 知识图谱可视化组件
 *
 * 使用：
 * ```tsx
 * <KnowledgeGraphView nodes={nodes} edges={edges} />
 * ```
 */
export function KnowledgeGraphView({
  nodes,
  edges,
  height = 600,
  showMiniMap = true,
  nodesDraggable = true,
  nodesConnectable = false,
}: KnowledgeGraphViewProps) {
  const containerStyle = useMemo<React.CSSProperties>(
    () => ({
      width: '100%',
      height: typeof height === 'number' ? `${height}px` : height,
      position: 'relative',
      background: 'var(--color-bg-1, #fafafa)',
      borderRadius: 8,
      overflow: 'hidden',
      border: '1px solid var(--color-border-2, #e5e6eb)',
    }),
    [height],
  );

  return (
    <div style={containerStyle} className='sparkfox-kg-view'>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        fitViewOptions={{ padding: 0.18, maxZoom: 1.35 }}
        minZoom={0.2}
        maxZoom={2.0}
        proOptions={{ hideAttribution: true }}
        nodesDraggable={nodesDraggable}
        nodesConnectable={nodesConnectable}
        edgesFocusable
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={22}
          size={1.2}
          color='#d1d5e5'
        />
        <Controls showFitView showInteractive={false} />
        {showMiniMap ? (
          <MiniMap
            pannable
            zoomable
            nodeColor={(n) =>
              typeof n.data?.color === 'string' ? (n.data.color as string) : '#3b82f6'
            }
            maskColor='rgba(248, 249, 252, 0.7)'
          />
        ) : null}
      </ReactFlow>
    </div>
  );
}

export default KnowledgeGraphView;
