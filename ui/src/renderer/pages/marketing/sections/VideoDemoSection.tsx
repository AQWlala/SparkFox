/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * VideoDemoSection — 推理链可视化 GIF 演示区块（spec §三 12.5.2 / 第二十二波 sub-agent B）
 *
 * 卖点策略：声明式优势描述 — 通过 GIF 演示直观展示推理链 8 步流程 + MULTI_ES 多跳路径 + 超边激活
 *
 * 内容：
 *   1. reasoning_chain_demo.gif — Step1..Step8 完整 8 步多跳推理流程演示
 *   2. multihop_demo.gif        — MULTI_ES 多跳路径 + 超边激活演示
 *
 * 占位方案：当前为 1x1 像素最小有效 GIF 占位（43 字节），未来由 scripts/generate_demo_gif.sh
 *           在 dev server 运行时通过 ffmpeg + gifsicle 生成真实录屏 GIF（< 5MB）。
 *
 * REFACTOR：GIF 路径常量从 constants.ts 集中引入（REASONING_CHAIN_GIF / MULTIHOP_GIF）
 */

import React from 'react';
import { Card, Typography } from '@arco-design/web-react';
import { REASONING_CHAIN_GIF, MULTIHOP_GIF } from '../constants';

const { Title, Paragraph } = Typography;

/**
 * VideoDemoSection — 推理链可视化 GIF 演示区块
 *
 * 通过 2 个 GIF 演示直观展示推理链能力，采用「声明式优势描述」策略：
 *   - 左侧 GIF：Step1..Step8 完整 8 步多跳推理流程
 *   - 右侧 GIF：MULTI_ES 多跳路径 + 超边激活
 *
 * 设计风格：Apple system style（macOS），使用 Arco Design Card + 双栏 grid 布局
 */
export function VideoDemoSection() {
  return (
    <section
      className="marketing-video-demo"
      data-section="video-demo"
    >
      <Title heading={2} className="marketing-video-demo__title">
        推理链可视化演示
      </Title>
      <Paragraph className="marketing-video-demo__desc">
        通过 GIF 演示直观感受 MULTI 8 步多跳推理流程 + MULTI_ES 多跳路径 + 超边激活的可视化能力。
      </Paragraph>

      {/* 双栏 GIF 演示：左侧推理链 8 步流程 + 右侧 MULTI_ES 多跳路径 */}
      <div className="marketing-video-demo__grid">
        {/* 左侧：推理链 8 步流程演示 GIF（reasoning_chain_demo.gif） */}
        <Card className="marketing-video-demo__item" bordered={false}>
          <img
            src={REASONING_CHAIN_GIF}
            alt="推理链 8 步流程演示（Step1..Step8 完整 8 步多跳推理流程）"
            className="marketing-video-demo__gif"
          />
          <Title heading={5} className="marketing-video-demo__item-title">
            推理链 8 步流程
          </Title>
          <Paragraph className="marketing-video-demo__item-desc">
            Step1..Step8 完整 8 步多跳推理流程（query 向量化 → 实体抽取 → 实体检索 → 事件检索 →
            三策略合并 → chunk 关联 → Rerank → 返回）
          </Paragraph>
        </Card>

        {/* 右侧：MULTI_ES 多跳路径 + 超边激活演示 GIF（multihop_demo.gif） */}
        <Card className="marketing-video-demo__item" bordered={false}>
          <img
            src={MULTIHOP_GIF}
            alt="MULTI_ES 多跳路径 + 超边激活演示"
            className="marketing-video-demo__gif"
          />
          <Title heading={5} className="marketing-video-demo__item-title">
            MULTI_ES 多跳路径 + 超边激活
          </Title>
          <Paragraph className="marketing-video-demo__item-desc">
            MULTI_ES 多跳路径 + 超边激活（hop=1/2/3 高亮），ReasoningChainPanel +
            KnowledgeGraphView 协同展示实体 → 事件 → chunk 三级溯源。
          </Paragraph>
        </Card>
      </div>
    </section>
  );
}

export default VideoDemoSection;
