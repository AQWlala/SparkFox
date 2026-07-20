/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox TokenStatsPanel — Token 用量统计面板
 *
 * 来源：OpenAkita TokenStatsView（清洁室重写）
 *
 * 保留 OpenAkita 特性：
 * - 6 个时间周期切换（1d/3d/1w/1m/6m/1y）
 * - 按端点分组 + 按操作类型分组 + MiniBar 占比可视化
 * - 时间线柱状图（24 小时 / 按天）
 * - 会话列表 + 用量记录表
 *
 * 改造点：
 * - shadcn/ui Card/Table → Arco Design Card + 原生 table
 * - safeFetch HTTP → useMonitorStore
 * - i18n → 硬编码中文
 * - Apple 主题：圆角 12px + 系统蓝 + SF Pro
 */

import React from 'react';
import { Card, Button, Table, Tag } from '@arco-design/web-react';
import {
  useMonitorStore,
  PERIOD_KEYS,
  PERIOD_LABELS,
  OPERATION_LABELS,
  fmtNum,
  fmtCost,
  fmtTime,
  type PeriodKey,
  type TokenSummaryRow,
  type TokenTimelineRow,
  type SessionRow,
  type UsageRecordRow,
} from '@renderer/store/monitorStore';
import { MiniBar } from './MiniBar';

const STAT_COLORS = ['#007AFF', '#34C759', '#FF9500', '#5856D6', '#FF2D55'];

/** 时间线柱状图（简易版，用 div 模拟） */
const TimelineChart: React.FC<{ data: TokenTimelineRow[] }> = ({ data }) => {
  const maxTokens = Math.max(...data.map((d) => d.total_tokens), 1);

  return (
    <div className='sf-timeline-chart'>
      {data.map((row, i) => {
        const heightPct = (row.total_tokens / maxTokens) * 100;
        const color = STAT_COLORS[i % STAT_COLORS.length];
        return (
          <div key={i} className='sf-timeline-bar-col' title={`${row.time_bucket}: ${fmtNum(row.total_tokens)} tokens`}>
            <div className='sf-timeline-bar-track'>
              <div
                className='sf-timeline-bar-fill'
                style={{
                  height: `${heightPct}%`,
                  background: `linear-gradient(180deg, ${color}, ${color}AA)`,
                }}
              />
            </div>
            <span className='sf-timeline-bar-label'>
              {row.time_bucket}
            </span>
          </div>
        );
      })}
    </div>
  );
};

/** 分组统计表（按端点 / 按操作类型） */
const SummaryTable: React.FC<{
  title: string;
  rows: TokenSummaryRow[];
  labelMap?: Record<string, string>;
}> = ({ title, rows, labelMap }) => {
  const maxTokens = Math.max(...rows.map((r) => r.total_tokens), 1);

  return (
    <Card className='sf-token-card' bordered={false}>
      <div className='sf-token-card-header'>
        <h3>{title}</h3>
        <span className='sf-token-card-count'>{rows.length} 项</span>
      </div>
      <div className='sf-token-summary-list'>
        {rows.map((row, i) => {
          const label = labelMap?.[row.group_key] || row.group_key;
          const color = STAT_COLORS[i % STAT_COLORS.length];
          const pct = (row.total_tokens / maxTokens) * 100;
          return (
            <div key={row.group_key} className='sf-token-summary-row'>
              <div className='sf-token-summary-row-header'>
                <span className='sf-token-summary-label' style={{ color }}>
                  ● {label}
                </span>
                <span className='sf-token-summary-tokens'>{fmtNum(row.total_tokens)} tokens</span>
              </div>
              <MiniBar value={row.total_tokens} max={maxTokens} color={color} height={4} />
              <div className='sf-token-summary-row-meta'>
                <span>请求 {row.request_count} 次</span>
                <span>输入 {fmtNum(row.total_input)}</span>
                <span>输出 {fmtNum(row.total_output)}</span>
                <span>费用 {fmtCost(row.total_cost)}</span>
                <span className='sf-token-summary-pct'>{pct.toFixed(1)}%</span>
              </div>
            </div>
          );
        })}
      </div>
    </Card>
  );
};

/** 会话列表 */
const SessionList: React.FC<{ sessions: SessionRow[] }> = ({ sessions }) => {
  return (
    <Card className='sf-token-card' bordered={false}>
      <div className='sf-token-card-header'>
        <h3>会话列表</h3>
        <span className='sf-token-card-count'>{sessions.length} 个会话</span>
      </div>
      <Table
        data={sessions}
        rowKey='session_id'
        pagination={false}
        scroll={{ y: 300 }}
        size='small'
        columns={[
          {
            title: '会话 ID',
            dataIndex: 'session_id',
            width: 120,
            render: (v: string) => <span className='sf-token-mono'>{v.slice(0, 12)}…</span>,
          },
          {
            title: '操作类型',
            dataIndex: 'operation_types',
            width: 100,
            render: (v: string) => (
              <Tag size='small' color='blue'>
                {OPERATION_LABELS[v] || v}
              </Tag>
            ),
          },
          {
            title: '端点',
            dataIndex: 'endpoints',
            width: 140,
          },
          {
            title: '请求数',
            dataIndex: 'request_count',
            width: 80,
            align: 'center' as const,
          },
          {
            title: '总 Tokens',
            dataIndex: 'total_tokens',
            width: 100,
            align: 'right' as const,
            render: (v: number) => fmtNum(v),
          },
          {
            title: '费用',
            dataIndex: 'total_cost',
            width: 90,
            align: 'right' as const,
            render: (v: number) => fmtCost(v),
          },
          {
            title: '最后调用',
            dataIndex: 'last_call',
            width: 100,
            render: (v: string) => fmtTime(v),
          },
        ]}
      />
    </Card>
  );
};

/** 用量记录表 */
const RecordsTable: React.FC<{ records: UsageRecordRow[] }> = ({ records }) => {
  return (
    <Card className='sf-token-card' bordered={false}>
      <div className='sf-token-card-header'>
        <h3>用量记录</h3>
        <span className='sf-token-card-count'>最近 {records.length} 条</span>
      </div>
      <Table
        data={records}
        rowKey='request_id'
        pagination={false}
        scroll={{ y: 300 }}
        size='small'
        columns={[
          {
            title: '时间',
            dataIndex: 'timestamp',
            width: 100,
            render: (v: string) => fmtTime(v),
          },
          {
            title: '端点',
            dataIndex: 'endpoint_name',
            width: 140,
          },
          {
            title: '模型',
            dataIndex: 'model',
            width: 140,
            render: (v: string) => <span className='sf-token-mono'>{v}</span>,
          },
          {
            title: '操作',
            dataIndex: 'operation_type',
            width: 100,
            render: (v: string) => (
              <Tag size='small' color='purple'>
                {OPERATION_LABELS[v] || v}
              </Tag>
            ),
          },
          {
            title: '输入',
            dataIndex: 'input_tokens',
            width: 80,
            align: 'right' as const,
            render: (v: number) => fmtNum(v),
          },
          {
            title: '输出',
            dataIndex: 'output_tokens',
            width: 80,
            align: 'right' as const,
            render: (v: number) => fmtNum(v),
          },
          {
            title: '总计',
            dataIndex: 'total_tokens',
            width: 90,
            align: 'right' as const,
            render: (v: number) => <strong>{fmtNum(v)}</strong>,
          },
          {
            title: '费用',
            dataIndex: 'estimated_cost',
            width: 90,
            align: 'right' as const,
            render: (v: number) => fmtCost(v),
          },
        ]}
      />
    </Card>
  );
};

export const TokenStatsPanel: React.FC = () => {
  const period = useMonitorStore((s) => s.period);
  const setPeriod = useMonitorStore((s) => s.setPeriod);
  const loading = useMonitorStore((s) => s.loading);
  const total = useMonitorStore((s) => s.total);
  const byEndpoint = useMonitorStore((s) => s.byEndpoint);
  const byOp = useMonitorStore((s) => s.byOp);
  const timeline = useMonitorStore((s) => s.timeline);
  const sessions = useMonitorStore((s) => s.sessions);
  const records = useMonitorStore((s) => s.records);

  return (
    <div className='sf-token-stats-panel'>
      {/* 时间周期切换 + 总计 */}
      <Card className='sf-token-card' bordered={false}>
        <div className='sf-token-period-header'>
          <h3>Token 用量统计</h3>
          <div className='sf-token-period-tabs'>
            {PERIOD_KEYS.map((k) => (
              <button
                key={k}
                type='button'
                className={`sf-token-period-tab${period === k ? ' active' : ''}`}
                onClick={() => setPeriod(k as PeriodKey)}
                disabled={loading}
              >
                {PERIOD_LABELS[k as PeriodKey]}
              </button>
            ))}
          </div>
        </div>

        {/* 总计数字 */}
        {total && (
          <div className='sf-token-total-grid'>
            <div className='sf-token-total-cell'>
              <div className='sf-token-total-label'>请求次数</div>
              <div className='sf-token-total-value'>{fmtNum(total.request_count)}</div>
            </div>
            <div className='sf-token-total-cell'>
              <div className='sf-token-total-label'>输入 Tokens</div>
              <div className='sf-token-total-value' style={{ color: '#007AFF' }}>
                {fmtNum(total.total_input)}
              </div>
            </div>
            <div className='sf-token-total-cell'>
              <div className='sf-token-total-label'>输出 Tokens</div>
              <div className='sf-token-total-value' style={{ color: '#34C759' }}>
                {fmtNum(total.total_output)}
              </div>
            </div>
            <div className='sf-token-total-cell'>
              <div className='sf-token-total-label'>缓存创建</div>
              <div className='sf-token-total-value' style={{ color: '#FF9500' }}>
                {fmtNum(total.total_cache_creation)}
              </div>
            </div>
            <div className='sf-token-total-cell'>
              <div className='sf-token-total-label'>缓存读取</div>
              <div className='sf-token-total-value' style={{ color: '#5856D6' }}>
                {fmtNum(total.total_cache_read)}
              </div>
            </div>
            <div className='sf-token-total-cell'>
              <div className='sf-token-total-label'>总费用</div>
              <div className='sf-token-total-value' style={{ color: '#FF2D55' }}>
                {fmtCost(total.total_cost)}
              </div>
            </div>
          </div>
        )}
      </Card>

      {/* 时间线柱状图 */}
      <Card className='sf-token-card' bordered={false}>
        <div className='sf-token-card-header'>
          <h3>用量趋势</h3>
          <span className='sf-token-card-count'>{timeline.length} 个时间点</span>
        </div>
        <TimelineChart data={timeline} />
      </Card>

      {/* 按端点 + 按操作类型 */}
      <div className='sf-token-summary-grid'>
        <SummaryTable title='按端点分组' rows={byEndpoint} />
        <SummaryTable title='按操作类型分组' rows={byOp} labelMap={OPERATION_LABELS} />
      </div>

      {/* 会话列表 */}
      <SessionList sessions={sessions} />

      {/* 用量记录 */}
      <RecordsTable records={records} />
    </div>
  );
};

export default TokenStatsPanel;
