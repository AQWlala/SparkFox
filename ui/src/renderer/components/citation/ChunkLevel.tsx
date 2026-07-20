/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ChunkLevel — L3 chunk 级子组件
 *
 * 对应 spec §三 10.10.1：在三级溯源抽屉中展示原文 chunk 层
 * U-03 修复：三级溯源缺失问题
 *
 * 注：ChunkRef 在 Citation 中可能为 null（PRIMARY 策略下 chunk 不一定存在），
 * 此时由父组件 CitationDetailDrawer 决定不渲染 ChunkLevel。
 */

import React from 'react';
import type { ChunkRef } from './types';
import styles from './CitationDetailDrawer.module.css';

interface ChunkLevelProps {
  chunk: ChunkRef;
}

const ChunkLevel: React.FC<ChunkLevelProps> = ({ chunk }) => {
  const [start, end] = chunk.span;
  return (
    <section className={`${styles.level} ${styles.levelL3}`} data-level='L3'>
      <header className={styles.levelHeader}>
        <span className={`${styles.levelTag} ${styles.levelTagL3}`}>L3</span>
        <span className={styles.levelTitle}>原文片段（Chunk）</span>
      </header>
      <div className={styles.fieldGrid}>
        <span className={styles.fieldLabel}>chunk_id</span>
        <span className={`${styles.fieldValue} ${styles.fieldValueMono}`}>{chunk.chunk_id}</span>

        <span className={styles.fieldLabel}>span</span>
        <span className={styles.fieldValue}>
          <span className={styles.chunkSpanBadge}>
            [{start}, {end})
          </span>
        </span>
      </div>
      <div className={styles.chunkText}>{chunk.text}</div>
    </section>
  );
};

export default ChunkLevel;
