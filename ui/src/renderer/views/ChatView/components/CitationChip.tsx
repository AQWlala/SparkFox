/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * CitationChip — 引用标签组件（F2）
 *
 * 用于在 AI 回复中展示知识库检索命中的文档块引用：
 * - 不同检索来源（Vector/Keyword/Rerank）用不同颜色区分
 * - 鼠标悬浮显示文档 ID、块 ID、来源与分数
 * - 点击触发 onClick 回调（可用于跳转到原文位置）
 */

import { Tag, Tooltip } from '@arco-design/web-react';

/** 引用数据结构 —— 与 spec 1958-1985 行一致 */
export interface CitationData {
  kdoc_id: string;
  chunk_id: number;
  span_start: number;
  span_end: number;
  score: number;
  source: 'Vector' | 'Keyword' | 'Rerank';
}

interface CitationChipProps {
  citation: CitationData;
  onClick?: (c: CitationData) => void;
}

/** 引用 chip —— 按 source 颜色区分（Vector=蓝 / Keyword=绿 / Rerank=橙） */
export function CitationChip({ citation, onClick }: CitationChipProps) {
  const sourceColor = citation.source === 'Vector' ? 'blue'
    : citation.source === 'Keyword' ? 'green' : 'orange';
  return (
    <Tooltip content={`文档 ${citation.kdoc_id} 块 ${citation.chunk_id}（${citation.source}，分数 ${citation.score.toFixed(3)})`}>
      {/* Arco Tag 无 clickable 属性，用 onClick + cursor:pointer 实现可点击 */}
      <Tag color={sourceColor} onClick={() => onClick?.(citation)} style={{ cursor: 'pointer' }}>
        [{citation.chunk_id}]
      </Tag>
    </Tooltip>
  );
}

export default CitationChip;
