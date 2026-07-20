/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * citation/types.ts — Citation 三级溯源类型定义
 *
 * 对应 spec §三 10.10.1 + 10.10.2（U-03 修复：三级溯源缺失问题）
 *
 * 三级溯源层级：
 *   L1 实体（Entity）— 知识图谱中的实体节点（entity_id + entity_type + name）
 *   L2 事件（Event） — 实体上的事件三元组（subject + predicate + object）
 *   L3 chunk         — 事件命中的原文片段（chunk_id + span + text）
 *
 * 与后端 sparkfox-knowledge SearchHit 扩展字段（hop / via_entities / chunk_span）对齐。
 */

/**
 * L1 实体引用
 * - entity_id: 实体唯一 ID（对应后端 EntityRef.entity_id）
 * - entity_type: 实体类型（person / org / concept / ...）
 * - name: 实体显示名称
 */
export interface EntityRef {
  entity_id: string;
  entity_type: string;
  name: string;
}

/**
 * L2 事件引用（三元组）
 * - event_id: 事件唯一 ID
 * - subject: 主体
 * - predicate: 谓词
 * - object: 客体
 */
export interface EventRef {
  event_id: string;
  subject: string;
  predicate: string;
  object: string;
}

/**
 * L3 chunk 引用（原文片段）
 * - chunk_id: chunk 唯一 ID
 * - span: 字符区间 [start, end)（对齐后端 chunk_span: (usize, usize)）
 * - text: chunk 原文
 */
export interface ChunkRef {
  chunk_id: string;
  span: [number, number];
  text: string;
}

/**
 * 三级溯源 Citation
 * - entity: L1 实体（必填）
 * - event: L2 事件（必填）
 * - chunk: L3 chunk（可选，MULTI/PRIMARY 策略下可能为 null）
 */
export interface Citation {
  entity: EntityRef;
  event: EventRef;
  chunk: ChunkRef | null;
}
