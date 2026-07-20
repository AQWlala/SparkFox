/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox MiniBar — 迷你进度条
 *
 * 来源：OpenAkita TokenStatsView MiniBar（清洁室重写）
 * 功能：圆角进度条，用于 Token 用量占比、任务进度等可视化
 */

import React from 'react';

export interface MiniBarProps {
  /** 当前值 */
  value: number;
  /** 最大值 */
  max: number;
  /** 颜色（默认系统蓝） */
  color?: string;
  /** 高度（默认 6px） */
  height?: number;
  /** 是否显示百分比文字 */
  showPct?: boolean;
}

export const MiniBar: React.FC<MiniBarProps> = ({
  value,
  max,
  color = '#007AFF',
  height = 6,
  showPct = false,
}) => {
  const pct = max > 0 ? Math.min((value / max) * 100, 100) : 0;

  return (
    <div className='sf-minibar'>
      <div
        className='sf-minibar-track'
        style={{ height }}
      >
        <div
          className='sf-minibar-fill'
          style={{
            width: `${pct}%`,
            background: color,
            height: '100%',
          }}
        />
      </div>
      {showPct && (
        <span className='sf-minibar-pct' style={{ color }}>
          {pct.toFixed(1)}%
        </span>
      )}
    </div>
  );
};

export default MiniBar;
