/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * L2Panel — L2 右侧自主行动面板
 *
 * 来源：BaiLongma src/ui/brain-ui/app-shell.js createSecondaryPanel()（清洁室重写为 React 组件）
 *
 * 保留 BaiLongma 特性：
 * - 右侧固定面板（width: 320px）
 * - 顶部统计栏（状态 / tok/s / 召回/h / 抽取/h）
 * - 中部"自主行动机制 · Tick"标题
 * - 思考流容器（ThoughtStream with streamKey=L2_STREAM_KEY）
 *
 * 改造点：
 * - BaiLongma 双面板（L1+L2）→ SparkFox 单栏对话（L1 嵌入气泡）+ L2 独立右侧面板
 * - BaiLongma 6 个统计指标 → SparkFox 精简为 3 个（状态 / 节点 / tok/s）
 * - BaiLongma 主题色（warm 暖色）→ Apple 系统蓝
 * - BaiLongma backdrop-filter blur → Apple 系统毛玻璃（保留）
 *
 * 用户选择 2B：保留双面板布局，L2 作为右侧独立面板
 */

import React from 'react';
import ThoughtStream from './ThoughtStream';
import AiActivityBadge from './AiActivityBadge';
import { L2_STREAM_KEY } from '@renderer/store/thinkingStore';

interface L2PanelProps {
  /** 是否折叠（默认 false） */
  collapsed?: boolean;
  /** 折叠/展开回调 */
  onToggle?: () => void;
}

const L2Panel: React.FC<L2PanelProps> = ({ collapsed = false, onToggle }) => {
  if (collapsed) {
    return (
      <aside className='sf-l2-panel collapsed'>
        <button
          type='button'
          className='sf-l2-panel-toggle'
          onClick={onToggle}
          aria-label='展开自主行动面板'
          title='展开自主行动面板（]）'
        >
          ◀
        </button>
      </aside>
    );
  }

  return (
    <aside className='sf-l2-panel'>
      {/* 顶部统计栏 */}
      <header className='sf-l2-panel-stats'>
        <div className='sf-stat'>
          <span className='sf-stat-label'>状态</span>
          <div className='sf-stat-value live'>
            <span className='sf-live-dot' />
            Token流
          </div>
        </div>
        <div className='sf-stat'>
          <span className='sf-stat-label'>节点</span>
          <div className='sf-stat-value' id='sf-node-count'>
            0
          </div>
        </div>
        <div className='sf-stat'>
          <span className='sf-stat-label'>tok/s</span>
          <div className='sf-stat-value' id='sf-tok-rate'>
            —
          </div>
        </div>
        <button
          type='button'
          className='sf-l2-panel-toggle'
          onClick={onToggle}
          aria-label='折叠自主行动面板'
          title='折叠自主行动面板（]）'
        >
          ▶
        </button>
      </header>

      {/* 中部标题 */}
      <div className='sf-l2-panel-meta'>
        <div>
          <div className='sf-l2-panel-title'>自主行动机制 · Tick</div>
          <div className='sf-l2-panel-subtitle'>心跳 · 思考 · 工具</div>
        </div>
        <span className='sf-l2-pill'>流式传输</span>
      </div>

      {/* AI 活动状态 */}
      <AiActivityBadge />

      {/* 思考流容器 */}
      <div className='sf-l2-stream'>
        <ThoughtStream
          streamKey={L2_STREAM_KEY}
          emptyText='等待 Tick 心跳…'
          maxLines={200}
        />
      </div>
    </aside>
  );
};

L2Panel.displayName = 'L2Panel';

export default L2Panel;
