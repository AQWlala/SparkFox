/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ReasoningStep — 推理链单步子组件（REFACTOR 阶段从 ReasoningChainPanel 提取）
 *
 * 来源：spec §三 10.9.2 (Sub-Step 10.7.2 / TDD-REFACTOR)
 *
 * 职责：
 * - 渲染单个推理步骤（Step1..Step7 中的一个）
 * - 显示 step 标识 + title + 可选 hop badge + content
 * - 接收 collapsed 状态与 onToggle/onClick 回调，由父组件统一管理折叠状态
 *
 * 设计说明：
 * - 折叠状态由父组件 ReasoningChainPanel 通过 useState 集中管理（便于"全部折叠/展开"等批量操作）
 * - hop 颜色由 getHopClassName 计算（hop1 蓝 / hop2 黄 / hop3 灰），CSS module 中定义对应类
 * - 此子组件为纯展示组件，无副作用，便于复用与测试
 * - 同时接受 onToggle（语义化名称）与 onClick（通用点击别名）回调；二者均触发折叠切换
 */

import React, { memo, useCallback } from 'react';
import { getHopClassName, formatHopLabel } from './ReasoningChainPanel';
import type { ReasoningStep as ReasoningStepType } from './ReasoningChainPanel';
import styles from './ReasoningChainPanel.module.css';

interface ReasoningStepProps {
  /** 步骤数据（含 step / title / content / hop?） */
  step: ReasoningStepType;
  /** 显式传递的 title（便于父组件源码可读性，与 step.title 等价） */
  title: string;
  /** 显式传递的 content（便于父组件源码可读性，与 step.content 等价） */
  content: string;
  /** 是否已折叠 */
  collapsed: boolean;
  /** 切换折叠状态回调（语义化名称） */
  onToggle: (step: string) => void;
  /** 点击事件回调别名（与 onToggle 等价，便于外部点击追踪） */
  onClick: (step: string) => void;
}

const ReasoningStepComponent: React.FC<ReasoningStepProps> = ({
  step,
  collapsed,
  onToggle,
  onClick,
}) => {
  const hopCls = getHopClassName(step.hop);
  const hopLabel = formatHopLabel(step.hop);

  const handleToggle = useCallback(() => {
    onToggle(step.step);
    onClick(step.step);
  }, [onToggle, onClick, step.step]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onToggle(step.step);
        onClick(step.step);
      }
    },
    [onToggle, onClick, step.step]
  );

  return (
    <div
      className={[
        styles.reasoningStep,
        collapsed ? styles.collapsed : '',
        hopCls ? styles[hopCls] : '',
      ]
        .filter(Boolean)
        .join(' ')}
    >
      <div
        className={styles.stepHeader}
        onClick={handleToggle}
        onKeyDown={handleKeyDown}
        role='button'
        tabIndex={0}
        aria-expanded={!collapsed}
      >
        <span className={styles.stepChevron}>{collapsed ? '▸' : '▾'}</span>
        <span className={styles.stepId}>{step.step}</span>
        <span className={styles.stepTitle}>{step.title}</span>
        {hopLabel && (
          <span
            className={[styles.hopBadge, hopCls ? styles[hopCls] : '']
              .filter(Boolean)
              .join(' ')}
          >
            {hopLabel}
          </span>
        )}
      </div>
      {!collapsed && <div className={styles.stepContent}>{step.content}</div>}
    </div>
  );
};

ReasoningStepComponent.displayName = 'ReasoningStep';

export default memo(ReasoningStepComponent);
