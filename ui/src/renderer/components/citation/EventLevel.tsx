/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EventLevel — L2 事件级子组件
 *
 * 对应 spec §三 10.10.1：在三级溯源抽屉中展示事件层三元组
 * U-03 修复：三级溯源缺失问题
 */

import React from 'react';
import type { EventRef } from './types';
import styles from './CitationDetailDrawer.module.css';

interface EventLevelProps {
  event: EventRef;
}

const EventLevel: React.FC<EventLevelProps> = ({ event }) => {
  return (
    <section className={`${styles.level} ${styles.levelL2}`} data-level='L2'>
      <header className={styles.levelHeader}>
        <span className={`${styles.levelTag} ${styles.levelTagL2}`}>L2</span>
        <span className={styles.levelTitle}>事件（Event 三元组）</span>
      </header>
      <div className={styles.fieldGrid}>
        <span className={styles.fieldLabel}>event_id</span>
        <span className={`${styles.fieldValue} ${styles.fieldValueMono}`}>{event.event_id}</span>

        <span className={styles.fieldLabel}>subject</span>
        <span className={styles.fieldValue}>{event.subject}</span>

        <span className={styles.fieldLabel}>predicate</span>
        <span className={styles.fieldValue}>{event.predicate}</span>

        <span className={styles.fieldLabel}>object</span>
        <span className={styles.fieldValue}>{event.object}</span>
      </div>
    </section>
  );
};

export default EventLevel;
