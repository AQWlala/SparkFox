/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * GraphCanvas — 知识图谱画布组件（spec §三 11.3.2 / 第 13 波并行 sub-step C）
 *
 * 本组件实现「最小可用」的图谱画布：
 *   - SVG 渲染节点（circle + text）和边（line + text）
 *   - 节点按 entity_type 着色（11 类颜色映射见 types.ts）
 *   - 节点 / 边点击回调（onNodeClick / onEdgeClick）
 *   - 11 类颜色图例（与画布并排展示）
 *   - 节点数超过 MAX_RENDER_NODES（1000）时自动截断，避免 SVG 性能崩溃
 *
 * 范围说明：spec §三 11.3.2 原本包含 @xyflow/react 实际渲染，但本波仅实施
 * 「11 类着色常量 + 图例 + 简单 SVG 节点展示」，实际 @xyflow/react 渲染推迟到 11.4.1。
 *
 * PoC 数据：在 KnowledgeGraphView/index.tsx 中用 useState mock 5 个节点 + 4 条边。
 */

import React from 'react';
import {
  ENTITY_TYPE_COLORS,
  ENTITY_TYPE_LABELS,
  type GraphCanvasProps,
  type GraphEdge,
  type GraphNode,
} from './types';
import styles from './styles.module.css';

/**
 * 单次渲染节点上限（spec §三 11.3.2 要求「1k 节点不崩溃」）。
 * 超出此上限时仅渲染前 MAX_RENDER_NODES 个节点，避免 SVG 元素过多导致性能崩溃。
 */
const MAX_RENDER_NODES = 1000;

/**
 * GraphCanvas 主组件。
 *
 * 实现要点：
 *   - 节点用 SVG circle + text 渲染，颜色由 ENTITY_TYPE_COLORS[node.type] 决定
 *   - 边用 SVG line + text 渲染（label 居中显示在 line 中点）
 *   - 节点 / 边绑定 onClick 触发 onNodeClick / onEdgeClick 回调
 *   - 节点数超过 MAX_RENDER_NODES 时仅渲染前 MAX_RENDER_NODES 个（slice 截断）
 *   - 图例横向排列 11 个色块 + 标签
 */
const GraphCanvas: React.FC<GraphCanvasProps> = ({
  nodes,
  edges,
  onNodeClick,
  onEdgeClick,
}) => {
  // 节点上限保护：超出 MAX_RENDER_NODES 时仅渲染前 MAX_RENDER_NODES 个
  const limitedNodes: GraphNode[] = nodes.slice(0, MAX_RENDER_NODES);

  // 构建 id → node 索引，用于在渲染边时查询 source / target 节点坐标
  const nodeMap = new Map<string, GraphNode>();
  for (const n of nodes) {
    nodeMap.set(n.id, n);
  }

  return (
    <div className={styles.canvasWrap}>
      {/* ─── 左侧：图谱画布（SVG 渲染节点 + 边） ─── */}
      <svg
        className={styles.canvasSvg}
        viewBox='0 0 800 600'
        preserveAspectRatio='xMidYMid meet'
        role='img'
        aria-label='知识图谱画布'
      >
        {/* ─── 边渲染（先渲染边，使其在节点下方） ─── */}
        {edges.map((edge: GraphEdge, idx: number) => {
          const src = nodeMap.get(edge.source);
          const tgt = nodeMap.get(edge.target);
          // 若 source / target 节点不存在（被截断或缺失），跳过此边
          if (!src || !tgt) return null;
          // 边中点坐标（用于渲染 label）
          const midX = (src.x + tgt.x) / 2;
          const midY = (src.y + tgt.y) / 2;
          return (
            <g
              key={`edge-${idx}`}
              className={styles.edgeGroup}
              onClick={() => onEdgeClick?.(edge)}
            >
              <line
                x1={src.x}
                y1={src.y}
                x2={tgt.x}
                y2={tgt.y}
                stroke='var(--color-text-3)'
                strokeWidth={1.5}
                className={styles.edgeLine}
              />
              {edge.label ? (
                <text
                  x={midX}
                  y={midY}
                  className={styles.edgeLabel}
                  textAnchor='middle'
                  dominantBaseline='middle'
                >
                  {edge.label}
                </text>
              ) : null}
            </g>
          );
        })}

        {/* ─── 节点渲染（circle + text） ─── */}
        {limitedNodes.map((node: GraphNode) => {
          // 按 entity_type 着色（types.ts 中 ENTITY_TYPE_COLORS 查表）
          const color = ENTITY_TYPE_COLORS[node.type] ?? ENTITY_TYPE_COLORS.OTHER;
          return (
            <g
              key={node.id}
              className={styles.nodeGroup}
              onClick={() => onNodeClick?.(node.id)}
            >
              <circle
                cx={node.x}
                cy={node.y}
                r={12}
                fill={color}
                stroke='var(--color-bg-1)'
                strokeWidth={2}
                className={styles.nodeCircle}
              />
              <text
                x={node.x}
                y={node.y + 26}
                className={styles.nodeLabel}
                textAnchor='middle'
                dominantBaseline='middle'
              >
                {node.label}
              </text>
            </g>
          );
        })}
      </svg>

      {/* ─── 右侧：11 类颜色图例 ─── */}
      <div className={styles.legend}>
        <div className={styles.legendTitle}>实体类型</div>
        <div className={styles.legendGrid}>
          {Object.keys(ENTITY_TYPE_COLORS).map((typeKey: string) => (
            <div key={typeKey} className={styles.legendItem}>
              <span
                className={styles.legendDot}
                style={{ backgroundColor: ENTITY_TYPE_COLORS[typeKey] }}
                aria-label={typeKey}
              />
              <span className={styles.legendLabel}>
                {ENTITY_TYPE_LABELS[typeKey] ?? typeKey}
              </span>
            </div>
          ))}
        </div>
        {/* 节点上限提示（仅当节点数被截断时显示） */}
        {nodes.length > MAX_RENDER_NODES && (
          <div className={styles.legendHint}>
            节点数 {nodes.length} 超过上限 {MAX_RENDER_NODES}，仅渲染前 {MAX_RENDER_NODES} 个
          </div>
        )}
      </div>
    </div>
  );
};

export default GraphCanvas;
