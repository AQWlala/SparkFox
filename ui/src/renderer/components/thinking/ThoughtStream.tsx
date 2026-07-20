/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ThoughtStream — 思考流主组件
 *
 * 来源：BaiLongma src/ui/brain-ui/thought-stream.js ThoughtStream 类（清洁室重写为 React 组件）
 *
 * 保留 BaiLongma 特性：
 * - 思考流容器（stream-inner）
 * - 多行 ThoughtLine 渲染
 * - 自动滚动到最新（scrollToLatest）
 * - 行数限制（trim）
 *
 * 改造点：
 * - DOM appendChild → React 组件 map
 * - BaiLongma 固定挂载点（#si-l1/#si-l2）→ props.streamKey 动态订阅
 * - 手动 scrollTop → useAutoScroll hook
 *
 * 用法：
 *   <ThoughtStream streamKey={messageClientId} />  // L1 嵌入消息气泡
 *   <ThoughtStream streamKey={L2_STREAM_KEY} />    // L2 右侧面板
 */

import React, { useMemo, useRef } from 'react';
import { useThinkingStore, L2_STREAM_KEY } from '@renderer/store/thinkingStore';
import { useAutoScroll } from '@renderer/hooks/chat/useAutoScroll';
import ThoughtLine from './ThoughtLine';

interface ThoughtStreamProps {
  /** stream 标识（L1 用 AI 消息 clientId，L2 用 L2_STREAM_KEY） */
  streamKey: string;
  /** 最大行数（超过则不显示最早的，默认 100） */
  maxLines?: number;
  /** 是否启用自动滚动（默认 true） */
  autoScroll?: boolean;
  /** 空状态文案（可选） */
  emptyText?: string;
}

const ThoughtStream: React.FC<ThoughtStreamProps> = ({
  streamKey,
  maxLines = 100,
  autoScroll = true,
  emptyText,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);

  // 订阅对应 stream 的 lines
  const lines = useThinkingStore((state) => {
    if (streamKey === L2_STREAM_KEY) return state.l2Stream;
    return state.l1Streams.get(streamKey) || [];
  });

  // 自动滚动：监听行数 + 最后一行内容
  const scrollContent = useMemo(() => {
    const last = lines[lines.length - 1];
    return `${lines.length}:${last?.type || ''}:${last?.tools.length || 0}`;
  }, [lines]);

  useAutoScroll({
    containerRef,
    content: scrollContent,
    enabled: autoScroll,
    threshold: 200,
  });

  // 超过 maxLines 则只显示最后 maxLines 行
  const visibleLines = lines.length > maxLines ? lines.slice(lines.length - maxLines) : lines;

  if (visibleLines.length === 0) {
    if (!emptyText) return null;
    return (
      <div className='sf-thought-stream' ref={containerRef}>
        <div className='sf-thought-stream-empty'>{emptyText}</div>
      </div>
    );
  }

  return (
    <div className='sf-thought-stream' ref={containerRef}>
      {visibleLines.map((line) => (
        <ThoughtLine key={line.id} line={line} streamKey={streamKey} />
      ))}
    </div>
  );
};

ThoughtStream.displayName = 'ThoughtStream';

export default ThoughtStream;
