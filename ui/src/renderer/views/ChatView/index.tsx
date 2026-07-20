/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox ChatView — 主对话页路由入口
 *
 * 来源：BaiLongma 对话展示（清洁室重写）+ SparkFox 路由
 * 功能：对话消息渲染 + 思考过程可视化（P0-模块 F 接入点）+ 记忆注入（P1-模块 D 接入点）
 *
 * 这是 /sparkfox/ 路由的默认页：
 * 1. 通过 useSparkfoxMessage 订阅 IPC responseStream（WS 频道 message.stream）
 * 2. 将 sendMessage 回调注入 ChatPanel.onSend，对接 ipcBridge.conversation.sendMessage
 * 3. 渲染 ChatPanel 主对话面板（L1 思考流由 ChatMessage 自动从 thinkingStore 读取）
 *
 * PoC 阶段（无后端）：提供"演示按钮"触发模拟思考流，用于验证可视化效果
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import ChatPanel from '@renderer/components/chat/ChatPanel';
import { SearchDegradeBanner } from '@renderer/components/search/SearchDegradeBanner';
import SearchStrategySelector from '@renderer/components/search/SearchStrategySelector';
import type { SearchStrategy } from '@renderer/components/search/constants';
import { DEFAULT_SEARCH_STRATEGY } from '@renderer/components/search/constants';
import { useSparkfoxMessage } from '@renderer/hooks/sparkfox/useSparkfoxMessage';
import { useChatStore, newClientMessageId } from '@renderer/store/chatStore';
import {
  useThinkingStore,
  L2_STREAM_KEY,
} from '@renderer/store/thinkingStore';

/** PoC 演示：模拟一轮完整的 L1 + L2 思考流（无后端时验证可视化） */
function useDemoTrigger() {
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

  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  const clearTimers = useCallback(() => {
    timersRef.current.forEach((t) => clearTimeout(t));
    timersRef.current = [];
  }, []);

  useEffect(() => () => clearTimers(), [clearTimers]);

  const runDemo = useCallback(() => {
    clearTimers();

    // 模拟用户消息
    const userClientId = newClientMessageId();
    addMessage({
      id: '',
      clientId: userClientId,
      role: 'user',
      content: '帮我看一下当前目录有哪些文件，并搜索一下"SparkFox"相关资料',
      channel: 'sparkfox',
      label: 'You',
    });

    // 模拟 AI 回复开始
    const aiMsgId = `demo-ai-${Date.now()}`;
    const schedule = (delay: number, fn: () => void) => {
      const t = setTimeout(fn, delay);
      timersRef.current.push(t);
    };

    schedule(300, () => {
      addMessage({
        id: aiMsgId,
        clientId: aiMsgId,
        role: 'assistant',
        content: '',
        channel: 'sparkfox',
        label: 'SparkFox',
      });
      setActiveL1Key(aiMsgId);
      beginRound(aiMsgId);
      startThinkingSession(aiMsgId, '思考中');
      setInputLocked(true, 'SparkFox 正在思考…');
    });

    // thought 1
    schedule(800, () => {
      setStatus(aiMsgId, '思考中: 理解用户意图', 'busy');
    });

    // tool 1: list_dir
    schedule(1500, () => {
      addToolCall(
        aiMsgId,
        'list_dir',
        { path: '.' },
        'src/\nui/\nCargo.toml\npackage.json\nREADME.md',
        true
      );
      recordAiActivity('list_dir');
    });

    // thought 2
    schedule(2200, () => {
      setStatus(aiMsgId, '思考中: 检索 SparkFox 相关资料', 'busy');
    });

    // tool 2: web_search
    schedule(2800, () => {
      addToolCall(
        aiMsgId,
        'web_search',
        { query: 'SparkFox 项目融合' },
        JSON.stringify([
          { title: 'SparkFox 融合蓝图 v1.0', url: 'https://example.com/1' },
          { title: '四项目深度分析报告', url: 'https://example.com/2' },
        ]),
        true
      );
      recordAiActivity('web_search');
    });

    // tool 3: search_memory
    schedule(3500, () => {
      addToolCall(
        aiMsgId,
        'search_memory',
        { query: 'SparkFox 思考流' },
        '找到 3 条相关记忆',
        true
      );
      recordAiActivity('search_memory');
    });

    // tool cycle end
    schedule(4200, () => {
      appendToolCycleEnd(aiMsgId);
    });

    // stop thinking + start content
    schedule(4500, () => {
      stopThinking(aiMsgId, '完成');
      setStatus(aiMsgId, '生成回复中', 'busy');
    });

    // streaming content chunks
    const chunks = [
      '我已查看当前目录，',
      '主要包含 `src/`、`ui/`、`Cargo.toml`、`package.json` 等文件。\n\n',
      '同时检索到与 **SparkFox** 相关的资料：\n\n',
      '1. SparkFox 融合蓝图 v1.0\n',
      '2. 四项目深度分析报告\n\n',
      '建议下一步接入 **P0-模块 G 信息热点追踪** 以完善可视化体系。',
    ];
    let chunkDelay = 4800;
    for (const chunk of chunks) {
      schedule(chunkDelay, () => {
        appendToLiveMessage(aiMsgId, chunk);
      });
      chunkDelay += 350;
    }

    // finish
    schedule(chunkDelay + 300, () => {
      finalizeLiveMessage(aiMsgId);
      endRound(aiMsgId);
      setInputLocked(false);
    });

    // ─── L2 Tick 心跳演示（同步触发，展示双面板布局） ───
    schedule(500, () => {
      beginRound(L2_STREAM_KEY);
      startThinkingSession(L2_STREAM_KEY, 'Tick 心跳');
      addLine(L2_STREAM_KEY, 'Tick', '自主行动机制启动');
    });
    schedule(1800, () => {
      addToolCall(
        L2_STREAM_KEY,
        'probe_memory',
        { depth: 'L1' },
        'L0-L5 全部健康',
        true
      );
      recordAiActivity('probe_memory');
    });
    schedule(3200, () => {
      addToolCall(
        L2_STREAM_KEY,
        'schedule_reminder',
        { when: 'in 1h', what: 'review PR' },
        '已安排提醒',
        true
      );
      recordAiActivity('schedule_reminder');
    });
    schedule(4400, () => {
      appendToolCycleEnd(L2_STREAM_KEY);
      stopThinking(L2_STREAM_KEY, 'Tick 完成');
      endRound(L2_STREAM_KEY);
    });
  }, [
    addMessage,
    appendToLiveMessage,
    finalizeLiveMessage,
    setInputLocked,
    beginRound,
    startThinkingSession,
    stopThinking,
    setStatus,
    addToolCall,
    appendToolCycleEnd,
    endRound,
    recordAiActivity,
    setActiveL1Key,
    addLine,
    clearTimers,
  ]);

  return { runDemo };
}

export default function ChatView() {
  // PoC 阶段：未配置真实 conversationId
  // - 不注入 onSend → ChatPanel 自动使用 simulateAIReply mock 流式回复
  // - 数据流1-5 由 useSparkfoxConversation hook 串联（记忆存储/注入/思考过程/监视推送）
  // 生产环境：从路由参数或全局 store 读取当前 conversationId，并通过 onSend 注入 IPC 实现
  useSparkfoxMessage({
    conversationId: null,
    onError: (msg) => {
      // eslint-disable-next-line no-console
      console.error('[SparkFox ChatView] IPC error:', msg);
    },
  });

  const { runDemo } = useDemoTrigger();

  // ─── SAG 检索策略 + 降级提示状态（v1.1.0 第十一波集成） ──────────────────────
  // PoC：本地 useState 管理；生产环境从全局 store / IPC 持久化
  const [searchStrategy, setSearchStrategy] = useState<SearchStrategy>(DEFAULT_SEARCH_STRATEGY);
  // PoC：演示降级场景（点击按钮切换）；生产环境从 SearchResult.is_degraded 读取
  const [isDegraded, setIsDegraded] = useState(false);

  return (
    <div style={{ position: 'relative', height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* 顶部工具栏：检索策略选择器 + 降级提示横幅（v1.1.0 第十一波集成） */}
      <div
        style={{
          flex: '0 0 auto',
          padding: '8px 12px',
          borderBottom: '1px solid var(--color-border-2, #e5e6eb)',
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          flexWrap: 'wrap',
        }}
      >
        <SearchStrategySelector
          value={searchStrategy}
          onChange={setSearchStrategy}
          compact
        />
        {/* PoC 演示：切换降级状态以验证横幅渲染 */}
        <button
          type='button'
          onClick={() => setIsDegraded((v) => !v)}
          style={{
            padding: '4px 10px',
            fontSize: 11,
            fontFamily: 'inherit',
            color: 'var(--color-text-2, #4e5969)',
            background: 'var(--color-fill-2, #f2f3f5)',
            border: '1px solid var(--color-border-2, #e5e6eb)',
            borderRadius: 4,
            cursor: 'pointer',
          }}
          title='PoC 演示：切换降级状态以验证横幅渲染'
        >
          {isDegraded ? '✓ 降级中' : '模拟降级'}
        </button>
      </div>
      <SearchDegradeBanner
        is_degraded={isDegraded}
        degrade_reason='未抽取事件'
        onDismiss={() => {
          // 用户关闭后，下次切换策略或新查询时 banner 会重新评估
          // eslint-disable-next-line no-console
          console.log('[SparkFox ChatView] degrade banner dismissed');
        }}
      />

      {/* 主对话区 */}
      <div style={{ flex: '1 1 auto', minHeight: 0, position: 'relative' }}>
        {/* PoC 模式：不传 onSend，自动走 simulateAIReply（含思考流 + 记忆存储 + 监视推送） */}
        <ChatPanel />
        {/* PoC 演示按钮 — 无后端时点击触发模拟思考流，验证可视化 */}
        <button
          type='button'
          onClick={runDemo}
          style={{
            position: 'absolute',
            top: 12,
            right: 12,
            zIndex: 10,
            padding: '6px 12px',
            fontSize: 12,
            fontFamily: '-apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif',
            color: '#fff',
            background: '#007AFF',
            border: 'none',
            borderRadius: 6,
            cursor: 'pointer',
            boxShadow: '0 2px 6px rgba(0, 122, 255, 0.3)',
          }}
          title='PoC 演示：模拟一轮 AI 思考流（L1 嵌入气泡 + L2 右侧面板）'
        >
          ▶ 演示思考流
        </button>
      </div>
    </div>
  );
}
