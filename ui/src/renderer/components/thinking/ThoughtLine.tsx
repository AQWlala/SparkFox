/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ThoughtLine — 单行思考流组件
 *
 * 来源：BaiLongma src/ui/brain-ui/thought-stream.js newLine() 方法（清洁室重写为 React 组件）
 *
 * 保留 BaiLongma 特性：
 * - 行头（dot + type + time）
 * - 行文本内容（line-text）
 * - 思考三点动画（line-thinking）
 * - 状态徽章（line-status busy/failed/stale）
 * - 工具调用列表（line-tool × N）
 *
 * 改造点：
 * - DOM 操作 → React 组件
 * - BaiLongma CSS 变量颜色 → Apple 主题色
 * - 工具调用展开/收起 → 调用 thinkingStore.toggleToolExpand
 */

import React, { memo, useCallback } from 'react';
import type { ThoughtLine as ThoughtLineType } from '@renderer/store/thinkingStore';
import { useThinkingStore } from '@renderer/store/thinkingStore';
import ThinkingDots from './ThinkingDots';
import ToolCallRow from './ToolCallRow';

interface ThoughtLineProps {
  line: ThoughtLineType;
  streamKey: string;
}

const ThoughtLineComponent: React.FC<ThoughtLineProps> = ({ line, streamKey }) => {
  const toggleToolExpand = useThinkingStore((s) => s.toggleToolExpand);

  const handleToggle = useCallback(
    (lineId: string, toolId: string) => {
      toggleToolExpand(streamKey, lineId, toolId);
    },
    [toggleToolExpand, streamKey]
  );

  return (
    <div className='sf-stream-line'>
      {/* 行头 */}
      <div className='sf-line-header'>
        <span className='sf-line-dot' />
        <span className='sf-line-type'>{line.type}</span>
        <span className='sf-line-time'>{line.time}</span>
      </div>

      {/* 行文本 */}
      {line.content && <div className='sf-line-text'>{line.content}</div>}

      {/* 思考三点 */}
      <ThinkingDots done={line.thinkingDone || !line.isThinking} />

      {/* 状态徽章 */}
      {line.status && (
        <div className={`sf-line-status ${line.status.kind}`}>{line.status.text}</div>
      )}

      {/* 工具调用列表 */}
      {line.tools.map((tool) => (
        <ToolCallRow
          key={tool.id}
          tool={tool}
          lineId={line.id}
          streamKey={streamKey}
          onToggleExpand={handleToggle}
        />
      ))}
    </div>
  );
};

ThoughtLineComponent.displayName = 'ThoughtLine';

export default memo(ThoughtLineComponent);
