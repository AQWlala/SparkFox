/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * 营销页 GIF 演示资源路径常量（spec §三 12.5.2 / 第二十二波 sub-agent B REFACTOR 阶段）
 *
 * 设计原则：
 *   1. 集中管理 GIF 资源路径常量，便于未来调整路径或切换为 CDN
 *   2. 路径字符串作为契约标识保留 —— 测试既扫描源码也检查文件存在性 + 大小
 *
 * GIF 录制规范（见 scripts/generate_demo_gif.sh）：
 *   - 时长：30s 循环（reasoning_chain）/ 20s 循环（multihop）
 *   - 分辨率：800px 宽，高度按比例
 *   - 帧率：10 fps
 *   - 压缩：gifsicle --optimize=3 --colors=128
 *   - 体积上限：< 5MB（验收标准）
 */

/**
 * 推理链 8 步流程演示 GIF 路径
 *
 * 内容：Step1..Step8 完整 8 步多跳推理流程
 *       （query 向量化 → 实体抽取 → 实体检索 → 事件检索 → 三策略合并 → chunk 关联 → Rerank → 返回）
 */
export const REASONING_CHAIN_GIF = './assets/reasoning_chain_demo.gif';

/**
 * MULTI_ES 多跳路径 + 超边激活演示 GIF 路径
 *
 * 内容：MULTI_ES 多跳路径 + 超边激活（hop=1/2/3 高亮）
 *       展示 ReasoningChainPanel + KnowledgeGraphView 的可视化能力
 */
export const MULTIHOP_GIF = './assets/multihop_demo.gif';
