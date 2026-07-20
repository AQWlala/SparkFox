/**
 * SparkFox Router — SparkFox 6 大路由入口
 *
 * 来源：SparkFox 全新设计
 * 功能：SparkFox 6 大路由（对话/Agent/监视/热点/记忆/设置）的独立路由
 *
 * 设计：
 * - 通过 `/sparkfox` 前缀访问，不破坏 NomiFun 原路由
 * - 复用 NomiFun 的 Layout 基座，但用 SparkFoxSider 替换原 Sider
 * - 每个路由 lazy load 对应 View 占位
 * - P0-模块 F：右侧 L2Panel（自主行动机制 · Tick，来自 BaiLongma）
 *
 * 已落地（v0.1 落地，v0.2 验证通过）
 */

import React, { Suspense, lazy, useState } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import SparkFoxSider from './SparkFoxSider';
import L2Panel from '@renderer/components/thinking/L2Panel';
import { sparkfoxRoutes } from '@renderer/router/routes';
import '@renderer/components/thinking/thinking.css';
import '@renderer/components/agent/agent.css';

// L2 面板折叠状态持久化 key
const L2_COLLAPSED_KEY = 'sparkfox-l2-collapsed';

// 占位 Loading 组件
const SparkFoxLoader: React.FC = () => (
  <div
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      color: 'var(--sf-color-text-tertiary, #AEAEB2)',
      fontFamily: 'var(--sf-font-family, -apple-system, sans-serif)',
    }}
  >
    加载中...
  </div>
);

// lazy load 所有 View
const ChatView = lazy(() => import('@renderer/views/ChatView'));
const AgentView = lazy(() => import('@renderer/views/AgentView'));
const MonitorView = lazy(() => import('@renderer/views/MonitorView'));
const HotspotView = lazy(() => import('@renderer/views/HotspotView'));
const MemoryView = lazy(() => import('@renderer/views/MemoryView'));
const SettingsView = lazy(() => import('@renderer/views/SettingsView'));

const viewMap: Record<string, React.LazyExoticComponent<React.FC>> = {
  ChatView,
  AgentView,
  MonitorView,
  HotspotView,
  MemoryView,
  SettingsView,
};

const SparkFoxRouter: React.FC = () => {
  // L2 面板折叠状态（从 localStorage 恢复，对应 BaiLongma panel-collapse.js）
  const [l2Collapsed, setL2Collapsed] = useState<boolean>(() => {
    try {
      return localStorage.getItem(L2_COLLAPSED_KEY) === '1';
    } catch {
      return false;
    }
  });

  const toggleL2 = React.useCallback(() => {
    setL2Collapsed((prev) => {
      const next = !prev;
      try {
        localStorage.setItem(L2_COLLAPSED_KEY, next ? '1' : '0');
      } catch {
        // ignore
      }
      return next;
    });
  }, []);

  // 快捷键：] 切换 L2 面板（对应 BaiLongma panel-collapse.js）
  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable)
      ) {
        return;
      }
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.key === ']') {
        e.preventDefault();
        toggleL2();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [toggleL2]);

  return (
    <div className='sparkfox-layout' style={{ display: 'flex', height: '100%', width: '100%' }}>
      <SparkFoxSider />
      <main
        className='sparkfox-main'
        style={{
          flex: 1,
          height: '100%',
          overflow: 'auto',
          background: 'var(--sf-color-bg-secondary, #F5F5F7)',
        }}
      >
        <Suspense fallback={<SparkFoxLoader />}>
          <Routes>
            <Route path='/' element={<ChatView />} />
            {/* Agent 子路由（/agents/* 由 AgentView 内部 Routes 分发） */}
            <Route path='/agents/*' element={<AgentView />} />
            <Route path='/agents' element={<AgentView />} />
            <Route path='/monitor' element={<MonitorView />} />
            <Route path='/hotspot' element={<HotspotView />} />
            <Route path='/memory' element={<MemoryView />} />
            <Route path='/settings' element={<SettingsView />} />
            <Route path='*' element={<Navigate to='/' replace />} />
          </Routes>
        </Suspense>
      </main>
      {/* L2 右侧自主行动面板（Tick 心跳流）— P0-模块 F 双面板布局 */}
      <L2Panel collapsed={l2Collapsed} onToggle={toggleL2} />
    </div>
  );
};

export default SparkFoxRouter;
