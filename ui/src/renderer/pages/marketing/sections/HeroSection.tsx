/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * HeroSection — 营销页首屏（spec §三 12.5.1 / 第二十一波 sub-step C）
 *
 * 卖点策略：声明式优势描述（不与外部产品直接对比）+ 数据主权强调
 * 内容：标题「中文多跳 SOTA」+ 副标题「SparkFox v1.1.0 推理引擎」+ 简短卖点描述
 *
 * REFACTOR：tagline 长文案从 copy/zh.ts 集中引入；
 *           「中文多跳 SOTA」标题与 zh-multihop-sota 标识作为契约字符串保留 inline
 */

import React from 'react';
import { Typography } from '@arco-design/web-react';
import { copy } from '../copy/zh';

const { Title, Paragraph } = Typography;

/**
 * HeroSection — 首屏
 *
 * 卖点：中文多跳 SOTA（zh-multihop-sota）
 * 文案策略：声明式优势描述，强调本地推理能力，不与外部产品直接对比
 */
export function HeroSection() {
  return (
    <section className="marketing-hero" data-section="zh-multihop-sota">
      <Title heading={1} className="marketing-hero__title">
        中文多跳 SOTA
      </Title>
      <Paragraph className="marketing-hero__subtitle">
        SparkFox v1.1.0 推理引擎
      </Paragraph>
      <Paragraph className="marketing-hero__tagline">
        {copy.hero.tagline}
      </Paragraph>
    </section>
  );
}

export default HeroSection;
