/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * AiActivityBadge — AI 活动状态徽章
 *
 * 来源：BaiLongma src/ui/brain-ui/app.js recordAiActivity/refreshAiActivity（清洁室重写为 React 组件）
 *
 * 保留 BaiLongma 特性：
 * - 60s 滑动窗口 + 15s 空闲阈值
 * - 8 大工具分组（扫描文件 / 改动文件 / 执行命令 / 上网 / 调取记忆 / 推送界面 / 处理多媒体 / 回复用户）
 * - 派生标签："空闲" / "刚完成" / "正在扫描文件" 等
 * - 派生详情："· 3 次工具 · 12s" / "5s 前停止"
 *
 * 改造点：
 * - setInterval 定时刷新 → React useEffect + useState
 * - DOM textContent → React 渲染
 * - BaiLongma CSS 变量颜色 → Apple 主题色
 *
 * 用法：
 *   <AiActivityBadge />  // 通常放在 ChatPanel 顶部
 */

import React, { useEffect, useMemo, useState } from 'react';
import { useThinkingStore, deriveAiActivity } from '@renderer/store/thinkingStore';

const AiActivityBadge: React.FC = () => {
  const log = useThinkingStore((s) => s.aiActivityLog);
  const firstTs = useThinkingStore((s) => s.aiActivityFirstTs);

  // 每秒刷新一次（对应 BaiLongma setInterval(refreshAiActivity, 1000)）
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const activity = useMemo(() => deriveAiActivity(log, firstTs, now), [log, firstTs, now]);

  return (
    <div className={`sf-ai-activity ${activity.state}`} data-state={activity.state}>
      <span className='sf-ai-activity-dot' />
      <span className='sf-ai-activity-label'>{activity.label}</span>
      {activity.detail && <span className='sf-ai-activity-detail'>{activity.detail}</span>}
    </div>
  );
};

AiActivityBadge.displayName = 'AiActivityBadge';

export default AiActivityBadge;
