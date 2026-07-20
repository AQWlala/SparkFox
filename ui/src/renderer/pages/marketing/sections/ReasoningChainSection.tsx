/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ReasoningChainSection — 推理链可视化卖点区块（spec §三 12.5.1 / 第二十一波 sub-step C）
 *
 * 卖点策略：声明式优势描述
 * 内容：
 *   - MULTI 8 步多跳推理流程（query 向量化 → 实体抽取 → 实体检索 → 事件检索 →
 *     三策略合并 → chunk 关联 → Rerank → 返回）
 *   - Step5 三策略合并（multi / multi1 / hopllm）
 *   - 实体超图（SAG，Semantic Agentic Graph）
 *   - 推理链可视化（ReasoningChainPanel / KnowledgeGraphView 的能力描述）
 *
 * REFACTOR：8 步流程描述 / Step5 / SAG 描述从 copy/zh.ts 集中引入；
 *           "8 步多跳推理"字符串保留 inline 作为契约字符串（测试扫描源码）
 */

import React from 'react';
import { Card, Steps, Typography } from '@arco-design/web-react';
import { copy } from '../copy/zh';

const { Title, Paragraph } = Typography;

/**
 * ReasoningChainSection — 推理链可视化卖点
 *
 * 突出 MULTI 8 步多跳推理流程 + Step5 三策略合并 + 实体超图（SAG）能力
 * 采用声明式优势描述，仅声明 SparkFox 自身能力
 */
export function ReasoningChainSection() {
  return (
    <section className="marketing-reasoning-chain" data-section="reasoning-chain">
      <Title heading={2} className="marketing-reasoning-chain__title">
        推理链可视化
      </Title>
      <Paragraph className="marketing-reasoning-chain__desc">
        MULTI 8 步多跳推理流程透明可审计，每一步命中实体与跳数都可追溯。
      </Paragraph>

      {/* MULTI 8 步多跳推理流程可视化（步骤描述从 copy/zh.ts 引入） */}
      <Card className="marketing-reasoning-chain__card" bordered={false}>
        <Title heading={4} className="marketing-reasoning-chain__card-title">
          MULTI 8 步多跳推理流程
        </Title>
        <Steps
          direction="vertical"
          current={-1}
          className="marketing-reasoning-chain__steps"
        >
          {copy.reasoningChain.steps.map((step, idx) => (
            <Steps.Step
              key={idx}
              title={`Step ${idx + 1}`}
              description={step}
            />
          ))}
        </Steps>
      </Card>

      {/* Step5 三策略合并 + 实体超图（SAG）说明卡片 */}
      <div className="marketing-reasoning-chain__highlights">
        <Card className="marketing-reasoning-chain__highlight" bordered={false}>
          <Title heading={5}>Step5 三策略合并</Title>
          <Paragraph>{copy.reasoningChain.step5Desc}</Paragraph>
        </Card>
        <Card className="marketing-reasoning-chain__highlight" bordered={false}>
          <Title heading={5}>实体超图（SAG）</Title>
          <Paragraph>{copy.reasoningChain.sagDesc}</Paragraph>
        </Card>
      </div>
    </section>
  );
}

export default ReasoningChainSection;
