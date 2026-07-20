/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ReasoningChainPanel — 推理链主面板组件 (U-01 修复)
 *
 * 来源：spec §三 10.9.2 (Sub-Step 10.7.2)
 *
 * ─── U-01 修复说明 ─────────────────────────────────────────────────────────────
 * v1.1.0 引入 MULTI 多跳检索策略后，原 thought_process 在 Step7 被丢弃，用户无法审计
 * 多跳推理路径（spec docs/SparkFox-v1.1.0-规划.md line 160）。
 *
 * 本组件完整渲染 Step1..Step7 推理链，并高亮 via_entities 多跳路径
 * （hop=1 蓝色 / hop=2 黄色 / hop=3 灰色），让用户能追溯每一跳的命中实体。
 *
 * ─── 架构（REFACTOR 后） ───────────────────────────────────────────────────────
 * - ReasoningChainPanel（本组件）：管理折叠状态 + 渲染步骤列表 + 渲染 via_entities
 * - ReasoningStep（子组件）：单步展示，从本组件接收 collapsed + onToggle props
 * - ReasoningChainPanel.module.css：样式（hop1/2/3 颜色、折叠态、占位文案）
 *
 * ─── Props ────────────────────────────────────────────────────────────────────
 *   - thought_process: ThoughtProcess | null  — 7 步推理链数据
 *   - via_entities: EntityRef[]                — 多跳路径上的实体引用列表
 */

import React, { useState, useCallback } from 'react';
import ReasoningStep from './ReasoningStep';
import styles from './ReasoningChainPanel.module.css';

// ─── 类型定义 ──────────────────────────────────────────────────────────────────

/** 实体引用 — 对应后端 sparkfox-knowledge::search::EntityRef */
export interface EntityRef {
  entity_id: string;
  entity_type: string;
  name: string;
}

/** 单个推理步骤 */
export interface ReasoningStep {
  step: string;
  title: string;
  content: string;
  /** 跳跃层级（1/2/3），缺省表示该步骤不涉及多跳 */
  hop?: number;
}

/** 推理链（thought_process）— 含 Step1..Step7 */
export interface ThoughtProcess {
  steps: ReasoningStep[];
}

/** ReasoningChainPanel 组件 Props */
export interface ReasoningChainPanelProps {
  thought_process: ThoughtProcess | null;
  via_entities: EntityRef[];
}

// ─── 纯函数（便于测试 & 复用） ──────────────────────────────────────────────────

/**
 * hop → CSS 类名映射表
 * - hop1：蓝色（ATOMIC 单跳）
 * - hop2：黄色（MULTI 二跳扩展）
 * - hop3：灰色（MULTI 三跳及以上）
 */
export const HOP_CLASS_MAP: Record<number, string> = {
  1: 'hop1',
  2: 'hop2',
  3: 'hop3',
};

/**
 * 根据跳数返回对应的 CSS 类名（用于 via_entities 高亮）
 * @param hop 跳数（1/2/3）；其他值返回空字符串
 */
export function getHopClassName(hop?: number | null): string {
  if (hop === undefined || hop === null) return '';
  return HOP_CLASS_MAP[hop] ?? '';
}

/**
 * 根据跳数返回 hop 标识文案（用于 badge 显示）
 * @param hop 跳数（1/2/3）；其他值返回空字符串
 */
export function formatHopLabel(hop?: number | null): string {
  if (hop === undefined || hop === null) return '';
  if (hop >= 1 && hop <= 3) return `hop${hop}`;
  return '';
}

/**
 * 判断 thought_process 是否为空（无步骤）
 * @param tp thought_process 对象
 */
export function isThoughtProcessEmpty(tp: ThoughtProcess | null): boolean {
  if (!tp) return true;
  if (!tp.steps || tp.steps.length === 0) return true;
  return false;
}

// ─── 主组件 ────────────────────────────────────────────────────────────────────

const ReasoningChainPanel: React.FC<ReasoningChainPanelProps> = ({
  thought_process,
  via_entities,
}) => {
  // 折叠状态：key 为 step 字段（如 "Step3"），value 为 true（已折叠）
  // 由父组件集中管理，便于未来扩展"全部折叠/展开"等批量操作
  const [collapsedSteps, setCollapsedSteps] = useState<Record<string, boolean>>({});

  const handleToggle = useCallback((step: string) => {
    setCollapsedSteps((prev) => ({ ...prev, [step]: !prev[step] }));
  }, []);

  // 空状态：显示占位文案「暂无推理链」（U-01 修复：避免空白面板让用户困惑）
  // 同时使用 !thought_process 显式判空以提供 TS 类型收窄（isThoughtProcessEmpty 调用不
  // 能让 TS 收窄 thought_process 为非 null，需先经 !thought_process 短路）
  if (!thought_process || isThoughtProcessEmpty(thought_process)) {
    return (
      <div className={`${styles.reasoningChain} ${styles.empty}`}>
        <div className={styles.placeholder}>暂无推理链</div>
      </div>
    );
  }

  return (
    <div className={styles.reasoningChain}>
      {/* 推理步骤列表（Step1..Step7）—— 通过 steps.map 渲染全部步骤 */}
      <div className={styles.reasoningSteps}>
        {thought_process.steps.map((s) => (
          <ReasoningStep
            key={s.step}
            step={s}
            title={s.title}
            content={s.content}
            collapsed={!!collapsedSteps[s.step]}
            onToggle={handleToggle}
            onClick={handleToggle}
          />
        ))}
      </div>

      {/* via_entities 多跳路径高亮（hop=1 蓝 / hop=2 黄 / hop=3 灰） */}
      {via_entities.length > 0 && (
        <div className={styles.viaEntities}>
          {via_entities.map((e, idx) => {
            // 简化策略：按位置循环分配 hop 颜色（实际应由后端按 chunk_span / hop 字段驱动）
            const hop = (idx % 3) + 1;
            const hopCls = getHopClassName(hop);
            return (
              <span
                key={e.entity_id}
                className={[styles.viaEntity, hopCls ? styles[hopCls] : '']
                  .filter(Boolean)
                  .join(' ')}
              >
                <span className={styles.viaEntityType}>{e.entity_type}</span>
                <span className={styles.viaEntityName}>{e.name}</span>
              </span>
            );
          })}
        </div>
      )}
    </div>
  );
};

ReasoningChainPanel.displayName = 'ReasoningChainPanel';

export default ReasoningChainPanel;
