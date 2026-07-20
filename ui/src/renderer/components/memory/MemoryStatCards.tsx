/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MemoryStatCards — 6 层记忆统计卡片
 *
 * 来源：OpenAkita MemoryView stats 顶部行（清洁室重写）
 * 功能：6 个 L0-L5 层卡片 + 总计概览
 */

import React from 'react';
import {
  LAYER_LABELS,
  LAYER_DESCRIPTIONS,
  LAYER_COLORS,
  type MemoryLayer,
  type MemoryStats,
} from '@renderer/store/memoryStore';

interface Props {
  stats: MemoryStats | null;
  activeLayer: MemoryLayer | 'all';
  onSelectLayer: (layer: MemoryLayer | 'all') => void;
}

const LAYER_ORDER: MemoryLayer[] = ['L0', 'L1', 'L2', 'L3', 'L4', 'L5'];

const MemoryStatCards: React.FC<Props> = ({ stats, activeLayer, onSelectLayer }) => {
  const total = stats?.total ?? 0;
  const avgImportance = stats?.avg_importance ?? 0;
  const recentActive = stats?.recent_active ?? 0;
  const expiringSoon = stats?.expiring_soon ?? 0;

  return (
    <div className='sf-memory-stat-grid'>
      {/* 总览卡 */}
      <div
        className={`sf-memory-stat-card sf-memory-stat-overview ${activeLayer === 'all' ? 'is-active' : ''}`}
        onClick={() => onSelectLayer('all')}
      >
        <div className='sf-memory-stat-overview-title'>全部记忆</div>
        <div className='sf-memory-stat-overview-value'>{total}</div>
        <div className='sf-memory-stat-overview-meta'>
          <span title='平均重要性'>★ {(avgImportance * 100).toFixed(0)}%</span>
          <span title='最近 7 天活跃'>↻ {recentActive}</span>
          <span title='即将过期'>⏰ {expiringSoon}</span>
        </div>
      </div>

      {/* 6 层卡 */}
      {LAYER_ORDER.map((layer) => {
        const count = stats?.by_layer[layer] ?? 0;
        const color = LAYER_COLORS[layer];
        const isActive = activeLayer === layer;
        return (
          <div
            key={layer}
            className={`sf-memory-stat-card sf-memory-stat-layer ${isActive ? 'is-active' : ''}`}
            onClick={() => onSelectLayer(isActive ? 'all' : layer)}
            style={{ '--sf-layer-color': color } as React.CSSProperties}
          >
            <div className='sf-memory-stat-layer-header'>
              <span className='sf-memory-stat-layer-dot' style={{ background: color }} />
              <span className='sf-memory-stat-layer-label'>{LAYER_LABELS[layer]}</span>
            </div>
            <div className='sf-memory-stat-layer-count' style={{ color }}>
              {count}
            </div>
            <div className='sf-memory-stat-layer-desc'>{LAYER_DESCRIPTIONS[layer]}</div>
          </div>
        );
      })}
    </div>
  );
};

export default MemoryStatCards;
