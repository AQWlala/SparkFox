/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * 知识图谱数据契约 — 前后端交互 DTO（spec §三 11.4.1）
 *
 * 本文件定义「知识图谱视图」与后端 / IPC 层交互的数据契约：
 *   - GraphNodeDTO：图谱节点 DTO（对应 entity 表一行）
 *   - GraphEdgeDTO：图谱边 DTO（对应 event_entity_relation 表一行）
 *   - GraphData：完整图谱数据（nodes + edges + meta）
 *   - fetchGraphData：PoC mock 函数（11.4.2 阶段替换为真实 IPC 调用）
 *   - dtoToFlowNode / dtoToFlowEdge：DTO → ReactFlow Node/Edge 转换
 *
 * 范围说明（spec §三 11.4.1）：
 *   - PoC 阶段（本波）使用 mock 数据填充 GraphData
 *   - 生产阶段（11.4.2）通过 IPC 从后端获取真实图谱数据
 *   - 后端契约对齐：与 sparkfox-graph crate / entity 表结构保持一致
 *
 * 与 types.ts 的关系：
 *   - types.ts 中 GraphNode / GraphEdge 是「SVG 渲染专用」类型（含 x/y 坐标）
 *   - graphContract.ts 中 GraphNodeDTO / GraphEdgeDTO 是「前后端交互」类型（无坐标，
 *     坐标由前端布局算法或随机生成）
 *   - 两者独立维护，避免 SVG 渲染细节污染后端契约
 */

import type { Edge, Node } from '@xyflow/react';
import { ENTITY_TYPE_COLORS } from './types';

/**
 * 图谱节点 DTO（后端返回格式，spec §三 11.4.1）。
 *
 * 字段对齐 sparkfox-graph crate 中 Entity 结构：
 *   - id：实体唯一标识（ent-{nanoid} 形式）
 *   - label：实体显示名称（如「张三」）
 *   - entity_type：实体类型枚举（对应 ENTITY_TYPE_COLORS 的 key，如 PERSON / LOCATION）
 *   - properties：附加属性（可选，如别名、标准化值等）
 */
export interface GraphNodeDTO {
  id: string;
  label: string;
  /** 实体类型，对应 ENTITY_TYPE_COLORS 的 key（PERSON / LOCATION / ORGANIZATION ...） */
  entity_type: string;
  /** 附加属性（可选，由后端填充） */
  properties?: Record<string, unknown>;
}

/**
 * 图谱边 DTO（后端返回格式，spec §三 11.4.1）。
 *
 * 字段对齐 event_entity_relation 表：
 *   - id：关系唯一标识（rel-{nanoid} 形式）
 *   - source / target：节点 id（GraphNodeDTO.id）
 *   - label：关系标签（如「出生于」「就职于」）
 *   - relation_type：关系类型枚举（可选，用于后端分类查询）
 */
export interface GraphEdgeDTO {
  id: string;
  source: string;
  target: string;
  /** 关系标签（如「出生于」「就职于」） */
  label?: string;
  /** 关系类型枚举（可选，如 WORK_AT / BORN_IN） */
  relation_type?: string;
}

/**
 * GraphData 元信息（spec §三 11.4.1）。
 *
 * 用于前端显示「数据被截断」等提示。
 */
export interface GraphDataMeta {
  /** 节点总数（可能大于 nodes.length，因为后端会做阀门截断） */
  total_nodes: number;
  /** 边总数（可能大于 edges.length，同上） */
  total_edges: number;
  /** 是否被阀门截断（true 表示数据量大，仅返回了部分节点 / 边） */
  truncated: boolean;
}

/**
 * 完整图谱数据 DTO（spec §三 11.4.1）。
 *
 * 由后端 IPC 命令返回，前端用此结构填充 GraphFlow / GraphCanvas。
 */
export interface GraphData {
  nodes: GraphNodeDTO[];
  edges: GraphEdgeDTO[];
  /** 元信息（可选，PoC 阶段可能不返回） */
  meta?: GraphDataMeta;
}

/**
 * 获取图谱数据的 PoC mock 函数（spec §三 11.4.1）。
 *
 * PoC 阶段返回 5 节点 + 4 边的 mock 数据，覆盖 5 种实体类型
 * （PERSON / LOCATION / ORGANIZATION / TIME / EVENT）。
 *
 * 11.4.2 阶段替换为真实 IPC 调用：
 *   ```ts
 *   const data = await invoke<GraphData>('graph_fetch', { kbId });
 *   ```
 *
 * @param kbId 知识库 ID（PoC 阶段未使用，保留参数以兼容未来 IPC 签名）
 */
export async function fetchGraphData(kbId: string): Promise<GraphData> {
  // PoC 阶段：kbId 参数暂未使用，保留以兼容未来 IPC 签名
  void kbId;
  return {
    nodes: [
      { id: 'ent-1', label: '张三', entity_type: 'PERSON' },
      { id: 'ent-2', label: '北京', entity_type: 'LOCATION' },
      { id: 'ent-3', label: '腾讯', entity_type: 'ORGANIZATION' },
      { id: 'ent-4', label: '2024年', entity_type: 'TIME' },
      { id: 'ent-5', label: '产品发布', entity_type: 'EVENT' },
    ],
    edges: [
      { id: 'e1', source: 'ent-1', target: 'ent-2', label: '位于' },
      { id: 'e2', source: 'ent-3', target: 'ent-2', label: '总部在' },
      { id: 'e3', source: 'ent-1', target: 'ent-3', label: '就职于' },
      { id: 'e4', source: 'ent-5', target: 'ent-4', label: '发生于' },
    ],
    meta: { total_nodes: 5, total_edges: 4, truncated: false },
  };
}

/**
 * DTO → ReactFlow Node 转换（spec §三 11.4.1）。
 *
 * 转换规则：
 *   - id：直接复用 dto.id
 *   - data.label / data.entity_type：供自定义节点组件读取
 *   - position：PoC 阶段用 Math.random 随机布局（600 × 400 区域），
 *     11.4.x 阶段替换为力导布局（d3-force / elkjs）
 *   - style.background：按 entity_type 查 ENTITY_TYPE_COLORS 着色
 *
 * @param dto 后端返回的 GraphNodeDTO
 */
export function dtoToFlowNode(dto: GraphNodeDTO): Node {
  return {
    id: dto.id,
    data: { label: dto.label, entity_type: dto.entity_type },
    // PoC 阶段随机布局，11.4.x 替换为力导布局
    position: { x: Math.random() * 600, y: Math.random() * 400 },
    style: {
      // 按 entity_type 着色（types.ts 中 ENTITY_TYPE_COLORS 查表）
      background: ENTITY_TYPE_COLORS[dto.entity_type] ?? '#000000',
    },
  };
}

/**
 * DTO → ReactFlow Edge 转换（spec §三 11.4.1）。
 *
 * 转换规则：
 *   - id / source / target：直接复用 dto 字段
 *   - label：复用 dto.label（关系描述，可选）
 *   - animated：开启流动动画，便于视觉区分
 *
 * @param dto 后端返回的 GraphEdgeDTO
 */
export function dtoToFlowEdge(dto: GraphEdgeDTO): Edge {
  return {
    id: dto.id,
    source: dto.source,
    target: dto.target,
    label: dto.label,
    // 开启流动动画（animated），使关系边在视觉上更突出
    animated: true,
  };
}
