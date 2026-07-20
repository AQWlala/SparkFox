/**
 * SparkFox 路由配置 — 路由定义
 *
 * 来源：SparkFox 全新设计
 * 功能：6 大路由（对话/Agent/监视/热点/记忆/设置）
 *
 * 已落地（v0.1 落地，v0.2 验证通过）
 */

import React, { lazy } from 'react';

const ChatView = lazy(() => import('@renderer/views/ChatView'));
const AgentView = lazy(() => import('@renderer/views/AgentView'));
const MonitorView = lazy(() => import('@renderer/views/MonitorView'));
const HotspotView = lazy(() => import('@renderer/views/HotspotView'));
const MemoryView = lazy(() => import('@renderer/views/MemoryView'));
const SettingsView = lazy(() => import('@renderer/views/SettingsView'));

export interface SparkFoxRoute {
  path: string;
  label: string;
  icon: string;
  element: React.LazyExoticComponent<React.FC>;
  priority: 'P0' | 'P1' | 'P2';
}

export const sparkfoxRoutes: SparkFoxRoute[] = [
  { path: '/',          label: '对话', icon: 'Message',      element: ChatView,     priority: 'P0' },
  { path: '/agents',    label: 'Agent', icon: 'Robot',       element: AgentView,    priority: 'P1' },
  { path: '/monitor',   label: '监视', icon: 'BarChart',     element: MonitorView,  priority: 'P1' },
  { path: '/settings',  label: '设置', icon: 'Settings',     element: SettingsView, priority: 'P1' },
  { path: '/hotspot',   label: '热点', icon: 'Newspaper',    element: HotspotView,  priority: 'P2' },
  { path: '/memory',    label: '记忆', icon: 'Brain',        element: MemoryView,   priority: 'P2' },
];
