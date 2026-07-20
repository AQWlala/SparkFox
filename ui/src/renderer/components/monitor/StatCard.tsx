/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox StatCard — 统计卡片（带 AnimatedNumber 动画数字）
 *
 * 来源：OpenAkita OrgDashboard AnimatedNumber + 统计卡片（清洁室重写）
 * 功能：
 * - 大数字 + 单位 + 子标签
 * - 趋势百分比（绿色上升 / 红色下降 / 灰色持平）
 * - 数字从旧值平滑过渡到新值（requestAnimationFrame）
 * - 左侧彩色图标圆圈
 *
 * Apple 风格：圆角 12px + 浅灰背景 + SF Pro 字体
 */

import React, { useRef } from 'react';
import { fmtNum, fmtCost, type StatCardData } from '@renderer/store/monitorStore';

/** 动画数字组件（借鉴 OpenAkita AnimatedNumber） */
const AnimatedNumber: React.FC<{ value: number; color: string }> = ({ value, color }) => {
  const spanRef = useRef<HTMLSpanElement>(null);
  const prevRef = useRef(0);
  const frameRef = useRef<number | null>(null);

  React.useEffect(() => {
    const el = spanRef.current;
    if (!el) return;
    const prev = prevRef.current;
    const diff = value - prev;
    if (diff === 0) return;

    const duration = 600;
    const start = performance.now();

    const tick = (now: number) => {
      const t = Math.min((now - start) / duration, 1);
      // ease-out cubic
      const eased = 1 - Math.pow(1 - t, 3);
      const current = prev + diff * eased;
      el.textContent = fmtNum(current);
      if (t < 1) {
        frameRef.current = requestAnimationFrame(tick);
      } else {
        prevRef.current = value;
        el.textContent = fmtNum(value);
      }
    };

    if (frameRef.current) cancelAnimationFrame(frameRef.current);
    frameRef.current = requestAnimationFrame(tick);

    return () => {
      if (frameRef.current) cancelAnimationFrame(frameRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);

  return (
    <span ref={spanRef} style={{ color, fontWeight: 700, fontSize: 28, letterSpacing: '-0.5px' }}>
      {fmtNum(value)}
    </span>
  );
};

export interface StatCardProps {
  data: StatCardData;
  /** 是否使用费用格式（¥ 前缀，6 位小数） */
  formatCost?: boolean;
}

export const StatCard: React.FC<StatCardProps> = ({ data, formatCost = false }) => {
  const { label, value, unit, color, icon, trend, sub_label } = data;

  const displayValue = formatCost ? fmtCost(value) : fmtNum(value);
  const trendColor = !trend || trend === 0 ? '#8E8E93' : trend > 0 ? '#34C759' : '#FF3B30';
  const trendIcon = !trend || trend === 0 ? '→' : trend > 0 ? '↑' : '↓';

  return (
    <div className='sf-stat-card' style={{ borderColor: `${color}33` }}>
      {/* 左侧图标 */}
      <div
        className='sf-stat-card-icon'
        style={{ background: `${color}1A`, color, fontSize: 22 }}
      >
        {icon}
      </div>

      {/* 右侧内容 */}
      <div className='sf-stat-card-content'>
        <div className='sf-stat-card-label'>{label}</div>
        <div className='sf-stat-card-value-row'>
          {formatCost ? (
            <span style={{ color, fontWeight: 700, fontSize: 28, letterSpacing: '-0.5px' }}>
              {displayValue}
            </span>
          ) : (
            <AnimatedNumber value={value} color={color} />
          )}
          {unit && <span className='sf-stat-card-unit'>{unit}</span>}
          {trend !== undefined && trend !== 0 && (
            <span
              className='sf-stat-card-trend'
              style={{ color: trendColor }}
            >
              {trendIcon} {Math.abs(trend)}%
            </span>
          )}
        </div>
        {sub_label && <div className='sf-stat-card-sub'>{sub_label}</div>}
      </div>
    </div>
  );
};

export default StatCard;
