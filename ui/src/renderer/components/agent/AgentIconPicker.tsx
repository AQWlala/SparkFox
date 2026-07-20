/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentIconPicker — Agent 图标选择器
 *
 * 来源：OpenAkita AgentManagerView.tsx 图标选择器部分（清洁室重写为独立组件）
 *
 * 保留 OpenAkita 特性：
 * - 6 大图标分类（common/people/animal/object/nature/symbol/svg）
 * - 每类 16 个 emoji + svg 类 28 个 SVG 路径图标
 * - 分类切换 Tab + 网格布局
 * - 实时预览（当前选中图标大图显示）
 *
 * 改造点：
 * - 从 AgentManagerView 内联状态 → 独立组件，通过 props 受控
 * - Apple 风格：圆角 8px + 系统蓝 #007AFF + 浅灰背景 #F5F5F7
 */

import React from 'react';
import { AGENT_SVG_ICONS, AgentIcon } from './AgentIcon';

const SVG_ICON_KEYS = Object.keys(AGENT_SVG_ICONS);

/** 6 大图标分类（完整迁移自 OpenAkita） */
export const ICON_CATEGORIES: Record<string, { label: string; icons: string[] }> = {
  common: {
    label: '常用',
    icons: [
      '🤖', '🧠', '💡', '🎯', '📊', '🔍', '🛠️', '📝',
      '🌐', '🚀', '⚡', '🎨', '📚', '🔬', '💻', '🎵',
    ],
  },
  people: {
    label: '人物',
    icons: [
      '👩‍💻', '👨‍💻', '👩‍🔬', '👨‍🏫', '👩‍🎨', '🧑‍💼', '🕵️', '🦸',
      '🧙', '👷', '👩‍⚕️', '🧑‍🍳', '👨‍🚀', '🥷', '🧝', '🧑‍🎓',
    ],
  },
  animal: {
    label: '动物',
    icons: [
      '🐶', '🐱', '🦊', '🐼', '🐨', '🦁', '🐯', '🐸',
      '🦉', '🐙', '🦋', '🐝', '🐬', '🐺', '🦅', '🐢',
    ],
  },
  object: {
    label: '物品',
    icons: [
      '📱', '🖥️', '⌨️', '🎮', '📡', '🔭', '🧲', '⚙️',
      '🗂️', '📦', '🏷️', '🔐', '🗺️', '🧩', '🪄', '💎',
    ],
  },
  nature: {
    label: '自然',
    icons: [
      '🌸', '🌻', '🌈', '🔥', '❄️', '🌙', '⭐', '☀️',
      '🌊', '🍀', '🌲', '🌋', '💫', '🪐', '🌍', '🌪️',
    ],
  },
  symbol: {
    label: '符号',
    icons: [
      '♟️', '🎲', '🏆', '🎪', '🎭', '🧿', '💠', '⚜️',
      '☯️', '♾️', '🔱', '❇️', '✨', '💥', '🔶', '🔷',
    ],
  },
  svg: {
    label: '线性',
    icons: SVG_ICON_KEYS.map((k) => `svg:${k}`),
  },
};

interface AgentIconPickerProps {
  /** 当前选中图标 */
  value: string;
  /** 当前主题色（用于 SVG 预览） */
  color?: string;
  /** 选中回调 */
  onChange: (icon: string) => void;
  /** 关闭回调（点击外部或关闭按钮） */
  onClose?: () => void;
}

export const AgentIconPicker: React.FC<AgentIconPickerProps> = ({
  value,
  color = '#007AFF',
  onChange,
  onClose,
}) => {
  const [activeCat, setActiveCat] = React.useState<keyof typeof ICON_CATEGORIES>('common');

  return (
    <div className='sf-agent-icon-picker'>
      {/* 头部：当前预览 + 关闭按钮 */}
      <div className='sf-agent-icon-picker-header'>
        <div className='sf-agent-icon-picker-preview'>
          <AgentIcon icon={value} color={color} size={40} />
          <div className='sf-agent-icon-picker-preview-label'>
            <div className='sf-agent-icon-picker-preview-name'>
              {value.startsWith('svg:') ? AGENT_SVG_ICONS[value.slice(4)]?.label || 'SVG' : value}
            </div>
            <div className='sf-agent-icon-picker-preview-hint'>选择图标</div>
          </div>
        </div>
        {onClose && (
          <button
            type='button'
            className='sf-agent-icon-picker-close'
            onClick={onClose}
            aria-label='关闭'
          >
            ✕
          </button>
        )}
      </div>

      {/* 分类切换 Tab */}
      <div className='sf-agent-icon-picker-tabs'>
        {Object.entries(ICON_CATEGORIES).map(([key, cat]) => (
          <button
            key={key}
            type='button'
            className={`sf-agent-icon-picker-tab${activeCat === key ? ' active' : ''}`}
            onClick={() => setActiveCat(key as keyof typeof ICON_CATEGORIES)}
          >
            {cat.label}
          </button>
        ))}
      </div>

      {/* 图标网格 */}
      <div className='sf-agent-icon-picker-grid'>
        {ICON_CATEGORIES[activeCat].icons.map((icon) => (
          <button
            key={icon}
            type='button'
            className={`sf-agent-icon-picker-cell${value === icon ? ' selected' : ''}`}
            onClick={() => onChange(icon)}
            title={icon.startsWith('svg:') ? AGENT_SVG_ICONS[icon.slice(4)]?.label : icon}
          >
            <AgentIcon icon={icon} color={color} size={24} />
          </button>
        ))}
      </div>
    </div>
  );
};

export default AgentIconPicker;
