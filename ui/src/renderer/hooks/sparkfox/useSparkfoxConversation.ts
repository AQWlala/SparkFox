/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox 对话编排器 — 统一管理所有数据流
 *
 * 串联的数据流：
 * - 数据流1：对话 → 记忆存储（用户消息 + AI 回复 → memoryStore）
 * - 数据流2：记忆 → 对话注入（新对话开始 → 检索相关记忆注入上下文）
 * - 数据流3：Agent 菜单 → Agent 调度 → 监视面板（Agent 选择 → 执行 → 状态推送）
 * - 数据流4：思考过程 → 对话展示（AI 响应 → thinkingStore → ChatMessage）
 * - 数据流5：热点 → 对话讨论（HotspotView → 发送到对话页）
 *
 * 使用方式：在 ChatPanel 中调用 useSparkfoxConversation()，获取 handleSend / handleAIReply / handleHotspotInject 等方法
 */

import { useCallback, useRef, useEffect } from 'react';
import { useChatStore, newClientMessageId, type ChatAttachment } from '@renderer/store/chatStore';
import { useMemoryStore, type MemoryLayer, type MemoryType } from '@renderer/store/memoryStore';
import { useAgentStore } from '@renderer/store/agentStore';
import { useMonitorStore } from '@renderer/store/monitorStore';
import { useHotspotStore } from '@renderer/store/hotspotStore';
import { useThinkingStore, L2_STREAM_KEY } from '@renderer/store/thinkingStore';
import {
  extractFromUserMessage,
  extractTechPreferences,
  extractEpisode,
  isNoiseMessage,
  type ExtractedMemory,
} from '@renderer/utils/sparkfox/memoryExtractor';

// ============================================================================
// Hook 定义
// ============================================================================

export interface SparkfoxConversationOptions {
  /** 是否启用记忆存储（默认 true） */
  enableMemoryStorage?: boolean;
  /** 是否启用记忆注入（默认 true） */
  enableMemoryInjection?: boolean;
  /** 是否启用思考过程记录（默认 true） */
  enableThinkingStream?: boolean;
  /** 是否启用监视面板事件推送（默认 true） */
  enableMonitorPush?: boolean;
}

export function useSparkfoxConversation(options: SparkfoxConversationOptions = {}) {
  const {
    enableMemoryStorage = true,
    enableMemoryInjection = true,
    enableThinkingStream = true,
    enableMonitorPush = true,
  } = options;

  // ─── Store actions（用 selector 避免重渲染） ───
  const addMessage = useChatStore((s) => s.addMessage);
  const finalizeLiveMessage = useChatStore((s) => s.finalizeLiveMessage);
  const messages = useChatStore((s) => s.messages);
  const messagesRef = useRef(messages);
  messagesRef.current = messages;

  const injectFromConversation = useMemoryStore((s) => s.injectFromConversation);
  const addMemory = useMemoryStore((s) => s.addMemory);
  const memoryInitialize = useMemoryStore((s) => s.initialize);
  const memories = useMemoryStore((s) => s.memories);

  const currentAgentId = useAgentStore((s) => s.currentAgentId);
  const agents = useAgentStore((s) => s.agents);

  const addActivity = useMonitorStore((s) => s.addActivity);
  const monitorInitialize = useMonitorStore((s) => s.initialize);

  const hotspotBuildContext = useHotspotStore((s) => s.buildContext);
  const pendingChatInjection = useHotspotStore((s) => s.pendingChatInjection);
  const clearPendingChatInjection = useHotspotStore((s) => s.clearPendingChatInjection);

  const beginRound = useThinkingStore((s) => s.beginRound);
  const startThinkingSession = useThinkingStore((s) => s.startThinkingSession);
  const stopThinking = useThinkingStore((s) => s.stopThinking);
  const endRound = useThinkingStore((s) => s.endRound);
  const setActiveL1Key = useThinkingStore((s) => s.setActiveL1Key);

  // ─── 初始化：确保所有 store 已初始化 ───
  useEffect(() => {
    memoryInitialize();
    monitorInitialize();
  }, [memoryInitialize, monitorInitialize]);

  // ─── 数据流1：对话 → 记忆存储 ───
  /**
   * 从用户消息中提取记忆并存储
   * - 短期记忆 L1：所有用户消息（非噪音）存入 L1（session 作用域）
   * - 长期记忆 L3：偏好 + 事实存入 L3（global 作用域）
   */
  const storeUserMessageToMemory = useCallback(
    (userMessage: string) => {
      if (!enableMemoryStorage) return;
      if (isNoiseMessage(userMessage)) return;

      // 1. 短期记忆 L1：存入当前对话上下文
      injectFromConversation(
        `用户：${userMessage.slice(0, 500)}`,
        'context' as MemoryType,
        'L1' as MemoryLayer,
      ).catch(() => {});

      // 2. 长期记忆 L3：提取偏好 + 事实
      const extracted: ExtractedMemory[] = [
        ...extractFromUserMessage(userMessage),
        ...extractTechPreferences(userMessage),
      ];

      for (const mem of extracted) {
        addMemory({
          layer: mem.layer,
          type: mem.type as MemoryType,
          content: mem.content,
          subject: mem.subject,
          predicate: mem.predicate,
          object: mem.object,
          source: '对话',
          importance_score: 0.7,
          confidence: mem.confidence,
          scope: 'global',
          priority: 'long_term',
        }).catch(() => {});
      }
    },
    [enableMemoryStorage, injectFromConversation, addMemory],
  );

  /**
   * 从完整对话中提取情节记忆并存储（AI 回复完成后调用）
   * - 情节记忆 L2：完整交互（用户问 + AI 答）
   */
  const storeEpisodeToMemory = useCallback(
    (userMessage: string, aiReply: string) => {
      if (!enableMemoryStorage) return;

      const episode = extractEpisode(userMessage, aiReply);
      if (!episode) return;

      injectFromConversation(
        episode.content,
        'episode' as MemoryType,
        'L2' as MemoryLayer,
      ).catch(() => {});
    },
    [enableMemoryStorage, injectFromConversation],
  );

  // ─── 数据流2：记忆 → 对话注入 ───
  /**
   * 根据当前用户消息，从记忆中检索相关信息，构建注入上下文
   * - 检索偏好记忆 → 注入到系统提示
   * - 检索相关事实/情节 → 注入到对话上下文
   */
  const buildMemoryContext = useCallback(
    (userMessage: string): string => {
      if (!enableMemoryInjection) return '';
      if (!memories.length) return '';

      const parts: string[] = [];

      // 1. 用户偏好（所有 preference 类型）
      const preferences = memories.filter((m) => m.type === 'preference' && m.scope === 'global');
      if (preferences.length) {
        const prefText = preferences
          .slice(0, 10)
          .map((m) => `- ${m.content}`)
          .join('\n');
        parts.push(`## 用户偏好（来自记忆系统）\n${prefText}`);
      }

      // 2. 用户事实（fact 类型）
      const facts = memories.filter((m) => m.type === 'fact' && m.scope === 'global');
      if (facts.length) {
        const factText = facts
          .slice(0, 8)
          .map((m) => `- ${m.content}`)
          .join('\n');
        parts.push(`## 用户信息（来自记忆系统）\n${factText}`);
      }

      // 3. 相关情节记忆（简单关键词匹配）
      const userKeywords = userMessage
        .replace(/[^\p{Script=Han}a-zA-Z0-9]+/gu, ' ')
        .split(/\s+/)
        .filter((s) => s.length >= 2)
        .slice(0, 5);

      if (userKeywords.length) {
        const relatedEpisodes = memories
          .filter((m) => m.layer === 'L2' && m.type === 'episode')
          .filter((m) => userKeywords.some((k) => m.content.includes(k)))
          .slice(0, 3);

        if (relatedEpisodes.length) {
          const epText = relatedEpisodes
            .map((m) => `- ${m.content.slice(0, 150)}`)
            .join('\n');
          parts.push(`## 相关历史对话（来自记忆系统）\n${epText}`);
        }
      }

      if (!parts.length) return '';

      return `## 记忆系统注入（仅供参考，不代表用户当前请求）
${parts.join('\n\n')}
`;
    },
    [enableMemoryInjection, memories],
  );

  /**
   * 构建热点上下文（数据流5：热点 → 对话）
   */
  const buildHotspotContext = useCallback(
    (userMessage: string): string => {
      return hotspotBuildContext(userMessage);
    },
    [hotspotBuildContext],
  );

  // ─── 数据流3：Agent 调度 → 监视面板 ───
  /**
   * 推送活动事件到监视面板
   */
  const pushMonitorEvent = useCallback(
    (params: {
      type?: 'message_out' | 'message_in' | 'tool_call' | 'tool_result' | 'agent_switch' | 'memory_op' | 'session_start' | 'session_end' | 'error';
      title: string;
      agentName?: string;
      detail?: string;
      tokens?: number;
      duration_ms?: number;
      status?: 'ok' | 'warn' | 'error';
    }) => {
      if (!enableMonitorPush) return;
      const currentAgent = agents.find((a) => a.id === currentAgentId);
      addActivity({
        id: `evt_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
        type: params.type || 'message_out',
        title: params.title,
        agent_name: params.agentName || currentAgent?.name || 'SparkFox',
        detail: params.detail,
        tokens: params.tokens,
        duration_ms: params.duration_ms,
        status: params.status || 'ok',
        timestamp: new Date().toISOString(),
      });
    },
    [enableMonitorPush, agents, currentAgentId, addActivity],
  );

  // ─── 数据流4：思考过程 → 对话展示 ───
  /**
   * 开始一轮新的 AI 思考过程
   */
  const beginThinkingRound = useCallback(
    (aiClientId: string) => {
      if (!enableThinkingStream) return;
      setActiveL1Key(aiClientId);
      beginRound(aiClientId);
      startThinkingSession(aiClientId, '正在思考…');
    },
    [enableThinkingStream, setActiveL1Key, beginRound, startThinkingSession],
  );

  /**
   * 结束 AI 思考过程
   */
  const endThinkingRound = useCallback(
    (aiClientId: string) => {
      if (!enableThinkingStream) return;
      stopThinking(aiClientId, '思考完成');
      endRound(aiClientId);
    },
    [enableThinkingStream, stopThinking, endRound],
  );

  // ─── 主流程：处理用户发送 ───
  /**
   * 处理用户发送消息（ChatPanel 调用入口）
   * 串联：本地 echo → 记忆存储 → 上下文构建 → Agent 调度 → 思考过程 → AI 回复（PoC mock）
   *
   * 返回构建好的上下文（包含记忆 + 热点），供 IPC 层使用
   */
  const handleSend = useCallback(
    (text: string, attachments: ChatAttachment[]): {
      userClientId: string;
      aiClientId: string;
      context: string;
    } => {
      const userClientId = newClientMessageId();
      const aiClientId = newClientMessageId();

      // 1. 本地 echo：用户消息立即显示
      addMessage({
        id: '',
        clientId: userClientId,
        role: 'user',
        content: text,
        channel: 'sparkfox',
        label: 'You',
        attachments,
      });

      // 2. 数据流1：存入记忆
      storeUserMessageToMemory(text);

      // 3. 数据流3：推送监视面板事件
      pushMonitorEvent({
        type: 'message_in',
        title: '用户发送消息',
        detail: `内容：${text.slice(0, 80)}${text.length > 80 ? '…' : ''}`,
      });

      // 4. 数据流2+5：构建注入上下文（记忆 + 热点）
      const memoryContext = buildMemoryContext(text);
      const hotspotContext = buildHotspotContext(text);
      const context = [memoryContext, hotspotContext].filter(Boolean).join('\n\n');

      // 5. 数据流4：开始 AI 思考过程
      // 添加 AI 占位消息（流式输出占位）
      addMessage({
        id: '',
        clientId: aiClientId,
        role: 'assistant',
        content: '',
        channel: 'sparkfox',
        label: agents.find((a) => a.id === currentAgentId)?.name || 'SparkFox',
        attachments: [],
        isStreaming: true,
      });
      beginThinkingRound(aiClientId);

      return { userClientId, aiClientId, context };
    },
    [
      addMessage,
      storeUserMessageToMemory,
      pushMonitorEvent,
      buildMemoryContext,
      buildHotspotContext,
      agents,
      currentAgentId,
      beginThinkingRound,
    ],
  );

  /**
   * 处理 AI 回复完成（IPC 层调用，或 PoC mock 自动调用）
   * 串联：停止思考 → 存储情节记忆 → 推送监视事件
   */
  const handleAIReply = useCallback(
    (userMessage: string, aiClientId: string, aiReply: string) => {
      // 1. 停止流式输出
      finalizeLiveMessage(aiClientId);

      // 2. 数据流4：结束思考过程
      endThinkingRound(aiClientId);

      // 3. 数据流1：存储情节记忆（L2）
      storeEpisodeToMemory(userMessage, aiReply);

      // 4. 数据流3：推送监视面板事件
      pushMonitorEvent({
        type: 'message_out',
        title: 'SparkFox 回复消息',
        detail: `生成回复，长度 ${aiReply.length} 字符`,
        tokens: Math.ceil(aiReply.length / 4), // 粗略估算
        status: 'ok',
      });
    },
    [finalizeLiveMessage, endThinkingRound, storeEpisodeToMemory, pushMonitorEvent],
  );

  /**
   * PoC：模拟 AI 回复（生产环境由 IPC 层替换）
   */
  const simulateAIReply = useCallback(
    (userMessage: string, aiClientId: string) => {
      const replies = [
        `收到你的消息："${userMessage.slice(0, 50)}${userMessage.length > 50 ? '…' : ''}"。我是 SparkFox，很高兴为你服务！`,
        `我理解你说的话。让我想想…\n\n关于这个问题，我的建议是：\n1. 先明确需求\n2. 分步骤执行\n3. 持续迭代优化\n\n需要我详细展开吗？`,
        `好的，我已经处理了你的请求。根据我的分析，这个问题的核心在于平衡效率与可维护性。`,
        `这是一个很好的问题。基于当前上下文，我建议从以下几个角度考虑：\n\n- **技术层面**：选择合适的工具栈\n- **业务层面**：明确核心指标\n- **用户体验**：注重交互细节`,
      ];
      const reply = replies[Math.floor(Math.random() * replies.length)];

      // 模拟流式输出
      const appendToLiveMessage = useChatStore.getState().appendToLiveMessage;
      const chunks = reply.match(/.{1,3}/g) || [reply];
      let i = 0;
      const timer = setInterval(() => {
        if (i >= chunks.length) {
          clearInterval(timer);
          handleAIReply(userMessage, aiClientId, reply);
          return;
        }
        appendToLiveMessage(aiClientId, chunks[i]);
        i++;
      }, 50);
    },
    [handleAIReply],
  );

  // ─── 数据流5：热点 → 对话 ───
  /**
   * 将热点信息注入到对话页讨论
   */
  const injectHotspotToChat = useCallback(
    (hotspotTitle: string, hotspotContent: string) => {
      const text = `📊 热点讨论：${hotspotTitle}\n\n${hotspotContent}`;
      const { aiClientId } = handleSend(text, []);
      simulateAIReply(text, aiClientId);
    },
    [handleSend, simulateAIReply],
  );

  // ─── 监听 pendingChatInjection：HotspotView 点击"发送到对话" → 自动注入 ───
  useEffect(() => {
    if (!pendingChatInjection) return;
    const item = pendingChatInjection;
    // 立即清除，避免重复触发
    clearPendingChatInjection();
    // 构建热点描述并注入对话
    const heatText = item.heat ? `（热度 ${item.heat}）` : '';
    const trendText = item.trend === 'up' ? '上升' : item.trend === 'down' ? '下降' : '持平';
    const content = `来源：${item.source}${heatText}，趋势${trendText}`;
    injectHotspotToChat(item.title, content);
  }, [pendingChatInjection, clearPendingChatInjection, injectHotspotToChat]);

  return {
    // 数据流1：对话 → 记忆
    handleSend,
    handleAIReply,
    simulateAIReply,
    storeUserMessageToMemory,
    storeEpisodeToMemory,

    // 数据流2：记忆 → 对话
    buildMemoryContext,

    // 数据流3：Agent → 监视
    pushMonitorEvent,

    // 数据流4：思考过程
    beginThinkingRound,
    endThinkingRound,

    // 数据流5：热点 → 对话
    injectHotspotToChat,
    buildHotspotContext,
  };
}
