/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * BenchmarkSection — Benchmark 数据展示区块（spec §三 12.5.1 / 第二十一波 sub-step C）
 *
 * 卖点策略：声明式优势描述 — 展示 4 策略（multi / multi1 / hopllm / multi_es）的
 * Recall@5 / Recall@10 / 平均延迟对比，数据来自 zh_multihop 数据集（50 查询）。
 *
 * 数据来源：./data/benchmark_results.json（v1.1.0 第二十波预估值，Task 12.3.2 完成后将更新）
 *
 * 采用声明式优势描述，仅展示 SparkFox 自身 4 策略（multi / multi1 / hopllm / multi_es）对比。
 *
 * REFACTOR：表格列标题 / Statistic 标签从 copy/zh.ts 集中引入
 */

import React from 'react';
import { Card, Progress, Statistic, Typography, Table } from '@arco-design/web-react';
import benchmarkResults from '../data/benchmark_results.json';
import { copy } from '../copy/zh';

const { Title, Paragraph } = Typography;

/**
 * BenchmarkSection — 4 策略对比数据展示
 *
 * 策略说明：
 *   - multi    ：BFS 多跳扩展（max_hop=3）
 *   - multi1   ：单跳剪枝（性能优先）
 *   - hopllm   ：LLM 引导多跳扩展（语义优先）
 *   - multi_es ：Elasticsearch 后端多跳（Recall@10 > 0.85 的优选策略）
 */
export function BenchmarkSection() {
  const { strategies, dataset } = benchmarkResults;

  // 表格列定义：策略名 / Recall@5 / Recall@10 / 平均延迟（标签从 copy/zh.ts 引入）
  const columns = [
    {
      title: copy.benchmark.colStrategy,
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: copy.benchmark.colRecallAt5,
      dataIndex: 'recall_at_5',
      key: 'recall_at_5',
      render: (value: number) => `${(value * 100).toFixed(1)}%`,
    },
    {
      title: copy.benchmark.colRecallAt10,
      dataIndex: 'recall_at_10',
      key: 'recall_at_10',
      render: (value: number) => `${(value * 100).toFixed(1)}%`,
    },
    {
      title: copy.benchmark.colLatency,
      dataIndex: 'avg_latency_ms',
      key: 'avg_latency_ms',
      render: (value: number) => `${value} ms`,
    },
  ];

  // 找出 Recall@10 最高的策略（multi_es，用于高亮）
  const bestStrategy = strategies.reduce((best, s) =>
    s.recall_at_10 > best.recall_at_10 ? s : best
  );

  return (
    <section className="marketing-benchmark" data-section="benchmark">
      <Title heading={2} className="marketing-benchmark__title">
        Benchmark 数据展示 · 4 策略对比
      </Title>
      <Paragraph className="marketing-benchmark__desc">
        SparkFox 在 zh_multihop 数据集上对比 4 种多跳检索策略（multi / multi1 / hopllm / multi_es）。
      </Paragraph>

      <Card className="marketing-benchmark__card" bordered={false}>
        {/* 4 策略汇总指标：multi / multi1 / hopllm / multi_es */}
        <div className="marketing-benchmark__stats">
          <Statistic title={copy.benchmark.statBestStrategy} value={bestStrategy.name} />
          <Statistic
            title={copy.benchmark.statBestRecall}
            value={(bestStrategy.recall_at_10 * 100).toFixed(1)}
            suffix="%"
          />
          <Statistic title={copy.benchmark.statStrategyCount} value={strategies.length} />
        </div>

        {/* Recall@10 进度条对比 */}
        <div className="marketing-benchmark__progress">
          {strategies.map((s) => (
            <div key={s.name} className="marketing-benchmark__progress-row">
              <span className="marketing-benchmark__progress-label">{s.name}</span>
              <Progress
                percent={Number((s.recall_at_10 * 100).toFixed(1))}
                status={s.name === 'multi_es' ? 'success' : 'normal'}
              />
            </div>
          ))}
        </div>

        {/* 详细对比表 */}
        <Table
          className="marketing-benchmark__table"
          columns={columns}
          data={strategies}
          rowKey="name"
          pagination={false}
          size="small"
        />

        <Paragraph className="marketing-benchmark__note">
          {copy.benchmark.datasetLabel}: {dataset}
        </Paragraph>
      </Card>
    </section>
  );
}

export default BenchmarkSection;
