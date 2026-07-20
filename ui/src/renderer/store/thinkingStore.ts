/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox thinkingStore — 思考过程状态管理
 *
 * 来源：BaiLongma src/ui/brain-ui/thought-stream.js（ThoughtStream 类，清洁室重写为 Zustand store）
 *
 * 保留 BaiLongma 特性：
 * - 思考流（ThoughtLine）：type / time / content / thinking dots / tools[]
 * - 工具调用（ToolCall）：name / args / result / status / subject / detail / expanded
 * - 状态徽章（ ThoughtStatus）：busy / failed / stale + 超时降级
 * - 双面板（L1 用户消息流 + L2 Tick 心跳流）
 * - beginRound / startThinking / stopThinking / end 轮次状态机
 *
 * 改造点：
 * - class 实例（L1/L2）→ 单一 store + streams Map<messageClientId, ThoughtLine[]>
 * - L2 独立思考流（tickStream）作为右侧面板数据源
 * - DOM 操作 → React 组件订阅 store
 * - readCSSVar 主题色 → 由组件 CSS 变量直接引用
 * - 新增 AI 活动状态派生（60s 滑动窗口 + 8 大工具分组）
 */

import { create } from 'zustand';
// Tauri 2 IPC：监听后端推送的 'thought_pushed' 事件（非 Tauri 环境下 listen 会抛错，需 try/catch）
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  isFailureResult,
  formatToolSubject,
  formatToolDetail,
  classifyTool,
  type ToolStatus,
} from '@renderer/utils/sparkfox/toolMap';

// ─── 类型定义 ───

/** 思考状态徽章 */
export type ThinkingStatusKind = 'busy' | 'failed' | 'stale' | 'idle';

/** 思考状态 */
export interface ThinkingStatus {
  text: string;
  kind: ThinkingStatusKind;
  /** 超时降级时间戳（ms），0 表示不降级 */
  staleAt?: number;
  staleText?: string;
}

/** 工具调用（对应 BaiLongma line-tool） */
export interface ToolCall {
  id: string;
  name: string;
  args: Record<string, unknown>;
  result: string | null;
  ok: boolean | null; // null = 未完成
  status: ToolStatus | 'running';
  subject: string;
  detail: string;
  expandable: boolean;
  expanded: boolean;
  createdAt: number;
}

/** 单行思考流（对应 BaiLongma stream-line） */
export interface ThoughtLine {
  id: string;
  /** 行类型（"思考中" / "用户消息" / "心跳 Tick" / "工具调用" 等） */
  type: string;
  /** 时间戳标签（HH:MM:SS） */
  time: string;
  /** 行文本内容（可选） */
  content?: string;
  /** 是否正在思考（显示三点动画） */
  isThinking: boolean;
  /** 思考是否完成（完成后隐藏三点） */
  thinkingDone: boolean;
  /** 状态徽章（可选） */
  status: ThinkingStatus | null;
  /** 工具调用列表 */
  tools: ToolCall[];
  /** 是否有工具调用 */
  hadToolCall: boolean;
  /** 是否有工具失败 */
  toolFailed: boolean;
  /** 是否已结束（end() 调用后） */
  ended: boolean;
  /** 创建时间（ms） */
  createdAt: number;
}

/** AI 活动状态记录 */
export interface AiActivityEntry {
  name: string;
  ts: number;
  group: string;
}

/** AI 活动状态派生结果 */
export interface AiActivityState {
  state: 'idle' | 'busy';
  label: string;
  detail: string;
  /** 60s 窗口内工具调用次数 */
  count: number;
  /** 持续秒数 */
  elapsedSec: number;
}

// ─── 常量 ───

export const AI_ACTIVITY_WINDOW_MS = 60_000;
export const AI_ACTIVITY_IDLE_AFTER_MS = 15_000;

/** L1 面板 streamKey（用户消息流） */
export const L1_STREAM_KEY = '__l1_tick__';

/** L2 面板 streamKey（Tick 心跳流，右侧面板） */
export const L2_STREAM_KEY = '__l2_tick__';

// ─── 工具函数 ───

/** 当前时间戳标签（HH:MM:SS） */
export function tStamp(date: Date = new Date()): string {
  const hh = String(date.getHours()).padStart(2, '0');
  const mm = String(date.getMinutes()).padStart(2, '0');
  const ss = String(date.getSeconds()).padStart(2, '0');
  return `${hh}:${mm}:${ss}`;
}

/** 生成唯一 ID */
function newId(prefix = 'tl'): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

// ─── Store 接口 ───

interface ThinkingState {
  /** L1 思考流：按 AI 消息 clientId 索引（用户消息触发的处理流，嵌入消息气泡） */
  l1Streams: Map<string, ThoughtLine[]>;
  /** 当前活跃的 L1 streamKey（最近一次 message_received 的 clientId） */
  activeL1Key: string | null;
  /** L2 思考流：单一数组（Tick 心跳触发的自主行动流，右侧面板） */
  l2Stream: ThoughtLine[];
  /** 当前活跃面板路径（'l1' / 'l2'，对应 BaiLongma currentPath） */
  currentPath: 'l1' | 'l2';
  /** AI 活动状态记录（60s 滑动窗口） */
  aiActivityLog: AiActivityEntry[];
  /** AI 活动起始时间戳 */
  aiActivityFirstTs: number;

  // ─── L1 Actions（按 clientId 操作） ───

  /** 开始新一轮（清理上一轮残留） */
  beginRound: (streamKey: string) => void;
  /** 新建一行思考流 */
  addLine: (streamKey: string, type: string, content?: string) => void;
  /** 开始思考会话（复用或新建 thinkingLine） */
  startThinkingSession: (streamKey: string, thinkingLabel?: string) => void;
  /** 开始思考（显示三点动画） */
  startThinking: (streamKey: string) => void;
  /** 停止思考（隐藏三点） */
  stopThinking: (streamKey: string, doneLabel?: string) => void;
  /** 设置状态徽章 */
  setStatus: (streamKey: string, text: string, kind?: ThinkingStatusKind) => void;
  /** 设置超时降级状态徽章 */
  setTimedStatus: (
    streamKey: string,
    text: string,
    kind: ThinkingStatusKind,
    options: { staleAfterMs?: number; staleText?: string }
  ) => void;
  /** 清除状态徽章 */
  clearStatus: (streamKey: string) => void;
  /** 添加工具调用（BaiLongma tool() 方法） */
  addToolCall: (
    streamKey: string,
    name: string,
    args: Record<string, unknown>,
    result: string | null,
    ok?: boolean
  ) => void;
  /** 添加工具调用结束行 */
  appendToolCycleEnd: (streamKey: string) => void;
  /** 整轮结束 */
  endRound: (streamKey: string) => void;
  /** 切换工具调用展开/收起 */
  toggleToolExpand: (streamKey: string, lineId: string, toolId: string) => void;
  /** 清空指定 stream */
  clearStream: (streamKey: string) => void;

  // ─── L2 Actions（别名，固定 streamKey = L2_STREAM_KEY） ───
  // L2 是单一数组，复用 L1 API 即可

  // ─── 路由 Actions ───
  /** 设置当前活跃面板路径 */
  setCurrentPath: (path: 'l1' | 'l2') => void;
  /** 设置当前活跃 L1 streamKey（用户消息到达时调用） */
  setActiveL1Key: (clientId: string) => void;

  // ─── AI 活动状态 Actions ───
  /** 记录工具调用（用于 AI 活动状态派生） */
  recordAiActivity: (name: string) => void;
  /** 清空 AI 活动记录 */
  clearAiActivity: () => void;

  // ─── Tauri IPC 对接：监听后端推送的思考块（非 Tauri 环境下 no-op） ───
  /** 后端事件监听取消函数（内部使用，避免重复订阅） */
  _backendEventUnlisten: UnlistenFn | null;
  /**
   * 订阅后端 'thought_pushed' 事件（Tauri IPC: listen）
   * 收到思考块时写入 L2 心跳流；非 Tauri 环境下静默 no-op
   */
  subscribeToBackendEvents: () => Promise<void>;
  /** 取消后端事件订阅（组件卸载或重新订阅时调用） */
  unsubscribeFromBackendEvents: () => void;

  // ─── 选择器（实际由组件用 useThinkingStore + selector 实现） ───
}

// ─── 内部工具：获取或创建 stream ───

function getL1Stream(state: ThinkingState, streamKey: string): ThoughtLine[] {
  if (streamKey === L2_STREAM_KEY) return state.l2Stream;
  return state.l1Streams.get(streamKey) || [];
}

function setL1Stream(
  state: ThinkingState,
  streamKey: string,
  lines: ThoughtLine[]
): Partial<ThinkingState> {
  if (streamKey === L2_STREAM_KEY) {
    return { l2Stream: lines };
  }
  const newMap = new Map(state.l1Streams);
  newMap.set(streamKey, lines);
  return { l1Streams: newMap };
}

/** 找到当前 stream 中可思考的行（最后一个 isThinking && !ended 的行） */
function findThinkingLine(lines: ThoughtLine[]): ThoughtLine | null {
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    if (!line.ended) return line;
  }
  return null;
}

// ─── Store 实现 ───

export const useThinkingStore = create<ThinkingState>((set, get) => ({
  l1Streams: new Map(),
  activeL1Key: null,
  l2Stream: [],
  currentPath: 'l2',
  aiActivityLog: [],
  aiActivityFirstTs: 0,
  _backendEventUnlisten: null,

  // ─── L1 Actions ───

  beginRound: (streamKey) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      // 清理最后一行的思考状态（兜底：上一轮若被打断）
      const newLines = lines.map((line, idx) =>
        idx === lines.length - 1
          ? { ...line, isThinking: false, thinkingDone: true, status: null }
          : line
      );
      return setL1Stream(state, streamKey, newLines);
    });
  },

  addLine: (streamKey, type, content) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const newLine: ThoughtLine = {
        id: newId(),
        type,
        time: tStamp(),
        content,
        isThinking: false,
        thinkingDone: false,
        status: null,
        tools: [],
        hadToolCall: false,
        toolFailed: false,
        ended: false,
        createdAt: Date.now(),
      };
      return setL1Stream(state, streamKey, [...lines, newLine]);
    });
  },

  startThinkingSession: (streamKey, thinkingLabel = '思考中') => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const lastLine = lines[lines.length - 1];
      if (lastLine && !lastLine.ended) {
        // 复用当前行
        const newLines = lines.map((line, idx) =>
          idx === lines.length - 1
            ? { ...line, type: thinkingLabel, time: tStamp(), isThinking: true, status: null }
            : line
        );
        return setL1Stream(state, streamKey, newLines);
      }
      // 新建思考行
      const newLine: ThoughtLine = {
        id: newId(),
        type: thinkingLabel,
        time: tStamp(),
        isThinking: true,
        thinkingDone: false,
        status: null,
        tools: [],
        hadToolCall: false,
        toolFailed: false,
        ended: false,
        createdAt: Date.now(),
      };
      return setL1Stream(state, streamKey, [...lines, newLine]);
    });
  },

  startThinking: (streamKey) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      let touched = false;
      const newLines = lines.map((line) => {
        if (!touched && !line.ended && !line.thinkingDone) {
          touched = true;
          return { ...line, isThinking: true };
        }
        return line;
      });
      if (!touched) {
        // 没有可用行，新建一行
        const newLine: ThoughtLine = {
          id: newId(),
          type: '思考中',
          time: tStamp(),
          isThinking: true,
          thinkingDone: false,
          status: null,
          tools: [],
          hadToolCall: false,
          toolFailed: false,
          ended: false,
          createdAt: Date.now(),
        };
        return setL1Stream(state, streamKey, [...newLines, newLine]);
      }
      return setL1Stream(state, streamKey, newLines);
    });
  },

  stopThinking: (streamKey, doneLabel) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      let touched = false;
      const newLines = lines.map((line) => {
        if (!touched && line.isThinking) {
          touched = true;
          return {
            ...line,
            isThinking: false,
            thinkingDone: true,
            type: doneLabel || line.type,
            status: null,
          };
        }
        return line;
      });
      return setL1Stream(state, streamKey, newLines);
    });
  },

  setStatus: (streamKey, text, kind = 'busy') => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      let touched = false;
      const newLines = lines.map((line) => {
        if (!touched && !line.ended) {
          touched = true;
          return { ...line, status: { text, kind } };
        }
        return line;
      });
      if (!touched) {
        // 没有可用行，先新建"思考中"行
        const newLine: ThoughtLine = {
          id: newId(),
          type: '思考中',
          time: tStamp(),
          isThinking: false,
          thinkingDone: false,
          status: { text, kind },
          tools: [],
          hadToolCall: false,
          toolFailed: false,
          ended: false,
          createdAt: Date.now(),
        };
        return setL1Stream(state, streamKey, [...newLines, newLine]);
      }
      return setL1Stream(state, streamKey, newLines);
    });
  },

  setTimedStatus: (streamKey, text, kind, options) => {
    const staleAfterMs = Number(options.staleAfterMs || 0);
    const staleText = options.staleText || text;
    const staleAt = staleAfterMs ? Date.now() + staleAfterMs : 0;
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      let touched = false;
      const newLines = lines.map((line) => {
        if (!touched && !line.ended) {
          touched = true;
          return {
            ...line,
            status: { text, kind, staleAt, staleText },
          };
        }
        return line;
      });
      if (!touched) {
        const newLine: ThoughtLine = {
          id: newId(),
          type: '思考中',
          time: tStamp(),
          isThinking: false,
          thinkingDone: false,
          status: { text, kind, staleAt, staleText },
          tools: [],
          hadToolCall: false,
          toolFailed: false,
          ended: false,
          createdAt: Date.now(),
        };
        return setL1Stream(state, streamKey, [...newLines, newLine]);
      }
      return setL1Stream(state, streamKey, newLines);
    });
  },

  clearStatus: (streamKey) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const newLines = lines.map((line) =>
        !line.ended ? { ...line, status: null } : line
      );
      return setL1Stream(state, streamKey, newLines);
    });
  },

  addToolCall: (streamKey, name, args, result, ok) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const resultStr = result == null ? '' : String(result);
      const failure = ok === false || (ok !== true && isFailureResult(resultStr));
      const status: ToolStatus | 'running' = failure ? 'failed' : 'success';
      const subject = formatToolSubject(name, args, null);
      const detail = formatToolDetail(name, args, resultStr);
      const expandable = Boolean(detail);

      const newTool: ToolCall = {
        id: newId('tool'),
        name,
        args,
        result,
        ok: failure ? false : ok ?? true,
        status,
        subject,
        detail,
        expandable,
        expanded: false,
        createdAt: Date.now(),
      };

      let touched = false;
      const newLines = lines.map((line) => {
        if (!touched && !line.ended) {
          touched = true;
          return {
            ...line,
            isThinking: false,
            thinkingDone: true,
            status: null,
            hadToolCall: true,
            toolFailed: line.toolFailed || failure,
            tools: [...line.tools, newTool],
          };
        }
        return line;
      });

      if (!touched) {
        // 没有可用行，新建"工具调用"行
        const newLine: ThoughtLine = {
          id: newId(),
          type: '工具调用',
          time: tStamp(),
          isThinking: false,
          thinkingDone: true,
          status: null,
          tools: [newTool],
          hadToolCall: true,
          toolFailed: failure,
          ended: false,
          createdAt: Date.now(),
        };
        return setL1Stream(state, streamKey, [...newLines, newLine]);
      }
      return setL1Stream(state, streamKey, newLines);
    });
  },

  appendToolCycleEnd: (streamKey) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const lastLine = lines[lines.length - 1];
      const toolFailed = lastLine?.toolFailed || false;
      const hadToolCall = lastLine?.hadToolCall || false;

      const endTool: ToolCall = {
        id: newId('tool'),
        name: '__cycle_end__',
        args: {},
        result: null,
        ok: !toolFailed,
        status: toolFailed ? 'failed' : 'ended',
        subject: '',
        detail: '',
        expandable: false,
        expanded: false,
        createdAt: Date.now(),
      };

      const newLines = lines.map((line, idx) => {
        if (idx === lines.length - 1 && !line.ended) {
          return {
            ...line,
            tools: [...line.tools, endTool],
            ended: true,
          };
        }
        return line;
      });

      // 如果最后一行已 ended，新建一行
      if (newLines[newLines.length - 1]?.ended) {
        return setL1Stream(state, streamKey, newLines);
      }
      const endLine: ThoughtLine = {
        id: newId(),
        type: hadToolCall ? '工具调用结束' : '本轮结束',
        time: tStamp(),
        isThinking: false,
        thinkingDone: true,
        status: null,
        tools: [endTool],
        hadToolCall,
        toolFailed,
        ended: true,
        createdAt: Date.now(),
      };
      return setL1Stream(state, streamKey, [...newLines, endLine]);
    });
  },

  endRound: (streamKey) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const newLines = lines.map((line, idx) => {
        if (idx === lines.length - 1 && !line.ended) {
          return {
            ...line,
            isThinking: false,
            thinkingDone: true,
            status: null,
            ended: true,
          };
        }
        return line;
      });
      return setL1Stream(state, streamKey, newLines);
    });
  },

  toggleToolExpand: (streamKey, lineId, toolId) => {
    set((state) => {
      const lines = getL1Stream(state, streamKey);
      const newLines = lines.map((line) => {
        if (line.id !== lineId) return line;
        return {
          ...line,
          tools: line.tools.map((tool) =>
            tool.id === toolId ? { ...tool, expanded: !tool.expanded } : tool
          ),
        };
      });
      return setL1Stream(state, streamKey, newLines);
    });
  },

  clearStream: (streamKey) => {
    set((state) => {
      if (streamKey === L2_STREAM_KEY) {
        return { l2Stream: [] };
      }
      const newMap = new Map(state.l1Streams);
      newMap.delete(streamKey);
      return { l1Streams: newMap };
    });
  },

  // ─── 路由 Actions ───

  setCurrentPath: (path) => set({ currentPath: path }),

  setActiveL1Key: (clientId) => set({ activeL1Key: clientId, currentPath: 'l1' }),

  // ─── AI 活动状态 Actions ───

  recordAiActivity: (name) => {
    if (!name) return;
    const now = Date.now();
    set((state) => {
      const log = state.aiActivityLog;
      const firstTs = log.length === 0 ? now : state.aiActivityFirstTs;
      return {
        aiActivityLog: [...log, { name, ts: now, group: classifyTool(name) }],
        aiActivityFirstTs: firstTs,
      };
    });
  },

  clearAiActivity: () => set({ aiActivityLog: [], aiActivityFirstTs: 0 }),

  // ─── Tauri IPC 对接：监听后端 'thought_pushed' 事件 ───

  subscribeToBackendEvents: async () => {
    // 幂等：已订阅则不重复订阅
    if (get()._backendEventUnlisten) return;
    try {
      const unlisten = await listen<{ subject?: string; description?: string; content?: string }>(
        'thought_pushed',
        (event) => {
          const payload = event.payload || {};
          const subject = typeof payload.subject === 'string' ? payload.subject : '';
          const description = typeof payload.description === 'string' ? payload.description : '';
          const content = typeof payload.content === 'string' ? payload.content : '';
          // 后端推送的思考块 → 写入 L2 心跳流（自主行动流）
          const label = subject || description ? `思考中: ${subject || description}` : '后端思考';
          get().addLine(L2_STREAM_KEY, label, content || description);
          get().recordAiActivity('backend_thought');
        },
      );
      set({ _backendEventUnlisten: unlisten });
    } catch (e) {
      // 非 Tauri 环境或事件未注册，静默 no-op
      console.warn('[thinkingStore] subscribeToBackendEvents failed, no-op:', e);
    }
  },

  unsubscribeFromBackendEvents: () => {
    const unlisten = get()._backendEventUnlisten;
    if (unlisten) {
      try {
        unlisten();
      } catch {
        /* ignore */
      }
      set({ _backendEventUnlisten: null });
    }
  },
}));

// ─── AI 活动状态派生 selector（组件使用） ───

/**
 * 派生 AI 活动状态（60s 滑动窗口 + 15s 空闲阈值）
 *
 * @returns AiActivityState
 *   - state: 'idle' | 'busy'
 *   - label: '空闲' | '刚完成' | '正在扫描文件' 等
 *   - detail: '· 3 次工具 · 12s' | '5s 前停止'
 *   - count: 60s 窗口内工具调用次数
 *   - elapsedSec: 持续秒数
 */
export function deriveAiActivity(
  log: AiActivityEntry[],
  firstTs: number,
  now: number = Date.now()
): AiActivityState {
  // 滑动窗口：移除 60s 之前的记录
  const windowed = log.filter((e) => now - e.ts <= AI_ACTIVITY_WINDOW_MS);

  if (windowed.length === 0) {
    return { state: 'idle', label: '空闲', detail: '', count: 0, elapsedSec: 0 };
  }

  const lastTs = windowed[windowed.length - 1].ts;

  // 15s 无活动 → 空闲
  if (now - lastTs > AI_ACTIVITY_IDLE_AFTER_MS) {
    const ago = Math.round((now - lastTs) / 1000);
    return {
      state: 'idle',
      label: '刚完成',
      detail: `${ago}s 前停止`,
      count: windowed.length,
      elapsedSec: Math.round((now - (firstTs || lastTs)) / 1000),
    };
  }

  // 统计窗口内 dominant 分组
  const counts: Record<string, number> = {};
  for (const e of windowed) {
    counts[e.group] = (counts[e.group] || 0) + 1;
  }
  let domGroup = '处理事务';
  let domCount = 0;
  for (const [g, c] of Object.entries(counts)) {
    if (c > domCount) {
      domCount = c;
      domGroup = g;
    }
  }

  const elapsed = Math.round((now - (firstTs || lastTs)) / 1000);
  return {
    state: 'busy',
    label: `正在${domGroup}`,
    detail: `· ${windowed.length} 次工具 · ${elapsed}s`,
    count: windowed.length,
    elapsedSec: elapsed,
  };
}
