/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * ExtractionProgressCard 测试 — U-05 修复：SAG 5 状态机联动
 *
 * 测试策略：与项目既有约定一致，采用「源码扫描」模式（参考 knowledgeCardLayout.test.ts、
 * knowledgeConsumersSectionVisual.test.ts、knowledgeMountHintVisual.test.ts），
 * 通过 readFileSync 读取组件源码并断言关键契约。这样无需引入 React testing-library
 * 即可验证组件结构与状态机联动行为。
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const componentSource = readFileSync(new URL('./ExtractionProgressCard.tsx', import.meta.url), 'utf8');
const constantsSource = readFileSync(new URL('./constants.ts', import.meta.url), 'utf8');
const hookSource = readFileSync(new URL('./useExtractionStatus.ts', import.meta.url), 'utf8');

describe('ExtractionProgressCard — U-05 修复：SAG 5 状态机联动', () => {
  test('test_renders_5_states: 渲染 PENDING/PARSING/PARSED/EXTRACTING/COMPLETED 5 状态', () => {
    // constants.ts 必须定义全部 5 个状态字面量
    expect(constantsSource.includes("'PENDING'")).toBe(true);
    expect(constantsSource.includes("'PARSING'")).toBe(true);
    expect(constantsSource.includes("'PARSED'")).toBe(true);
    expect(constantsSource.includes("'EXTRACTING'")).toBe(true);
    expect(constantsSource.includes("'COMPLETED'")).toBe(true);

    // 组件必须处理全部 5 个状态（在 STATUS_ICON 映射或渲染分支中出现）
    expect(componentSource.includes('PENDING')).toBe(true);
    expect(componentSource.includes('PARSING')).toBe(true);
    expect(componentSource.includes('PARSED')).toBe(true);
    expect(componentSource.includes('EXTRACTING')).toBe(true);
    expect(componentSource.includes('COMPLETED')).toBe(true);

    // 组件必须为每个状态准备图标（STATUS_ICON 映射）
    expect(componentSource.includes('STATUS_ICON')).toBe(true);
  });

  test('test_progress_bar_linked_to_state: 进度条与状态机联动（PENDING=10% / PARSING=30% / PARSED=50% / EXTRACTING=80% / COMPLETED=100%）', () => {
    // constants.ts 必须定义状态 → 进度百分比映射
    expect(constantsSource.includes('EXTRACTION_STATUS_PROGRESS')).toBe(true);
    expect(constantsSource.includes('PENDING: 10')).toBe(true);
    expect(constantsSource.includes('PARSING: 30')).toBe(true);
    expect(constantsSource.includes('PARSED: 50')).toBe(true);
    expect(constantsSource.includes('EXTRACTING: 80')).toBe(true);
    expect(constantsSource.includes('COMPLETED: 100')).toBe(true);

    // hook 必须返回 progress 字段
    expect(hookSource.includes('progress')).toBe(true);

    // 组件使用 Arco Progress 组件渲染进度条，且 percent 来自 hook（非写死）
    expect(componentSource.includes('Progress')).toBe(true);
    expect(componentSource.includes('percent={progress}')).toBe(true);
  });

  test('test_extracting_shows_event_entity_count: EXTRACTING 阶段显示已抽取 event_count + entity_count', () => {
    // 组件 props 必须接收 event_count 和 entity_count
    expect(componentSource.includes('event_count')).toBe(true);
    expect(componentSource.includes('entity_count')).toBe(true);

    // 组件必须在 EXTRACTING 状态分支中渲染这两个计数（用正则跨行匹配）
    const extractingMatch = componentSource.match(/EXTRACTING[\s\S]{0,800}?event_count[\s\S]{0,400}?entity_count/);
    expect(extractingMatch).not.toBeNull();

    // 必须包含中文展示文案「事件」「实体」
    expect(componentSource.includes('事件')).toBe(true);
    expect(componentSource.includes('实体')).toBe(true);
  });

  test('test_state_transition_correct: PENDING → PARSING → PARSED → EXTRACTING → COMPLETED 转换顺序正确', () => {
    // constants.ts 必须定义 EXTRACTION_STATUS_ORDER 数组保证状态机顺序
    expect(constantsSource.includes('EXTRACTION_STATUS_ORDER')).toBe(true);

    // 5 个状态字面量在 constants.ts 中按 PENDING → PARSING → PARSED → EXTRACTING → COMPLETED 顺序出现
    const pendingIdx = constantsSource.indexOf("'PENDING'");
    const parsingIdx = constantsSource.indexOf("'PARSING'");
    const parsedIdx = constantsSource.indexOf("'PARSED'");
    const extractingIdx = constantsSource.indexOf("'EXTRACTING'");
    const completedIdx = constantsSource.indexOf("'COMPLETED'");

    expect(pendingIdx).toBeGreaterThan(-1);
    expect(parsingIdx).toBeGreaterThan(pendingIdx);
    expect(parsedIdx).toBeGreaterThan(parsingIdx);
    expect(extractingIdx).toBeGreaterThan(parsedIdx);
    expect(completedIdx).toBeGreaterThan(extractingIdx);
  });

  test('test_knowledge_detail_page_embeds_card: 组件可被 KnowledgeDetailPage 嵌入（导出 default + named）', () => {
    // 组件必须同时导出 default 和 named，方便 KnowledgeDetailPage 用任一方式引入
    expect(componentSource.includes('export default')).toBe(true);
    expect(componentSource.match(/export\s*\{\s*[^}]*ExtractionProgressCard[^}]*\}/)).not.toBeNull();

    // 组件必须有明确的 Props 类型定义，便于 KnowledgeDetailPage 嵌入时获得类型提示
    expect(componentSource.includes('ExtractionProgressCardProps')).toBe(true);

    // ExtractionStatus 类型从 constants.ts 导出，KnowledgeDetailPage 也可复用
    expect(constantsSource.includes('ExtractionStatus')).toBe(true);
    expect(constantsSource.match(/export\s+type\s+ExtractionStatus/)).not.toBeNull();
  });
});
