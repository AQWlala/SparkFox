/**
 * SparkFox HotspotPlatformLists — 4 平台热榜列表
 *
 * 来源：BaiLongma src/ui/brain-ui/hotspot.js（清洁室重写为 React + Arco Design）
 * 功能：抖音 / 小红书 / 微信 / 微博 4 平台 Tab，每平台 Top 10 热榜
 * 交互：点击条目 → selectHotspot；显示排名/标签/热度/趋势
 * 数据流5：每条热点提供"发送到对话"按钮 → sendToChat(item) → ChatPanel 监听后注入对话
 */

import { useMemo } from 'react';
import { Tabs, Empty, Tooltip } from '@arco-design/web-react';
import { IconMessage } from '@arco-design/web-react/icon';
import {
  PLATFORM_ORDER,
  PLATFORM_LABELS,
  PLATFORM_COLORS,
  TREND_ICONS,
  useHotspotStore,
  type Platform,
  type HotspotItem,
} from '@renderer/store/hotspotStore';

const TabPane = Tabs.TabPane;

/** 标签 → CSS class 映射 */
function tagClass(tag: string): string {
  switch (tag) {
    case '热':
      return 'hot';
    case '荐':
      return 'recommend';
    case '新':
      return 'new';
    case '辟谣':
      return 'refute';
    case '活动':
      return 'activity';
    default:
      return 'hot';
  }
}

/** 单条热点 */
function HotspotRow({
  item,
  selected,
  onClick,
  onSendToChat,
}: {
  item: HotspotItem;
  selected: boolean;
  onClick: () => void;
  onSendToChat: () => void;
}) {
  return (
    <div
      className={`hotspot-item${selected ? ' selected' : ''}`}
      onClick={onClick}
      role='button'
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClick();
        }
      }}
    >
      <div className='hotspot-rank'>{item.rank}</div>
      <div className='hotspot-content'>
        <div className='hotspot-title-line'>
          <span className={`hotspot-tag ${tagClass(item.tag)}`}>{item.tag}</span>
          <span className='hotspot-title-text' title={item.title}>
            {item.title}
          </span>
          {item.isNew && <span className='hotspot-new-badge'>NEW</span>}
        </div>
        <div className='hotspot-meta-line'>
          {item.heat && <span className='hotspot-heat'>{item.heat}</span>}
          <span className={`hotspot-trend ${item.trend}`}>
            {TREND_ICONS[item.trend]} {item.trend === 'up' ? '上升' : item.trend === 'down' ? '下降' : '持平'}
          </span>
          <span className='hotspot-source'>#{item.source}</span>
        </div>
      </div>
      {/* 数据流5：发送到对话按钮 */}
      <Tooltip content='发送到对话页讨论'>
        <button
          type='button'
          className='hotspot-send-chat-btn'
          onClick={(e) => {
            e.stopPropagation();
            onSendToChat();
          }}
          aria-label='发送到对话'
        >
          <IconMessage />
        </button>
      </Tooltip>
    </div>
  );
}

/** 单平台列表 */
function PlatformList({
  platform,
  items,
  selected,
  onSelect,
  onSendToChat,
}: {
  platform: Platform;
  items: HotspotItem[];
  selected: HotspotItem | null;
  onSelect: (item: HotspotItem) => void;
  onSendToChat: (item: HotspotItem) => void;
}) {
  if (!items.length) {
    return (
      <div className='hotspot-empty'>
        <div className='hotspot-empty-icon'>∅</div>
        <div>{PLATFORM_LABELS[platform]}暂无热榜数据</div>
      </div>
    );
  }
  return (
    <>
      {items.map((item) => (
        <HotspotRow
          key={item.id}
          item={item}
          selected={selected?.id === item.id}
          onClick={() => onSelect(item)}
          onSendToChat={() => onSendToChat(item)}
        />
      ))}
    </>
  );
}

export default function HotspotPlatformLists() {
  const hotspotLists = useHotspotStore((s) => s.hotspotLists);
  const selectedHotspot = useHotspotStore((s) => s.selectedHotspot);
  const selectHotspot = useHotspotStore((s) => s.selectHotspot);
  const sendToChat = useHotspotStore((s) => s.sendToChat);
  const meta = useHotspotStore((s) => s.meta);

  // 缓存每平台列表，避免重新渲染
  const lists = useMemo(() => hotspotLists, [hotspotLists]);

  // 状态计数
  const statusCounts = useMemo(() => {
    const counts: Record<Platform, number> = {
      douyin: 0,
      xiaohongshu: 0,
      wechat: 0,
      weibo: 0,
    };
    for (const p of PLATFORM_ORDER) {
      counts[p] = (lists[p] || []).length;
    }
    return counts;
  }, [lists]);

  /** 数据流5：发送到对话 → 设置 pendingChatInjection + 跳转对话页 */
  const handleSendToChat = (item: HotspotItem) => {
    sendToChat(item);
    // 跳转到对话页（HashRouter）
    if (window.location.hash !== '#/sparkfox/') {
      window.location.hash = '#/sparkfox/';
    }
  };

  return (
    <div className='hotspot-platforms'>
      <Tabs
        defaultActiveTab='douyin'
        size='small'
        style={{ height: '100%' }}
        type='line'
      >
        {PLATFORM_ORDER.map((p) => {
          const status = meta.status[p];
          const ok = status?.ok ?? false;
          return (
            <TabPane
              key={p}
              title={
                <span className='hotspot-tab-title'>
                  <span
                    className='hotspot-tab-dot'
                    style={{ background: ok ? PLATFORM_COLORS[p] : '#c7c7cc' }}
                  />
                  {PLATFORM_LABELS[p]}
                  <span className='hotspot-tab-count'>{statusCounts[p]}</span>
                </span>
              }
            >
              <PlatformList
                platform={p}
                items={lists[p] || []}
                selected={selectedHotspot}
                onSelect={selectHotspot}
                onSendToChat={handleSendToChat}
              />
            </TabPane>
          );
        })}
      </Tabs>
    </div>
  );
}
