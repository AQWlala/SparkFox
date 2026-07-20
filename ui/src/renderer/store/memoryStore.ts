/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox memoryStore — 记忆状态管理（三层架构 → 6 层映射）
 *
 * 来源：OpenAkita memory/{unified_store,retrieval,extractor,retention,types}
 *       （清洁室重写为 TypeScript Zustand store，PoC mock 数据）
 *
 * 三层架构（OpenAkita 原型）：
 * - SemanticMemory（语义记忆，实体-属性结构）
 * - Episode（情节记忆，完整交互故事）
 * - Scratchpad（工作记忆草稿本，跨 session）
 *
 * 映射到 SparkFox 6 层（Pangu Nebula 蓝图）：
 * - L0 工作记忆（Scratchpad）— 跨 session 草稿本
 * - L1 短期记忆 — 当前会话上下文片段
 * - L2 情节记忆（Episode）— 完整交互故事
 * - L3 语义记忆（SemanticMemory）— 实体-属性结构
 * - L4 程序记忆 — 技能/规则/经验
 * - L5 元认知 — 自我反思/价值判断
 *
 * PoC：所有数据为内置 mock，无后端依赖
 */

import { create } from 'zustand';
// Tauri 2 IPC：memory_list / memory_put 失败时静默回退到 mock 数据
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** 6 层记忆标识 */
export type MemoryLayer = 'L0' | 'L1' | 'L2' | 'L3' | 'L4' | 'L5';

/** 记忆类型（参考 OpenAkita MemoryType，扩展 L4/L5 类别） */
export type MemoryType =
  | 'fact' // 事实
  | 'preference' // 偏好
  | 'skill' // 技能
  | 'rule' // 规则
  | 'error' // 错误
  | 'experience' // 经验
  | 'persona_trait' // 人格特质
  | 'context' // 上下文
  | 'episode' // 情节（L2）
  | 'scratch' // 草稿（L0）
  | 'reflection'; // 反思（L5）

/** 优先级（决定保留时长） */
export type MemoryPriority = 'transient' | 'short_term' | 'long_term' | 'permanent';

/** 作用域 */
export type MemoryScope = 'global' | 'agent' | 'session';

/** 单条记忆 — 综合 OpenAkita SemanticMemory + Episode + Scratchpad */
export interface MemoryItem {
  id: string;
  layer: MemoryLayer;
  type: MemoryType;
  priority: MemoryPriority;
  scope: MemoryScope;
  content: string;
  /** 来源：对话/工具/系统/手动 */
  source: string;
  /** 实体-属性结构（L3 语义记忆） */
  subject?: string;
  predicate?: string;
  object?: string;
  tags: string[];
  importance_score: number; // 0-1
  confidence: number; // 0-1
  access_count: number;
  created_at: string | null; // ISO-8601
  updated_at: string | null;
  last_accessed_at: string | null;
  expires_at: string | null;
  /** L2 情节记忆：关联 Agent / 工具 / 对话 */
  related_agents?: string[];
  related_tools?: string[];
  conversation_id?: string;
  /** L5 元认知：是否被自我反思修改 */
  reflected?: boolean;
}

/** 统计信息 */
export interface MemoryStats {
  total: number;
  by_layer: Record<MemoryLayer, number>;
  by_type: Record<string, number>;
  avg_importance: number;
  avg_confidence: number;
  total_access: number;
  /** 最近活跃（最近 7 天访问过的） */
  recent_active: number;
  /** 即将过期（7 天内） */
  expiring_soon: number;
}

/** 排序选项 */
export type SortField =
  | 'importance_score'
  | 'created_at'
  | 'updated_at'
  | 'last_accessed_at'
  | 'access_count';

export type SortOrder = 'asc' | 'desc';

/** 图谱节点（用于可视化） */
export interface MemoryGraphNode {
  id: string;
  label: string;
  layer: MemoryLayer;
  type: MemoryType;
  importance: number;
  /** 关联节点 ID 列表 */
  edges: string[];
}

// ============================================================================
// 常量
// ============================================================================

export const LAYER_LABELS: Record<MemoryLayer, string> = {
  L0: 'L0 · 工作记忆',
  L1: 'L1 · 短期记忆',
  L2: 'L2 · 情节记忆',
  L3: 'L3 · 语义记忆',
  L4: 'L4 · 程序记忆',
  L5: 'L5 · 元认知',
};

export const LAYER_DESCRIPTIONS: Record<MemoryLayer, string> = {
  L0: '跨 session 草稿本，临时变量',
  L1: '当前会话上下文片段',
  L2: '完整交互故事，时间线',
  L3: '实体-属性结构化知识',
  L4: '可复用技能、规则、经验',
  L5: '自我反思、价值判断、目标校准',
};

export const LAYER_COLORS: Record<MemoryLayer, string> = {
  L0: '#8E8E93', // 系统灰
  L1: '#5856D6', // 系统紫
  L2: '#FF9500', // 系统橙
  L3: '#007AFF', // 系统蓝
  L4: '#34C759', // 系统绿
  L5: '#FF2D55', // 系统粉红
};

export const TYPE_LABELS: Record<MemoryType, string> = {
  fact: '事实',
  preference: '偏好',
  skill: '技能',
  rule: '规则',
  error: '错误',
  experience: '经验',
  persona_trait: '人格',
  context: '上下文',
  episode: '情节',
  scratch: '草稿',
  reflection: '反思',
};

export const TYPE_COLORS: Record<MemoryType, string> = {
  fact: '#007AFF',
  preference: '#5856D6',
  skill: '#34C759',
  rule: '#FF9500',
  error: '#FF3B30',
  experience: '#5AC8FA',
  persona_trait: '#FF2D55',
  context: '#8E8E93',
  episode: '#AF52DE',
  scratch: '#AEAEB2',
  reflection: '#FF6482',
};

export const PRIORITY_LABELS: Record<MemoryPriority, string> = {
  transient: '瞬时',
  short_term: '短期',
  long_term: '长期',
  permanent: '永久',
};

export const SCOPE_LABELS: Record<MemoryScope, string> = {
  global: '全局',
  agent: 'Agent',
  session: '会话',
};

/** 默认排序 */
export const DEFAULT_SORT: { field: SortField; order: SortOrder } = {
  field: 'importance_score',
  order: 'desc',
};

/** PoC：TTL 映射（参考 OpenAkita retention.py） */
export const PRIORITY_TTL_DAYS: Record<MemoryPriority, number | null> = {
  transient: 1,
  short_term: 3,
  long_term: 30,
  permanent: null,
};

// ============================================================================
// Mock 数据生成器
// ============================================================================

const now = new Date();
const isoDaysAgo = (days: number) =>
  new Date(now.getTime() - days * 86400000).toISOString();
const isoHoursAgo = (hours: number) =>
  new Date(now.getTime() - hours * 3600000).toISOString();
const isoMinutesAgo = (minutes: number) =>
  new Date(now.getTime() - minutes * 60000).toISOString();

const _id = (prefix: string, n: number) => `${prefix}_${String(n).padStart(4, '0')}`;

/** Mock：L0 工作记忆（草稿本） */
const mockL0: MemoryItem[] = [
  {
    id: _id('L0', 1),
    layer: 'L0',
    type: 'scratch',
    priority: 'transient',
    scope: 'session',
    content: '当前任务：分析 SparkFox 监视面板的 Token 用量异常',
    source: '对话',
    tags: ['当前任务', '监视面板'],
    importance_score: 0.4,
    confidence: 0.9,
    access_count: 3,
    created_at: isoMinutesAgo(15),
    updated_at: isoMinutesAgo(2),
    last_accessed_at: isoMinutesAgo(2),
    expires_at: isoHoursAgo(-2), // 2 小时后过期
  },
  {
    id: _id('L0', 2),
    layer: 'L0',
    type: 'scratch',
    priority: 'transient',
    scope: 'session',
    content: '用户参考的文件：D:/xin kaifa/SparkFox/docs/RFC-003.md',
    source: '对话',
    tags: ['文件路径'],
    importance_score: 0.3,
    confidence: 1.0,
    access_count: 1,
    created_at: isoMinutesAgo(8),
    updated_at: isoMinutesAgo(8),
    last_accessed_at: isoMinutesAgo(8),
    expires_at: isoHoursAgo(-2),
  },
  {
    id: _id('L0', 3),
    layer: 'L0',
    type: 'context',
    priority: 'transient',
    scope: 'session',
    content: '上轮对话提到：用户希望保留双面板布局',
    source: '对话',
    tags: ['上下文'],
    importance_score: 0.5,
    confidence: 0.8,
    access_count: 2,
    created_at: isoMinutesAgo(30),
    updated_at: isoMinutesAgo(20),
    last_accessed_at: isoMinutesAgo(20),
    expires_at: isoHoursAgo(-1),
  },
];

/** Mock：L1 短期记忆（当前会话） */
const mockL1: MemoryItem[] = [
  {
    id: _id('L1', 1),
    layer: 'L1',
    type: 'context',
    priority: 'short_term',
    scope: 'session',
    content: '本次会话主题：SparkFox P1-模块 D 接入',
    source: '系统',
    tags: ['会话主题'],
    importance_score: 0.6,
    confidence: 0.95,
    access_count: 5,
    created_at: isoHoursAgo(2),
    updated_at: isoMinutesAgo(30),
    last_accessed_at: isoMinutesAgo(30),
    expires_at: isoHoursAgo(-22),
  },
  {
    id: _id('L1', 2),
    layer: 'L1',
    type: 'context',
    priority: 'short_term',
    scope: 'session',
    content: '当前活跃 Agent：默认助手 / 代码工程师 / 数据分析师',
    source: '系统',
    tags: ['Agent 状态'],
    importance_score: 0.5,
    confidence: 0.9,
    access_count: 4,
    created_at: isoHoursAgo(1),
    updated_at: isoMinutesAgo(15),
    last_accessed_at: isoMinutesAgo(15),
    expires_at: isoHoursAgo(-23),
  },
];

/** Mock：L2 情节记忆 */
const mockL2: MemoryItem[] = [
  {
    id: _id('L2', 1),
    layer: 'L2',
    type: 'episode',
    priority: 'long_term',
    scope: 'global',
    content:
      '2026-07-17 用户请求将 BaiLongma 思考过程可视化组件集成到 SparkFox。'
      + '完成 ThoughtStream 类拆分为 4 个 React 组件（ThinkingDots/ToolCallRow/ThoughtLine/ThoughtStream），'
      + '嵌入 L1 消息气泡 + L2 右侧独立面板。验证 11 项浏览器检查通过。',
    source: '对话',
    tags: ['BaiLongma', 'ThoughtStream', 'P0-模块F'],
    importance_score: 0.8,
    confidence: 0.95,
    access_count: 8,
    created_at: isoDaysAgo(1),
    updated_at: isoDaysAgo(1),
    last_accessed_at: isoHoursAgo(3),
    expires_at: null,
    related_agents: ['default'],
    related_tools: [],
    conversation_id: 'conv_20260717_001',
  },
  {
    id: _id('L2', 2),
    layer: 'L2',
    type: 'episode',
    priority: 'long_term',
    scope: 'global',
    content:
      '2026-07-18 完成 P1-模块 B（Agent 菜单系统，来自 OpenAkita）。'
      + '创建 agentStore + 4 子组件 + 3 View，保留 22 字段 AgentProfile，'
      + '迁移 7 类图标。typecheck 通过 + 7 项浏览器检查 PASS。',
    source: '对话',
    tags: ['OpenAkita', 'Agent菜单', 'P1-模块B'],
    importance_score: 0.85,
    confidence: 0.95,
    access_count: 12,
    created_at: isoHoursAgo(8),
    updated_at: isoHoursAgo(6),
    last_accessed_at: isoHoursAgo(1),
    expires_at: null,
    related_agents: ['default'],
    related_tools: [],
    conversation_id: 'conv_20260718_001',
  },
  {
    id: _id('L2', 3),
    layer: 'L2',
    type: 'episode',
    priority: 'short_term',
    scope: 'session',
    content:
      '2026-07-18 用户指出 NomiFun 记忆系统过时，希望借鉴 OpenAkita 三层记忆。'
      + '决定保留 Pangu Nebula 6 层架构作为基础，融合 OpenAkita SemanticMemory + Episode + Scratchpad。',
    source: '对话',
    tags: ['OpenAkita', 'NomiFun', '记忆系统', '决策'],
    importance_score: 0.9,
    confidence: 0.95,
    access_count: 6,
    created_at: isoHoursAgo(5),
    updated_at: isoHoursAgo(5),
    last_accessed_at: isoHoursAgo(2),
    expires_at: null,
    related_agents: ['default'],
    related_tools: [],
    conversation_id: 'conv_20260718_002',
  },
];

/** Mock：L3 语义记忆（实体-属性结构） */
const mockL3: MemoryItem[] = [
  {
    id: _id('L3', 1),
    layer: 'L3',
    type: 'fact',
    priority: 'permanent',
    scope: 'global',
    content: '用户操作系统为 Windows',
    source: '对话',
    subject: '用户',
    predicate: '使用操作系统',
    object: 'Windows',
    tags: ['环境', 'OS'],
    importance_score: 0.85,
    confidence: 0.95,
    access_count: 25,
    created_at: isoDaysAgo(30),
    updated_at: isoDaysAgo(30),
    last_accessed_at: isoHoursAgo(2),
    expires_at: null,
  },
  {
    id: _id('L3', 2),
    layer: 'L3',
    type: 'preference',
    priority: 'permanent',
    scope: 'global',
    content: '用户偏好 macOS 风格的桌面应用设计',
    source: '对话',
    subject: '用户',
    predicate: '偏好设计风格',
    object: 'macOS / Apple System',
    tags: ['偏好', 'UI'],
    importance_score: 0.9,
    confidence: 0.95,
    access_count: 32,
    created_at: isoDaysAgo(25),
    updated_at: isoDaysAgo(10),
    last_accessed_at: isoHoursAgo(1),
    expires_at: null,
  },
  {
    id: _id('L3', 3),
    layer: 'L3',
    type: 'fact',
    priority: 'long_term',
    scope: 'global',
    content: '用户常用技术栈：Python、TypeScript、Rust、Tauri、Preact',
    source: '对话',
    subject: '用户',
    predicate: '熟悉技术栈',
    object: 'Python / TS / Rust / Tauri / Preact',
    tags: ['技术栈'],
    importance_score: 0.88,
    confidence: 0.9,
    access_count: 18,
    created_at: isoDaysAgo(20),
    updated_at: isoDaysAgo(5),
    last_accessed_at: isoHoursAgo(4),
    expires_at: null,
  },
  {
    id: _id('L3', 4),
    layer: 'L3',
    type: 'preference',
    priority: 'permanent',
    scope: 'global',
    content: '用户沟通语言：中文',
    source: '对话',
    subject: '用户',
    predicate: '使用语言',
    object: '中文',
    tags: ['语言'],
    importance_score: 0.95,
    confidence: 1.0,
    access_count: 50,
    created_at: isoDaysAgo(60),
    updated_at: isoDaysAgo(60),
    last_accessed_at: isoMinutesAgo(10),
    expires_at: null,
  },
  {
    id: _id('L3', 5),
    layer: 'L3',
    type: 'persona_trait',
    priority: 'long_term',
    scope: 'agent',
    content: '用户偏好"双主控 + 蜂群 worker + persona 自进化"架构设计',
    source: '对话',
    subject: '用户',
    predicate: '偏好架构',
    object: '双主控 + 蜂群 + persona 自进化',
    tags: ['架构', '偏好'],
    importance_score: 0.92,
    confidence: 0.9,
    access_count: 9,
    created_at: isoDaysAgo(15),
    updated_at: isoDaysAgo(2),
    last_accessed_at: isoHoursAgo(6),
    expires_at: null,
  },
];

/** Mock：L4 程序记忆（技能/规则/经验） */
const mockL4: MemoryItem[] = [
  {
    id: _id('L4', 1),
    layer: 'L4',
    type: 'skill',
    priority: 'permanent',
    scope: 'global',
    content:
      '使用 Arco Design 组件时：TextArea = Input.TextArea，TabPane = Tabs.TabPane，'
      + 'TabPane 用 title 属性（非 tab），Select 多选用 mode="multiple"。',
    source: '经验',
    tags: ['Arco Design', '前端', '踩坑'],
    importance_score: 0.85,
    confidence: 0.95,
    access_count: 14,
    created_at: isoDaysAgo(2),
    updated_at: isoDaysAgo(1),
    last_accessed_at: isoHoursAgo(3),
    expires_at: null,
  },
  {
    id: _id('L4', 2),
    layer: 'L4',
    type: 'rule',
    priority: 'permanent',
    scope: 'global',
    content: 'SparkFox 路由必须通过 /#/sparkfox/ 访问（HashRouter），不能省略 #',
    source: '系统',
    tags: ['路由', '规则'],
    importance_score: 0.9,
    confidence: 1.0,
    access_count: 22,
    created_at: isoDaysAgo(3),
    updated_at: isoDaysAgo(1),
    last_accessed_at: isoHoursAgo(2),
    expires_at: null,
  },
  {
    id: _id('L4', 3),
    layer: 'L4',
    type: 'experience',
    priority: 'long_term',
    scope: 'global',
    content:
      'NomiFun IPC 架构基于 WebSocket + HTTP REST 的 ipcBridge 抽象层，'
      + '而非 Tauri 的 listen()/invoke()。所有 LLM 流式响应通过单一 WS 频道 message.stream 推送。',
    source: '经验',
    tags: ['NomiFun', 'IPC', '架构'],
    importance_score: 0.92,
    confidence: 0.95,
    access_count: 7,
    created_at: isoDaysAgo(1),
    updated_at: isoDaysAgo(1),
    last_accessed_at: isoHoursAgo(5),
    expires_at: null,
  },
  {
    id: _id('L4', 4),
    layer: 'L4',
    type: 'rule',
    priority: 'permanent',
    scope: 'global',
    content: '所有 BaiLongma MIT 组件必须经过清洁室重写以保持 AGPL 合规',
    source: '系统',
    tags: ['AGPL', '合规', '规则'],
    importance_score: 0.95,
    confidence: 1.0,
    access_count: 11,
    created_at: isoDaysAgo(5),
    updated_at: isoDaysAgo(2),
    last_accessed_at: isoHoursAgo(8),
    expires_at: null,
  },
];

/** Mock：L5 元认知 */
const mockL5: MemoryItem[] = [
  {
    id: _id('L5', 1),
    layer: 'L5',
    type: 'reflection',
    priority: 'long_term',
    scope: 'global',
    content:
      '反思：在 P1-模块 B 集成时，过早假设 Arco Design 与 shadcn/ui API 一致，'
      + '导致 5 个 TypeScript 错误。后续接入新组件库应先做 API 调研。',
    source: '反思',
    tags: ['反思', 'Arco Design', '教训'],
    importance_score: 0.85,
    confidence: 0.9,
    access_count: 4,
    created_at: isoDaysAgo(1),
    updated_at: isoDaysAgo(1),
    last_accessed_at: isoHoursAgo(4),
    expires_at: null,
    reflected: true,
  },
  {
    id: _id('L5', 2),
    layer: 'L5',
    type: 'reflection',
    priority: 'permanent',
    scope: 'global',
    content:
      '价值判断：用户偏好"先完成单版本，再迭代"的开发节奏，'
      + '应避免增量推送，等待所有问题解决后再统一提交。',
    source: '反思',
    tags: ['价值判断', '开发节奏'],
    importance_score: 0.92,
    confidence: 0.95,
    access_count: 8,
    created_at: isoDaysAgo(7),
    updated_at: isoDaysAgo(3),
    last_accessed_at: isoHoursAgo(12),
    expires_at: null,
    reflected: true,
  },
];

/** 全部 mock 记忆 */
const ALL_MOCK_MEMORIES: MemoryItem[] = [
  ...mockL0,
  ...mockL1,
  ...mockL2,
  ...mockL3,
  ...mockL4,
  ...mockL5,
];

// ============================================================================
// 工具函数
// ============================================================================

/** 计算统计信息 */
function computeStats(memories: MemoryItem[]): MemoryStats {
  const by_layer: Record<MemoryLayer, number> = {
    L0: 0,
    L1: 0,
    L2: 0,
    L3: 0,
    L4: 0,
    L5: 0,
  };
  const by_type: Record<string, number> = {};
  let importanceSum = 0;
  let confidenceSum = 0;
  let accessSum = 0;
  let recentActive = 0;
  let expiringSoon = 0;
  const sevenDaysAgo = now.getTime() - 7 * 86400000;
  const sevenDaysLater = now.getTime() + 7 * 86400000;

  for (const m of memories) {
    by_layer[m.layer] += 1;
    by_type[m.type] = (by_type[m.type] || 0) + 1;
    importanceSum += m.importance_score;
    confidenceSum += m.confidence;
    accessSum += m.access_count;
    if (m.last_accessed_at) {
      try {
        if (new Date(m.last_accessed_at).getTime() >= sevenDaysAgo) {
          recentActive += 1;
        }
      } catch {
        /* ignore */
      }
    }
    if (m.expires_at) {
      try {
        const exp = new Date(m.expires_at).getTime();
        if (exp >= now.getTime() && exp <= sevenDaysLater) {
          expiringSoon += 1;
        }
      } catch {
        /* ignore */
      }
    }
  }

  return {
    total: memories.length,
    by_layer,
    by_type,
    avg_importance: memories.length ? importanceSum / memories.length : 0,
    avg_confidence: memories.length ? confidenceSum / memories.length : 0,
    total_access: accessSum,
    recent_active: recentActive,
    expiring_soon: expiringSoon,
  };
}

/** 生成短 ID */
function shortId(): string {
  return `mem_${Math.random().toString(36).slice(2, 10)}`;
}

/** 排序记忆 */
function sortMemories(
  memories: MemoryItem[],
  field: SortField,
  order: SortOrder,
): MemoryItem[] {
  const sorted = [...memories].sort((a, b) => {
    let av: number | string = '';
    let bv: number | string = '';
    if (field === 'importance_score' || field === 'access_count') {
      av = a[field];
      bv = b[field];
    } else {
      av = a[field] || '';
      bv = b[field] || '';
    }
    if (typeof av === 'number' && typeof bv === 'number') {
      return av - bv;
    }
    const as = String(av);
    const bs = String(bv);
    if (as < bs) return -1;
    if (as > bs) return 1;
    return 0;
  });
  return order === 'desc' ? sorted.reverse() : sorted;
}

/** 生成图谱节点（基于记忆间关联） */
function buildGraph(memories: MemoryItem[]): MemoryGraphNode[] {
  return memories.map((m) => {
    const edges: string[] = [];
    // 同层 + 共享 tag 视为关联
    for (const other of memories) {
      if (other.id === m.id) continue;
      if (other.layer === m.layer) {
        const sharedTag = m.tags.some((t) => other.tags.includes(t));
        if (sharedTag) edges.push(other.id);
      }
      // L2 情节关联 L3 语义（subject 命中 content）
      if (m.layer === 'L2' && other.layer === 'L3' && other.subject) {
        if (m.content.includes(other.subject)) edges.push(other.id);
      }
    }
    return {
      id: m.id,
      label:
        m.content.length > 24
          ? m.content.slice(0, 24) + '…'
          : m.content,
      layer: m.layer,
      type: m.type,
      importance: m.importance_score,
      edges,
    };
  });
}

// ============================================================================
// Store 接口
// ============================================================================

interface MemoryState {
  initialized: boolean;
  loading: boolean;
  memories: MemoryItem[];
  selectedIds: Set<string>;
  editingId: string | null;
  editDraft: Partial<MemoryItem> | null;
  searchQuery: string;
  filterLayer: MemoryLayer | 'all';
  filterType: MemoryType | 'all';
  sortBy: SortField;
  sortOrder: SortOrder;
  page: number;
  pageSize: number;
  viewMode: 'list' | 'graph';
  stats: MemoryStats | null;
  graphNodes: MemoryGraphNode[];

  // Actions
  initialize: () => Promise<void>;
  refresh: () => Promise<void>;
  setSearch: (q: string) => void;
  setFilterLayer: (layer: MemoryLayer | 'all') => void;
  setFilterType: (type: MemoryType | 'all') => void;
  setSort: (field: SortField, order: SortOrder) => void;
  setPage: (page: number) => void;
  setViewMode: (mode: 'list' | 'graph') => void;
  toggleSelect: (id: string) => void;
  selectAll: () => void;
  clearSelection: () => void;
  startEdit: (id: string) => void;
  cancelEdit: () => void;
  saveEdit: () => Promise<void>;
  deleteOne: (id: string) => Promise<void>;
  batchDelete: () => Promise<void>;
  addMemory: (item: Partial<MemoryItem>) => Promise<void>;
  accessMemory: (id: string) => void;
  /** 对话组件调用：注入一条新记忆（PoC：直接 add） */
  injectFromConversation: (content: string, type: MemoryType, layer: MemoryLayer) => Promise<void>;

  // ─── Tauri IPC 对接（渐进增强：失败时回退到 mock，不抛异常） ───
  /**
   * 从后端拉取指定层的记忆列表（Tauri IPC: memory_list）
   * @param layer 记忆层（L0~L5）；失败时保留现有 mock 数据
   */
  syncFromBackend: (layer: MemoryLayer) => Promise<void>;
  /**
   * 将一条记忆写入后端（Tauri IPC: memory_put）
   * @param layer 记忆层
   * @param entry 记忆条目（部分字段，由后端补全 id/timestamps）
   */
  putToBackend: (layer: MemoryLayer, entry: Partial<MemoryItem>) => Promise<void>;
}

// ============================================================================
// Store 实现
// ============================================================================

const PAGE_SIZE = 30;

export const useMemoryStore = create<MemoryState>((set, get) => ({
  initialized: false,
  loading: false,
  memories: [],
  selectedIds: new Set<string>(),
  editingId: null,
  editDraft: null,
  searchQuery: '',
  filterLayer: 'all',
  filterType: 'all',
  sortBy: DEFAULT_SORT.field,
  sortOrder: DEFAULT_SORT.order,
  page: 0,
  pageSize: PAGE_SIZE,
  viewMode: 'list',
  stats: null,
  graphNodes: [],

  initialize: async () => {
    if (get().initialized) return;
    set({ loading: true });
    // 模拟异步加载
    await new Promise((r) => setTimeout(r, 200));
    const memories = ALL_MOCK_MEMORIES;
    set({
      memories,
      stats: computeStats(memories),
      graphNodes: buildGraph(memories),
      initialized: true,
      loading: false,
    });
  },

  refresh: async () => {
    set({ loading: true });
    await new Promise((r) => setTimeout(r, 200));
    const memories = get().memories;
    set({
      stats: computeStats(memories),
      graphNodes: buildGraph(memories),
      loading: false,
    });
  },

  setSearch: (q) => set({ searchQuery: q, page: 0 }),

  setFilterLayer: (layer) => set({ filterLayer: layer, page: 0 }),

  setFilterType: (type) => set({ filterType: type, page: 0 }),

  setSort: (field, order) => set({ sortBy: field, sortOrder: order, page: 0 }),

  setPage: (page) => set({ page }),

  setViewMode: (mode) => set({ viewMode: mode }),

  toggleSelect: (id) =>
    set((state) => {
      const next = new Set(state.selectedIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { selectedIds: next };
    }),

  selectAll: () =>
    set((state) => ({
      selectedIds: new Set(state.memories.map((m) => m.id)),
    })),

  clearSelection: () => set({ selectedIds: new Set<string>() }),

  startEdit: (id) => {
    const m = get().memories.find((x) => x.id === id);
    if (!m) return;
    set({ editingId: id, editDraft: { ...m } });
  },

  cancelEdit: () => set({ editingId: null, editDraft: null }),

  saveEdit: async () => {
    const { editingId, editDraft, memories } = get();
    if (!editingId || !editDraft) return;
    await new Promise((r) => setTimeout(r, 150));
    const updated: MemoryItem = {
      ...(memories.find((m) => m.id === editingId) as MemoryItem),
      ...editDraft,
      updated_at: new Date().toISOString(),
    } as MemoryItem;
    const next = memories.map((m) => (m.id === editingId ? updated : m));
    set({
      memories: next,
      stats: computeStats(next),
      graphNodes: buildGraph(next),
      editingId: null,
      editDraft: null,
    });
  },

  deleteOne: async (id) => {
    await new Promise((r) => setTimeout(r, 100));
    const next = get().memories.filter((m) => m.id !== id);
    const sel = new Set(get().selectedIds);
    sel.delete(id);
    set({
      memories: next,
      stats: computeStats(next),
      graphNodes: buildGraph(next),
      selectedIds: sel,
    });
  },

  batchDelete: async () => {
    const sel = get().selectedIds;
    if (sel.size === 0) return;
    await new Promise((r) => setTimeout(r, 150));
    const next = get().memories.filter((m) => !sel.has(m.id));
    set({
      memories: next,
      stats: computeStats(next),
      graphNodes: buildGraph(next),
      selectedIds: new Set<string>(),
    });
  },

  addMemory: async (item) => {
    await new Promise((r) => setTimeout(r, 100));
    const nowIso = new Date().toISOString();
    const newItem: MemoryItem = {
      id: shortId(),
      layer: item.layer || 'L3',
      type: item.type || 'fact',
      priority: item.priority || 'long_term',
      scope: item.scope || 'global',
      content: item.content || '',
      source: item.source || '手动',
      subject: item.subject,
      predicate: item.predicate,
      object: item.object,
      tags: item.tags || [],
      importance_score: item.importance_score ?? 0.5,
      confidence: item.confidence ?? 0.8,
      access_count: 0,
      created_at: nowIso,
      updated_at: nowIso,
      last_accessed_at: nowIso,
      expires_at: item.expires_at || null,
      related_agents: item.related_agents,
      related_tools: item.related_tools,
      conversation_id: item.conversation_id,
      reflected: item.reflected,
    };
    const next = [newItem, ...get().memories];
    set({
      memories: next,
      stats: computeStats(next),
      graphNodes: buildGraph(next),
    });
  },

  accessMemory: (id) => {
    const { memories } = get();
    const next = memories.map((m) =>
      m.id === id
        ? {
            ...m,
            access_count: m.access_count + 1,
            last_accessed_at: new Date().toISOString(),
          }
        : m,
    );
    set({ memories: next });
  },

  injectFromConversation: async (content, type, layer) => {
    await get().addMemory({
      content,
      type,
      layer,
      source: '对话',
      priority: layer === 'L0' ? 'transient' : layer === 'L1' ? 'short_term' : 'long_term',
      scope: layer === 'L0' || layer === 'L1' ? 'session' : 'global',
      importance_score: 0.6,
      confidence: 0.8,
      tags: ['对话注入'],
    });
  },

  // ─── Tauri IPC 对接：渐进增强，失败回退 mock ───

  syncFromBackend: async (layer) => {
    try {
      const remote = await invoke<MemoryItem[]>('memory_list', { layer, limit: 100 });
      if (Array.isArray(remote) && remote.length > 0) {
        // 后端有数据 → 合并到现有 memories（替换同层条目，保留其他层 mock）
        const others = get().memories.filter((m) => m.layer !== layer);
        const merged = [...others, ...remote];
        set({
          memories: merged,
          stats: computeStats(merged),
          graphNodes: buildGraph(merged),
        });
      }
      // 后端返回空 → 保留现有 mock 数据
    } catch (e) {
      // 非 Tauri 环境或 command 未注册，静默回退
      console.warn(`[memoryStore] syncFromBackend(${layer}) failed, using mock:`, e);
    }
  },

  putToBackend: async (layer, entry) => {
    try {
      await invoke('memory_put', { layer, entry });
      // 写入成功后，本地同步追加（保证 UI 即时反馈）
      await get().addMemory({ ...entry, layer });
    } catch (e) {
      // 后端写入失败：仍写入本地 mock，保证 UI 可用
      console.warn(`[memoryStore] putToBackend(${layer}) failed, local-only fallback:`, e);
      await get().addMemory({ ...entry, layer });
    }
  },
}));

// ============================================================================
// 选择器（外部使用）
// ============================================================================

/** 过滤 + 排序后的记忆列表 */
export function selectFilteredMemories(state: MemoryState): MemoryItem[] {
  let list = state.memories;
  if (state.filterLayer !== 'all') {
    list = list.filter((m) => m.layer === state.filterLayer);
  }
  if (state.filterType !== 'all') {
    list = list.filter((m) => m.type === state.filterType);
  }
  if (state.searchQuery.trim()) {
    const q = state.searchQuery.trim().toLowerCase();
    list = list.filter(
      (m) =>
        m.content.toLowerCase().includes(q) ||
        m.tags.some((t) => t.toLowerCase().includes(q)) ||
        (m.subject || '').toLowerCase().includes(q) ||
        (m.object || '').toLowerCase().includes(q),
    );
  }
  return sortMemories(list, state.sortBy, state.sortOrder);
}

/** 分页后的记忆列表 */
export function selectPagedMemories(state: MemoryState): MemoryItem[] {
  const filtered = selectFilteredMemories(state);
  const start = state.page * state.pageSize;
  return filtered.slice(start, start + state.pageSize);
}

/** 总过滤后数量 */
export function selectFilteredCount(state: MemoryState): number {
  return selectFilteredMemories(state).length;
}
