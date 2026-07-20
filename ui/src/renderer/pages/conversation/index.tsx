import { ipcBridge } from '@/common';
import { Message } from '@arco-design/web-react';
import React, { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import useSWR from 'swr';
import ChatConversation from './components/ChatConversation';
import MessageListSkeleton from './Messages/components/MessageListSkeleton';
import { useAutoTitle } from '@/renderer/hooks/chat/useAutoTitle';
import { getConversationOrNull } from '@/renderer/pages/conversation/utils/conversationCache';
import { parseConversationId } from '@/common/types/ids';
// F2 引用 chip 集成 —— 解析 LLM 回复中的 [citation:...] 标记
import { CitationChip, type CitationData } from '@renderer/views/ChatView/components/CitationChip';

/**
 * parseCitations —— 解析 LLM 响应中的 [citation:kdoc_xxx:chunk_5:0:128] 标记
 *
 * 将标记替换为 [n] 序号，并收集对应的 CitationData 列表。
 * 调用方渲染 text + citations.map(c => <CitationChip citation={c} />) 即可。
 *
 * 与 spec 2004-2027 行一致。
 */
export function parseCitations(text: string): { text: string; citations: CitationData[] } {
  const regex = /\[citation:([^:]+):(\d+):(\d+):(\d+)\]/g;
  const citations: CitationData[] = [];
  const cleanText = text.replace(regex, (_, kdoc: string, chunk: string, start: string, end: string) => {
    citations.push({
      kdoc_id: kdoc,
      chunk_id: parseInt(chunk, 10),
      span_start: parseInt(start, 10),
      span_end: parseInt(end, 10),
      score: 0,
      source: 'Vector',
    });
    return `[${citations.length}]`;
  });
  return { text: cleanText, citations };
}

// 重新导出 CitationChip / CitationData，方便消费方从本模块统一引入
export { CitationChip, type CitationData };

const ChatConversationIndex: React.FC = () => {
  const { id } = useParams();
  // Validate the route string once at the boundary; every downstream layer
  // keeps the same canonical conversation entity ID.
  const conversationId = id != null ? parseConversationId(id) : undefined;
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { syncTitleFromHistory } = useAutoTitle();
  const notFoundHandledIdRef = useRef<string | undefined>(undefined);
  const defaultConversationTitle = t('conversation.welcome.newConversation');

  const { data, isLoading, mutate } = useSWR(id ? `conversation/${id}` : null, () => {
    return getConversationOrNull(conversationId!);
  });

  useEffect(() => {
    if (!id) return;

    return ipcBridge.conversation.listChanged.on((event) => {
      if (event.conversation_id !== conversationId || (event.action !== 'updated' && event.action !== 'created')) {
        return;
      }

      void mutate();
    });
  }, [id, conversationId, mutate]);

  useEffect(() => {
    if (!data || data.name !== defaultConversationTitle) {
      return;
    }

    void syncTitleFromHistory(data.id);
  }, [data, defaultConversationTitle, syncTitleFromHistory]);

  // 会话不存在（例如从历史栈回到已删除会话）时，提示并替换路由到首页，
  // 避免渲染空骨架。每个 id 只触发一次。
  // Conversation does not exist (e.g. navigating back to a deleted one via
  // browser history): show a toast and replace the route with home, so we
  // don't render an empty skeleton. Fire at most once per id.
  useEffect(() => {
    if (!id || isLoading || data || notFoundHandledIdRef.current === id) return;
    notFoundHandledIdRef.current = id;
    Message.warning(t('conversation.notFound'));
    navigate('/', { replace: true });
  }, [id, isLoading, data, navigate, t]);

  if (isLoading) return <MessageListSkeleton />;
  return <ChatConversation conversation={data ?? undefined}></ChatConversation>;
};

export default ChatConversationIndex;
