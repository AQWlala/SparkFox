/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * CitationDetailDrawer — 三级溯源抽屉主组件
 *
 * 对应 spec §三 10.10.1 + 10.10.2（U-03 修复：三级溯源缺失问题）
 *
 * 三级层级：
 *   L1 实体（EntityLevel）  — entity_id + name + entity_type
 *   L2 事件（EventLevel）   — event_id + subject + predicate + object
 *   L3 chunk（ChunkLevel）  — chunk_id + span + 原文片段（可选，chunk=null 时不渲染）
 *
 * 集成 Arco Design Drawer，受控 visible + onClose 回调。
 * 由 CitationChip 点击触发，集成到 ChatView/ChatMessage 由主 agent 在第十波验收时统一完成。
 */

import React from 'react';
import { Drawer } from '@arco-design/web-react';

import EntityLevel from './EntityLevel';
import EventLevel from './EventLevel';
import ChunkLevel from './ChunkLevel';
import type { Citation } from './types';
import styles from './CitationDetailDrawer.module.css';

export interface CitationDetailDrawerProps {
  citation: Citation | null;
  visible: boolean;
  onClose: () => void;
}

const CitationDetailDrawer: React.FC<CitationDetailDrawerProps> = ({ citation, visible, onClose }) => {
  return (
    <Drawer
      title='三级溯源'
      visible={visible}
      onCancel={onClose}
      placement='right'
      width={520}
      okButtonProps={{ style: { display: 'none' } }}
      cancelText='关闭'
    >
      {!citation ? (
        <div className={styles.placeholder}>暂无溯源信息</div>
      ) : (
        <div className={styles.drawerBody}>
          <EntityLevel entity={citation.entity} />
          <EventLevel event={citation.event} />
          {citation.chunk ? <ChunkLevel chunk={citation.chunk} /> : null}
        </div>
      )}
    </Drawer>
  );
};

export default CitationDetailDrawer;
