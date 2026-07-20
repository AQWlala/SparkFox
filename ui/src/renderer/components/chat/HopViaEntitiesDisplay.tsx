/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * HopViaEntitiesDisplay — 行内 hop/via_entities 展示组件（spec §三 11.5.2 / 第 17 波并行 sub-step A）
 *
 * 与 KnowledgeGraphView/MultiHopPathView 不同，本组件是「行内紧凑展示」而非「Card 详情」：
 *   - 嵌入到 ChatMessage 的 assistant 消息底部（CitationChip 列表后）
 *   - 每个 SearchHit 一行：`[hop=N] [via: entity1 → entity2 → entity3]`
 *   - hop Tag 颜色映射与 MultiHopPathView 完全一致（hop1 蓝 / hop2 黄 / hop3 灰）
 *   - via_entities 用 Arco Tag 显示，hover 时 Arco Tooltip 显示 entity_type
 *   - via_entities 之间用 → 箭头分隔，体现多跳路径
 *   - 无 hits 时不渲染（return null）
 *
 * 类型设计：
 *   - SearchHit / EntityRef 类型在组件内部 export 定义，与 MultiHopPathView 保持字段一致
 *     （entity_id / entity_type / text + event_id / score / hop / via_entities / chunk_id）
 *   - 字段对齐后端 sparkfox-knowledge::search::SearchHit（11.6.x 后端检索实现）
 *
 * 范围说明（spec §三 11.5.2）：
 *   - 本波仅实施前端渲染（PoC：mock hit 数据驱动）
 *   - 真实 hit 数据由 11.4.2（IPC）+ 11.6.1（hnswlib-rs 后端检索）填充
 *   - via_entities Tag 的 onClick 通过 onEntityClick 回调暴露给父组件
 */

import React from 'react';
import { Space, Tag, Tooltip } from '@arco-design/web-react';
import styles from './HopViaEntitiesDisplay.module.css';

// ─── 类型定义（组件内部 export，与 MultiHopPathView 字段一致） ──────────────────

/**
 * 实体引用 — 对应后端 sparkfox-knowledge::search::EntityRef。
 * 用于 via_entities 字段，表示多跳路径上经过的实体。
 *
 * 注意：与 citation/types.ts 的 EntityRef 不同（那里用 name，这里用 text），
 * 这里与 MultiHopPathView 保持一致，便于跨视图类型互换。
 */
export interface EntityRef {
  entity_id: string;
  entity_type: string;
  /** 实体文本（实体名称或表面形式） */
  text: string;
}

/**
 * 检索命中 — 对应后端 sparkfox-knowledge::search::SearchHit。
 * 由后端检索（11.6.1 hnswlib-rs）返回，前端在 ChatMessage 中显示对应 hit。
 */
export interface SearchHit {
  event_id: string;
  /** 相关性得分（0..1，越高越相关） */
  score: number;
  /** 跳跃层级（1=ATOMIC 单跳 / 2=MULTI 二跳 / 3=MULTI 三跳及以上 / null=未分级） */
  hop: number | null;
  /** 经过的实体列表（多跳路径上的中间实体） */
  via_entities: EntityRef[];
  /** 命中的 chunk_id（可选，11.6.x 后端填充） */
  chunk_id?: string | null;
}

// ─── hop 颜色映射（与 MultiHopPathView 保持完全一致） ──────────────────────────

/**
 * hop → CSS 类名映射表（spec §三 11.5.2）。
 * - hop1：蓝色（ATOMIC 单跳，#007aff）
 * - hop2：黄色（MULTI 二跳扩展，#ff9500）
 * - hop3：灰色（MULTI 三跳及以上，#6e6e73）
 *
 * 与 MultiHopPathView.HOP_COLOR_MAP / ReasoningChainPanel.HOP_CLASS_MAP
 * 命名与配色完全对齐，便于用户跨视图形成「hop=1 蓝 / hop=2 黄 / hop=3 灰」
 * 的统一视觉记忆。
 */
export const HOP_COLOR_MAP: Record<number, string> = {
  1: 'hop1', // hop1 蓝（ATOMIC 单跳）
  2: 'hop2', // hop2 黄（MULTI 二跳扩展）
  3: 'hop3', // hop3 灰（MULTI 三跳及以上）
};

/**
 * 根据 hop 返回对应 CSS 类名。
 * @param hop 跳数（1/2/3）；其他值（含 null/undefined）返回空字符串
 */
export function getHopColorClass(hop: number | null | undefined): string {
  if (hop === null || hop === undefined) return '';
  return HOP_COLOR_MAP[hop] ?? '';
}

/**
 * 根据 hop 返回显示标签（如「hop1」「hop2」「hop3」）。
 * @param hop 跳数；null/undefined 返回「hop=?」，其他值返回「hop=N」
 */
export function formatHopLabel(hop: number | null | undefined): string {
  if (hop === null || hop === undefined) return 'hop=?';
  if (hop >= 1 && hop <= 3) return `hop${hop}`;
  return `hop=${hop}`;
}

// ─── Props ─────────────────────────────────────────────────────────────────────

/**
 * HopViaEntitiesDisplay 组件 Props（spec §三 11.5.2）。
 *
 * - hits：检索命中列表（含 hop / via_entities）；空数组时不渲染
 * - onEntityClick：via_entities Tag 点击回调（参数为 entity_id，父组件可跳转到对应实体）
 */
export interface HopViaEntitiesDisplayProps {
  hits: SearchHit[];
  onEntityClick?: (entityId: string) => void;
}

// ─── 主组件 ───────────────────────────────────────────────────────────────────

/**
 * HopViaEntitiesDisplay 主组件。
 *
 * 渲染逻辑：
 *   - hits 为空数组：return null（不渲染任何 DOM 节点）
 *   - hits 非空：渲染容器，每个 hit 一行（[hop=N] [via: e1 → e2 → e3]）
 *
 * 行内布局：
 *   - 行首：hop Tag（颜色由 getHopColorClass(hop) 注入）
 *   - 行中：via_entities 区（每个 entity 一个 Tag + 箭头分隔）
 *   - via_entities 为空：仅显示 hop Tag
 *   - entity Tag hover：Tooltip 显示 entity_type
 */
const HopViaEntitiesDisplay: React.FC<HopViaEntitiesDisplayProps> = ({
  hits,
  onEntityClick,
}) => {
  // ─── 无 hits 时：不渲染（避免空 div 浪费 DOM 节点） ───
  if (!hits || hits.length === 0) {
    return null;
  }

  // ─── via_entities Tag 点击回调（PoC：跳转到对应实体节点） ───
  const handleEntityClick = (entityId: string) => {
    // eslint-disable-next-line no-console
    console.log('[HopViaEntitiesDisplay] via_entity tag clicked:', entityId);
    onEntityClick?.(entityId);
  };

  return (
    <div className={styles.container} data-role='hop-via-entities-display'>
      {hits.map((hit, hitIdx) => {
        const hopColorClass = getHopColorClass(hit.hop);
        const hopLabel = formatHopLabel(hit.hop);
        const hopTagClassName = `${styles.hopTag} ${
          hopColorClass ? styles[hopColorClass] : styles.hopUnknown
        }`;

        return (
          <div
            key={`hop-via-hit-${hit.event_id}-${hitIdx}`}
            className={styles.hitRow}
          >
            {/* ─── hop Tag：颜色映射 hop1 蓝 / hop2 黄 / hop3 灰 ─── */}
            <span className={hopTagClassName}>{hopLabel}</span>

            {/* ─── via_entities Tag 列表 + 箭头分隔 ─── */}
            {hit.via_entities.length > 0 && (
              <span className={styles.viaEntities}>
                <span className={styles.viaEntitiesLabel}>via:</span>
                <Space size={2} wrap>
                  {hit.via_entities.map((ent, entIdx) => (
                    <React.Fragment key={ent.entity_id}>
                      {/* 箭头分隔（首个实体前不显示） */}
                      {entIdx > 0 && (
                        <span className={styles.arrow} aria-hidden='true'>
                          →
                        </span>
                      )}
                      {/* 实体 Tag：hover 时 Tooltip 显示 entity_type */}
                      <Tooltip
                        content={`${ent.entity_type} · ${ent.entity_id}`}
                        mini
                      >
                        <Tag
                          className={styles.entityTag}
                          onClick={() => handleEntityClick(ent.entity_id)}
                        >
                          {ent.text}
                        </Tag>
                      </Tooltip>
                    </React.Fragment>
                  ))}
                </Space>
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
};

HopViaEntitiesDisplay.displayName = 'HopViaEntitiesDisplay';

export default HopViaEntitiesDisplay;
