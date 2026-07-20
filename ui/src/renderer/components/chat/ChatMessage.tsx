/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ChatMessage — 单条消息渲染组件
 *
 * 来源：BaiLongma src/ui/brain-ui/chat.js addMsg() 函数（清洁室重写为 React + TS）
 *
 * 保留 BaiLongma 特性：
 * - 用户/AI 消息左右分栏（role-user / role-assistant）
 * - 频道标签徽章（friendlyChannelLabel）
 * - 附件预览（图片）
 * - 流式光标（is-streaming class，由 CSS ::after 实现）
 *
 * 改造点：
 * - DOM 操作 → React 函数组件
 * - createMarkdownBody() → NomiFun MarkdownView（react-markdown + remark-gfm）
 * - 预留 ThoughtStream 插槽（message.thoughtStreamSlot，P0-模块 F 接入点）
 */

import React, { memo } from 'react';
import MarkdownView from '@renderer/components/Markdown';
import ThoughtStream from '@renderer/components/thinking/ThoughtStream';
import CitationChip from '@renderer/components/citation/CitationChip';
import type { Citation } from '@renderer/components/citation/types';
import { useThinkingStore } from '@renderer/store/thinkingStore';
import type { ChatMessage } from '@renderer/store/chatStore';
import { friendlyChannelLabel } from '@renderer/store/chatStore';

interface ChatMessageProps {
  message: ChatMessage;
  agentName?: string;
  /** 引用列表（v1.1.0 第十一波集成：来自 SearchResult.citations） */
  citations?: Citation[];
}

const ChatMessageComponent: React.FC<ChatMessageProps> = ({
  message,
  agentName = 'SparkFox',
  citations,
}) => {
  const { role, content, channel, label, attachments, isStreaming, thoughtStreamSlot } = message;

  // 标签：优先 message.label → 频道友好标签 → role 默认（You / agentName / System）
  const labelText =
    label ||
    friendlyChannelLabel(channel) ||
    (role === 'user' ? 'You' : role === 'assistant' ? agentName : 'System');

  // 仅渲染图片附件（其他类型附件后续扩展）
  const imageAttachments = attachments.filter((a) => a.type === 'image' && a.dataUrl);

  // ─── L1 思考流：仅 AI 消息且该 clientId 有对应 stream 时渲染 ───
  // 订阅 thinkingStore.l1Streams 中该消息的 stream（不存在则返回空数组）
  const l1Lines = useThinkingStore((s) =>
    role === 'assistant' ? s.l1Streams.get(message.clientId) || null : null
  );
  const shouldRenderThoughtStream = role === 'assistant' && l1Lines !== null && l1Lines.length > 0;

  // ─── Citation Chip 列表（v1.1.0 第十一波集成） ──────────────────────────────
  // 仅 assistant 消息且非流式输出中时渲染引用徽标，避免流式过程中频繁挂载抽屉
  const shouldRenderCitations = role === 'assistant' && !isStreaming && (citations?.length ?? 0) > 0;

  return (
    <div className={`sparkfox-msg-row role-${role}`}>
      <div className={`sparkfox-msg-bubble${isStreaming ? ' is-streaming' : ''}`}>
        <div className='sparkfox-msg-label'>{labelText}</div>

        {imageAttachments.length > 0 && (
          <div className='sparkfox-msg-attachments'>
            {imageAttachments.map((a) => (
              <img
                key={a.id}
                src={a.dataUrl}
                alt={a.name}
                className='sparkfox-msg-attachment-img'
              />
            ))}
          </div>
        )}

        {/* ThoughtStream 插槽 — P0-模块 F 接入点
            优先使用外部传入的 slot（记忆注入等后续模块），
            否则若该 AI 消息有 L1 思考流，自动渲染 ThoughtStream */}
        {thoughtStreamSlot ? (
          <div className='sparkfox-thought-slot'>{thoughtStreamSlot}</div>
        ) : shouldRenderThoughtStream ? (
          <div className='sparkfox-thought-slot'>
            <ThoughtStream streamKey={message.clientId} maxLines={50} />
          </div>
        ) : null}

        {/* Markdown 渲染 — 复用 NomiFun 已有 react-markdown + remark-gfm + KaTeX 链路 */}
        <MarkdownView>{content || ''}</MarkdownView>

        {/* 引用徽标列表 — v1.1.0 第十一波集成（spec §三 10.10.2）
            每个 CitationChip 点击后弹出 CitationDetailDrawer 三级溯源抽屉 */}
        {shouldRenderCitations && (
          <div
            className='sparkfox-msg-citations'
            style={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: 4,
              marginTop: 8,
              paddingTop: 6,
              borderTop: '1px solid var(--color-border-2, #e5e6eb)',
            }}
          >
            {citations!.map((c, idx) => (
              <CitationChip key={c.entity.entity_id} citation={c}>
                [{idx + 1}]
              </CitationChip>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

ChatMessageComponent.displayName = 'ChatMessage';

export default memo(ChatMessageComponent);
