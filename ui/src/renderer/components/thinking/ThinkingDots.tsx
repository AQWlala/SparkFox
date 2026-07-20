/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ThinkingDots — 思考三点动画组件
 *
 * 来源：BaiLongma src/ui/brain-ui/styles.css line-thinking（清洁室重写为 React 组件）
 *
 * 保留 BaiLongma 特性：
 * - 三个圆点错峰跳动（0.15s / 0.3s / 0.45s 延迟）
 * - think-dot 动画（0%→80%→100% opacity 0.25, 40% opacity 1 translateY -2px）
 * - 思考完成后隐藏（line-thinking.done { display: none }）
 *
 * 改造点：
 * - CSS class 切换 → React 条件渲染
 * - BaiLongma CSS 变量颜色 → Apple 系统蓝 --sf-color-primary
 */

import React from 'react';

interface ThinkingDotsProps {
  /** 是否已完成思考（完成后隐藏） */
  done?: boolean;
}

const ThinkingDots: React.FC<ThinkingDotsProps> = ({ done = false }) => {
  if (done) return null;
  return (
    <div className='sf-thinking-dots' aria-label='思考中'>
      <span className='sf-thinking-dot' />
      <span className='sf-thinking-dot' />
      <span className='sf-thinking-dot' />
    </div>
  );
};

ThinkingDots.displayName = 'ThinkingDots';

export default ThinkingDots;
