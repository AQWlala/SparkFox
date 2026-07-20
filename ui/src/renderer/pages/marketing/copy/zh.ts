/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * 营销页中文文案集中管理（spec §三 12.5.1 / 第二十一波 sub-step C REFACTOR 阶段）
 *
 * 设计原则：
 *   1. 集中管理营销页文案，便于国际化（i18n）扩展与文案审阅
 *   2. 短标识符 / 测试关键字符串（如 slogan、标题、"8 步" 等）仍保留在对应 Section
 *      文件中作为 inline literal —— 既是契约标识也便于源码扫描测试
 *   3. 较长的描述性文案（pillar 描述、step 描述、Statistic 标签等）在此集中维护
 *   4. 全部文案遵循「声明式优势描述」策略，不与外部产品直接对比
 */

/**
 * 营销页文案根对象
 *
 * 结构与 4 个 Section 一一对应：
 *   - hero           : HeroSection 文案
 *   - benchmark      : BenchmarkSection 文案
 *   - dataSovereignty : DataSovereigntySection 文案
 *   - reasoningChain : ReasoningChainSection 文案
 */
export const copy = {
  /** HeroSection — 首屏文案（标题「中文多跳 SOTA」与 data-section 标识保留在 HeroSection.tsx 内联） */
  hero: {
    tagline: '本地优先的多跳语义检索 · 端到端加密 · AGPL 守护',
  },

  /** BenchmarkSection — 数据展示文案（4 策略名 multi/multi1/hopllm/multi_es 保留在 BenchmarkSection.tsx 注释 + JSON 数据中） */
  benchmark: {
    colStrategy: '策略',
    colRecallAt5: 'Recall@5',
    colRecallAt10: 'Recall@10',
    colLatency: '平均延迟',
    statBestStrategy: '优选策略',
    statBestRecall: '最高 Recall@10',
    statStrategyCount: '策略数',
    datasetLabel: '数据来源',
  },

  /** DataSovereigntySection — 数据主权文案（slogan「别把第二大脑租给别人...」保留在 DataSovereigntySection.tsx 内联，作为契约字符串） */
  dataSovereignty: {
    /** 三大支柱：本地优先 / 端到端加密 / AGPL 合规 */
    pillars: [
      {
        title: '本地优先',
        description:
          '所有数据驻留本机，无云账号、无遥测、无订阅。唯一的外发流量是你显式配置的 LLM 调用。',
      },
      {
        title: '端到端加密',
        description: 'E2EE 保护记忆与思考链路，密钥仅本机持有，云服务无法读取。',
      },
      {
        title: 'AGPL 合规',
        description:
          'AGPL-3.0-only 强 copyleft 许可证，确保衍生作品同样开源，守护数据主权承诺。',
      },
    ],
  },

  /** ReasoningChainSection — 推理链文案（"8 步多跳推理"字符串保留在 ReasoningChainSection.tsx 内联，作为契约字符串） */
  reasoningChain: {
    /** MULTI 8 步流程的 8 个步骤描述（Step1-Step8） */
    steps: [
      'query 向量化',
      '实体抽取',
      '实体检索',
      '事件检索',
      '三策略合并（multi / multi1 / hopllm）',
      'chunk 关联',
      'Rerank',
      '返回 SearchResult',
    ],
    /** Step5 三策略合并卡片描述 */
    step5Desc:
      '多跳扩展（multi）+ 单跳剪枝（multi1）+ LLM 引导（hopllm）三路并行，Step5 合并去重确保 Recall 与延迟的平衡。',
    /** 实体超图（SAG）卡片描述 */
    sagDesc:
      'Semantic Agentic Graph — 实体 → 事件 → chunk 三级溯源，ReasoningChainPanel 高亮 hop=1/2/3 多跳路径，KnowledgeGraphView 可视化全图。',
  },
} as const;

export type MarketingCopy = typeof copy;
