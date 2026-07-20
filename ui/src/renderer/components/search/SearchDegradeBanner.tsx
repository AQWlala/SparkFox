/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SearchDegradeBanner —— 检索降级提示横幅组件（spec §三 10.12.2 / U-06b 修复）
 *
 * 背景：
 *   后端 sparkfox-knowledge crate 在 event 表为空时，ATOMIC/MULTI 检索策略无法使用，
 *   会自动降级到 VECTOR 检索。SearchResult.degrade_reason 字段标识降级原因
 *   （如「未抽取事件」），SearchHit.is_degraded 标识是否降级。
 *
 *   U-06b 修复：当检索发生降级时，前端缺失对用户的明确提示。本组件在 ChatView 顶部
 *   展示一条 warning 横幅，告知用户当前检索已降级到 VECTOR 模式，并允许手动关闭。
 *
 * Props：
 *   - is_degraded: 是否处于降级状态（来自 SearchResult / SearchHit）
 *   - degrade_reason: 降级原因文案（可选，默认「未抽取事件」）
 *   - onDismiss: 用户手动关闭横幅时的回调（可选）
 */

import { Alert } from '@arco-design/web-react';
import { useDegradeBanner } from './useDegradeBanner';
import styles from './SearchDegradeBanner.module.css';

export interface SearchDegradeBannerProps {
  /** 当前检索结果是否处于降级状态 */
  is_degraded: boolean;
  /** 降级原因（来自后端 degrade_reason 字段） */
  degrade_reason?: string;
  /** 用户手动关闭横幅时的回调 */
  onDismiss?: () => void;
}

/**
 * 检索降级提示横幅
 *
 * 当 is_degraded=true 时展示 warning Alert，文案包含「未抽取事件」关键词
 * 与降级目标「VECTOR 检索」。用户可点击关闭按钮 dismiss 横幅。
 */
export function SearchDegradeBanner({ is_degraded, degrade_reason, onDismiss }: SearchDegradeBannerProps) {
  const { visible, dismiss } = useDegradeBanner(is_degraded);

  // event 表有数据时（is_degraded=false）或用户已手动关闭时，隐藏横幅
  if (!is_degraded || !visible) return null;

  // spec 明确要求文案含「未抽取事件」关键词；若后端提供 degrade_reason 则优先使用
  const reason = degrade_reason || '未抽取事件';
  const content = `${reason}，已降级到 VECTOR 检索`;

  const handleClose = () => {
    dismiss();
    onDismiss?.();
  };

  return (
    <Alert
      className={styles.banner}
      type='warning'
      closable
      onClose={handleClose}
      content={content}
    />
  );
}

export default SearchDegradeBanner;
