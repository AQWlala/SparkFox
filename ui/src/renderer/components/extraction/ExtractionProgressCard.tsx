/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

import React from 'react';
import { Progress } from '@arco-design/web-react';
import { CheckOne, FileText, LoadingFour, MagicHat, Remind } from '@icon-park/react';
import { useExtractionStatus } from './useExtractionStatus';
import { type ExtractionStatus } from './constants';
import styles from './ExtractionProgressCard.module.css';

/**
 * ExtractionProgressCard Props
 *
 * `status` 来自后端 `knowledge_event.status` 字段（5 状态之一）；
 * `event_count` / `entity_count` 在 EXTRACTING 阶段由后端实时回传，COMPLETED 阶段为最终总数。
 */
export interface ExtractionProgressCardProps {
  status: ExtractionStatus;
  /** 已抽取的事件数量（EXTRACTING/COMPLETED 阶段显示） */
  event_count?: number;
  /** 已抽取的实体数量（EXTRACTING/COMPLETED 阶段显示） */
  entity_count?: number;
  /** 可选的额外 className，便于 KnowledgeDetailPage 嵌入时调整布局 */
  className?: string;
}

/**
 * 状态 → 图标映射
 *
 * - PENDING:    Remind   （待处理）
 * - PARSING:    LoadingFour（解析中，旋转动画由 CSS 控制）
 * - PARSED:     FileText （已解析）
 * - EXTRACTING: MagicHat （抽取中，LLM 魔法帽）
 * - COMPLETED:  CheckOne （已完成）
 */
const STATUS_ICON: Record<ExtractionStatus, React.ReactNode> = {
  PENDING: <Remind theme='outline' size='16' />,
  PARSING: <LoadingFour theme='outline' size='16' />,
  PARSED: <FileText theme='outline' size='16' />,
  EXTRACTING: <MagicHat theme='outline' size='16' />,
  COMPLETED: <CheckOne theme='filled' size='16' />,
};

/**
 * ExtractionProgressCard —— U-05 修复组件
 *
 * SAG 提取管线 5 状态机联动卡片，替代原 KnowledgeDetailPage 的假进度条：
 *
 *   PENDING(10%) → PARSING(30%) → PARSED(50%) → EXTRACTING(80%) → COMPLETED(100%)
 *
 * 核心特性：
 *   1. 进度条与状态机严格联动（不再线性增长）
 *   2. EXTRACTING 阶段额外显示「已抽取 N 个事件 / M 个实体」
 *   3. COMPLETED 阶段显示完成图标与最终计数
 *   4. 同时导出 default + named，方便 KnowledgeDetailPage 任一方式引入
 *
 * @example
 * ```tsx
 * // 默认导入
 * import ExtractionProgressCard from '@/renderer/components/extraction/ExtractionProgressCard';
 * <ExtractionProgressCard status='EXTRACTING' event_count={42} entity_count={128} />
 *
 * // 命名导入
 * import { ExtractionProgressCard } from '@/renderer/components/extraction/ExtractionProgressCard';
 * ```
 */
const ExtractionProgressCard: React.FC<ExtractionProgressCardProps> = ({
  status,
  event_count,
  entity_count,
  className,
}) => {
  const { progress, label, color, isCompleted } = useExtractionStatus(status);
  const icon = STATUS_ICON[status];
  const eventCount = event_count ?? 0;
  const entityCount = entity_count ?? 0;

  return (
    <div className={`${styles.card} ${className ?? ''}`} data-status={status} data-testid='extraction-progress-card'>
      <div className={styles.header}>
        <span className={styles.statusIcon} style={{ background: `${color}1f`, color }} aria-hidden='true'>
          {icon}
        </span>
        <span className={styles.statusLabel}>{label}</span>
        <span className={styles.percent}>{progress}%</span>
      </div>

      <Progress
        percent={progress}
        status={isCompleted ? 'success' : 'normal'}
        showText={false}
        strokeWidth={6}
        color={color}
      />

      {status === 'EXTRACTING' && (
        <div className={styles.countsRow} data-testid='extraction-counts-row'>
          <span className={styles.countItem}>
            已抽取 <span className={styles.countValue}>{eventCount}</span> 个事件
          </span>
          <span className={styles.countItem}>
            <span className={styles.countValue}>{entityCount}</span> 个实体
          </span>
        </div>
      )}

      {isCompleted && (
        <div className={styles.countsRow} data-testid='extraction-completed-row'>
          <span className={styles.completedBadge}>
            <CheckOne theme='filled' size='14' />
            抽取完成
          </span>
          <span className={styles.countItem}>
            共 <span className={styles.countValue}>{eventCount}</span> 个事件
          </span>
          <span className={styles.countItem}>
            <span className={styles.countValue}>{entityCount}</span> 个实体
          </span>
        </div>
      )}
    </div>
  );
};

export default ExtractionProgressCard;
export { ExtractionProgressCard };
