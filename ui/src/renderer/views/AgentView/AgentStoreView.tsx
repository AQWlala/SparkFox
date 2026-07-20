/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentStoreView — Agent 商店页面
 *
 * 来源：OpenAkita AgentStoreView.tsx（清洁室重写）
 *
 * 功能：
 * - 搜索 / 分类 / 排序 / 分页
 * - Agent 卡片展示（图标 + 名称 + 描述 + 下载量 + 评分 + 标签）
 * - 一键安装（转为本地 AgentProfile）
 *
 * 改造点：
 * - safeFetch → useAgentStore.searchStoreAgents / installFromStore
 * - toast (sonner) → Arco Message
 * - Apple 风格：圆角 8px + 系统蓝 + SF Pro
 */

import React from 'react';
import { Input, Select, Button, Message, Spin } from '@arco-design/web-react';
import { useAgentStore, type StoreAgent } from '@renderer/store/agentStore';
import { AgentIcon } from '@renderer/components/agent/AgentIcon';

const { Option } = Select;

// 商店分类
const STORE_CATEGORIES = [
  { id: '', label: '全部' },
  { id: 'research', label: '研究' },
  { id: 'productivity', label: '生产力' },
  { id: 'data', label: '数据' },
];

// 排序选项
const SORT_OPTIONS = [
  { value: 'downloads', label: '下载量' },
  { value: 'rating', label: '评分' },
];

export const AgentStoreView: React.FC = () => {
  const storeAgents = useAgentStore((s) => s.storeAgents);
  const storeLoading = useAgentStore((s) => s.storeLoading);
  const storeTotal = useAgentStore((s) => s.storeTotal);
  const searchStoreAgents = useAgentStore((s) => s.searchStoreAgents);
  const installFromStore = useAgentStore((s) => s.installFromStore);
  const initialize = useAgentStore((s) => s.initialize);

  const [query, setQuery] = React.useState('');
  const [category, setCategory] = React.useState('');
  const [sort, setSort] = React.useState('downloads');
  const [installing, setInstalling] = React.useState<Set<string>>(new Set());

  // 初始化 + 搜索
  React.useEffect(() => {
    initialize();
  }, [initialize]);

  const doSearch = React.useCallback(() => {
    searchStoreAgents({ q: query, category, sort, page: 1 });
  }, [query, category, sort, searchStoreAgents]);

  React.useEffect(() => {
    doSearch();
  }, [doSearch]);

  const handleInstall = async (agent: StoreAgent) => {
    const key = agent.id;
    setInstalling((prev) => {
      const next = new Set(prev);
      next.add(key);
      return next;
    });
    try {
      await installFromStore(key);
      Message.success(`已安装: ${agent.name}`);
    } catch (err) {
      Message.error(err instanceof Error ? err.message : '安装失败');
    } finally {
      setInstalling((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  return (
    <div className='sf-agent-store-view'>
      {/* 顶部标题栏 */}
      <header className='sf-agent-store-header'>
        <div className='sf-agent-store-title'>
          <h1>Agent 商店</h1>
          <span className='sf-agent-store-total'>共 {storeTotal} 个</span>
        </div>
      </header>

      {/* 搜索 / 过滤栏 */}
      <div className='sf-agent-store-toolbar'>
        <Input
          placeholder='搜索 Agent...'
          value={query}
          onChange={setQuery}
          onPressEnter={doSearch}
          allowClear
          style={{ width: 240 }}
        />
        <Select value={category} onChange={setCategory} style={{ width: 140 }}>
          {STORE_CATEGORIES.map((c) => (
            <Option key={c.id} value={c.id}>
              {c.label}
            </Option>
          ))}
        </Select>
        <Select value={sort} onChange={setSort} style={{ width: 120 }}>
          {SORT_OPTIONS.map((s) => (
            <Option key={s.value} value={s.value}>
              {s.label}
            </Option>
          ))}
        </Select>
        <Button type='primary' size='small' onClick={doSearch}>
          搜索
        </Button>
      </div>

      {/* Agent 卡片网格 */}
      <Spin loading={storeLoading} style={{ display: 'block', padding: 40 }}>
        {storeAgents.length === 0 && !storeLoading ? (
          <div className='sf-agent-store-empty'>
            <div className='sf-agent-store-empty-icon'>🔍</div>
            <div>未找到匹配的 Agent</div>
          </div>
        ) : (
          <div className='sf-agent-store-grid'>
            {storeAgents.map((agent) => (
              <div
                key={agent.id}
                className={`sf-agent-store-card${agent.isFeatured ? ' featured' : ''}`}
              >
                {agent.isFeatured && (
                  <span className='sf-agent-store-card-featured'>★ 推荐</span>
                )}
                <div className='sf-agent-store-card-header'>
                  <AgentIcon icon='🤖' color='#007AFF' size={40} />
                  <div className='sf-agent-store-card-info'>
                    <div className='sf-agent-store-card-name'>{agent.name}</div>
                    <div className='sf-agent-store-card-author'>
                      by {agent.authorName || '匿名'}
                    </div>
                  </div>
                </div>
                <div className='sf-agent-store-card-desc'>{agent.description}</div>
                <div className='sf-agent-store-card-tags'>
                  {(agent.tags || []).map((tag) => (
                    <span key={tag} className='sf-agent-store-card-tag'>
                      {tag}
                    </span>
                  ))}
                </div>
                <div className='sf-agent-store-card-stats'>
                  <span title='下载量'>⬇ {agent.downloads.toLocaleString()}</span>
                  <span title='评分'>⭐ {agent.avgRating?.toFixed(1) || '-'}</span>
                  <span title='版本'>v{agent.latestVersion || '?'}</span>
                  <span title='许可证'>{agent.license || '-'}</span>
                </div>
                <Button
                  type='primary'
                  size='small'
                  long
                  loading={installing.has(agent.id)}
                  onClick={() => handleInstall(agent)}
                >
                  {installing.has(agent.id) ? '安装中...' : '一键安装'}
                </Button>
              </div>
            ))}
          </div>
        )}
      </Spin>
    </div>
  );
};

export default AgentStoreView;
