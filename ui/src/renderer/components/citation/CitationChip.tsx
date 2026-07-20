/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * CitationChip — 点击触发三级溯源抽屉的 chip 组件
 *
 * 对应 spec §三 10.10.2（U-03 修复：三级溯源缺失问题）
 *
 * 用法：
 *   <CitationChip citation={citation}>[1]</CitationChip>
 *
 * 行为：
 *   - 默认渲染为可点击的小徽标（继承 children 显示编号或符号）
 *   - 内部 useState 控制抽屉 visible
 *   - 点击 chip 打开 CitationDetailDrawer，展示 L1/L2/L3 三级溯源
 *
 * 集成到 ChatView/ChatMessage 由主 agent 在第十波验收时统一完成，
 * 当前组件不修改任何现有文件。
 */

import React, { ReactNode, useState } from 'react';

import CitationDetailDrawer from './CitationDetailDrawer';
import type { Citation } from './types';

export interface CitationChipProps {
  citation: Citation;
  children?: ReactNode;
}

const CitationChip: React.FC<CitationChipProps> = ({ citation, children }) => {
  const [visible, setVisible] = useState(false);

  return (
    <>
      <button
        type='button'
        onClick={() => setVisible(true)}
        className='sf-citation-chip'
        data-role='citation-chip'
        aria-label='查看三级溯源'
      >
        {children ?? `[${citation.entity.entity_id.slice(0, 4)}]`}
      </button>

      <CitationDetailDrawer
        citation={citation}
        visible={visible}
        onClose={() => setVisible(false)}
      />
    </>
  );
};

export default CitationChip;
