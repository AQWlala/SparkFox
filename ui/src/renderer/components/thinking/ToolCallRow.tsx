/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ToolCallRow — 工具调用行组件
 *
 * 来源：BaiLongma src/ui/brain-ui/thought-stream.js tool() 方法（清洁室重写为 React 组件）
 *
 * 保留 BaiLongma 特性：
 * - 图标 + 中文名 + subject + 状态徽章 + 折叠箭头
 * - chevron 旋转动画（▸ → ▾）
 * - 折叠态默认，点击展开 detail
 * - 状态颜色：success（绿）/ failed（中性灰）/ ended（中性灰）
 * - 工具调用结束行（__cycle_end__）
 *
 * 改造点：
 * - DOM addEventListener click → React onClick
 * - classList.toggle → React state（由 thinkingStore 管理）
 * - BaiLongma CSS 变量颜色 → Apple 主题色
 */

import React, { memo, useCallback } from 'react';
import { TOOL_ZH, TOOL_ICON } from '@renderer/utils/sparkfox/toolMap';
import type { ToolCall } from '@renderer/store/thinkingStore';

interface ToolCallRowProps {
  tool: ToolCall;
  lineId: string;
  streamKey: string;
  onToggleExpand?: (lineId: string, toolId: string) => void;
}

const ToolCallRowComponent: React.FC<ToolCallRowProps> = ({
  tool,
  lineId,
  streamKey,
  onToggleExpand,
}) => {
  const { name, subject, detail, expandable, expanded, status, ok } = tool;

  // 工具调用结束行（__cycle_end__）
  const isCycleEnd = name === '__cycle_end__';
  const icon = isCycleEnd ? (ok === false ? '⚠' : '◎') : TOOL_ICON[name] || '🔧';
  const zh = isCycleEnd
    ? ok === false
      ? '工具调用结束'
      : '本轮结束'
    : TOOL_ZH[name] || name;

  // 状态徽章
  const statusIcon = isCycleEnd
    ? ok === false
      ? '已结束'
      : '完成'
    : status === 'failed'
      ? '✗ 失败'
      : status === 'ended'
        ? '完成'
        : '✓ 成功';

  const statusCls = isCycleEnd
    ? ok === false
      ? 'failed'
      : 'ended'
    : status === 'failed'
      ? 'failed'
      : status === 'ended'
        ? 'ended'
        : 'success';

  const handleClick = useCallback(() => {
    if (!expandable || isCycleEnd) return;
    onToggleExpand?.(lineId, tool.id);
  }, [expandable, isCycleEnd, lineId, tool.id, onToggleExpand]);

  return (
    <>
      <div
        className={`sf-line-tool done tool-${statusCls}${expandable && !isCycleEnd ? ' expandable' : ''}${expanded ? ' expanded' : ''}`}
        onClick={handleClick}
        role={expandable && !isCycleEnd ? 'button' : undefined}
        tabIndex={expandable && !isCycleEnd ? 0 : undefined}
      >
        <span className='sf-tool-chevron'>
          {expandable && !isCycleEnd ? (expanded ? '▾' : '▸') : ''}
        </span>
        <span className='sf-tool-icon'>{icon}</span>
        <span className='sf-tool-name'>{zh}</span>
        {subject && (
          <>
            <span className='sf-tool-sep'>·</span>
            <span className='sf-tool-subject' title={subject}>
              {subject}
            </span>
          </>
        )}
        <span className={`sf-tool-status ${statusCls}`}>{statusIcon}</span>
      </div>
      {expandable && !isCycleEnd && (
        <div className={`sf-line-tool-detail${expanded ? '' : ' collapsed'}`}>{detail}</div>
      )}
    </>
  );
};

ToolCallRowComponent.displayName = 'ToolCallRow';

export default memo(ToolCallRowComponent);
