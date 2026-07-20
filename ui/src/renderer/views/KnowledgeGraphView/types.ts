/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * KnowledgeGraphView 类型定义（spec §三 11.3.2 / 第 13 波并行 sub-step C）
 *
 * 本文件定义：
 *   - ENTITY_TYPE_COLORS：11 类实体颜色映射常量（人名红 / 地名蓝 / 机构绿 ...）
 *   - ENTITY_TYPE_LABELS：11 类实体中文标签映射（用于图例展示）
 *   - EntityType：11 类实体类型联合
 *   - GraphNode / GraphEdge / GraphCanvasProps：图谱画布组件 Props 类型
 *
 * 着色映射对应 spec §三 11.3.2：
 *   人名红 / 地名蓝 / 机构绿 / 时间黄 / 数字紫 / 事件橙 /
 *   物品青 / 概念粉 / 法律棕 / 疾病灰 / 其他黑
 */

/**
 * 11 类实体颜色映射（spec §三 11.3.2）。
 * key 为 ENTITY_TYPE_COLORS 的 key（如 'PERSON'），value 为 #RRGGBB 颜色。
 *
 * 颜色选择参考 Arco Design 调色板，确保 11 类互异且在浅/深主题下均可读。
 */
export const ENTITY_TYPE_COLORS: Record<string, string> = {
  PERSON: '#E5484D', // 人名-红
  LOCATION: '#3691F7', // 地名-蓝
  ORGANIZATION: '#1FBC9C', // 机构-绿
  TIME: '#FFB224', // 时间-黄
  NUMBER: '#8E4EC6', // 数字-紫
  EVENT: '#F5680A', // 事件-橙
  OBJECT: '#009099', // 物品-青
  CONCEPT: '#D33682', // 概念-粉
  LAW: '#8C6A46', // 法律-棕
  DISEASE: '#7D8590', // 疾病-灰
  OTHER: '#000000', // 其他-黑
};

/**
 * 11 类实体中文标签映射（用于图例展示）。
 * 与 ENTITY_TYPE_COLORS 的 key 一一对应。
 */
export const ENTITY_TYPE_LABELS: Record<string, string> = {
  PERSON: '人名',
  LOCATION: '地名',
  ORGANIZATION: '机构',
  TIME: '时间',
  NUMBER: '数字',
  EVENT: '事件',
  OBJECT: '物品',
  CONCEPT: '概念',
  LAW: '法律',
  DISEASE: '疾病',
  OTHER: '其他',
};

/**
 * 11 类实体类型联合（spec §三 11.3.2）。
 * 用于 GraphNode.type 的类型约束。
 */
export type EntityType =
  | 'PERSON'
  | 'LOCATION'
  | 'ORGANIZATION'
  | 'TIME'
  | 'NUMBER'
  | 'EVENT'
  | 'OBJECT'
  | 'CONCEPT'
  | 'LAW'
  | 'DISEASE'
  | 'OTHER';

/**
 * 图谱节点（GraphNode）—— 表示一个实体。
 * x / y 为节点在 SVG 画布中的坐标（像素）。
 */
export interface GraphNode {
  id: string;
  label: string;
  /** 实体类型，对应 ENTITY_TYPE_COLORS 的 key */
  type: string;
  /** 节点 X 坐标（像素） */
  x: number;
  /** 节点 Y 坐标（像素） */
  y: number;
}

/**
 * 图谱边（GraphEdge）—— 表示一条 event_entity_relation 关系。
 * source / target 为 GraphNode.id；label 为关系描述（可选）。
 */
export interface GraphEdge {
  source: string;
  target: string;
  /** 关系标签（如「出生于」「就职于」） */
  label?: string;
}

/**
 * GraphCanvas 组件 Props。
 */
export interface GraphCanvasProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** 节点点击回调（参数为 nodeId） */
  onNodeClick?: (nodeId: string) => void;
  /** 边点击回调（参数为 edge 对象） */
  onEdgeClick?: (edge: GraphEdge) => void;
}
