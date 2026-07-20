/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentView — Agent 路由分发器
 *
 * 来源：SparkFox 全新设计
 * 功能：根据子路由分发到 4 个 Agent 子页面
 *
 * 子路由：
 * - /agents          → AgentManagerView（默认）
 * - /agents/store    → AgentStoreView
 * - /agents/dashboard → AgentDashboardView
 * - /agents/system   → AgentSystemView（全局偏好 / 调试 / 性能调优）
 */

import React, { Suspense, lazy } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';

const AgentManagerView = lazy(() => import('./AgentManagerView'));
const AgentStoreView = lazy(() => import('./AgentStoreView'));
const AgentDashboardView = lazy(() => import('./AgentDashboardView'));
const AgentSystemView = lazy(() => import('./AgentSystemView'));

const AgentLoader: React.FC = () => (
  <div
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      color: '#AEAEB2',
      fontFamily: '-apple-system, sans-serif',
    }}
  >
    加载中...
  </div>
);

const AgentView: React.FC = () => {
  return (
    <Suspense fallback={<AgentLoader />}>
      <Routes>
        <Route path='/' element={<AgentManagerView />} />
        <Route path='/store' element={<AgentStoreView />} />
        <Route path='/dashboard' element={<AgentDashboardView />} />
        <Route path='/system' element={<AgentSystemView />} />
        <Route path='*' element={<Navigate to='/' replace />} />
      </Routes>
    </Suspense>
  );
};

export default AgentView;
