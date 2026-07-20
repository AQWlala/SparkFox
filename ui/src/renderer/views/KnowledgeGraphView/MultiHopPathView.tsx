/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MultiHopPathView — 多跳检索路径可视化组件（spec §三 11.5.1 / 第 16 波并行 sub-step B）
 *
 * 本组件在 KnowledgeGraphView 的 GraphFlow 下方渲染「多跳检索路径」：
 *   - 顶部：标题「多跳检索路径」+ 关闭按钮（onClose 回调）
 *   - 中部：Arco Steps 横向展示 hop1 → hop2 → hop3 路径节点
 *   - 底部：via_entities Tag 列表（点击 Tag 可跳转到对应实体节点）
 *
 * 视觉规范（与 ReasoningChainPanel hop 颜色映射保持一致）：
 *   - hop1 = 蓝色（ATOMIC 单跳，#007aff）
 *   - hop2 = 黄色（MULTI 二跳扩展，#ff9500）
 *   - hop3 = 灰色（MULTI 三跳及以上，#6e6e73）
 *
 * 类型设计：
 *   - SearchHit / EntityRef 类型在组件内部 export 定义，避免修改 types.ts
 *     （types.ts 是 SVG 渲染专用类型，本组件的多跳检索语义独立维护）
 *   - 字段对齐后端 sparkfox-knowledge::search::SearchHit（11.6.x 后端检索实现）
 *
 * 范围说明（spec §三 11.5.1）：
 *   - 本波仅实施前端渲染（PoC：mock hit 数据驱动）
 *   - 真实 hit 数据由 11.4.2（IPC）+ 11.6.1（hnswlib-rs 后端检索）填充
 *   - via_entities Tag 的 onClick 跳转通过 onEntityClick 回调暴露给父组件
 */

import React from 'react';
import { Button, Card, Steps, Tag } from '@arco-design/web-react';
import styles from './MultiHopPathView.module.css';

// Arco 别名（与项目其他模块写法保持一致）
const Step = Steps.Step;

// ─── 类型定义（组件内部 export，避免修改 types.ts） ─────────────────────────────

/**
 * 实体引用 — 对应后端 sparkfox-knowledge::search::EntityRef。
 * 用于 via_entities 字段，表示多跳路径上经过的实体。
 */
export interface EntityRef {
  entity_id: string;
  entity_type: string;
  /** 实体文本（实体名称或表面形式） */
  text: string;
}

/**
 * 检索命中 — 对应后端 sparkfox-knowledge::search::SearchHit。
 * 由后端检索（11.6.1 hnswlib-rs）返回，前端在 GraphFlow 节点点击时显示对应 hit。
 */
export interface SearchHit {
  event_id: string;
  /** 相关性得分（0..1，越高越相关） */
  score: number;
  /** 跳跃层级（1=ATOMIC / 2=MULTI 二跳 / 3=MULTI 三跳及以上 / null=未分级） */
  hop: number | null;
  /** 经过的实体列表（多跳路径上的中间实体） */
  via_entities: EntityRef[];
  /** 命中的 chunk_id（可选，11.6.x 后端填充） */
  chunk_id?: string | null;
}

// ─── hop 颜色映射（与 ReasoningChainPanel HOP_CLASS_MAP 保持一致） ───────────────

/**
 * hop → CSS 类名映射表（spec §三 11.5.1）。
 * - hop1：蓝色（ATOMIC 单跳，#007aff）
 * - hop2：黄色（MULTI 二跳扩展，#ff9500）
 * - hop3：灰色（MULTI 三跳及以上，#6e6e73）
 *
 * 与 ReasoningChainPanel.module.css 的 hop1/hop2/hop3 类名完全对齐，
 * 便于用户跨视图形成「hop=1 蓝 / hop=2 黄 / hop=3 灰」的统一视觉记忆。
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
 * @param hop 跳数；其他值返回空字符串
 */
export function formatHopLabel(hop: number | null | undefined): string {
  if (hop === null || hop === undefined) return '';
  if (hop >= 1 && hop <= 3) return `hop${hop}`;
  return '';
}

// ─── Props ─────────────────────────────────────────────────────────────────────

/**
 * MultiHopPathView 组件 Props（spec §三 11.5.1）。
 *
 * - hit：当前选中的检索命中（含 hop / via_entities）；null 时显示占位
 * - onClose：关闭按钮回调（父组件隐藏 MultiHopPathView 容器）
 * - onEntityClick：via_entities Tag 点击回调（参数为 entity_id，父组件可跳转到对应节点）
 */
export interface MultiHopPathViewProps {
  hit: SearchHit | null;
  onClose?: () => void;
  onEntityClick?: (entityId: string) => void;
}

// ─── 主组件 ───────────────────────────────────────────────────────────────────

/**
 * MultiHopPathView 主组件。
 *
 * 渲染逻辑：
 *   - hit === null：渲染占位文案「点击图谱节点查看多跳路径」
 *   - hit 非 null：渲染 Card + Steps + via_entities Tag + score
 *
 * Steps 派生：根据 hit.via_entities 长度动态生成 Step 列表，
 *   每个 Step 的 className 由 getHopColorClass(hopIndex+1) 注入，
 *   实现 hop1 → hop2 → hop3 的颜色递进。
 */
const MultiHopPathView: React.FC<MultiHopPathViewProps> = ({
  hit,
  onClose,
  onEntityClick,
}) => {
  // ─── 无 hit 时：渲染占位文案 ───
  if (!hit) {
    return (
      <Card className={styles.pathView} bordered>
        <div className={styles.placeholder}>点击图谱节点查看多跳路径</div>
      </Card>
    );
  }

  // ─── 派生 Steps：via_entities 末尾追加「命中」节点表示最终命中 ───
  // 简化策略：Step 序号即 hop 序号（hop1 = via_entities[0]，hop2 = via_entities[1]，...）
  // 实际生产中应由后端在每个 EntityRef 上携带显式 hop 字段（11.6.x 阶段实现）
  interface StepItem {
    title: string;
    description: string;
    /** hop 颜色 CSS 类名（hop1 / hop2 / hop3 对应类） */
    className: string;
    /** hop 显示标签（如「hop1」「hop2」），空字符串表示不显示徽章 */
    hopLabel: string;
  }
  const stepsItems: StepItem[] = hit.via_entities.map((ent, idx) => {
    const hopNumber = idx + 1; // 1 / 2 / 3
    const colorClass = getHopColorClass(hopNumber); // hop1 / hop2 / hop3
    const hopLabel = formatHopLabel(hopNumber);
    return {
      title: ent.text,
      description: `${ent.entity_type} · ${hopLabel}`,
      className: colorClass ? styles[colorClass] : '',
      hopLabel,
    };
  });

  // 末尾追加「命中」Step（表示最终命中的 event）
  const hitHop = hit.hop ?? stepsItems.length + 1;
  const hitColorClass = getHopColorClass(hitHop);
  stepsItems.push({
    title: hit.event_id,
    description: `命中 · score: ${hit.score.toFixed(3)}`,
    className: hitColorClass ? styles[hitColorClass] : '',
    hopLabel: formatHopLabel(hitHop),
  });

  // 当前激活 Step（用于 Arco Steps current 属性，指向最终命中）
  const currentStep = stepsItems.length - 1;

  // ─── 关闭按钮回调（PoC：直接调用 props.onClose） ───
  const handleClose = () => {
    // eslint-disable-next-line no-console
    console.log('[MultiHopPathView] close button clicked');
    onClose?.();
  };

  // ─── via_entities Tag 点击回调（PoC：跳转到对应实体节点） ───
  const handleEntityClick = (entityId: string) => {
    // eslint-disable-next-line no-console
    console.log('[MultiHopPathView] via_entity tag clicked:', entityId);
    onEntityClick?.(entityId);
  };

  return (
    <Card className={styles.pathView} bordered>
      {/* ─── 顶部：标题 + 关闭按钮 ─── */}
      <div className={styles.header}>
        <h3 className={styles.title}>多跳检索路径</h3>
        <Button
          size='mini'
          type='text'
          onClick={handleClose}
        >
          关闭
        </Button>
      </div>

      {/* ─── 中部：Arco Steps 横向展示 hop1 → hop2 → hop3 路径 ─── */}
      <Steps current={currentStep} direction='horizontal' className={styles.steps}>
        {stepsItems.map((item, idx) => (
          <Step
            key={`hop-step-${idx}`}
            title={
              <span>
                {item.title}
                {item.hopLabel && (
                  <span className={`${styles.hopBadge} ${item.className}`}>
                    {item.hopLabel}
                  </span>
                )}
              </span>
            }
            description={item.description}
            className={item.className}
          />
        ))}
      </Steps>

      {/* ─── 底部：score + via_entities Tag 列表 ─── */}
      <div className={styles.score}>
        <span>score: {hit.score.toFixed(3)}</span>
      </div>

      {hit.via_entities.length > 0 && (
        <div className={styles.viaEntities}>
          <span className={styles.viaEntitiesLabel}>via_entities:</span>
          <div className={styles.viaEntitiesList}>
            {hit.via_entities.map((ent, idx) => {
              const hopNumber = idx + 1;
              const colorClass = getHopColorClass(hopNumber);
              return (
                <Tag
                  key={ent.entity_id}
                  className={`${styles.viaEntityTag} ${
                    colorClass ? styles[colorClass] : ''
                  }`}
                  onClick={() => handleEntityClick(ent.entity_id)}
                >
                  {ent.text} ({ent.entity_type})
                </Tag>
              );
            })}
          </div>
        </div>
      )}
    </Card>
  );
};

MultiHopPathView.displayName = 'MultiHopPathView';

export default MultiHopPathView;
