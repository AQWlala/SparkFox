/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * 检索策略常量定义 / Search strategy constants
 *
 * 用于 SearchStrategySelector 组件（spec §三 10.12.1 / sub-step 10.10.1）
 * 对应后端 `sparkfox-knowledge` crate 已实现的 4 种检索策略：
 *   - VECTOR    向量检索（默认）
 *   - ATOMIC    原子事件检索（10.5.1 + 10.5.2）
 *   - MULTI     多跳检索（10.8.2）
 *   - MULTI_ES  多跳 + ES（v2.0.0 计划，前端预留选项）
 *
 * U-06a 修复：策略选择器远离输入框问题
 *   旧版本将策略选择器放在远离输入框的位置，用户切换策略时需要移动视线/鼠标，
 *   影响对话流畅度。本组件提供 `compact` 模式，可紧贴输入框渲染，缩短操作路径。
 */

/**
 * 检索策略类型 / Search strategy type
 *
 * 与后端 `SearchStrategy` 枚举字符串保持一致，作为前后端契约的字面量联合类型。
 */
export type SearchStrategy = 'VECTOR' | 'ATOMIC' | 'MULTI' | 'MULTI_ES';

/**
 * 策略对应的 icon-park 图标名 / icon-park icon name for each strategy
 *
 * 仅取在 `@icon-park/react` 中已确认存在且已在仓库其他位置使用过的图标，
 * 避免引入未验证的图标导致打包失败。
 */
export type SearchStrategyIconName = 'Search' | 'Lightning' | 'Connection' | 'TransferData';

/**
 * 单个策略选项的描述结构 / Single strategy option descriptor
 */
export interface SearchStrategyOption {
  value: SearchStrategy;
  label: string;
  description: string;
  icon: SearchStrategyIconName;
  /** 是否为 v2.0.0 计划功能（前端预留，后端可能尚未实现） */
  upcoming?: boolean;
}

/**
 * 4 种检索策略的有序列表 / Ordered list of 4 search strategies
 *
 * 顺序决定了下拉/RadioGroup 中的渲染顺序；VECTOR 作为默认策略放在首位。
 */
export const SEARCH_STRATEGIES: readonly SearchStrategyOption[] = [
  {
    value: 'VECTOR',
    label: 'VECTOR',
    description: '向量检索',
    icon: 'Search',
  },
  {
    value: 'ATOMIC',
    label: 'ATOMIC',
    description: '原子事件检索',
    icon: 'Lightning',
  },
  {
    value: 'MULTI',
    label: 'MULTI',
    description: '多跳检索',
    icon: 'Connection',
  },
  {
    value: 'MULTI_ES',
    label: 'MULTI_ES',
    description: '多跳 + ES（v2.0.0）',
    icon: 'TransferData',
    upcoming: true,
  },
] as const;

/**
 * 默认检索策略 / Default search strategy
 *
 * 当调用方未提供 `value` 或传入非法值时使用；与后端 VECTOR 默认一致。
 */
export const DEFAULT_SEARCH_STRATEGY: SearchStrategy = 'VECTOR';

/**
 * 策略 → 中文标签映射 / Strategy → label map
 */
export const SEARCH_STRATEGY_LABELS: Record<SearchStrategy, string> = {
  VECTOR: '向量',
  ATOMIC: '原子',
  MULTI: '多跳',
  MULTI_ES: '多跳+ES',
};

/**
 * 策略 → 完整描述映射 / Strategy → full description map
 */
export const SEARCH_STRATEGY_DESCRIPTIONS: Record<SearchStrategy, string> = {
  VECTOR: '基于向量相似度的语义检索，适合自然语言提问。',
  ATOMIC: '基于原子事件的精确检索，适合结构化事实查询。',
  MULTI: '多跳图检索，跨越多个事件节点的复杂推理。',
  MULTI_ES: '多跳 + ElasticSearch 全文混合检索（v2.0.0 计划）。',
};

/**
 * 策略 → 图标名映射 / Strategy → icon name map
 */
export const SEARCH_STRATEGY_ICONS: Record<SearchStrategy, SearchStrategyIconName> = {
  VECTOR: 'Search',
  ATOMIC: 'Lightning',
  MULTI: 'Connection',
  MULTI_ES: 'TransferData',
};

/**
 * 根据 value 查找策略选项 / Look up strategy option by value
 *
 * 找不到时回退到 VECTOR（默认策略），保证调用方始终拿到一个可用 option。
 */
export function getSearchStrategyOption(value: SearchStrategy | string | undefined): SearchStrategyOption {
  if (!value) return SEARCH_STRATEGIES[0];
  const found = SEARCH_STRATEGIES.find((option) => option.value === value);
  return found ?? SEARCH_STRATEGIES[0];
}
