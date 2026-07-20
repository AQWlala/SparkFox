/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

import { Connection, Down, Lightning, Search, TransferData } from '@icon-park/react';
import { Dropdown, Menu, Radio } from '@arco-design/web-react';
import type { IIconProps } from '@icon-park/react/es/runtime';
import React, { useCallback, useMemo, useState } from 'react';

import {
  DEFAULT_SEARCH_STRATEGY,
  getSearchStrategyOption,
  SEARCH_STRATEGIES,
  type SearchStrategy,
  type SearchStrategyIconName,
} from './constants';
import styles from './SearchStrategySelector.module.css';

/**
 * icon-park 图标组件映射表 / icon-park icon component map
 *
 * 将常量中的图标名（字符串）映射到实际导入的图标组件，
 * 保证只引入已验证存在的图标。
 */
const ICON_COMPONENTS: Record<SearchStrategyIconName, React.FC<IIconProps>> = {
  Search,
  Lightning,
  Connection,
  TransferData,
};

const RadioGroup = Radio.Group;

/**
 * SearchStrategySelector 组件属性 / SearchStrategySelector props
 */
export interface SearchStrategySelectorProps {
  /** 当前选中的检索策略 / Currently selected search strategy */
  value: SearchStrategy;
  /** 切换策略时触发 / Fired when user selects a different strategy */
  onChange: (strategy: SearchStrategy) => void;
  /**
   * 紧凑模式 / Compact mode
   *
   * - `true`  渲染为小图标下拉 pill，紧贴聊天输入框（U-06a 修复：策略选择器远离输入框问题）
   * - `false` (默认) 渲染为完整的 RadioGroup，每个策略含 label + description + icon
   */
  compact?: boolean;
  /** 额外类名 / Additional className */
  className?: string;
  /** 是否禁用 / Disable the selector */
  disabled?: boolean;
}

/**
 * SearchStrategySelector — 检索策略选择器
 *
 * spec §三 10.12.1 / sub-step 10.10.1
 *
 * 提供 4 种检索策略切换：
 *   - VECTOR    向量检索（默认）
 *   - ATOMIC    原子事件检索
 *   - MULTI     多跳检索
 *   - MULTI_ES  多跳 + ES（v2.0.0 计划，前端预留）
 *
 * 两种渲染模式：
 *   - `compact=true`  小图标下拉，紧贴 SendBox 输入框渲染，缩短操作路径（U-06a 修复）
 *   - `compact=false` 完整 RadioGroup，每项含 label + description + icon，用于设置页等宽松布局
 *
 * 集成由主 agent 在第十波验收时统一完成（不修改 SendBox / ChatView）。
 */
const SearchStrategySelector: React.FC<SearchStrategySelectorProps> = ({
  value,
  onChange,
  compact = false,
  className,
  disabled = false,
}) => {
  const [dropdownVisible, setDropdownVisible] = useState(false);

  // 当前选中的策略 option；非法 value 回退到 VECTOR（默认策略）
  const currentOption = useMemo(() => getSearchStrategyOption(value), [value]);
  const CurrentIcon = ICON_COMPONENTS[currentOption.icon];

  const handleMenuClick = useCallback(
    (key: string) => {
      const next = key as SearchStrategy;
      if (next === value) {
        setDropdownVisible(false);
        return;
      }
      onChange(next);
      setDropdownVisible(false);
    },
    [onChange, value]
  );

  const menu = (
    <Menu
      onClickMenuItem={(key) => {
        handleMenuClick(key);
      }}
    >
      <Menu.ItemGroup title='检索策略'>
        {SEARCH_STRATEGIES.map((option) => {
          const Icon = ICON_COMPONENTS[option.icon];
          const isActive = option.value === value;
          return (
            <Menu.Item key={option.value} disabled={disabled}>
              <div
                className={`${styles.menuItem} ${isActive ? styles.menuItemActive : ''}`}
                data-testid={`strategy-option-${option.value}`}
                data-strategy={option.value}
                data-active={isActive ? 'true' : 'false'}
              >
                <span className={styles.menuItemIcon}>
                  <Icon size='14' />
                </span>
                <span className={styles.menuItemText}>
                  <span className={styles.menuItemLabel}>{option.label}</span>
                  <span className={styles.menuItemDesc}>{option.description}</span>
                </span>
                {isActive && <span className={styles.menuItemCheck}>✓</span>}
                {option.upcoming && <span className={styles.menuItemBadge}>v2.0</span>}
              </div>
            </Menu.Item>
          );
        })}
      </Menu.ItemGroup>
    </Menu>
  );

  // ─── 紧凑模式：小图标下拉 pill，紧贴输入框 ───
  if (compact) {
    return (
      <Dropdown
        trigger='click'
        popupVisible={dropdownVisible}
        onVisibleChange={(visible) => {
          if (!disabled) setDropdownVisible(visible);
        }}
        droplist={menu}
        disabled={disabled}
      >
        <button
          type='button'
          className={`${styles.compactTrigger} ${className ?? ''}`}
          data-testid='search-strategy-selector'
          data-mode='compact'
          data-strategy={value}
          data-default-strategy={DEFAULT_SEARCH_STRATEGY}
          disabled={disabled}
          aria-label={`检索策略：${currentOption.label}`}
        >
          <span className={styles.compactIcon}>
            <CurrentIcon size='14' />
          </span>
          <span className={styles.compactLabel}>{currentOption.label}</span>
          <Down size={10} className={styles.compactArrow} />
        </button>
      </Dropdown>
    );
  }

  // ─── 完整模式：RadioGroup，每项含 icon + label + description ───
  return (
    <div
      className={`${styles.fullWrapper} ${className ?? ''}`}
      data-testid='search-strategy-selector'
      data-mode='full'
      data-strategy={value}
      data-default-strategy={DEFAULT_SEARCH_STRATEGY}
    >
      <RadioGroup
        type='button'
        size='small'
        value={value}
        disabled={disabled}
        onChange={(next) => {
          onChange(next as SearchStrategy);
        }}
        className={styles.fullGroup}
      >
        {SEARCH_STRATEGIES.map((option) => {
          const Icon = ICON_COMPONENTS[option.icon];
          return (
            <Radio key={option.value} value={option.value}>
              <span
                className={styles.fullOption}
                data-testid={`strategy-option-${option.value}`}
                data-strategy={option.value}
              >
                <span className={styles.fullOptionIcon}>
                  <Icon size='14' />
                </span>
                <span className={styles.fullOptionLabel}>{option.label}</span>
                <span className={styles.fullOptionDesc}>{option.description}</span>
                {option.upcoming && <span className={styles.fullOptionBadge}>v2.0</span>}
              </span>
            </Radio>
          );
        })}
      </RadioGroup>
    </div>
  );
};

export default SearchStrategySelector;
