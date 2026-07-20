/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * useSparkfoxMessage — SparkFox 消息流 IPC hook
 *
 * 来源：NomiFun useNomiMessage.ts（仿写，结合 BaiLongma ThoughtStream 8 大方法）
 *
 * 设计：
 * - 订阅 `ipcBridge.conversation.responseStream`（WS 频道 `message.stream`）
 * - 不新增 Tauri 事件通道，复用 NomiFun 已有 IPC 抽象
 * - 将 IResponseMessage 事件映射到 thinkingStore + chatStore actions
 *
 * 事件路由（L1 / L2 双面板，对应 BaiLongma 双面板设计）：
 * - L1（嵌入 AI 消息气泡）：origin = null/'companion'/'idmm' 的常规用户消息触发流
 * - L2（右侧 Tick 面板）：origin = 'cron'/'autowork' 的自主行动 Tick 流
 *
 * 事件映射（BaiLongma → SparkFox）：
 * - stream_start / start     → beginRound + startThinkingSession + 新建 AI 消息
 * - thought                  → setStatus('思考中: ' + subject)
 * - content / text           → appendToLiveMessage
 * - tool_group               → addToolCall + recordAiActivity
 * - stream_end / finish      → finalizeLiveMessage + stopThinking + endRound
 * - turn_completed           → (仅指标，无思考流 action)
 * - error                    → setStatus('failed') + finalizeLiveMessage + endRound
 *
 * 用户选择 7B：现在就对接 NomiFun IPC
 */

import { useCallback, useEffect, useRef } from 'react';
import { ipcBridge } from '@/common';
import type { IResponseMessage } from '@/common/adapter/ipcBridge';
import type { ConversationId } from '@/common/types/ids';
import {
  useChatStore,
  type ChatAttachment,
} from '@renderer/store/chatStore';
import {
  useThinkingStore,
  L2_STREAM_KEY,
} from '@renderer/store/thinkingStore';

/** 判断是否为 L2 Tick 流事件（自主行动机制） */
function isTickEvent(message: IResponseMessage): boolean {
  const origin = (message.origin || '').toLowerCase();
  return origin === 'cron' || origin === 'autowork';
}

/** 从 thought 数据中提取 subject / description */
function extractThoughtData(data: unknown): { subject: string; description: string } {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    return { subject: '', description: String(data || '') };
  }
  const record = data as Record<string, unknown>;
  return {
    subject: record.subject != null ? String(record.subject) : '',
    description: record.description != null ? String(record.description) : '',
  };
}

/** 从 content 数据中提取文本块 */
function extractContentChunk(data: unknown): string {
  if (typeof data === 'string') return data;
  if (!data || typeof data !== 'object') return '';
  const record = data as Record<string, unknown>;
  // NomiFun content 事件格式：{ content: string, ... } 或 { delta: string, ... }
  if (typeof record.content === 'string') return record.content;
  if (typeof record.delta === 'string') return record.delta;
  if (typeof record.text === 'string') return record.text;
  return '';
}

/** 从 tool_group 数据中提取工具列表 */
function extractToolGroup(data: unknown): Array<{
  name: string;
  status: string;
  description?: string;
  args?: Record<string, unknown>;
  result?: string | null;
  ok?: boolean;
}> {
  if (!Array.isArray(data)) return [];
  return data
    .filter((item): item is Record<string, unknown> => !!item && typeof item === 'object' && !Array.isArray(item))
    .map((tool) => ({
      name: typeof tool.name === 'string' ? tool.name : '',
      status: typeof tool.status === 'string' ? tool.status : '',
      description: typeof tool.description === 'string' ? tool.description : undefined,
      args: (tool.args && typeof tool.args === 'object' && !Array.isArray(tool.args)
        ? tool.args as Record<string, unknown>
        : {}),
      result: typeof tool.result === 'string' ? tool.result : null,
      ok: typeof tool.ok === 'boolean' ? tool.ok : undefined,
    }));
}

interface UseSparkfoxMessageOptions {
  /** 仅监听该会话的事件；不传则监听全部（PoC 阶段便于演示） */
  conversationId?: ConversationId | string | null;
  /** 错误回调 */
  onError?: (message: IResponseMessage) => void;
  /** 是否只读模式（不发送消息，仅接收） */
  readOnly?: boolean;
}

interface UseSparkfoxMessageResult {
  /** 发送消息到 IPC（POST /api/conversations/{id}/messages） */
  sendMessage: (text: string, attachments: ChatAttachment[]) => Promise<void>;
  /** 当前是否正在流式输出 */
  isStreaming: boolean;
}

/**
 * SparkFox 消息流 IPC hook
 *
 * 用法：
 *   const { sendMessage, isStreaming } = useSparkfoxMessage({
 *     conversationId: 'sparkfox-default',
 *   });
 *   <ChatPanel onSend={sendMessage} />
 */
export function useSparkfoxMessage(
  options: UseSparkfoxMessageOptions = {}
): UseSparkfoxMessageResult {
  const { conversationId = null, onError, readOnly = false } = options;

  // ─── Stores ───
  const addMessage = useChatStore((s) => s.addMessage);
  const appendToLiveMessage = useChatStore((s) => s.appendToLiveMessage);
  const finalizeLiveMessage = useChatStore((s) => s.finalizeLiveMessage);
  const setInputLocked = useChatStore((s) => s.setInputLocked);

  const beginRound = useThinkingStore((s) => s.beginRound);
  const startThinkingSession = useThinkingStore((s) => s.startThinkingSession);
  const stopThinking = useThinkingStore((s) => s.stopThinking);
  const setStatus = useThinkingStore((s) => s.setStatus);
  const addToolCall = useThinkingStore((s) => s.addToolCall);
  const appendToolCycleEnd = useThinkingStore((s) => s.appendToolCycleEnd);
  const endRound = useThinkingStore((s) => s.endRound);
  const recordAiActivity = useThinkingStore((s) => s.recordAiActivity);
  const setActiveL1Key = useThinkingStore((s) => s.setActiveL1Key);
  const addLine = useThinkingStore((s) => s.addLine);

  // ─── Refs（避免 re-subscribe） ───
  const onErrorRef = useRef(onError);
  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  // 活跃 L1 streamKey（最近一次 start 事件的 msg_id）
  const activeL1KeyRef = useRef<string | null>(null);
  // 已处理的 msg_id（避免重复创建 AI 消息）
  const startedMsgIdsRef = useRef<Set<string>>(new Set());

  // ─── 订阅 responseStream ───
  useEffect(() => {
    const unsubscribe = ipcBridge.conversation.responseStream.on((message) => {
      // 会话过滤
      if (conversationId && message.conversation_id && String(message.conversation_id) !== String(conversationId)) {
        return;
      }

      const msgId = message.msg_id || '';
      const isTick = isTickEvent(message);
      const streamKey = isTick ? L2_STREAM_KEY : msgId;

      switch (message.type) {
        case 'start': {
          // L1：新建 AI 消息（避免重复）
          if (!isTick && msgId && !startedMsgIdsRef.current.has(msgId)) {
            startedMsgIdsRef.current.add(msgId);
            activeL1KeyRef.current = msgId;

            // 在 chatStore 中新建 AI 消息（用 msg_id 作为 clientId，便于后续 appendToLiveMessage）
            addMessage({
              id: msgId,
              clientId: msgId,
              role: 'assistant',
              content: '',
              channel: 'sparkfox',
              label: 'SparkFox',
            });

            setActiveL1Key(msgId);
            beginRound(msgId);
            startThinkingSession(msgId, '思考中');
            setInputLocked(true, 'SparkFox 正在思考…');
          }

          // L2：Tick 心跳开始
          if (isTick) {
            beginRound(L2_STREAM_KEY);
            startThinkingSession(L2_STREAM_KEY, 'Tick 心跳');
            addLine(L2_STREAM_KEY, 'Tick', '自主行动机制启动');
          }
          break;
        }

        case 'thought': {
          if (!streamKey) break;
          const { subject, description } = extractThoughtData(message.data);
          const label = subject || description ? `思考中: ${subject || description}` : '思考中';
          setStatus(streamKey, label, 'busy');

          // L2 心跳流的 thought 也记录到活动状态
          if (isTick) {
            recordAiActivity('tick_thought');
          }
          break;
        }

        case 'content':
        case 'text': {
          if (!isTick && msgId) {
            const chunk = extractContentChunk(message.data);
            if (chunk) {
              appendToLiveMessage(msgId, chunk);
            }
          }
          // Tick 流的 content 不写入 chat，仅记录到 L2 思考流
          if (isTick && streamKey) {
            const chunk = extractContentChunk(message.data);
            if (chunk) {
              addLine(L2_STREAM_KEY, '内容', chunk);
            }
          }
          break;
        }

        case 'tool_group': {
          if (!streamKey) break;
          const tools = extractToolGroup(message.data);
          for (const tool of tools) {
            // 记录到思考流
            addToolCall(
              streamKey,
              tool.name || 'unknown_tool',
              tool.args || {},
              tool.result ?? null,
              tool.ok
            );
            // 记录到 AI 活动状态
            recordAiActivity(tool.name || 'unknown_tool');
          }
          break;
        }

        case 'finish': {
          if (!isTick && msgId) {
            finalizeLiveMessage(msgId);
            stopThinking(msgId, '完成');
            appendToolCycleEnd(msgId);
            endRound(msgId);
            setInputLocked(false);
            activeL1KeyRef.current = null;
          }
          if (isTick && streamKey) {
            stopThinking(L2_STREAM_KEY, 'Tick 完成');
            appendToolCycleEnd(L2_STREAM_KEY);
            endRound(L2_STREAM_KEY);
          }
          break;
        }

        case 'turn_completed': {
          // 仅用于指标展示，不触发思考流 action
          // TODO: 后续可在 ChatPanel 底部展示 token 用量
          break;
        }

        case 'error': {
          if (!isTick && msgId) {
            const errMsg =
              typeof message.data === 'string'
                ? message.data
                : (message.data as { message?: string })?.message || '未知错误';
            setStatus(msgId, `错误: ${errMsg}`, 'failed');
            finalizeLiveMessage(msgId);
            endRound(msgId);
            setInputLocked(false);
            activeL1KeyRef.current = null;
          }
          if (isTick && streamKey) {
            setStatus(L2_STREAM_KEY, 'Tick 错误', 'failed');
            endRound(L2_STREAM_KEY);
          }
          onErrorRef.current?.(message);
          break;
        }

        default: {
          // 其他事件类型（permission / config_changed / artifact 等）暂不处理
          // 后续模块接入时扩展
          break;
        }
      }
    });

    return unsubscribe;
    // 不依赖 conversationId，避免会话切换时重新订阅（用 ref 过滤即可）
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ─── 发送消息 ───
  const sendMessage = useCallback(
    async (text: string, attachments: ChatAttachment[]) => {
      if (readOnly) {
        // eslint-disable-next-line no-console
        console.warn('[SparkFox] readOnly 模式，消息未发送');
        return;
      }

      if (!conversationId) {
        // PoC 阶段无后端：仅本地 echo + 模拟 AI 回复
        // eslint-disable-next-line no-console
        console.warn('[SparkFox] 未配置 conversationId，消息仅本地 echo', { text, attachments });
        return;
      }

      try {
        // 提取附件 ID（如果有）
        const files = attachments
          .map((a) => a.id)
          .filter((id): id is string => Boolean(id));

        await ipcBridge.conversation.sendMessage.invoke({
          conversation_id: conversationId as ConversationId,
          input: text,
          files: files.length > 0 ? files : undefined,
        });
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error('[SparkFox] sendMessage 失败:', err);
        setInputLocked(false);
        onErrorRef.current?.({
          type: 'error',
          data: { message: String(err) },
          msg_id: '' as never,
          conversation_id: conversationId as ConversationId,
        });
      }
    },
    [conversationId, readOnly, setInputLocked]
  );

  // ─── isStreaming 派生 ───
  const liveMessageId = useChatStore((s) => s.liveMessageId);
  const isStreaming = liveMessageId !== null;

  return {
    sendMessage,
    isStreaming,
  };
}

export default useSparkfoxMessage;
