/**
 * SparkFox HotspotClock — 实时时钟
 *
 * 来源：BaiLongma src/ui/brain-ui/hotspot.js clock 模块（清洁室重写为 React）
 * 功能：HH:mm:ss 实时更新，每秒触发 updateClock
 */

import { useEffect } from 'react';
import { useHotspotStore } from '@renderer/store/hotspotStore';

export default function HotspotClock() {
  const clock = useHotspotStore((s) => s.clock);
  const updateClock = useHotspotStore((s) => s.updateClock);

  useEffect(() => {
    updateClock();
    const timer = setInterval(updateClock, 1000);
    return () => clearInterval(timer);
  }, [updateClock]);

  return (
    <div className='hotspot-clock' aria-label='当前时间'>
      {clock || '--:--:--'}
    </div>
  );
}
