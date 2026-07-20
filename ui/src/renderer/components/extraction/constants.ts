/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com)
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * U-05 修复：SAG 5 状态机常量定义
 *
 * 后端 `sparkfox-knowledge` crate 的 `knowledge_event.status` 字段有 5 种状态，
 * 严格按 PENDING → PARSING → PARSED → EXTRACTING → COMPLETED 转换。
 *
 * 每个状态对应一个「与状态机联动」的固定进度百分比（非简单线性插值，避免假进度条
 * 与真实状态脱节）：
 *   - PENDING:    10%  （待处理：已入队，等待 worker 拾取）
 *   - PARSING:    30%  （解析中：文档读取 + 切片）
 *   - PARSED:     50%  （已解析：切片完成，准备 LLM 抽取）
 *   - EXTRACTING: 80%  （抽取中：LLM 提取 event/entity）
 *   - COMPLETED:  100% （已完成：可被检索）
 *
 * @see docs/SparkFox-v1.1.0-规划.md §10.11.1
 */

/** SAG 抽取状态机 5 状态字面量类型 */
export type ExtractionStatus = 'PENDING' | 'PARSING' | 'PARSED' | 'EXTRACTING' | 'COMPLETED';

/**
 * 状态机严格转换顺序：PENDING → PARSING → PARSED → EXTRACTING → COMPLETED
 *
 * 用于：
 *   - 计算 Steps 当前步骤索引
 *   - 校验后端推送的状态跳变是否合法（不能回退）
 */
export const EXTRACTION_STATUS_ORDER: ExtractionStatus[] = [
  'PENDING',
  'PARSING',
  'PARSED',
  'EXTRACTING',
  'COMPLETED',
];

/**
 * 状态 → 进度百分比映射（U-05 核心修复点）
 *
 * 进度条与状态机严格联动：进入某状态即跳到对应百分比，不再随时间线性增长，
 * 避免出现「进度条到 90% 但实际还在 PARSING」的脱节现象。
 */
export const EXTRACTION_STATUS_PROGRESS: Record<ExtractionStatus, number> = {
  PENDING: 10,
  PARSING: 30,
  PARSED: 50,
  EXTRACTING: 80,
  COMPLETED: 100,
};

/** 状态 → 中文标签映射 */
export const EXTRACTION_STATUS_LABEL: Record<ExtractionStatus, string> = {
  PENDING: '待处理',
  PARSING: '解析中',
  PARSED: '已解析',
  EXTRACTING: '抽取中',
  COMPLETED: '已完成',
};

/**
 * 状态 → Arco 主题色 CSS 变量映射
 *
 * - PENDING 用 warning 色（提示用户等待）
 * - PARSING / PARSED / EXTRACTING 用 primary 色（进行中）
 * - COMPLETED 用 success 色（成功）
 */
export const EXTRACTION_STATUS_COLOR: Record<ExtractionStatus, string> = {
  PENDING: 'rgb(var(--warning-6))',
  PARSING: 'rgb(var(--primary-6))',
  PARSED: 'rgb(var(--primary-6))',
  EXTRACTING: 'rgb(var(--primary-6))',
  COMPLETED: 'rgb(var(--success-6))',
};
