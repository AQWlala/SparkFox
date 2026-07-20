/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * useDegradeBanner —— 检索降级横幅状态 hook（spec §三 10.12.2 / U-06b 修复）
 *
 * 职责：
 *   - 维护横幅 visible 状态
 *   - 当 is_degraded 由 false 变为 true 时自动展示横幅
 *   - 当 is_degraded 由 true 变为 false 时自动隐藏横幅
 *   - 提供 dismiss() 方法供用户手动关闭（点击 Alert 关闭按钮）
 *
 * 设计说明：
 *   - 用户手动 dismiss 后，若 is_degraded 仍为 true，横幅保持隐藏，直到下次降级发生
 *     （即 is_degraded 先变为 false 再变为 true 才会重新展示），避免打扰用户
 *   - 该 hook 由 SearchDegradeBanner 组件消费，组件层负责渲染 Arco Alert
 */

import { useEffect, useState } from 'react';

export interface UseDegradeBannerResult {
  /** 横幅当前是否可见 */
  visible: boolean;
  /** 用户手动关闭横幅 */
  dismiss: () => void;
}

/**
 * 检索降级横幅状态 hook
 *
 * @param is_degraded 当前检索结果是否处于降级状态
 * @returns { visible, dismiss }
 */
export function useDegradeBanner(is_degraded: boolean): UseDegradeBannerResult {
  const [visible, setVisible] = useState<boolean>(is_degraded);

  // 当降级状态变化时同步 visible：
  // - 进入降级（false → true）：自动展示
  // - 退出降级（true → false）：自动隐藏
  useEffect(() => {
    setVisible(is_degraded);
  }, [is_degraded]);

  const dismiss = () => setVisible(false);

  return { visible, dismiss };
}

export default useDegradeBanner;
