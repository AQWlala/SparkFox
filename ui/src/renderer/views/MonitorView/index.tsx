/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox MonitorView — 监视面板主页面
 *
 * 来源：OpenAkita TokenStatsView + OrgDashboard + OrgMonitorPanel
 *       （清洁室重写为统一监视面板）
 *
 * 组合：
 * - StatCard × 4（统计卡片：Token / 费用 / Agent / 工具调用）
 * - TokenStatsPanel（Token 用量统计：时间周期 + 时间线 + 分组 + 会话 + 记录）
 * - ActivityFeed（实时活动流：事件列表 + 过滤 + 实时模式）
 *
 * 改造点：
 * - OpenAkita 3 个独立 View → 统一 MonitorView
 * - safeFetch HTTP → useMonitorStore（PoC: mock 数据）
 * - 组织/节点概念 → Agent + 对话 + 工具调用
 * - Apple 主题：圆角 12px + 系统蓝 + SF Pro
 */

import React from 'react';
import { Button } from '@arco-design/web-react';
import { useMonitorStore } from '@renderer/store/monitorStore';
import { StatCard } from '@renderer/components/monitor/StatCard';
import { TokenStatsPanel } from '@renderer/components/monitor/TokenStatsPanel';
import { ActivityFeed } from '@renderer/components/monitor/ActivityFeed';
import '@renderer/components/monitor/monitor.css';

const MonitorView: React.FC = () => {
  const initialize = useMonitorStore((s) => s.initialize);
  const refresh = useMonitorStore((s) => s.refresh);
  const loading = useMonitorStore((s) => s.loading);
  const statCards = useMonitorStore((s) => s.statCards);

  // 初始化（加载 mock 数据）
  React.useEffect(() => {
    initialize();
  }, [initialize]);

  const handleRefresh = async () => {
    await refresh();
  };

  return (
    <div className='sf-monitor-view'>
      {/* 顶部标题栏 */}
      <header className='sf-monitor-header'>
        <div className='sf-monitor-title'>
          <h1>监视面板</h1>
          <p>实时监控 Agent 运行状态、Token 用量、工具调用与活动事件</p>
        </div>
        <div className='sf-monitor-actions'>
          <Button
            type='secondary'
            size='small'
            onClick={handleRefresh}
            loading={loading}
          >
            ↻ 刷新
          </Button>
        </div>
      </header>

      {/* 统计卡片网格（4 个） */}
      <div className='sf-stat-cards-grid'>
        {statCards.map((card, i) => (
          <StatCard key={card.label} data={card} formatCost={i === 1} />
        ))}
      </div>

      {/* Token 用量统计面板 */}
      <TokenStatsPanel />

      {/* 实时活动流 */}
      <ActivityFeed />
    </div>
  );
};

export default MonitorView;
