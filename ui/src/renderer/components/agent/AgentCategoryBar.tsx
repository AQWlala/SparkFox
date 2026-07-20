/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentCategoryBar — Agent 分类栏
 *
 * 来源：OpenAkita AgentManagerView.tsx 顶部分类栏（清洁室重写为独立组件）
 *
 * 功能：
 * - 显示所有分类（全部 + 内置 + 自定义 + 用户创建）
 * - 切换激活分类（过滤 Agent 列表）
 * - 新增分类（弹出输入框）
 *
 * Apple 风格：胶囊形 Pill + 系统蓝激活态 + SF Pro 字体
 */

import React from 'react';
import { useAgentStore, type CategoryInfo } from '@renderer/store/agentStore';

interface AgentCategoryBarProps {
  /** 是否显示"新增分类"按钮 */
  showAddButton?: boolean;
}

export const AgentCategoryBar: React.FC<AgentCategoryBarProps> = ({ showAddButton = true }) => {
  const categories = useAgentStore((s) => s.categories);
  const activeCategory = useAgentStore((s) => s.activeCategory);
  const setActiveCategory = useAgentStore((s) => s.setActiveCategory);
  const addCategory = useAgentStore((s) => s.addCategory);

  const [adding, setAdding] = React.useState(false);
  const [newLabel, setNewLabel] = React.useState('');
  const [newColor, setNewColor] = React.useState('#007AFF');

  const handleAdd = async () => {
    if (!newLabel.trim()) {
      setAdding(false);
      return;
    }
    await addCategory(newLabel, newColor);
    setNewLabel('');
    setNewColor('#007AFF');
    setAdding(false);
  };

  return (
    <div className='sf-agent-category-bar'>
      {/* "全部" Pill */}
      <button
        type='button'
        className={`sf-agent-category-pill${activeCategory === '' ? ' active' : ''}`}
        onClick={() => setActiveCategory('')}
      >
        全部
      </button>

      {/* 各分类 Pill */}
      {categories.map((cat: CategoryInfo) => (
        <button
          key={cat.id}
          type='button'
          className={`sf-agent-category-pill${activeCategory === cat.id ? ' active' : ''}`}
          onClick={() => setActiveCategory(cat.id)}
          style={activeCategory === cat.id ? { background: cat.color, borderColor: cat.color } : { color: cat.color, borderColor: `${cat.color}55` }}
        >
          <span className='sf-agent-category-pill-dot' style={{ background: cat.color }} />
          {cat.label}
          <span className='sf-agent-category-pill-count'>{cat.agent_count}</span>
        </button>
      ))}

      {/* 新增分类 */}
      {showAddButton && !adding && (
        <button
          type='button'
          className='sf-agent-category-pill sf-agent-category-add'
          onClick={() => setAdding(true)}
          title='新增分类'
        >
          +
        </button>
      )}

      {adding && (
        <div className='sf-agent-category-add-form'>
          <input
            type='text'
            placeholder='分类名称'
            value={newLabel}
            onChange={(e) => setNewLabel(e.target.value)}
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleAdd();
              if (e.key === 'Escape') setAdding(false);
            }}
          />
          <input
            type='color'
            value={newColor}
            onChange={(e) => setNewColor(e.target.value)}
            title='选择颜色'
          />
          <button type='button' onClick={handleAdd}>确定</button>
          <button type='button' onClick={() => setAdding(false)}>取消</button>
        </div>
      )}
    </div>
  );
};

export default AgentCategoryBar;
