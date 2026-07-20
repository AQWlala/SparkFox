/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * DataSovereigntySection — 数据主权卖点区块（spec §三 12.5.1 / 第二十一波 sub-step C）
 *
 * 卖点策略：声明式优势描述 + 数据主权强调（核心 slogan 来自 README.md）
 * 三大卖点：本地优先 + 端到端加密 + AGPL 合规
 *
 * Slogan：「别把第二大脑租给别人——你的思考，不该成为别人的养料」
 *         （来自 README.md line 10，营销文案中集中体现数据主权主张）
 *
 * REFACTOR：三大支柱文案从 copy/zh.ts 集中引入；
 *           slogan 作为契约字符串保留 inline（测试扫描 DataSovereigntySection.tsx 源码）
 */

import React from 'react';
import { Card, Typography } from '@arco-design/web-react';
import { copy } from '../copy/zh';

const { Title, Paragraph } = Typography;

/**
 * DataSovereigntySection — 数据主权三大支柱
 *
 * 设计哲学：用户偏好「声明式优势描述」，采用 slogan 阐述数据主权的价值观主张，
 * 不直接与外部产品对比。
 *
 * Slogan 来自 README.md：「别把第二大脑租给别人——你的思考，不该成为别人的养料」
 */
export function DataSovereigntySection() {
  return (
    <section className="marketing-data-sovereignty" data-section="data-sovereignty">
      <Title heading={2} className="marketing-data-sovereignty__title">
        数据主权至上
      </Title>

      {/* 核心 slogan — 数据主权价值观宣言（来自 README.md，契约字符串保留 inline） */}
      <Paragraph className="marketing-data-sovereignty__slogan">
        别把第二大脑租给别人——你的思考，不该成为别人的养料
      </Paragraph>

      {/* 三大支柱：本地优先 / 端到端加密 / AGPL 合规（文案从 copy/zh.ts 引入） */}
      <div className="marketing-data-sovereignty__pillars">
        {copy.dataSovereignty.pillars.map((pillar) => (
          <Card
            key={pillar.title}
            className="marketing-data-sovereignty__pillar"
            bordered={false}
          >
            <Title heading={4} className="marketing-data-sovereignty__pillar-title">
              {pillar.title}
            </Title>
            <Paragraph className="marketing-data-sovereignty__pillar-desc">
              {pillar.description}
            </Paragraph>
          </Card>
        ))}
      </div>
    </section>
  );
}

export default DataSovereigntySection;
