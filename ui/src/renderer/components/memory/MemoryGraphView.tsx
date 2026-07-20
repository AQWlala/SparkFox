/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MemoryGraphView — 记忆图谱可视化（SVG 简化版）
 *
 * 来源：OpenAkita MemoryGraph3D（清洁室重写为 2D SVG，PoC 不引入 3D 依赖）
 * 功能：
 * - 圆形节点（按层着色）+ 连线（关联记忆）
 * - 节点大小 = importance
 * - 悬停显示完整标签
 * - 点击节点 → 选中并触发 accessMemory
 */

import React from 'react';
import {
  useMemoryStore,
  LAYER_COLORS,
  TYPE_LABELS,
} from '@renderer/store/memoryStore';

interface NodePos {
  id: string;
  x: number;
  y: number;
}

const WIDTH = 760;
const HEIGHT = 480;
const RADIUS = 200;

/** 简单圆形布局：按 layer 分 6 组，每组环形分布 */
function layout(nodes: { id: string; layer: string }[]): NodePos[] {
  const groups: Record<string, string[]> = {};
  for (const n of nodes) {
    if (!groups[n.layer]) groups[n.layer] = [];
    groups[n.layer].push(n.id);
  }
  const layers = ['L0', 'L1', 'L2', 'L3', 'L4', 'L5'];
  const positions: NodePos[] = [];
  layers.forEach((layer, layerIdx) => {
    const ids = groups[layer] || [];
    const count = ids.length;
    // 每个 layer 占据一个 60° 扇形
    const baseAngle = (layerIdx * 60 - 90) * (Math.PI / 180);
    const spread = (60 * Math.PI) / 180;
    ids.forEach((id, i) => {
      const t = count <= 1 ? 0.5 : i / (count - 1);
      const angle = baseAngle + (t - 0.5) * spread * 0.8;
      // 偶数层靠内圈，奇数层靠外圈（避免重叠）
      const r = RADIUS - (layerIdx % 2 === 0 ? 60 : 0);
      positions.push({
        id,
        x: WIDTH / 2 + Math.cos(angle) * r,
        y: HEIGHT / 2 + Math.sin(angle) * r,
      });
    });
  });
  return positions;
}

const MemoryGraphView: React.FC = () => {
  const nodes = useMemoryStore((s) => s.graphNodes);
  const accessMemory = useMemoryStore((s) => s.accessMemory);
  const startEdit = useMemoryStore((s) => s.startEdit);

  const positions = React.useMemo(() => layout(nodes), [nodes]);
  const posMap = React.useMemo(() => {
    const m = new Map<string, NodePos>();
    for (const p of positions) m.set(p.id, p);
    return m;
  }, [positions]);

  const [hovered, setHovered] = React.useState<string | null>(null);

  return (
    <div className='sf-memory-graph-view'>
      <svg width={WIDTH} height={HEIGHT} className='sf-memory-graph-svg'>
        {/* 连线 */}
        {nodes.map((n) => {
          const src = posMap.get(n.id);
          if (!src) return null;
          return n.edges.map((targetId) => {
            const tgt = posMap.get(targetId);
            if (!tgt) return null;
            return (
              <line
                key={`${n.id}-${targetId}`}
                x1={src.x}
                y1={src.y}
                x2={tgt.x}
                y2={tgt.y}
                className='sf-memory-graph-edge'
              />
            );
          });
        })}
        {/* 节点 */}
        {nodes.map((n) => {
          const pos = posMap.get(n.id);
          if (!pos) return null;
          const color = LAYER_COLORS[n.layer];
          const r = 8 + n.importance * 14;
          const isHovered = hovered === n.id;
          return (
            <g
              key={n.id}
              transform={`translate(${pos.x},${pos.y})`}
              className='sf-memory-graph-node'
              onMouseEnter={() => setHovered(n.id)}
              onMouseLeave={() => setHovered(null)}
              onClick={() => {
                accessMemory(n.id);
                startEdit(n.id);
              }}
            >
              <circle
                r={r}
                fill={color}
                fillOpacity={isHovered ? 0.95 : 0.75}
                stroke={color}
                strokeWidth={isHovered ? 2 : 1}
              />
              <text
                y={r + 12}
                textAnchor='middle'
                className='sf-memory-graph-label'
                fill={isHovered ? '#1D1D1F' : '#86868B'}
              >
                {n.label}
              </text>
            </g>
          );
        })}
      </svg>
      <div className='sf-memory-graph-legend'>
        {(['L0', 'L1', 'L2', 'L3', 'L4', 'L5'] as const).map((l) => (
          <span key={l} className='sf-memory-graph-legend-item'>
            <span className='sf-memory-graph-legend-dot' style={{ background: LAYER_COLORS[l] }} />
            {l}
          </span>
        ))}
        <span className='sf-memory-graph-legend-hint'>
          悬停查看 · 点击编辑
        </span>
      </div>
    </div>
  );
};

export default MemoryGraphView;
