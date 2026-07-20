/**
 * SparkFox HotspotTicker — 底部跑马灯
 *
 * 来源：BaiLongma src/ui/brain-ui/hotspot.js ticker 模块（清洁室重写为 React）
 * 功能：横向滚动跑马灯，重复一次实现无缝衔接
 * 样式：暗色背景 + 红色 LIVE 标签 + 时间戳
 */

import { useMemo } from 'react';
import { useHotspotStore } from '@renderer/store/hotspotStore';

export default function HotspotTicker() {
  const tickerItems = useHotspotStore((s) => s.tickerItems);

  // 复制一份用于无缝滚动
  const doubled = useMemo(() => [...tickerItems, ...tickerItems], [tickerItems]);

  if (!tickerItems.length) {
    return (
      <div className='hotspot-ticker'>
        <span className='hotspot-ticker-label'>LIVE</span>
        <div className='hotspot-ticker-track'>
          <span className='hotspot-ticker-content'>暂无实时快讯</span>
        </div>
      </div>
    );
  }

  return (
    <div className='hotspot-ticker'>
      <span className='hotspot-ticker-label'>LIVE</span>
      <div className='hotspot-ticker-track'>
        <div className='hotspot-ticker-content'>
          {doubled.map((item, idx) => (
            <span className='hotspot-ticker-item' key={`${item.id}_${idx}`}>
              <span className='hotspot-ticker-time'>{item.time}</span>
              <span>{item.text}</span>
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}
