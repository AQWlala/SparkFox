/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * chatStore: 对话状态管理（基于 BaiLongma chat.js 重写为 Zustand store）
 *
 * 保留 BaiLongma 5 大核心特性：
 * 1. 双重去重（renderedMessageIds + recentRenderedKeys + TTL 2min）
 * 2. 流式气泡（liveEl 机制改为 liveMessageId state）
 * 3. 自适应输入框（autoGrowInput）
 * 4. 空闲占位符切换（idlePlaceholder）
 * 5. 频道标签（friendlyChannelLabel: WeChat/Feishu）
 *
 * 移除 5 大 BaiLongma 特有逻辑：
 * 1. 音频上下文（ensureAudioContext / playJarvisAlert）
 * 2. 激活预热锁（applyActivationWarmupLock / sessionStorage）
 * 3. 按住空格说话（PUSH_TO_TALK_PLACEHOLDER）— 保留占位文案但改为 SparkFox 提示
 * 4. 自动关闭定时器（scheduleClose / closeChat）
 * 5. Electron ipcRenderer（ui-bridge.js）
 */

import { create } from 'zustand';
// Tauri 2 IPC：knowledge_search 检索知识库（失败返回空数组，不影响对话流）
import { invoke } from '@tauri-apps/api/core';

// ─── 类型定义 ───

export type ChatRole = 'user' | 'assistant' | 'system';

export type ChatChannel = 'wechat' | 'feishu' | 'sparkfox' | '';

export interface ChatAttachment {
  id: string;
  type: 'image' | 'file';
  name: string;
  mime: string;
  size: number;
  dataUrl?: string; // 图片预览
}

export interface ChatMessage {
  id: string; // 服务端消息 ID（可为空，本地生成）
  clientId: string; // 客户端消息 ID（本地去重）
  role: ChatRole;
  content: string;
  channel: ChatChannel;
  label: string; // 频道友好标签
  createdAt: number; // 毫秒时间戳
  attachments: ChatAttachment[];
  isStreaming?: boolean; // 是否正在流式输出
  thoughtStreamSlot?: React.ReactNode; // ThoughtStream 插槽（P0-模块 F 接入点）
}

export interface ChatInputState {
  value: string;
  isLocked: boolean;
  lockReason: string;
  isFocused: boolean;
  attachments: ChatAttachment[];
}

// ─── 常量 ───

/** 双重去重 TTL（2 分钟） */
export const RENDER_DEDUPE_TTL_MS = 2 * 60 * 1000;

/** 最大粘贴图片数 */
export const MAX_PASTED_IMAGES = 8;

/** 最大图片字节 */
export const MAX_PASTED_IMAGE_BYTES = 12 * 1024 * 1024;

/** 按住空格说话占位符（SparkFox 改为"即将支持"提示） */
export const PUSH_TO_TALK_PLACEHOLDER = '按住空格键开始说话（即将支持）';

/** 默认输入占位符 */
export const DEFAULT_INPUT_PLACEHOLDER = '给 SparkFox 发消息…';

// ─── Store 接口 ───

interface ChatState {
  // 状态
  messages: ChatMessage[];
  input: ChatInputState;
  liveMessageId: string | null; // 正在流式输出的消息 ID（替代 BaiLongma liveEl）
  isHovering: boolean;

  // 去重内部状态（不暴露给组件）
  _renderedMessageIds: Set<string>;
  _recentRenderedKeys: Map<string, number>;
  _pendingLocalSends: Set<string>;

  // Actions
  addMessage: (msg: Omit<ChatMessage, 'clientId' | 'createdAt' | 'attachments'> & Partial<Pick<ChatMessage, 'clientId' | 'createdAt' | 'attachments'>>) => boolean;
  claimRenderedMessage: (params: {
    messageId?: string;
    role: ChatRole;
    text: string;
    label?: string;
    source?: 'event' | 'history';
    dedupe?: boolean;
  }) => boolean;
  updateMessage: (clientId: string, updates: Partial<ChatMessage>) => void;
  appendToLiveMessage: (clientId: string, chunk: string) => void;
  finalizeLiveMessage: (clientId: string) => void;
  clearMessages: () => void;
  setInputValue: (value: string) => void;
  setInputLocked: (locked: boolean, reason?: string) => void;
  setInputFocused: (focused: boolean) => void;
  addAttachment: (attachment: ChatAttachment) => boolean;
  removeAttachment: (id: string) => void;
  clearAttachments: () => void;
  setHovering: (hovering: boolean) => void;
  getIdlePlaceholder: () => string;

  // ─── Tauri IPC 对接（渐进增强：失败时返回空数组，不破坏对话流） ───
  /**
   * 调用后端知识库检索（Tauri IPC: knowledge_search）
   * @param query 查询文本
   * @param mode 检索模式（如 'semantic' / 'keyword' / 'hybrid'），默认 'semantic'
   * @returns 检索结果数组（后端返回 serde_json::Value[]）；失败返回空数组
   */
  searchKnowledge: (query: string, mode?: string) => Promise<unknown[]>;
}

// ─── 工具函数 ───

/** 频道友好标签（仅保留 WeChat + Feishu，符合中国市场定位） */
export function friendlyChannelLabel(channel: ChatChannel | string): string {
  if (!channel) return '';
  const c = String(channel).toUpperCase();
  if (c === 'WECHAT_CLAWBOT' || c === 'WECHAT_OFFICIAL' || c === 'WECHAT' || c === 'WECHAT') return 'WeChat';
  if (c === 'FEISHU') return 'Feishu';
  return '';
}

/** 生成客户端消息 ID */
export function newClientMessageId(): string {
  return `sf-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

/** 归一化消息 ID */
function normalizeMessageId(value: unknown): string {
  if (value === undefined || value === null || value === '') return '';
  return String(value);
}

/** 生成去重键 */
function renderedKey(role: ChatRole, text: string, label: string): string {
  const cleanText = String(text || '').replace(/\s+/g, ' ').trim();
  if (!cleanText) return '';
  return `${role}\n${label}\n${cleanText}`;
}

/** 清理过期去重键 */
function pruneRenderedKeys(map: Map<string, number>, now: number = Date.now()): void {
  for (const [key, ts] of map) {
    if (now - ts > RENDER_DEDUPE_TTL_MS) map.delete(key);
  }
}

/** 本地发送去重键 */
export function localSendKey(channel: ChatChannel, text: string, attachments: ChatAttachment[]): string {
  const attKey = attachments.map(a => a.id).join(',');
  return `${channel}|${text}|${attKey}`;
}

// ─── Store 实现 ───

export const useChatStore = create<ChatState>((set, get) => ({
  messages: [],
  input: {
    value: '',
    isLocked: false,
    lockReason: '',
    isFocused: false,
    attachments: [],
  },
  liveMessageId: null,
  isHovering: false,
  _renderedMessageIds: new Set(),
  _recentRenderedKeys: new Map(),
  _pendingLocalSends: new Set(),

  addMessage: (msg) => {
    const clientId = msg.clientId || newClientMessageId();
    const createdAt = msg.createdAt || Date.now();
    const attachments = msg.attachments || [];
    const fullMsg: ChatMessage = {
      ...msg,
      clientId,
      createdAt,
      attachments,
    };
    set(state => ({
      messages: [...state.messages, fullMsg],
    }));
    return true;
  },

  claimRenderedMessage: ({ messageId, role, text, label = '', source = 'event', dedupe = true }) => {
    const state = get();
    const id = normalizeMessageId(messageId);
    const now = Date.now();
    pruneRenderedKeys(state._recentRenderedKeys, now);

    if (id && state._renderedMessageIds.has(id)) return false;

    const key = renderedKey(role, text, label);
    const allowContentDedupe = source === 'history' || !id;
    if (dedupe && allowContentDedupe && key && state._recentRenderedKeys.has(key)) {
      if (id) state._renderedMessageIds.add(id);
      return false;
    }

    if (id) state._renderedMessageIds.add(id);
    if (key && (!id || dedupe === false)) state._recentRenderedKeys.set(key, now);
    return true;
  },

  updateMessage: (clientId, updates) => {
    set(state => ({
      messages: state.messages.map(m => (m.clientId === clientId ? { ...m, ...updates } : m)),
    }));
  },

  appendToLiveMessage: (clientId, chunk) => {
    set(state => ({
      messages: state.messages.map(m =>
        m.clientId === clientId ? { ...m, content: m.content + chunk, isStreaming: true } : m
      ),
      liveMessageId: clientId,
    }));
  },

  finalizeLiveMessage: (clientId) => {
    set(state => ({
      messages: state.messages.map(m =>
        m.clientId === clientId ? { ...m, isStreaming: false } : m
      ),
      liveMessageId: null,
    }));
  },

  clearMessages: () => {
    set({
      messages: [],
      liveMessageId: null,
      _renderedMessageIds: new Set(),
      _recentRenderedKeys: new Map(),
      _pendingLocalSends: new Set(),
    });
  },

  setInputValue: (value) => {
    set(state => ({ input: { ...state.input, value } }));
  },

  setInputLocked: (locked, reason = '') => {
    set(state => ({
      input: { ...state.input, isLocked: locked, lockReason: reason },
    }));
  },

  setInputFocused: (focused) => {
    set(state => ({ input: { ...state.input, isFocused: focused } }));
  },

  addAttachment: (attachment) => {
    const state = get();
    if (state.input.attachments.length >= MAX_PASTED_IMAGES) return false;
    if (attachment.size > MAX_PASTED_IMAGE_BYTES) return false;
    set(state => ({
      input: { ...state.input, attachments: [...state.input.attachments, attachment] },
    }));
    return true;
  },

  removeAttachment: (id) => {
    set(state => ({
      input: { ...state.input, attachments: state.input.attachments.filter(a => a.id !== id) },
    }));
  },

  clearAttachments: () => {
    set(state => ({ input: { ...state.input, attachments: [] } }));
  },

  setHovering: (hovering) => {
    set({ isHovering: hovering });
  },

  getIdlePlaceholder: () => {
    const state = get();
    return state.input.isFocused ? DEFAULT_INPUT_PLACEHOLDER : PUSH_TO_TALK_PLACEHOLDER;
  },

  // ─── Tauri IPC 对接：知识库检索，失败返回空数组 ───

  searchKnowledge: async (query, mode = 'semantic') => {
    if (!query || !query.trim()) return [];
    try {
      const result = await invoke<unknown[]>('knowledge_search', { query, mode });
      return Array.isArray(result) ? result : [];
    } catch (e) {
      // 非 Tauri 环境或 command 未注册，静默返回空数组
      console.warn('[chatStore] searchKnowledge failed, returning empty:', e);
      return [];
    }
  },
}));
