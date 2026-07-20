/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentListPanel — Agent 列表面板
 *
 * 来源：OpenAkita AgentManagerView.tsx 左列表部分（清洁室重写为独立组件）
 *
 * 功能：
 * - 显示所有 Agent 卡片（按分类过滤 + 支持隐藏项）
 * - 卡片显示：图标 / 名称 / 描述 / 类型徽章 / 工具数 / 技能数
 * - 点击卡片：选中并打开编辑器
 * - 顶部分类栏 + 搜索框
 * - 批量选择（多选模式）
 *
 * Apple 风格：圆角 8px + 系统蓝选中态 + SF Pro 字体
 */

import React from 'react';
import { useAgentStore, type AgentProfile } from '@renderer/store/agentStore';
import { AgentIcon } from './AgentIcon';
import { AgentCategoryBar } from './AgentCategoryBar';

interface AgentListPanelProps {
  /** 是否显示分类栏 */
  showCategoryBar?: boolean;
  /** 是否允许多选 */
  multiSelect?: boolean;
}

export const AgentListPanel: React.FC<AgentListPanelProps> = ({
  showCategoryBar = true,
  multiSelect = false,
}) => {
  const agents = useAgentStore((s) => s.agents);
  const categories = useAgentStore((s) => s.categories);
  const activeCategory = useAgentStore((s) => s.activeCategory);
  const showHidden = useAgentStore((s) => s.showHidden);
  const setShowHidden = useAgentStore((s) => s.setShowHidden);
  const currentAgentId = useAgentStore((s) => s.currentAgentId);
  const setCurrentAgent = useAgentStore((s) => s.setCurrentAgent);
  const openEditor = useAgentStore((s) => s.openEditor);
  const batchSelected = useAgentStore((s) => s.batchSelected);
  const setBatchSelected = useAgentStore((s) => s.setBatchSelected);

  const [search, setSearch] = React.useState('');

  // 过滤逻辑：分类 + 搜索 + 隐藏项
  const filtered = React.useMemo(() => {
    let list = agents;
    if (activeCategory) {
      list = list.filter((a) => a.category === activeCategory);
    }
    if (!showHidden) {
      list = list.filter((a) => !a.hidden);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(
        (a) => a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q)
      );
    }
    return list;
  }, [agents, activeCategory, showHidden, search]);

  const handleCardClick = (agent: AgentProfile, e: React.MouseEvent) => {
    if (multiSelect && (e.metaKey || e.ctrlKey || e.shiftKey)) {
      const next = new Set(batchSelected);
      if (next.has(agent.id)) {
        next.delete(agent.id);
      } else {
        next.add(agent.id);
      }
      setBatchSelected(next);
      return;
    }
    setCurrentAgent(agent.id);
    openEditor(agent);
  };

  return (
    <div className='sf-agent-list-panel'>
      {/* 顶部搜索 + 工具栏 */}
      <div className='sf-agent-list-toolbar'>
        <input
          type='text'
          className='sf-agent-search-input'
          placeholder='搜索 Agent...'
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <button
          type='button'
          className={`sf-agent-toolbar-btn${showHidden ? ' active' : ''}`}
          onClick={() => setShowHidden(!showHidden)}
          title={showHidden ? '隐藏已隐藏项' : '显示已隐藏项'}
        >
          {showHidden ? '👁️' : '🙈'}
        </button>
      </div>

      {/* 分类栏 */}
      {showCategoryBar && <AgentCategoryBar />}

      {/* Agent 列表 */}
      <div className='sf-agent-list'>
        {filtered.length === 0 ? (
          <div className='sf-agent-list-empty'>
            <div className='sf-agent-list-empty-icon'>🤖</div>
            <div className='sf-agent-list-empty-text'>
              {search ? '未找到匹配的 Agent' : '暂无 Agent，点击右上角新建'}
            </div>
          </div>
        ) : (
          filtered.map((agent) => {
            const isSelected = currentAgentId === agent.id;
            const isBatchSelected = batchSelected.has(agent.id);
            const cat = categories.find((c) => c.id === agent.category);
            return (
              <button
                key={agent.id}
                type='button'
                className={`sf-agent-card${isSelected ? ' selected' : ''}${isBatchSelected ? ' batch-selected' : ''}`}
                onClick={(e) => handleCardClick(agent, e)}
              >
                {/* 批量选择指示器 */}
                {multiSelect && isBatchSelected && (
                  <span className='sf-agent-card-check'>✓</span>
                )}

                {/* 图标 */}
                <div className='sf-agent-card-icon' style={{ background: `${agent.color}1A` }}>
                  <AgentIcon icon={agent.icon} color={agent.color} size={28} />
                </div>

                {/* 主信息 */}
                <div className='sf-agent-card-main'>
                  <div className='sf-agent-card-name'>
                    {agent.name}
                    {agent.hidden && <span className='sf-agent-card-hidden-tag' title='已隐藏'>🚫</span>}
                  </div>
                  <div className='sf-agent-card-desc'>{agent.description}</div>
                  <div className='sf-agent-card-meta'>
                    {cat && (
                      <span className='sf-agent-card-cat' style={{ color: cat.color, background: `${cat.color}1A` }}>
                        {cat.label}
                      </span>
                    )}
                    <span className='sf-agent-card-stat' title='已启用工具数'>
                      ⚡ {agent.tools_mode === 'all' ? '全部工具' : `${agent.tools.length} 个工具`}
                    </span>
                    <span className='sf-agent-card-stat' title='已启用技能数'>
                      🎯 {agent.skills_mode === 'all' ? '全部技能' : `${agent.skills.length} 个技能`}
                    </span>
                    {agent.identity_mode === 'isolated' && (
                      <span className='sf-agent-card-stat' title='隔离身份'>🔒 隔离</span>
                    )}
                  </div>
                </div>

                {/* 类型徽章 */}
                <div className='sf-agent-card-type-badge'>
                  {agent.type === 'builtin' ? '内置' : agent.user_customized ? '自定义' : '商店'}
                </div>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
};

export default AgentListPanel;
