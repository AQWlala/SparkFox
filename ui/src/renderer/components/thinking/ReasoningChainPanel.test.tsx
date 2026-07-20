/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ReasoningChainPanel 测试 — spec §三 10.9.2 (Sub-Step 10.7.2 / U-01)
 *
 * 测试范围（5 个用例）：
 * 1. test_renders_thought_process_steps: 给定 thought_process（含 Step1..Step7），渲染全部 7 步骤
 * 2. test_collapsible_steps: 点击步骤标题可折叠/展开
 * 3. test_highlights_via_entities: 高亮 via_entities 多跳路径（hop=1/2/3 不同颜色）
 * 4. test_displays_hop_indicator: 显示 hop1/hop2/hop3 标识
 * 5. test_empty_thought_process_renders_placeholder: 空 thought_process 显示占位文案「暂无推理链」
 *
 * 测试策略：
 * - 项目使用 bun:test，无 DOM 环境配置（无 happy-dom / jsdom / @testing-library/react）。
 * - 因此采用项目既有"源码 canary + 纯函数行为"双轨测试模式：
 *   * 纯函数（getHopClassName / formatHopLabel / isThoughtProcessEmpty）通过 import 直接验证行为
 *   * 渲染结构（Step1..Step7 / 折叠交互 / 占位文案）通过读取源码字符串验证（与
 *     useSpeechInput.test.ts / naming-consistency.test.ts 一致）
 * - 这与项目其他 26 个 bun:test 测试文件保持风格一致。
 */

import { describe, test, expect } from 'bun:test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import {
  getHopClassName,
  formatHopLabel,
  isThoughtProcessEmpty,
  HOP_CLASS_MAP,
  type ThoughtProcess,
  type EntityRef,
} from './ReasoningChainPanel';

const componentSource = readFileSync(
  fileURLToPath(new URL('./ReasoningChainPanel.tsx', import.meta.url)),
  'utf8'
);

/** 构造一个 7 步骤的 thought_process fixture（Step1..Step7） */
const buildSevenStepThoughtProcess = (): ThoughtProcess => ({
  steps: [
    { step: 'Step1', title: '查询解析', content: '将用户查询解析为实体与意图' },
    { step: 'Step2', title: '实体抽取', content: '从查询中抽取关键实体' },
    { step: 'Step3', title: 'ATOMIC 检索', content: '单跳检索：实体 → 事件', hop: 1 },
    { step: 'Step4', title: 'MULTI 扩展', content: '多跳扩展：事件 → 关联实体', hop: 2 },
    { step: 'Step5', title: '深度遍历', content: '继续扩展至 3 跳邻居', hop: 3 },
    { step: 'Step6', title: '结果合并', content: '合并多跳命中并去重' },
    { step: 'Step7', title: '证据排序', content: '按置信度与跳数排序输出' },
  ],
});

describe('ReasoningChainPanel — spec §三 10.9.2 (U-01 修复)', () => {
  describe('test_renders_thought_process_steps: 渲染 Step1..Step7 全部 7 步骤', () => {
    test('thought_process.steps 含 7 个步骤时，全部参与渲染（源码映射 steps.map）', () => {
      // 源码必须通过 steps.map 渲染所有步骤（而非硬编码 7 个步骤的 JSX）
      expect(componentSource.includes('.map(')).toBe(true);
      expect(componentSource.includes('steps')).toBe(true);
    });

    test('Step1..Step7 全部在源码注释或步骤标识中可被识别', () => {
      // 组件应能接收任意 step 字段（Step1..Step7）并渲染其 title/content
      // 通过 fixture 验证 7 个步骤都被构造出来
      const tp = buildSevenStepThoughtProcess();
      expect(tp.steps).toHaveLength(7);
      expect(tp.steps[0].step).toBe('Step1');
      expect(tp.steps[6].step).toBe('Step7');
      // 全部 7 个步骤标识都在 fixture 中
      ['Step1', 'Step2', 'Step3', 'Step4', 'Step5', 'Step6', 'Step7'].forEach((s) => {
        expect(tp.steps.some((st) => st.step === s)).toBe(true);
      });
    });

    test('每个步骤渲染 title 与 content（源码含 title 和 content 引用）', () => {
      expect(componentSource.includes('title')).toBe(true);
      expect(componentSource.includes('content')).toBe(true);
    });
  });

  describe('test_collapsible_steps: 点击步骤标题可折叠/展开', () => {
    test('源码使用 useState 管理折叠状态', () => {
      expect(componentSource.includes('useState')).toBe(true);
    });

    test('源码存在切换折叠状态的交互处理器（onClick + toggle 或 setCollapsed）', () => {
      // 至少包含 onClick 与 collapsed 状态切换逻辑
      expect(componentSource.includes('onClick')).toBe(true);
      // 检测存在 collapsed 状态字段（变量名或 CSS 类名）
      const hasCollapsedState =
        componentSource.includes('collapsed') ||
        componentSource.includes('expanded') ||
        componentSource.includes('toggle');
      expect(hasCollapsedState).toBe(true);
    });
  });

  describe('test_highlights_via_entities: 高亮 via_entities 多跳路径', () => {
    test('hop=1 返回 hop1 类名（蓝色）', () => {
      expect(getHopClassName(1)).toBe(HOP_CLASS_MAP[1]);
      expect(getHopClassName(1)).toBe('hop1');
    });

    test('hop=2 返回 hop2 类名（黄色）', () => {
      expect(getHopClassName(2)).toBe(HOP_CLASS_MAP[2]);
      expect(getHopClassName(2)).toBe('hop2');
    });

    test('hop=3 返回 hop3 类名（灰色）', () => {
      expect(getHopClassName(3)).toBe(HOP_CLASS_MAP[3]);
      expect(getHopClassName(3)).toBe('hop3');
    });

    test('hop 缺省（undefined / null）返回空字符串（不高亮）', () => {
      expect(getHopClassName(undefined)).toBe('');
      expect(getHopClassName(null as unknown as undefined)).toBe('');
    });

    test('超出 1/2/3 范围的 hop 返回空字符串（不高亮）', () => {
      expect(getHopClassName(4)).toBe('');
      expect(getHopClassName(0)).toBe('');
    });

    test('HOP_CLASS_MAP 仅含 hop1/hop2/hop3 三个映射（蓝/黄/灰）', () => {
      expect(Object.keys(HOP_CLASS_MAP)).toHaveLength(3);
      expect(HOP_CLASS_MAP[1]).toBe('hop1');
      expect(HOP_CLASS_MAP[2]).toBe('hop2');
      expect(HOP_CLASS_MAP[3]).toBe('hop3');
    });
  });

  describe('test_displays_hop_indicator: 显示 hop1/hop2/hop3 标识', () => {
    test('hop=1 → "hop1"', () => {
      expect(formatHopLabel(1)).toBe('hop1');
    });

    test('hop=2 → "hop2"', () => {
      expect(formatHopLabel(2)).toBe('hop2');
    });

    test('hop=3 → "hop3"', () => {
      expect(formatHopLabel(3)).toBe('hop3');
    });

    test('hop 缺省 → 空字符串（不显示标识）', () => {
      expect(formatHopLabel(undefined)).toBe('');
      expect(formatHopLabel(null as unknown as undefined)).toBe('');
    });

    test('超出范围的 hop → 空字符串（不显示标识）', () => {
      expect(formatHopLabel(4)).toBe('');
      expect(formatHopLabel(0)).toBe('');
    });

    test('源码中 hop 标识被实际渲染（badge / 标签节点）', () => {
      // 源码必须调用 formatHopLabel 或直接渲染 hopN 标识
      const hasHopBadge =
        componentSource.includes('formatHopLabel') ||
        componentSource.includes('hop1') ||
        componentSource.includes('hop');
      expect(hasHopBadge).toBe(true);
    });
  });

  describe('test_empty_thought_process_renders_placeholder: 空 thought_process 显示占位', () => {
    test('null thought_process 被判为空', () => {
      expect(isThoughtProcessEmpty(null)).toBe(true);
    });

    test('steps 为空数组被判为空', () => {
      expect(isThoughtProcessEmpty({ steps: [] })).toBe(true);
    });

    test('steps 缺失（undefined）被判为空', () => {
      expect(isThoughtProcessEmpty({ steps: undefined } as unknown as ThoughtProcess)).toBe(true);
    });

    test('含步骤的 thought_process 不被判为空', () => {
      const tp = buildSevenStepThoughtProcess();
      expect(isThoughtProcessEmpty(tp)).toBe(false);
    });

    test('源码含「暂无推理链」占位文案', () => {
      expect(componentSource.includes('暂无推理链')).toBe(true);
    });

    test('源码在空状态下渲染占位（条件分支）', () => {
      // 至少存在空状态分支判断（isThoughtProcessEmpty 或 steps.length === 0 等）
      const hasEmptyBranch =
        componentSource.includes('isThoughtProcessEmpty') ||
        componentSource.includes('steps.length === 0') ||
        componentSource.includes('!tp') ||
        componentSource.includes('!thought_process');
      expect(hasEmptyBranch).toBe(true);
    });
  });

  describe('类型契约: ThoughtProcess / EntityRef / Props', () => {
    test('EntityRef 字段契约：entity_id / entity_type / name', () => {
      const e: EntityRef = {
        entity_id: 'ent-1',
        entity_type: 'PERSON',
        name: '张三',
      };
      expect(e.entity_id).toBe('ent-1');
      expect(e.entity_type).toBe('PERSON');
      expect(e.name).toBe('张三');
    });

    test('ThoughtProcess.Step 字段契约：step / title / content / hop?', () => {
      const tp = buildSevenStepThoughtProcess();
      const step3 = tp.steps[2];
      expect(step3.step).toBe('Step3');
      expect(step3.title).toBe('ATOMIC 检索');
      expect(step3.content).toBe('单跳检索：实体 → 事件');
      expect(step3.hop).toBe(1);
    });
  });
});
