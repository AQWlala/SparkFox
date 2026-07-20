/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox monitorStore — 监视面板状态管理
 *
 * 来源：OpenAkita TokenStatsView + OrgDashboard + OrgMonitorPanel
 *       （清洁室重写为 TypeScript Zustand store，PoC mock 数据）
 *
 * 功能：
 * - 4 个统计卡片（Token 总量 / 总费用 / Agent 数 / 工具调用数）
 * - Token 用量统计（6 时间周期 + 按端点/操作分组 + 时间线 + 会话 + 记录）
 * - 实时活动流（事件列表 + 过滤 + 实时模式）
 *
 * 改造点：
 * - OpenAkita safeFetch HTTP + WS → PoC mock 数据
 * - 组织/节点概念 → Agent + 对话 + 工具调用
 * - useTranslation i18n → 硬编码中文
 * - 实时模式：定时生成事件（PoC 不依赖 WS）
 */

import { create } from 'zustand';
// Tauri 2 IPC：monitor_stats / monitor_ack 失败时静默回退到 mock 数据
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** 统计卡片数据 */
export interface StatCardData {
  label: string;
  value: number;
  unit?: string;
  color: string;
  icon: string;
  trend?: number;
  sub_label?: string;
}

/** 时间周期 */
export type PeriodKey = '1d' | '3d' | '1w' | '1m' | '6m' | '1y';

/** Token 总计 */
export interface TokenTotal {
  request_count: number;
  total_input: number;
  total_output: number;
  total_cache_creation: number;
  total_cache_read: number;
  total_cost: number;
}

/** 分组统计行 */
export interface TokenSummaryRow {
  group_key: string;
  total_tokens: number;
  request_count: number;
  total_input: number;
  total_output: number;
  total_cost: number;
}

/** 时间线行 */
export interface TokenTimelineRow {
  time_bucket: string;
  total_tokens: number;
}

/** 会话行 */
export interface SessionRow {
  session_id: string;
  operation_types: string;
  endpoints: string;
  request_count: number;
  total_tokens: number;
  total_cost: number;
  last_call: string;
}

/** 用量记录行 */
export interface UsageRecordRow {
  request_id: string;
  timestamp: string;
  endpoint_name: string;
  model: string;
  operation_type: string;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  estimated_cost: number;
}

/** 活动事件类型 */
export type ActivityEventType =
  | 'message_out'
  | 'message_in'
  | 'tool_call'
  | 'tool_result'
  | 'agent_switch'
  | 'memory_op'
  | 'error'
  | 'session_start'
  | 'session_end';

/** 活动事件 */
export interface ActivityEvent {
  id: string;
  type: ActivityEventType;
  title: string;
  agent_name: string;
  detail?: string;
  tokens?: number;
  duration_ms?: number;
  status?: 'ok' | 'warn' | 'error';
  timestamp: string;
}

// ============================================================================
// 常量
// ============================================================================

export const PERIOD_KEYS: PeriodKey[] = ['1d', '3d', '1w', '1m', '6m', '1y'];

export const PERIOD_LABELS: Record<PeriodKey, string> = {
  '1d': '近 1 天',
  '3d': '近 3 天',
  '1w': '近 1 周',
  '1m': '近 1 月',
  '6m': '近 6 月',
  '1y': '近 1 年',
};

export const OPERATION_LABELS: Record<string, string> = {
  chat: '对话',
  completion: '补全',
  embedding: '嵌入',
  tool_call: '工具调用',
  agent_switch: 'Agent 切换',
  memory_op: '记忆操作',
  summary: '摘要',
};

export const ACTIVITY_TYPE_LABELS: Record<ActivityEventType, string> = {
  message_out: '消息输出',
  message_in: '消息输入',
  tool_call: '工具调用',
  tool_result: '工具结果',
  agent_switch: 'Agent 切换',
  memory_op: '记忆操作',
  error: '错误',
  session_start: '会话开始',
  session_end: '会话结束',
};

export const ACTIVITY_TYPE_COLORS: Record<ActivityEventType, string> = {
  message_out: '#007AFF',
  message_in: '#5AC8FA',
  tool_call: '#FF9500',
  tool_result: '#34C759',
  agent_switch: '#5856D6',
  memory_op: '#FF2D55',
  error: '#FF3B30',
  session_start: '#8E8E93',
  session_end: '#8E8E93',
};

// ============================================================================
// 工具函数
// ============================================================================

/** 格式化数字（千分位） */
export function fmtNum(value: number): string {
  if (!Number.isFinite(value)) return '0';
  if (Math.abs(value) >= 1e9) return `${(value / 1e9).toFixed(2)}B`;
  if (Math.abs(value) >= 1e6) return `${(value / 1e6).toFixed(2)}M`;
  if (Math.abs(value) >= 1e4) return `${(value / 1e3).toFixed(1)}K`;
  return Math.round(value).toLocaleString('en-US');
}

/** 格式化费用（¥ 前缀，6 位小数） */
export function fmtCost(value: number): string {
  if (!Number.isFinite(value) || value === 0) return '¥0.000000';
  if (value < 0.01) return `¥${value.toFixed(6)}`;
  if (value < 1) return `¥${value.toFixed(4)}`;
  return `¥${value.toFixed(2)}`;
}

/** 格式化时间（HH:mm:ss） */
export function fmtTime(value: string | number): string {
  if (!value) return '';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return '';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** 格式化相对时间（刚刚 / X 分钟前 / X 小时前） */
export function fmtRelative(value: string | number): string {
  if (!value) return '';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return '';
  const diff = Date.now() - d.getTime();
  if (diff < 60_000) return '刚刚';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`;
  return `${Math.floor(diff / 86_400_000)} 天前`;
}

// ============================================================================
// Mock 数据生成
// ============================================================================

const AGENT_NAMES = ['主星·编排者', '化身·灵魂分身', '星尘群-α', '星尘群-β', '星魂·反思者'];
const ENDPOINTS = ['claude-3.5-sonnet', 'gpt-4o', 'deepseek-v3', 'qwen-max', 'embedding-3'];
const MODELS = ['claude-3-5-sonnet-20241022', 'gpt-4o-2024-08-06', 'deepseek-chat', 'qwen-max-latest', 'text-embedding-3-large'];
const OPERATIONS: (keyof typeof OPERATION_LABELS)[] = ['chat', 'completion', 'embedding', 'tool_call', 'agent_switch', 'memory_op', 'summary'];
const TOOL_NAMES = ['web_search', 'file_read', 'code_execute', 'memory_query', 'http_request', 'calendar_check'];

function rand(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function randPick<T>(arr: T[]): T {
  return arr[rand(0, arr.length - 1)];
}

function isoTime(offsetMs: number): string {
  return new Date(Date.now() - offsetMs).toISOString();
}

function genSessionId(): string {
  return `sess_${Math.random().toString(36).slice(2, 14)}`;
}

function genRequestId(): string {
  return `req_${Math.random().toString(36).slice(2, 14)}`;
}

/** 生成统计卡片数据 */
function genStatCards(): StatCardData[] {
  return [
    {
      label: 'Token 总量',
      value: 1284562,
      unit: 'tokens',
      color: '#007AFF',
      icon: '⚡',
      trend: 12,
      sub_label: '今日 +15.2K',
    },
    {
      label: '总费用',
      value: 8.4523,
      unit: 'CNY',
      color: '#FF2D55',
      icon: '¥',
      trend: -5,
      sub_label: '今日 ¥0.82',
    },
    {
      label: '活跃 Agent',
      value: 5,
      unit: '个',
      color: '#34C759',
      icon: '🤖',
      trend: 0,
      sub_label: '共 12 个 Agent',
    },
    {
      label: '工具调用',
      value: 348,
      unit: '次',
      color: '#FF9500',
      icon: '🔧',
      trend: 8,
      sub_label: '成功率 96.8%',
    },
  ];
}

/** 生成 Token 总计（按周期缩放） */
function genTokenTotal(period: PeriodKey): TokenTotal {
  const scale: Record<PeriodKey, number> = { '1d': 1, '3d': 3, '1w': 7, '1m': 30, '6m': 180, '1y': 365 };
  const s = scale[period];
  return {
    request_count: Math.round(156 * s + rand(-10, 10)),
    total_input: Math.round(824000 * s + rand(-5000, 5000)),
    total_output: Math.round(460000 * s + rand(-3000, 3000)),
    total_cache_creation: Math.round(120000 * s + rand(-1000, 1000)),
    total_cache_read: Math.round(85000 * s + rand(-800, 800)),
    total_cost: Number((8.45 * s + rand(-0.5, 0.5)).toFixed(4)),
  };
}

/** 生成按端点分组 */
function genByEndpoint(): TokenSummaryRow[] {
  return ENDPOINTS.map((ep, i) => ({
    group_key: ep,
    total_tokens: rand(50000, 500000),
    request_count: rand(20, 200),
    total_input: rand(40000, 350000),
    total_output: rand(10000, 150000),
    total_cost: Number((Math.random() * 5).toFixed(4)),
  })).sort((a, b) => b.total_tokens - a.total_tokens);
}

/** 生成按操作类型分组 */
function genByOp(): TokenSummaryRow[] {
  return OPERATIONS.map((op) => ({
    group_key: op,
    total_tokens: rand(20000, 300000),
    request_count: rand(15, 150),
    total_input: rand(15000, 200000),
    total_output: rand(5000, 100000),
    total_cost: Number((Math.random() * 3).toFixed(4)),
  })).sort((a, b) => b.total_tokens - a.total_tokens);
}

/** 生成时间线 */
function genTimeline(period: PeriodKey): TokenTimelineRow[] {
  const buckets: string[] = [];
  const now = new Date();
  if (period === '1d') {
    // 24 小时
    for (let i = 23; i >= 0; i--) {
      const d = new Date(now.getTime() - i * 3_600_000);
      buckets.push(`${String(d.getHours()).padStart(2, '0')}:00`);
    }
  } else if (period === '3d') {
    for (let i = 0; i < 24; i++) buckets.push(`T-${23 - i}`);
  } else if (period === '1w') {
    const days = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];
    for (let i = 6; i >= 0; i--) buckets.push(days[(now.getDay() - i + 6) % 7]);
  } else if (period === '1m') {
    for (let i = 29; i >= 0; i--) {
      const d = new Date(now.getTime() - i * 86_400_000);
      buckets.push(`${d.getMonth() + 1}/${d.getDate()}`);
    }
  } else if (period === '6m') {
    for (let i = 5; i >= 0; i--) {
      const d = new Date(now.getFullYear(), now.getMonth() - i, 1);
      buckets.push(`${d.getMonth() + 1}月`);
    }
  } else {
    for (let i = 11; i >= 0; i--) {
      const d = new Date(now.getFullYear(), now.getMonth() - i, 1);
      buckets.push(`${d.getFullYear()}/${d.getMonth() + 1}`);
    }
  }
  return buckets.map((b) => ({ time_bucket: b, total_tokens: rand(1000, 50000) }));
}

/** 生成会话列表 */
function genSessions(): SessionRow[] {
  return Array.from({ length: 8 }, () => ({
    session_id: genSessionId(),
    operation_types: randPick(OPERATIONS),
    endpoints: randPick(ENDPOINTS),
    request_count: rand(3, 45),
    total_tokens: rand(2000, 80000),
    total_cost: Number((Math.random() * 2).toFixed(4)),
    last_call: isoTime(rand(0, 3_600_000 * 3)),
  }));
}

/** 生成用量记录 */
function genRecords(): UsageRecordRow[] {
  return Array.from({ length: 20 }, () => {
    const op = randPick(OPERATIONS);
    const inputTokens = rand(100, 8000);
    const outputTokens = op === 'embedding' ? rand(1, 100) : rand(50, 4000);
    return {
      request_id: genRequestId(),
      timestamp: isoTime(rand(0, 3_600_000 * 6)),
      endpoint_name: randPick(ENDPOINTS),
      model: randPick(MODELS),
      operation_type: op,
      input_tokens: inputTokens,
      output_tokens: outputTokens,
      total_tokens: inputTokens + outputTokens,
      estimated_cost: Number((Math.random() * 0.5).toFixed(6)),
    };
  }).sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
}

/** 生成单条活动事件 */
function genActivityEvent(): ActivityEvent {
  const type = randPick<ActivityEventType>([
    'message_out', 'message_in', 'tool_call', 'tool_result',
    'agent_switch', 'memory_op', 'session_start', 'session_end',
  ]);
  const agent = randPick(AGENT_NAMES);
  const tool = randPick(TOOL_NAMES);
  const titles: Record<ActivityEventType, string> = {
    message_out: `${agent} 回复消息`,
    message_in: `用户发送消息`,
    tool_call: `${agent} 调用工具 ${tool}`,
    tool_result: `工具 ${tool} 返回结果`,
    agent_switch: `切换到 ${agent}`,
    memory_op: `${agent} 操作记忆`,
    error: `${agent} 处理出错`,
    session_start: `新会话开始`,
    session_end: `会话结束`,
  };
  const details: Record<ActivityEventType, string> = {
    message_out: `生成回复，长度 ${rand(50, 2000)} 字符`,
    message_in: `用户输入：${rand(5, 50)} 字符`,
    tool_call: `参数：{ query: "${randPick(['天气', '新闻', '股票', '文档', '代码'])}" }`,
    tool_result: `返回 ${rand(100, 5000)} 字节，耗时 ${rand(50, 2000)}ms`,
    agent_switch: `从 ${randPick(AGENT_NAMES)} 切换到 ${agent}`,
    memory_op: `${randPick(['读取', '写入', '更新', '删除'])} L${rand(0, 5)} 记忆`,
    error: `API 返回 500 错误：${randPick(['超时', '限流', '模型不可用', '参数错误'])}`,
    session_start: `会话 ID: ${genSessionId()}`,
    session_end: `会话持续 ${rand(1, 60)} 分钟`,
  };
  const status: ActivityEvent['status'] = type === 'error' ? 'error' : type === 'tool_call' && Math.random() < 0.1 ? 'warn' : 'ok';

  return {
    id: `evt_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    type,
    title: titles[type],
    agent_name: agent,
    detail: details[type],
    tokens: type === 'message_out' || type === 'message_in' ? rand(100, 3000) : undefined,
    duration_ms: type === 'tool_call' || type === 'tool_result' ? rand(20, 3000) : undefined,
    status,
    timestamp: new Date().toISOString(),
  };
}

/** 生成初始活动事件列表 */
function genInitialActivities(): ActivityEvent[] {
  const events: ActivityEvent[] = [];
  for (let i = 0; i < 15; i++) {
    const evt = genActivityEvent();
    evt.timestamp = isoTime(i * rand(30_000, 300_000));
    events.push(evt);
  }
  return events;
}

// ============================================================================
// Store 接口
// ============================================================================

interface MonitorState {
  initialized: boolean;
  loading: boolean;

  // 统计卡片
  statCards: StatCardData[];

  // Token 用量统计
  period: PeriodKey;
  total: TokenTotal | null;
  byEndpoint: TokenSummaryRow[];
  byOp: TokenSummaryRow[];
  timeline: TokenTimelineRow[];
  sessions: SessionRow[];
  records: UsageRecordRow[];

  // 活动流
  activities: ActivityEvent[];
  liveMode: boolean;

  // Actions
  initialize: () => Promise<void>;
  refresh: () => Promise<void>;
  setPeriod: (period: PeriodKey) => void;
  addActivity: (event?: ActivityEvent) => void;
  clearActivities: () => void;
  toggleLiveMode: () => void;

  // ─── Tauri IPC 对接（渐进增强：失败时回退到 mock，不抛异常） ───
  /**
   * 从后端拉取监视统计（Tauri IPC: monitor_stats）
   * @param period 时间周期；失败时回退到本地 mock 生成
   */
  fetchStatsFromBackend: (period: PeriodKey) => Promise<void>;
  /**
   * 确认（ack）一条监视事件（Tauri IPC: monitor_ack）
   * @param eventId 事件 ID
   */
  ackEvent: (eventId: string) => Promise<void>;
}

// ============================================================================
// Store 实现
// ============================================================================

export const useMonitorStore = create<MonitorState>((set, get) => ({
  initialized: false,
  loading: false,

  statCards: [],

  period: '1d',
  total: null,
  byEndpoint: [],
  byOp: [],
  timeline: [],
  sessions: [],
  records: [],

  activities: [],
  liveMode: false,

  initialize: async () => {
    if (get().initialized) return;
    set({ loading: true });
    await new Promise((r) => setTimeout(r, 200));
    set({
      statCards: genStatCards(),
      total: genTokenTotal('1d'),
      byEndpoint: genByEndpoint(),
      byOp: genByOp(),
      timeline: genTimeline('1d'),
      sessions: genSessions(),
      records: genRecords(),
      activities: genInitialActivities(),
      initialized: true,
      loading: false,
    });
  },

  refresh: async () => {
    set({ loading: true });
    await new Promise((r) => setTimeout(r, 200));
    const period = get().period;
    set({
      statCards: genStatCards(),
      total: genTokenTotal(period),
      byEndpoint: genByEndpoint(),
      byOp: genByOp(),
      timeline: genTimeline(period),
      sessions: genSessions(),
      records: genRecords(),
      loading: false,
    });
  },

  setPeriod: (period) => {
    set({ period, loading: true });
    setTimeout(() => {
      set({
        total: genTokenTotal(period),
        byEndpoint: genByEndpoint(),
        byOp: genByOp(),
        timeline: genTimeline(period),
        sessions: genSessions(),
        records: genRecords(),
        loading: false,
      });
    }, 200);
  },

  addActivity: (event) => {
    const evt = event || genActivityEvent();
    set((state) => ({
      activities: [evt, ...state.activities].slice(0, 200),
    }));
  },

  clearActivities: () => set({ activities: [] }),

  toggleLiveMode: () => set((state) => ({ liveMode: !state.liveMode })),

  // ─── Tauri IPC 对接：渐进增强，失败回退 mock ───

  fetchStatsFromBackend: async (period) => {
    try {
      const stats = await invoke<Record<string, unknown>>('monitor_stats', { period });
      if (stats && typeof stats === 'object') {
        // 后端返回的统计对象 → 合并到现有统计卡片（保留 mock 卡片定义，仅刷新数值）
        // 后端字段名约定：{ total_tokens, total_cost, agent_count, tool_call_count, ... }
        set((state) => ({
          statCards: state.statCards.map((card) => {
            const key = card.label;
            if (key === 'Token 总量' && typeof stats.total_tokens === 'number') {
              return { ...card, value: stats.total_tokens as number };
            }
            if (key === '总费用' && typeof stats.total_cost === 'number') {
              return { ...card, value: stats.total_cost as number };
            }
            if (key === '活跃 Agent' && typeof stats.agent_count === 'number') {
              return { ...card, value: stats.agent_count as number };
            }
            if (key === '工具调用' && typeof stats.tool_call_count === 'number') {
              return { ...card, value: stats.tool_call_count as number };
            }
            return card;
          }),
        }));
      }
      // 后端未返回有效数据 → 保留现有 mock 数据
    } catch (e) {
      // 非 Tauri 环境或 command 未注册，静默回退
      console.warn(`[monitorStore] fetchStatsFromBackend(${period}) failed, using mock:`, e);
    }
  },

  ackEvent: async (eventId) => {
    try {
      await invoke('monitor_ack', { eventId });
      // ack 成功后从本地活动列表移除该事件
      set((state) => ({
        activities: state.activities.filter((evt) => evt.id !== eventId),
      }));
    } catch (e) {
      // 后端 ack 失败：保留事件，仅记录警告
      console.warn(`[monitorStore] ackEvent(${eventId}) failed:`, e);
    }
  },
}));
