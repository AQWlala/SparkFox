/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EntityLevel — L1 实体级子组件
 *
 * 对应 spec §三 10.10.1：在三级溯源抽屉中展示实体层信息
 * U-03 修复：三级溯源缺失问题
 */

import React from 'react';
import type { EntityRef } from './types';
import styles from './CitationDetailDrawer.module.css';

interface EntityLevelProps {
  entity: EntityRef;
}

const EntityLevel: React.FC<EntityLevelProps> = ({ entity }) => {
  return (
    <section className={`${styles.level} ${styles.levelL1}`} data-level='L1'>
      <header className={styles.levelHeader}>
        <span className={`${styles.levelTag} ${styles.levelTagL1}`}>L1</span>
        <span className={styles.levelTitle}>实体（Entity）</span>
      </header>
      <div className={styles.fieldGrid}>
        <span className={styles.fieldLabel}>entity_id</span>
        <span className={`${styles.fieldValue} ${styles.fieldValueMono}`}>{entity.entity_id}</span>

        <span className={styles.fieldLabel}>name</span>
        <span className={styles.fieldValue}>{entity.name}</span>

        <span className={styles.fieldLabel}>entity_type</span>
        <span className={styles.fieldValue}>{entity.entity_type}</span>
      </div>
    </section>
  );
};

export default EntityLevel;
