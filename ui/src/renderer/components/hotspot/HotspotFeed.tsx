/**
 * SparkFox HotspotFeed — 实时事件流卡片
 *
 * 来源：BaiLongma src/ui/brain-ui/hotspot.js feed 模块（清洁室重写为 React）
 * 功能：单卡片展示，自动/手动切换，时间/类别/标题/描述/地点
 * 自动播放：每 5 秒切换至下一条（feedAuto=true 时）
 */

import { useEffect } from 'react';
import { Button, Empty } from '@arco-design/web-react';
import { IconLeft, IconRight, IconPause, IconCaretRight } from '@arco-design/web-react/icon';
import { useHotspotStore } from '@renderer/store/hotspotStore';

export default function HotspotFeed() {
  const feedItems = useHotspotStore((s) => s.feedItems);
  const feedIndex = useHotspotStore((s) => s.feedIndex);
  const feedAuto = useHotspotStore((s) => s.feedAuto);
  const nextFeed = useHotspotStore((s) => s.nextFeed);
  const prevFeed = useHotspotStore((s) => s.prevFeed);
  const setFeedAuto = useHotspotStore((s) => s.setFeedAuto);

  // 自动播放：5 秒切换
  useEffect(() => {
    if (!feedAuto || feedItems.length === 0) return;
    const timer = setInterval(() => {
      nextFeed();
    }, 5000);
    return () => clearInterval(timer);
  }, [feedAuto, feedItems.length, nextFeed]);

  const current = feedItems[feedIndex];

  return (
    <div className='hotspot-feed'>
      <div className='hotspot-feed-header'>
        <div className='hotspot-feed-title'>实时事件流</div>
        <div className='hotspot-feed-controls'>
          <Button
            size='mini'
            type='text'
            icon={<IconPause />}
            onClick={() => setFeedAuto(false)}
            disabled={!feedAuto}
            aria-label='暂停自动播放'
          />
          <Button
            size='mini'
            type='text'
            icon={<IconCaretRight />}
            onClick={() => setFeedAuto(true)}
            disabled={feedAuto}
            aria-label='开启自动播放'
          />
        </div>
      </div>

      <div className='hotspot-feed-body'>
        {!current ? (
          <Empty description='暂无实时事件' />
        ) : (
          <div className='hotspot-feed-card' key={current.id} style={{ borderLeftColor: current.catColor }}>
            <div className='hotspot-feed-card-head'>
              <span className='hotspot-feed-time'>{current.time}</span>
              <span
                className='hotspot-feed-cat'
                style={{ background: current.catColor }}
              >
                {current.cat}
              </span>
              <span className='hotspot-feed-loc'>📍 {current.loc}</span>
            </div>
            <div className='hotspot-feed-card-title'>{current.title}</div>
            <div className='hotspot-feed-card-desc'>{current.desc}</div>
          </div>
        )}
      </div>

      <div className='hotspot-feed-pager'>
        <Button
          size='mini'
          type='text'
          icon={<IconLeft />}
          onClick={prevFeed}
          disabled={feedItems.length === 0}
          aria-label='上一条'
        />
        <span className='hotspot-feed-pager-text'>
          {feedItems.length > 0 ? `${feedIndex + 1} / ${feedItems.length}` : '0 / 0'}
        </span>
        <Button
          size='mini'
          type='text'
          icon={<IconRight />}
          onClick={nextFeed}
          disabled={feedItems.length === 0}
          aria-label='下一条'
        />
      </div>
    </div>
  );
}
