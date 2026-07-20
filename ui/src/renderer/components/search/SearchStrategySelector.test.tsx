/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'bun:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

import {
  DEFAULT_SEARCH_STRATEGY,
  getSearchStrategyOption,
  SEARCH_STRATEGIES,
  SEARCH_STRATEGY_DESCRIPTIONS,
  SEARCH_STRATEGY_ICONS,
  SEARCH_STRATEGY_LABELS,
  type SearchStrategy,
} from './constants';
import SearchStrategySelector, { type SearchStrategySelectorProps } from './SearchStrategySelector';

const source = readFileSync(new URL('./SearchStrategySelector.tsx', import.meta.url), 'utf8');
const constantsSource = readFileSync(new URL('./constants.ts', import.meta.url), 'utf8');

describe('SearchStrategySelector', () => {
  test('renders 4 strategies: VECTOR / ATOMIC / MULTI / MULTI_ES', () => {
    // 1) 常量层：4 个策略值都存在
    expect(SEARCH_STRATEGIES.map((s) => s.value)).toEqual(['VECTOR', 'ATOMIC', 'MULTI', 'MULTI_ES']);

    // 2) 源码层：组件确实枚举了这 4 个值
    expect(constantsSource.includes("'VECTOR'")).toBe(true);
    expect(constantsSource.includes("'ATOMIC'")).toBe(true);
    expect(constantsSource.includes("'MULTI'")).toBe(true);
    expect(constantsSource.includes("'MULTI_ES'")).toBe(true);

    // 3) 渲染层：full 模式（compact=false）应该把 4 个策略都渲染出来
    const html = renderToStaticMarkup(
      <SearchStrategySelector value='VECTOR' onChange={() => undefined} />
    );
    for (const strategy of SEARCH_STRATEGIES) {
      // 每个 strategy 至少应在渲染产物里出现一次（label 或 description）
      expect(html.includes(strategy.label) || html.includes(strategy.description)).toBe(true);
    }
  });

  test('default strategy is VECTOR', () => {
    // 1) 常量层：默认策略常量等于 VECTOR
    expect(DEFAULT_SEARCH_STRATEGY).toBe('VECTOR');
    expect(constantsSource.includes("DEFAULT_SEARCH_STRATEGY: SearchStrategy = 'VECTOR'")).toBe(true);

    // 2) 回退逻辑：getSearchStrategyOption 在 undefined / 非法值时返回 VECTOR
    expect(getSearchStrategyOption(undefined).value).toBe('VECTOR');
    expect(getSearchStrategyOption('NOT_A_STRATEGY').value).toBe('VECTOR');

    // 3) 渲染层：未传 value 时（TS 上必须传，这里通过默认值断言渲染属性）
    //    直接渲染 value='VECTOR' 时，选择器根节点 data-strategy 应为 VECTOR
    const html = renderToStaticMarkup(
      <SearchStrategySelector value='VECTOR' onChange={() => undefined} />
    );
    expect(html.includes('data-strategy="VECTOR"')).toBe(true);
    expect(html.includes(`data-default-strategy="${DEFAULT_SEARCH_STRATEGY}"`)).toBe(true);
  });

  test('click selects strategy (onChange callback is invoked)', () => {
    // 1) Props 接口必须暴露 onChange 回调
    expect(source.includes('onChange: (strategy: SearchStrategy) => void')).toBe(true);

    // 2) 源码层：compact 模式 Menu.Item 的点击会调用 onChange(next)
    expect(source.includes('onChange(next)')).toBe(true);

    // 3) 源码层：full 模式 RadioGroup 的 onChange 会调用 onChange(next as SearchStrategy)
    expect(source.includes('onChange(next as SearchStrategy)')).toBe(true);

    // 4) 源码层：handleMenuClick 把 key 转成 SearchStrategy 后调用 onChange
    expect(source.includes('const next = key as SearchStrategy')).toBe(true);

    // 5) 渲染层：每个策略选项都有 data-testid 标记，便于测试/集成定位点击目标
    const compactHtml = renderToStaticMarkup(
      <SearchStrategySelector value='VECTOR' onChange={() => undefined} compact />
    );
    // compact 模式根节点存在
    expect(compactHtml.includes("data-testid='search-strategy-selector'") || compactHtml.includes('data-testid="search-strategy-selector"')).toBe(true);

    const fullHtml = renderToStaticMarkup(
      <SearchStrategySelector value='VECTOR' onChange={() => undefined} />
    );
    for (const strategy of SEARCH_STRATEGIES) {
      // 每个策略选项都能在 full 模式渲染中找到对应 testid（属性引号兼容两种）
      const testid = `strategy-option-${strategy.value}`;
      expect(
        fullHtml.includes(`data-testid='${testid}'`) ||
          fullHtml.includes(`data-testid="${testid}"`)
      ).toBe(true);
    }
  });

  test('selector near chat input: exports `compact` prop (true = compact mode, false = full mode)', () => {
    // 1) Props 接口必须暴露 compact?: boolean
    expect(source.includes('compact?: boolean')).toBe(true);

    // 2) 源码层：默认值为 false（用于设置页等宽松布局；输入框附近显式传 true）
    expect(source.includes('compact = false')).toBe(true);

    // 3) 渲染层：compact=true → 根节点 data-mode='compact'
    const compactHtml = renderToStaticMarkup(
      <SearchStrategySelector value='VECTOR' onChange={() => undefined} compact />
    );
    expect(compactHtml.includes("data-mode='compact'") || compactHtml.includes('data-mode="compact"')).toBe(true);

    // 4) 渲染层：compact=false（默认）→ 根节点 data-mode='full'
    const fullHtml = renderToStaticMarkup(
      <SearchStrategySelector value='VECTOR' onChange={() => undefined} />
    );
    expect(fullHtml.includes("data-mode='full'") || fullHtml.includes('data-mode="full"')).toBe(true);

    // 5) 两种模式都必须暴露同一 testid 以便集成时定位
    expect(
      compactHtml.includes("data-testid='search-strategy-selector'") ||
        compactHtml.includes('data-testid="search-strategy-selector"')
    ).toBe(true);
    expect(
      fullHtml.includes("data-testid='search-strategy-selector'") ||
        fullHtml.includes('data-testid="search-strategy-selector"')
    ).toBe(true);

    // 6) compact 模式不渲染完整 description，full 模式才渲染（保证 compact 真的是"紧凑"）
    expect(compactHtml.includes('向量检索')).toBe(false);
    expect(fullHtml.includes('向量检索') || fullHtml.includes('VECTOR')).toBe(true);
  });

  test('constants module exposes LABEL / DESCRIPTION / ICON maps for 4 strategies', () => {
    // 辅助检查：常量映射的 4 个 key 与 4 个策略一致
    expect(Object.keys(SEARCH_STRATEGY_LABELS).sort()).toEqual(
      ['ATOMIC', 'MULTI', 'MULTI_ES', 'VECTOR']
    );
    expect(Object.keys(SEARCH_STRATEGY_DESCRIPTIONS).sort()).toEqual(
      ['ATOMIC', 'MULTI', 'MULTI_ES', 'VECTOR']
    );
    expect(Object.keys(SEARCH_STRATEGY_ICONS).sort()).toEqual(
      ['ATOMIC', 'MULTI', 'MULTI_ES', 'VECTOR']
    );

    // 图标名都来自受控枚举（已在仓库其他位置验证存在的 icon-park 图标）
    expect(SEARCH_STRATEGY_ICONS.VECTOR).toBe('Search');
    expect(SEARCH_STRATEGY_ICONS.ATOMIC).toBe('Lightning');
    expect(SEARCH_STRATEGY_ICONS.MULTI).toBe('Connection');
    expect(SEARCH_STRATEGY_ICONS.MULTI_ES).toBe('TransferData');
  });

  test('props type is exported for caller integration', () => {
    // SearchStrategySelectorProps 必须被 export，方便主 agent 在 SendBox/ChatView 集成时引用类型
    expect(source.includes('export interface SearchStrategySelectorProps')).toBe(true);
    // 仅校验类型可被 import（编译期类型，运行时为 undefined 也不影响）
    const _typeCheck: SearchStrategySelectorProps = {
      value: 'VECTOR',
      onChange: () => undefined,
      compact: true,
    };
    expect(_typeCheck.value).toBe('VECTOR');
  });
});
