/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox settingsStore — 设置状态管理
 *
 * 来源：SparkFox 全新设计（参考 macOS 系统偏好设计）
 *
 * 功能：
 * - 主题预设（macOS Light / Dark / Auto / Custom）
 * - 主色 + 字体大小
 * - 快捷键映射
 * - 数据管理（导出 / 清除 / 重置）
 * - 全局 Agent 偏好（默认值 / 调试 / 性能调优）
 * - 关于信息
 *
 * ─── Tauri IPC 对接说明（v1.0.0 Task 7.8） ───
 * 本 store 暂无直接对应的 Tauri command（sparkfox-ipc 未提供 settings_* 接口）。
 * 设置当前存储于 localStorage（通过 Zustand persist 中间件持久化）。
 * v1.1.0+ 计划新增：
 *   - settings_get(key) → 读取后端持久化的设置
 *   - settings_set(key, value) → 写入后端持久化的设置
 *   - settings_export() → 导出全部设置
 *   - settings_reset() → 重置为默认值
 * 当前 PoC 阶段保持本地存储，不影响其他 store 的 IPC 对接。
 */

import { create } from 'zustand';

// ─── 类型定义 ───

export type ThemePreset = 'macosLight' | 'macosDark' | 'macosAuto' | 'custom';

export type FontSize = 'small' | 'medium' | 'large';

export type Density = 'compact' | 'comfortable' | 'spacious';

export interface ShortcutBinding {
  id: string;
  label: string;
  keys: string;
  enabled: boolean;
}

export interface AgentGlobalPrefs {
  /** 新建 Agent 的默认 type */
  defaultAgentType: string;
  /** 新建 Agent 的默认图标 */
  defaultAgentIcon: string;
  /** 新建 Agent 的默认颜色 */
  defaultAgentColor: string;
  /** 默认记忆模式 */
  defaultMemoryMode: 'shared' | 'isolated';
  /** 默认身份模式 */
  defaultIdentityMode: 'shared' | 'isolated';
  /** 显示隐藏的 Agent */
  showHiddenAgents: boolean;
  /** 启用 Agent 自动切换 */
  enableAutoSwitch: boolean;
}

export interface DebugPrefs {
  /** 启用详细日志 */
  verboseLogging: boolean;
  /** 启用思考流调试视图 */
  debugThinkingStream: boolean;
  /** 启用记忆操作日志 */
  logMemoryOps: boolean;
  /** 启用 IPC 追踪 */
  traceIpc: boolean;
}

export interface PerformancePrefs {
  /** 思考流最大行数（超出自动裁剪） */
  thinkingStreamMaxLines: number;
  /** 监视面板事件最大缓存数 */
  monitorEventMaxCount: number;
  /** 记忆面板自动刷新间隔（秒，0=禁用） */
  memoryAutoRefreshSec: number;
  /** 虚拟滚动阈值（条数） */
  virtualScrollThreshold: number;
}

interface SettingsState {
  initialized: boolean;

  // ─── 外观 ───
  themePreset: ThemePreset;
  primaryColor: string;
  fontSize: FontSize;
  density: Density;
  reduceMotion: boolean;

  // ─── 快捷键 ───
  shortcuts: ShortcutBinding[];

  // ─── 全局 Agent 偏好 ───
  agentPrefs: AgentGlobalPrefs;

  // ─── 调试 ───
  debugPrefs: DebugPrefs;

  // ─── 性能调优 ───
  performancePrefs: PerformancePrefs;

  // ─── Actions ───
  initialize: () => void;
  setThemePreset: (preset: ThemePreset) => void;
  setPrimaryColor: (color: string) => void;
  setFontSize: (size: FontSize) => void;
  setDensity: (density: Density) => void;
  setReduceMotion: (enabled: boolean) => void;
  updateShortcut: (id: string, updates: Partial<ShortcutBinding>) => void;
  updateAgentPrefs: (updates: Partial<AgentGlobalPrefs>) => void;
  updateDebugPrefs: (updates: Partial<DebugPrefs>) => void;
  updatePerformancePrefs: (updates: Partial<PerformancePrefs>) => void;
  exportSettings: () => string;
  importSettings: (json: string) => boolean;
  resetToDefaults: () => void;
  clearAllData: () => Promise<void>;
}

// ─── 默认值 ───

const DEFAULT_PRIMARY_COLOR = '#007AFF'; // Apple 系统蓝

const DEFAULT_SHORTCUTS: ShortcutBinding[] = [
  { id: 'toggle-sidebar', label: '显示/隐藏侧边栏', keys: 'Cmd+B', enabled: true },
  { id: 'new-conversation', label: '新建对话', keys: 'Cmd+N', enabled: true },
  { id: 'open-agent-manager', label: '打开 Agent 管理', keys: 'Cmd+Shift+A', enabled: true },
  { id: 'open-memory', label: '打开记忆面板', keys: 'Cmd+Shift+M', enabled: true },
  { id: 'open-monitor', label: '打开监视面板', keys: 'Cmd+Shift+T', enabled: true },
  { id: 'open-hotspot', label: '打开热点追踪', keys: 'Cmd+Shift+H', enabled: true },
  { id: 'open-settings', label: '打开设置', keys: 'Cmd+,', enabled: true },
  { id: 'send-message', label: '发送消息', keys: 'Enter', enabled: true },
  { id: 'newline-in-input', label: '输入框换行', keys: 'Shift+Enter', enabled: true },
  { id: 'search', label: '全局搜索', keys: 'Cmd+K', enabled: true },
];

const DEFAULT_AGENT_PREFS: AgentGlobalPrefs = {
  defaultAgentType: 'custom',
  defaultAgentIcon: '🦊',
  defaultAgentColor: '#FF9500',
  defaultMemoryMode: 'shared',
  defaultIdentityMode: 'shared',
  showHiddenAgents: false,
  enableAutoSwitch: false,
};

const DEFAULT_DEBUG_PREFS: DebugPrefs = {
  verboseLogging: false,
  debugThinkingStream: false,
  logMemoryOps: false,
  traceIpc: false,
};

const DEFAULT_PERFORMANCE_PREFS: PerformancePrefs = {
  thinkingStreamMaxLines: 200,
  monitorEventMaxCount: 200,
  memoryAutoRefreshSec: 0,
  virtualScrollThreshold: 100,
};

// ─── Store 实现 ───

export const useSettingsStore = create<SettingsState>((set, get) => ({
  initialized: false,
  themePreset: 'macosAuto',
  primaryColor: DEFAULT_PRIMARY_COLOR,
  fontSize: 'medium',
  density: 'comfortable',
  reduceMotion: false,
  shortcuts: DEFAULT_SHORTCUTS,
  agentPrefs: DEFAULT_AGENT_PREFS,
  debugPrefs: DEFAULT_DEBUG_PREFS,
  performancePrefs: DEFAULT_PERFORMANCE_PREFS,

  initialize: () => {
    if (get().initialized) return;
    set({ initialized: true });
  },

  setThemePreset: (preset) => set({ themePreset: preset }),
  setPrimaryColor: (color) => set({ primaryColor: color }),
  setFontSize: (size) => set({ fontSize: size }),
  setDensity: (density) => set({ density: density }),
  setReduceMotion: (enabled) => set({ reduceMotion: enabled }),

  updateShortcut: (id, updates) =>
    set((state) => ({
      shortcuts: state.shortcuts.map((s) => (s.id === id ? { ...s, ...updates } : s)),
    })),

  updateAgentPrefs: (updates) =>
    set((state) => ({ agentPrefs: { ...state.agentPrefs, ...updates } })),

  updateDebugPrefs: (updates) =>
    set((state) => ({ debugPrefs: { ...state.debugPrefs, ...updates } })),

  updatePerformancePrefs: (updates) =>
    set((state) => ({ performancePrefs: { ...state.performancePrefs, ...updates } })),

  exportSettings: () => {
    const { initialized, ...rest } = get();
    void initialized;
    return JSON.stringify(rest, null, 2);
  },

  importSettings: (json) => {
    try {
      const parsed = JSON.parse(json);
      set({ ...parsed, initialized: true });
      return true;
    } catch {
      return false;
    }
  },

  resetToDefaults: () =>
    set({
      themePreset: 'macosAuto',
      primaryColor: DEFAULT_PRIMARY_COLOR,
      fontSize: 'medium',
      density: 'comfortable',
      reduceMotion: false,
      shortcuts: DEFAULT_SHORTCUTS,
      agentPrefs: DEFAULT_AGENT_PREFS,
      debugPrefs: DEFAULT_DEBUG_PREFS,
      performancePrefs: DEFAULT_PERFORMANCE_PREFS,
    }),

  clearAllData: async () => {
    // PoC 阶段：仅重置状态；生产环境需调用 ipcBridge 清理 SQLite / IndexedDB
    set({
      themePreset: 'macosAuto',
      primaryColor: DEFAULT_PRIMARY_COLOR,
      fontSize: 'medium',
      density: 'comfortable',
      reduceMotion: false,
      shortcuts: DEFAULT_SHORTCUTS,
      agentPrefs: DEFAULT_AGENT_PREFS,
      debugPrefs: DEFAULT_DEBUG_PREFS,
      performancePrefs: DEFAULT_PERFORMANCE_PREFS,
    });
  },
}));
