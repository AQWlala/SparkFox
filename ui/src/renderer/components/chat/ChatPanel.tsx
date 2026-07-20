/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ChatPanel — SparkFox 对话主面板
 *
 * 来源：BaiLongma src/ui/brain-ui/chat.js initChat()（清洁室重写为 React + TS）
 *
 * 保留 BaiLongma 特性：
 * - 消息列表渲染（messages.map → ChatMessage）
 * - 滚动跟随（useAutoScroll，复用 NomiFun 已有 hook）
 * - 悬停状态管理（isHovering，替代 BaiLongma isHoveringChat）
 * - 空状态占位（无消息时显示引导文案）
 * - 本地 echo（用户发送后立即在气泡中显示）
 *
 * 改造点：
 * - DOM 操作 → React 函数组件
 * - 手动 scrollTop → useAutoScroll hook
 * - BaiLongma chatHistory open/close → 始终展开（SparkFox 是单栏对话，无自动关闭）
 * - 接入 useSparkfoxConversation hook，统一管理 5 个数据流
 *
 * 数据流串联（由 useSparkfoxConversation hook 实现）：
 * - 数据流1：对话 → 记忆存储
 * - 数据流2：记忆 → 对话注入
 * - 数据流3：Agent 菜单 → 监视面板
 * - 数据流4：思考过程 → 对话展示
 * - 数据流5：热点 → 对话讨论（HotspotView 调用 injectHotspotToChat）
 */

import React, { useCallback, useMemo, useRef, useState } from 'react';
import {
  useChatStore,
  type ChatAttachment,
} from '@renderer/store/chatStore';
import { useAgentStore } from '@renderer/store/agentStore';
import { useAutoScroll } from '@renderer/hooks/chat/useAutoScroll';
import { useSparkfoxConversation } from '@renderer/hooks/sparkfox/useSparkfoxConversation';
import ReasoningChainPanel, {
  type ThoughtProcess,
  type EntityRef,
} from '@renderer/components/thinking/ReasoningChainPanel';
import ChatMessage from './ChatMessage';
import ChatInput from './ChatInput';
import './chat.css';

interface ChatPanelProps {
  /** 发送消息回调（生产环境由父组件对接 IPC；未注入时使用 simulateAIReply mock 回复） */
  onSend?: (text: string, attachments: ChatAttachment[], context: string) => void;
  /** Agent 显示名（覆盖 agentStore 中的当前 Agent 名） */
  agentName?: string;
}

const ChatPanel: React.FC<ChatPanelProps> = ({ onSend, agentName }) => {
  const messages = useChatStore((s) => s.messages);
  const clearAttachments = useChatStore((s) => s.clearAttachments);
  const setHovering = useChatStore((s) => s.setHovering);

  // 当前 Agent 名称（从 agentStore 读取，可被 props 覆盖）
  const currentAgentId = useAgentStore((s) => s.currentAgentId);
  const agents = useAgentStore((s) => s.agents);
  const currentAgent = agents.find((a) => a.id === currentAgentId);
  const displayName = agentName || currentAgent?.name || 'SparkFox';

  // ─── 接入统一对话编排器（串联所有数据流） ───
  const {
    handleSend: orchestrateSend,
    simulateAIReply,
  } = useSparkfoxConversation();

  const messagesRef = useRef<HTMLDivElement>(null);

  // useAutoScroll 监听的内容：消息数量 + 最后一条消息内容（流式更新时触发）
  const scrollContent = useMemo(() => {
    const last = messages[messages.length - 1];
    return `${messages.length}:${last?.content || ''}`;
  }, [messages]);

  useAutoScroll({
    containerRef: messagesRef,
    content: scrollContent,
    enabled: true,
    threshold: 200,
  });

  // ─── SAG 推理链面板状态（v1.1.0 第十一波集成） ─────────────────────────────
  // PoC：本地 mock 推理链数据；生产环境从 SearchResult.thought_process 读取
  const [showReasoningPanel, setShowReasoningPanel] = useState(false);
  const mockThoughtProcess: ThoughtProcess = useMemo(
    () => ({
      steps: [
        { step: 'Step1', title: '理解用户意图', content: '解析用户查询，识别核心实体与意图' },
        { step: 'Step2', title: '实体识别', content: 'jieba 分词 + LLM NER 抽取关键实体', hop: 1 },
        { step: 'Step3', title: 'ATOMIC 检索', content: '基于实体 ID 查询 event_entity_relation 表', hop: 1 },
        { step: 'Step4', title: 'MULTI 多跳扩展', content: 'BFS 遍历 2-3 跳，score=1/hop 衰减', hop: 2 },
        { step: 'Step5', title: 'Rerank 重排', content: 'BGE reranker 对候选 events 重排' },
        { step: 'Step6', title: 'Prompt 组装', content: '将 top-k events 注入 LLM 上下文' },
        { step: 'Step7', title: '生成回复', content: 'LLM 基于检索结果生成最终回复' },
      ],
    }),
    [],
  );
  const mockViaEntities: EntityRef[] = useMemo(
    () => [
      { entity_id: 'e_001', entity_type: 'PERSON', name: '张三' },
      { entity_id: 'e_042', entity_type: 'ORGANIZATION', name: 'SparkFox 团队' },
      { entity_id: 'e_108', entity_type: 'CONCEPT', name: 'SAG 检索' },
    ],
    [],
  );

  // ─── 发送处理 ───
  // 通过 useSparkfoxConversation 串联：本地 echo → 记忆存储 → 上下文构建 → Agent 调度 → 思考过程 → AI 回复
  const handleSend = useCallback(
    (text: string, attachments: ChatAttachment[]) => {
      // 1. 调用编排器（内部完成本地 echo、记忆存储、监视推送、思考过程开始、AI 占位消息）
      const { aiClientId, context } = orchestrateSend(text, attachments);

      // 2. 清空附件
      clearAttachments();

      // 3. 触发 AI 回复
      if (onSend) {
        // 生产环境：交由父组件对接 IPC
        onSend(text, attachments, context);
      } else {
        // PoC 环境：使用 mock 流式回复
        simulateAIReply(text, aiClientId);
      }
    },
    [orchestrateSend, clearAttachments, onSend, simulateAIReply],
  );

  return (
    <div
      className='sparkfox-chat'
      onMouseEnter={() => setHovering(true)}
      onMouseLeave={() => setHovering(false)}
      style={{ display: 'flex', height: '100%' }}
    >
      {/* 主对话区 */}
      <div style={{ flex: '1 1 auto', minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        {messages.length === 0 ? (
          <div className='sparkfox-chat-empty'>
            <div className='sparkfox-chat-empty-icon'>💬</div>
            <div className='sparkfox-chat-empty-text'>和 {displayName} 开始对话吧</div>
          </div>
        ) : (
          <div className='sparkfox-chat-messages' ref={messagesRef}>
            {messages.map((msg) => (
              <ChatMessage key={msg.clientId} message={msg} agentName={displayName} />
            ))}
          </div>
        )}
        <ChatInput onSend={handleSend} />
      </div>

      {/* SAG 推理链侧边栏（v1.1.0 第十一波集成 / spec §三 10.9.2）
          PoC：点击按钮切换显示，展示 Step1..Step7 + via_entities 多跳路径
          生产环境：由 SearchResult.thought_process 自动驱动 */}
      <div
        style={{
          flex: '0 0 auto',
          width: showReasoningPanel ? 320 : 40,
          borderLeft: '1px solid var(--color-border-2, #e5e6eb)',
          transition: 'width 0.2s ease',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <button
          type='button'
          onClick={() => setShowReasoningPanel((v) => !v)}
          style={{
            padding: '8px 12px',
            fontSize: 11,
            color: 'var(--color-text-2, #4e5969)',
            background: 'transparent',
            border: 'none',
            borderBottom: showReasoningPanel ? '1px solid var(--color-border-2, #e5e6eb)' : 'none',
            cursor: 'pointer',
            textAlign: 'left',
            width: '100%',
          }}
          title='切换 SAG 推理链面板'
        >
          {showReasoningPanel ? '◀ 收起推理链' : '▶ 推理链'}
        </button>
        {showReasoningPanel && (
          <div style={{ flex: '1 1 auto', overflow: 'auto', padding: 12 }}>
            <ReasoningChainPanel
              thought_process={mockThoughtProcess}
              via_entities={mockViaEntities}
            />
          </div>
        )}
      </div>
    </div>
  );
};

ChatPanel.displayName = 'ChatPanel';

export default ChatPanel;
