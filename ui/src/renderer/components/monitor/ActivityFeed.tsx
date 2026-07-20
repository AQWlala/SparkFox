/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox ActivityFeed — 实时活动流
 *
 * 来源：OpenAkita OrgMonitorPanel 最近活动 + OrgDashboard 活动流（清洁室重写）
 *
 * 保留 OpenAkita 特性：
 * - 可展开/折叠的活动事件列表
 * - 按事件类型着色（消息/工具/错误等）
 * - 时间戳 + Agent 名称 + 事件标题 + 详情
 * - Token 数 + 持续时间显示
 *
 * 改造点：
 * - safeFetch HTTP + WS → useMonitorStore.addActivity（PoC: 定时生成）
 * - 组织/节点概念 → Agent + 对话
 * - 新增实时模式开关（liveMode：每 3-5s 自动生成事件）
 * - 新增事件类型过滤器
 * - Apple 主题：圆角 + 系统色 + SF Pro
 */

import React from 'react';
import { Button, Switch, Tag } from '@arco-design/web-react';
import {
  useMonitorStore,
  ACTIVITY_TYPE_LABELS,
  ACTIVITY_TYPE_COLORS,
  fmtTime,
  fmtRelative,
  fmtNum,
  type ActivityEvent,
  type ActivityEventType,
} from '@renderer/store/monitorStore';

/** 事件类型过滤器选项 */
const FILTER_TYPES: ActivityEventType[] = [
  'message_out',
  'message_in',
  'tool_call',
  'tool_result',
  'agent_switch',
  'memory_op',
  'error',
  'session_start',
  'session_end',
];

/** 单条活动事件行 */
const ActivityRow: React.FC<{ event: ActivityEvent }> = ({ event }) => {
  const [expanded, setExpanded] = React.useState(false);
  const color = ACTIVITY_TYPE_COLORS[event.type];
  const hasDetail = !!event.detail;

  return (
    <div
      className={`sf-activity-row${expanded ? ' expanded' : ''}`}
      onClick={() => hasDetail && setExpanded(!expanded)}
      style={{ borderLeftColor: color }}
    >
      <div className='sf-activity-row-header'>
        {/* 类型圆点 */}
        <span className='sf-activity-dot' style={{ background: color }} />

        {/* 类型标签 */}
        <Tag size='small' style={{ background: `${color}1A`, color, border: 'none' }}>
          {ACTIVITY_TYPE_LABELS[event.type]}
        </Tag>

        {/* 标题 */}
        <span className='sf-activity-title'>{event.title}</span>

        {/* Token 数 */}
        {event.tokens !== undefined && (
          <span className='sf-activity-tokens' title='Token 消耗'>
            ⚡ {fmtNum(event.tokens)}
          </span>
        )}

        {/* 持续时间 */}
        {event.duration_ms !== undefined && (
          <span className='sf-activity-duration' title='耗时'>
            ⏱ {event.duration_ms}ms
          </span>
        )}

        {/* 状态徽章 */}
        {event.status === 'error' && <span className='sf-activity-status error'>✗</span>}
        {event.status === 'warn' && <span className='sf-activity-status warn'>⚠</span>}

        {/* 时间戳 */}
        <span className='sf-activity-time' title={fmtTime(event.timestamp)}>
          {fmtRelative(event.timestamp)}
        </span>

        {/* 展开箭头 */}
        {hasDetail && (
          <span className={`sf-activity-arrow${expanded ? ' expanded' : ''}`}>▸</span>
        )}
      </div>

      {/* 详情（展开时显示） */}
      {hasDetail && expanded && (
        <div className='sf-activity-detail'>
          <div className='sf-activity-detail-row'>
            <span className='sf-activity-detail-label'>Agent:</span>
            <span>{event.agent_name}</span>
          </div>
          <div className='sf-activity-detail-row'>
            <span className='sf-activity-detail-label'>详情:</span>
            <span>{event.detail}</span>
          </div>
          <div className='sf-activity-detail-row'>
            <span className='sf-activity-detail-label'>时间:</span>
            <span>{fmtTime(event.timestamp)}</span>
          </div>
        </div>
      )}
    </div>
  );
};

export const ActivityFeed: React.FC = () => {
  const activities = useMonitorStore((s) => s.activities);
  const liveMode = useMonitorStore((s) => s.liveMode);
  const toggleLiveMode = useMonitorStore((s) => s.toggleLiveMode);
  const addActivity = useMonitorStore((s) => s.addActivity);
  const clearActivities = useMonitorStore((s) => s.clearActivities);

  // 事件类型过滤器
  const [enabledTypes, setEnabledTypes] = React.useState<Set<ActivityEventType>>(
    new Set(FILTER_TYPES)
  );

  // 实时模式：每 3-5s 生成一条事件
  React.useEffect(() => {
    if (!liveMode) return;
    const interval = setInterval(() => {
      addActivity();
    }, 3000 + Math.random() * 2000);
    return () => clearInterval(interval);
  }, [liveMode, addActivity]);

  // 过滤后的事件
  const filtered = React.useMemo(() => {
    return activities.filter((a) => enabledTypes.has(a.type));
  }, [activities, enabledTypes]);

  const toggleType = (type: ActivityEventType) => {
    setEnabledTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  };

  return (
    <div className='sf-activity-feed'>
      {/* 头部：标题 + 实时模式 + 操作 */}
      <div className='sf-activity-feed-header'>
        <div className='sf-activity-feed-title'>
          <h3>实时活动流</h3>
          <span className='sf-activity-feed-count'>{filtered.length} 条</span>
          {liveMode && (
            <span className='sf-activity-live-badge'>
              <span className='sf-activity-live-dot' /> LIVE
            </span>
          )}
        </div>
        <div className='sf-activity-feed-actions'>
          <div className='sf-activity-live-toggle'>
            <span>实时模式</span>
            <Switch checked={liveMode} onChange={toggleLiveMode} size='small' />
          </div>
          <Button size='small' type='secondary' onClick={() => addActivity()} disabled={liveMode}>
            + 生成事件
          </Button>
          <Button size='small' type='outline' onClick={clearActivities}>
            清空
          </Button>
        </div>
      </div>

      {/* 事件类型过滤器 */}
      <div className='sf-activity-filter-bar'>
        {FILTER_TYPES.map((type) => {
          const enabled = enabledTypes.has(type);
          const color = ACTIVITY_TYPE_COLORS[type];
          return (
            <button
              key={type}
              type='button'
              className={`sf-activity-filter-chip${enabled ? ' active' : ''}`}
              onClick={() => toggleType(type)}
              style={
                enabled
                  ? { background: color, borderColor: color }
                  : { color, borderColor: `${color}55` }
              }
            >
              {ACTIVITY_TYPE_LABELS[type]}
            </button>
          );
        })}
      </div>

      {/* 事件列表 */}
      <div className='sf-activity-list'>
        {filtered.length === 0 ? (
          <div className='sf-activity-empty'>
            <div className='sf-activity-empty-icon'>📭</div>
            <div>暂无活动事件</div>
            <div className='sf-activity-empty-hint'>
              点击"+ 生成事件"或开启实时模式查看活动流
            </div>
          </div>
        ) : (
          filtered.map((event) => <ActivityRow key={event.id} event={event} />)
        )}
      </div>
    </div>
  );
};

export default ActivityFeed;
