/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * useAiActivity — AI 活动状态派生 hook
 *
 * 来源：BaiLongma src/ui/brain-ui/app.js recordAiActivity/refreshAiActivity（清洁室重写为 hook）
 *
 * 设计：
 * - 从 thinkingStore 读取 aiActivityLog + aiActivityFirstTs
 * - 每秒刷新一次（对应 BaiLongma setInterval(refreshAiActivity, 1000)）
 * - 通过 deriveAiActivity 派生当前状态（60s 滑动窗口 + 15s 空闲阈值）
 *
 * 用法：
 *   const activity = useAiActivity();
 *   // activity.state: 'idle' | 'busy'
 *   // activity.label: '空闲' | '刚完成' | '正在扫描文件' 等
 *   // activity.detail: '· 3 次工具 · 12s' | '5s 前停止'
 *
 * 注：AiActivityBadge 组件已内联实现此逻辑；本 hook 用于其他需要 AI 活动状态的场景
 * （如 ChatPanel 顶部状态栏、SparkFoxSider 状态指示器等）
 */

import { useEffect, useMemo, useState } from 'react';
import { useThinkingStore, deriveAiActivity, type AiActivityState } from '@renderer/store/thinkingStore';

export function useAiActivity(): AiActivityState {
  const log = useThinkingStore((s) => s.aiActivityLog);
  const firstTs = useThinkingStore((s) => s.aiActivityFirstTs);

  // 每秒刷新一次
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  return useMemo(() => deriveAiActivity(log, firstTs, now), [log, firstTs, now]);
}

export default useAiActivity;
