/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox MemoryView — 记忆管理页主视图
 *
 * 来源：OpenAkita MemoryView.tsx（清洁室重写为 SparkFox 6 层架构）
 *
 * 组合：
 * - MemoryStatCards（顶部 6 层统计 + 总览）
 * - 工具栏：列表/图谱切换 + 刷新 + 注入演示
 * - MemoryListPanel（列表模式）或 MemoryGraphView（图谱模式）
 * - MemoryEditorSheet（编辑抽屉，全局浮层）
 *
 * 改造点：
 * - OpenAkita SemanticMemory + Episode + Scratchpad → SparkFox L0-L5 6 层
 * - safeFetch HTTP → useMemoryStore（PoC: mock 数据）
 * - shadcn/ui → Arco Design + Apple 主题
 * - 3D 图谱 → 2D SVG（PoC 不引入 3D 依赖）
 * - 对话注入接口：useMemoryStore.injectFromConversation
 */

import React from 'react';
import { Button, Message } from '@arco-design/web-react';
import { useMemoryStore } from '@renderer/store/memoryStore';
import MemoryStatCards from '@renderer/components/memory/MemoryStatCards';
import MemoryListPanel from '@renderer/components/memory/MemoryListPanel';
import MemoryGraphView from '@renderer/components/memory/MemoryGraphView';
import MemoryEditorSheet from '@renderer/components/memory/MemoryEditorSheet';
import '@renderer/components/memory/memory.css';

const MemoryView: React.FC = () => {
  const initialize = useMemoryStore((s) => s.initialize);
  const refresh = useMemoryStore((s) => s.refresh);
  const loading = useMemoryStore((s) => s.loading);
  const stats = useMemoryStore((s) => s.stats);
  const viewMode = useMemoryStore((s) => s.viewMode);
  const setViewMode = useMemoryStore((s) => s.setViewMode);
  const filterLayer = useMemoryStore((s) => s.filterLayer);
  const setFilterLayer = useMemoryStore((s) => s.setFilterLayer);
  const injectFromConversation = useMemoryStore((s) => s.injectFromConversation);

  React.useEffect(() => {
    initialize();
  }, [initialize]);

  const handleRefresh = async () => {
    await refresh();
    Message.success('已刷新');
  };

  /** PoC：模拟对话注入一条新记忆 */
  const handleDemoInject = async () => {
    await injectFromConversation(
      `[演示注入 ${new Date().toLocaleTimeString()}] 用户在监视面板查看 Token 用量`,
      'context',
      'L1',
    );
    Message.success('已通过对话注入接口写入 L1 短期记忆');
  };

  return (
    <div className='sf-memory-view'>
      {/* 顶部标题栏 */}
      <header className='sf-memory-header'>
        <div className='sf-memory-title'>
          <h1>记忆管理</h1>
          <p>SparkFox 6 层记忆体系（L0 工作记忆 / L1 短期 / L2 情节 / L3 语义 / L4 程序 / L5 元认知）</p>
        </div>
        <div className='sf-memory-actions'>
          <Button
            type='secondary'
            size='small'
            onClick={handleDemoInject}
            title='调用对话注入接口（PoC 演示）'
          >
            + 注入演示
          </Button>
          <Button
            type='secondary'
            size='small'
            onClick={handleRefresh}
            loading={loading}
          >
            ↻ 刷新
          </Button>
          <div className='sf-memory-view-mode-toggle'>
            <Button
              type={viewMode === 'list' ? 'primary' : 'secondary'}
              size='small'
              onClick={() => setViewMode('list')}
            >
              ☰ 列表
            </Button>
            <Button
              type={viewMode === 'graph' ? 'primary' : 'secondary'}
              size='small'
              onClick={() => setViewMode('graph')}
            >
              ◉ 图谱
            </Button>
          </div>
        </div>
      </header>

      {/* 6 层统计卡片 */}
      <MemoryStatCards
        stats={stats}
        activeLayer={filterLayer}
        onSelectLayer={setFilterLayer}
      />

      {/* 列表 / 图谱 */}
      {viewMode === 'list' ? <MemoryListPanel /> : <MemoryGraphView />}

      {/* 编辑抽屉（全局浮层） */}
      <MemoryEditorSheet />
    </div>
  );
};

export default MemoryView;
