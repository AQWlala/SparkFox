/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentDashboardView — Agent 仪表盘（简单实现）
 *
 * 来源：OpenAkita AgentDashboardView.tsx（清洁室简化重写）
 *
 * 功能（PoC 简单实现）：
 * - 显示当前活跃 Agent 概览（图标 + 名称 + 描述 + 配置摘要）
 * - 全部 Agent 列表 + 快速切换当前 Agent
 * - 关键统计（总数 / 内置数 / 自定义数 / 隔离身份数）
 *
 * 改造点：
 * - OpenAkita 复杂仪表盘 → 简化版概览（实际实现见 Phase 2）
 * - 对接 monitorStore 数据源（后续模块接入时扩展）
 * - Apple 风格：圆角 8px + 系统蓝 + SF Pro
 */

import React from 'react';
import { useAgentStore } from '@renderer/store/agentStore';
import { AgentIcon } from '@renderer/components/agent/AgentIcon';

export const AgentDashboardView: React.FC = () => {
  const agents = useAgentStore((s) => s.agents);
  const currentAgentId = useAgentStore((s) => s.currentAgentId);
  const setCurrentAgent = useAgentStore((s) => s.setCurrentAgent);
  const initialize = useAgentStore((s) => s.initialize);
  const fetchMemoryStats = useAgentStore((s) => s.fetchMemoryStats);

  React.useEffect(() => {
    initialize();
  }, [initialize]);

  const currentAgent = agents.find((a) => a.id === currentAgentId) || agents[0];

  // 统计
  const stats = React.useMemo(() => {
    return {
      total: agents.length,
      builtin: agents.filter((a) => a.type === 'builtin').length,
      custom: agents.filter((a) => a.type === 'custom').length,
      isolated: agents.filter((a) => a.identity_mode === 'isolated').length,
    };
  }, [agents]);

  // 当前 Agent 记忆统计
  const [memStats, setMemStats] = React.useState<{ exists: boolean; semantic_count: number; db_size_bytes: number } | null>(null);
  React.useEffect(() => {
    if (currentAgent) {
      fetchMemoryStats(currentAgent.id).then(setMemStats).catch(() => setMemStats(null));
    }
  }, [currentAgent, fetchMemoryStats]);

  return (
    <div className='sf-agent-dashboard-view'>
      <header className='sf-agent-dashboard-header'>
        <h1>Agent 仪表盘</h1>
      </header>

      {/* 关键统计 */}
      <div className='sf-agent-dashboard-stats'>
        <div className='sf-agent-dashboard-stat-card'>
          <div className='sf-agent-dashboard-stat-label'>Agent 总数</div>
          <div className='sf-agent-dashboard-stat-value'>{stats.total}</div>
        </div>
        <div className='sf-agent-dashboard-stat-card'>
          <div className='sf-agent-dashboard-stat-label'>内置 Agent</div>
          <div className='sf-agent-dashboard-stat-value'>{stats.builtin}</div>
        </div>
        <div className='sf-agent-dashboard-stat-card'>
          <div className='sf-agent-dashboard-stat-label'>自定义 Agent</div>
          <div className='sf-agent-dashboard-stat-value'>{stats.custom}</div>
        </div>
        <div className='sf-agent-dashboard-stat-card'>
          <div className='sf-agent-dashboard-stat-label'>隔离身份</div>
          <div className='sf-agent-dashboard-stat-value'>{stats.isolated}</div>
        </div>
      </div>

      {/* 当前活跃 Agent 概览 */}
      {currentAgent && (
        <div className='sf-agent-dashboard-current'>
          <h2>当前活跃 Agent</h2>
          <div className='sf-agent-dashboard-current-card'>
            <div
              className='sf-agent-dashboard-current-icon'
              style={{ background: `${currentAgent.color}1A` }}
            >
              <AgentIcon icon={currentAgent.icon} color={currentAgent.color} size={48} />
            </div>
            <div className='sf-agent-dashboard-current-info'>
              <div className='sf-agent-dashboard-current-name'>{currentAgent.name}</div>
              <div className='sf-agent-dashboard-current-desc'>{currentAgent.description}</div>
              <div className='sf-agent-dashboard-current-meta'>
                <span>类型: {currentAgent.type === 'builtin' ? '内置' : '自定义'}</span>
                <span>身份: {currentAgent.identity_mode === 'isolated' ? '隔离' : '共享'}</span>
                <span>记忆: {currentAgent.memory_mode === 'isolated' ? '隔离' : '共享'}</span>
                {memStats && (
                  <span>语义记忆: {memStats.semantic_count} 条</span>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 全部 Agent 快速切换 */}
      <div className='sf-agent-dashboard-list'>
        <h2>快速切换</h2>
        <div className='sf-agent-dashboard-list-grid'>
          {agents.map((agent) => (
            <button
              key={agent.id}
              type='button'
              className={`sf-agent-dashboard-list-card${agent.id === currentAgentId ? ' active' : ''}`}
              onClick={() => setCurrentAgent(agent.id)}
            >
              <AgentIcon icon={agent.icon} color={agent.color} size={24} />
              <div className='sf-agent-dashboard-list-card-name'>{agent.name}</div>
              {agent.id === currentAgentId && (
                <span className='sf-agent-dashboard-list-card-active'>✓</span>
              )}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};

export default AgentDashboardView;
