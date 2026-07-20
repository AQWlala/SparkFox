/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  EXTRACTION_STATUS_COLOR,
  EXTRACTION_STATUS_LABEL,
  EXTRACTION_STATUS_PROGRESS,
  type ExtractionStatus,
} from './constants';

/**
 * useExtractionStatus 返回的状态机信息
 */
export interface ExtractionStatusInfo {
  /** 进度百分比（0-100），来自 EXTRACTION_STATUS_PROGRESS 映射 */
  progress: number;
  /** 中文标签 */
  label: string;
  /** Arco 主题色 CSS 变量字符串 */
  color: string;
  /** 是否为终态（COMPLETED） */
  isCompleted: boolean;
  /** 是否为运行中状态（PARSING / EXTRACTING） */
  isRunning: boolean;
}

/**
 * U-05 修复：SAG 5 状态机 hook
 *
 * 把 status 字符串映射成进度条需要的 { progress, label, color, isCompleted, isRunning }，
 * 让进度条与后端实际状态严格联动，避免假进度条。
 *
 * @example
 * ```tsx
 * const { progress, label, color, isCompleted } = useExtractionStatus('EXTRACTING');
 * // progress === 80, label === '抽取中', color === 'rgb(var(--primary-6))', isCompleted === false
 * ```
 */
export function useExtractionStatus(status: ExtractionStatus): ExtractionStatusInfo {
  return {
    progress: EXTRACTION_STATUS_PROGRESS[status],
    label: EXTRACTION_STATUS_LABEL[status],
    color: EXTRACTION_STATUS_COLOR[status],
    isCompleted: status === 'COMPLETED',
    isRunning: status === 'PARSING' || status === 'EXTRACTING',
  };
}
