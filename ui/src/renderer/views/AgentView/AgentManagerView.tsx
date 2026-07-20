/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentManagerView — Agent 管理主页面
 *
 * 来源：OpenAkita AgentManagerView.tsx（清洁室重写为 4 子组件 + 主页面）
 *
 * 组合：
 * - AgentListPanel（左列表）
 * - AgentEditorSheet（右编辑抽屉，按需弹出）
 * - AgentCategoryBar（顶部分类栏，已内嵌于 AgentListPanel）
 * - AgentIconPicker（图标选择器，已内嵌于 AgentEditorSheet）
 *
 * 改造点：
 * - OpenAkita 单文件 1700+ 行 → 拆分为 4 个独立组件 + 主页面
 * - useState 散落状态 → useAgentStore 集中管理
 * - safeFetch → store actions
 * - Apple 风格：圆角 8px + 系统蓝 + SF Pro
 */

import React from 'react';
import { Button, Message } from '@arco-design/web-react';
import { useAgentStore } from '@renderer/store/agentStore';
import { AgentListPanel } from '@renderer/components/agent/AgentListPanel';
import { AgentEditorSheet } from '@renderer/components/agent/AgentEditorSheet';

export const AgentManagerView: React.FC = () => {
  const initialize = useAgentStore((s) => s.initialize);
  const fetchManagerState = useAgentStore((s) => s.fetchManagerState);
  const loading = useAgentStore((s) => s.loading);
  const agents = useAgentStore((s) => s.agents);
  const openEditor = useAgentStore((s) => s.openEditor);

  // 初始化（加载 mock 数据）
  React.useEffect(() => {
    initialize();
    fetchManagerState();
  }, [initialize, fetchManagerState]);

  const handleCreate = () => {
    openEditor(undefined);
  };

  const handleRefresh = async () => {
    try {
      await fetchManagerState();
      Message.success('已刷新');
    } catch {
      Message.error('刷新失败');
    }
  };

  return (
    <div className='sf-agent-manager-view'>
      {/* 顶部标题栏 */}
      <header className='sf-agent-manager-header'>
        <div className='sf-agent-manager-title'>
          <h1>Agent 管理</h1>
          <span className='sf-agent-manager-count'>{agents.length} 个 Agent</span>
        </div>
        <div className='sf-agent-manager-actions'>
          <Button
            type='secondary'
            size='small'
            onClick={handleRefresh}
            loading={loading}
          >
            ↻ 刷新
          </Button>
          <Button
            type='primary'
            size='small'
            onClick={handleCreate}
          >
            + 新建 Agent
          </Button>
        </div>
      </header>

      {/* 列表面板 */}
      <AgentListPanel />

      {/* 编辑器抽屉（受 store 控制） */}
      <AgentEditorSheet />
    </div>
  );
};

export default AgentManagerView;
