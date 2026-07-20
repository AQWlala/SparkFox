/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentSystemView — Agent 系统设置页
 *
 * 来源：SparkFox 全新设计（参考 macOS 系统偏好设置布局）
 *
 * 功能：
 * - 全局 Agent 偏好（默认值 / 显示 / 自动切换）
 * - 调试开关（详细日志 / 思考流调试 / 记忆操作日志 / IPC 追踪）
 * - 性能调优（思考流行数 / 监视事件上限 / 记忆刷新 / 虚拟滚动）
 */

import React from 'react';
import { Button, Message, Switch, InputNumber, Select } from '@arco-design/web-react';
import { useSettingsStore } from '@renderer/store/settingsStore';
import { useAgentStore } from '@renderer/store/agentStore';
import '@renderer/components/agent/agent.css';

const ICON_OPTIONS = ['🦊', '🤖', '🧠', '👾', '🐱', '🐼', '🦉', '🐢', '🚀', '⚡'];
const COLOR_OPTIONS = ['#FF9500', '#007AFF', '#5856D6', '#34C759', '#FF3B30', '#FF2D55', '#AF52DE', '#00C7BE'];
const { Option } = Select;

const AgentSystemView: React.FC = () => {
  const agentPrefs = useSettingsStore((s) => s.agentPrefs);
  const updateAgentPrefs = useSettingsStore((s) => s.updateAgentPrefs);
  const debugPrefs = useSettingsStore((s) => s.debugPrefs);
  const updateDebugPrefs = useSettingsStore((s) => s.updateDebugPrefs);
  const performancePrefs = useSettingsStore((s) => s.performancePrefs);
  const updatePerformancePrefs = useSettingsStore((s) => s.updatePerformancePrefs);

  const agents = useAgentStore((s) => s.agents);
  const categories = useAgentStore((s) => s.categories);

  const handleExportPrefs = () => {
    const data = {
      agentPrefs,
      debugPrefs,
      performancePrefs,
      exportedAt: new Date().toISOString(),
    };
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sparkfox-agent-prefs-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    Message.success('已导出 Agent 系统偏好');
  };

  return (
    <div className='sf-agent-manager-view'>
      {/* 顶部标题栏 */}
      <header className='sf-agent-manager-header'>
        <div className='sf-agent-manager-title'>
          <h1>Agent 系统设置</h1>
          <span className='sf-agent-manager-count'>
            {agents.length} 个 Agent / {categories.length} 个分类
          </span>
        </div>
        <div className='sf-agent-manager-actions'>
          <Button type='secondary' size='small' onClick={handleExportPrefs}>
            ⬇ 导出偏好
          </Button>
        </div>
      </header>

      {/* 全局 Agent 偏好 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>全局 Agent 偏好</h2>
        <p className='sf-agent-system-section-desc'>
          新建 Agent 的默认值与全局行为控制
        </p>

        <div className='sf-agent-system-grid'>
          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>默认类型</label>
            <Select
              value={agentPrefs.defaultAgentType}
              onChange={(v) => updateAgentPrefs({ defaultAgentType: v })}
              style={{ width: 200 }}
              size='small'
            >
              <Option value='custom'>自定义</Option>
              <Option value='builtin'>内置</Option>
              <Option value='remote'>远程</Option>
            </Select>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>默认图标</label>
            <div className='sf-agent-system-icon-picker'>
              {ICON_OPTIONS.map((icon) => (
                <button
                  key={icon}
                  type='button'
                  className={`sf-agent-system-icon-btn ${agentPrefs.defaultAgentIcon === icon ? 'active' : ''}`}
                  onClick={() => updateAgentPrefs({ defaultAgentIcon: icon })}
                >
                  {icon}
                </button>
              ))}
            </div>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>默认颜色</label>
            <div className='sf-agent-system-color-picker'>
              {COLOR_OPTIONS.map((color) => (
                <button
                  key={color}
                  type='button'
                  className={`sf-agent-system-color-btn ${agentPrefs.defaultAgentColor === color ? 'active' : ''}`}
                  style={{ background: color }}
                  onClick={() => updateAgentPrefs({ defaultAgentColor: color })}
                  aria-label={`选择颜色 ${color}`}
                />
              ))}
            </div>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>默认记忆模式</label>
            <Select
              value={agentPrefs.defaultMemoryMode}
              onChange={(v) => updateAgentPrefs({ defaultMemoryMode: v })}
              style={{ width: 200 }}
              size='small'
            >
              <Option value='shared'>共享（全局记忆）</Option>
              <Option value='isolated'>隔离（独立记忆库）</Option>
            </Select>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>默认身份模式</label>
            <Select
              value={agentPrefs.defaultIdentityMode}
              onChange={(v) => updateAgentPrefs({ defaultIdentityMode: v })}
              style={{ width: 200 }}
              size='small'
            >
              <Option value='shared'>共享（全局身份文件）</Option>
              <Option value='isolated'>隔离（独立身份文件）</Option>
            </Select>
          </div>

          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>显示隐藏的 Agent</label>
            <Switch
              checked={agentPrefs.showHiddenAgents}
              onChange={(v) => updateAgentPrefs({ showHiddenAgents: v })}
              size='small'
            />
          </div>

          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>启用 Agent 自动切换</label>
            <Switch
              checked={agentPrefs.enableAutoSwitch}
              onChange={(v) => updateAgentPrefs({ enableAutoSwitch: v })}
              size='small'
            />
            <span className='sf-agent-system-hint'>
              根据对话内容自动选择最匹配的 Agent
            </span>
          </div>
        </div>
      </section>

      {/* 调试 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>调试</h2>
        <p className='sf-agent-system-section-desc'>
          开发与诊断用的调试开关
        </p>

        <div className='sf-agent-system-grid'>
          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>详细日志</label>
            <Switch
              checked={debugPrefs.verboseLogging}
              onChange={(v) => updateDebugPrefs({ verboseLogging: v })}
              size='small'
            />
            <span className='sf-agent-system-hint'>输出所有 store action 调用日志</span>
          </div>

          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>思考流调试视图</label>
            <Switch
              checked={debugPrefs.debugThinkingStream}
              onChange={(v) => updateDebugPrefs({ debugThinkingStream: v })}
              size='small'
            />
            <span className='sf-agent-system-hint'>显示思考流的内部状态字段</span>
          </div>

          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>记忆操作日志</label>
            <Switch
              checked={debugPrefs.logMemoryOps}
              onChange={(v) => updateDebugPrefs({ logMemoryOps: v })}
              size='small'
            />
            <span className='sf-agent-system-hint'>记录所有记忆读写操作</span>
          </div>

          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>IPC 追踪</label>
            <Switch
              checked={debugPrefs.traceIpc}
              onChange={(v) => updateDebugPrefs({ traceIpc: v })}
              size='small'
            />
            <span className='sf-agent-system-hint'>打印 ipcBridge 所有调用</span>
          </div>
        </div>
      </section>

      {/* 性能调优 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>性能调优</h2>
        <p className='sf-agent-system-section-desc'>
          控制内存占用与渲染性能
        </p>

        <div className='sf-agent-system-grid'>
          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>思考流最大行数</label>
            <InputNumber
              value={performancePrefs.thinkingStreamMaxLines}
              onChange={(v) => updatePerformancePrefs({ thinkingStreamMaxLines: Number(v) || 200 })}
              min={50}
              max={1000}
              step={50}
              style={{ width: 120 }}
              size='small'
            />
            <span className='sf-agent-system-hint'>超出后自动裁剪旧行</span>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>监视面板事件上限</label>
            <InputNumber
              value={performancePrefs.monitorEventMaxCount}
              onChange={(v) => updatePerformancePrefs({ monitorEventMaxCount: Number(v) || 200 })}
              min={50}
              max={1000}
              step={50}
              style={{ width: 120 }}
              size='small'
            />
            <span className='sf-agent-system-hint'>超出后丢弃最旧事件</span>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>记忆面板自动刷新</label>
            <InputNumber
              value={performancePrefs.memoryAutoRefreshSec}
              onChange={(v) => updatePerformancePrefs({ memoryAutoRefreshSec: Number(v) || 0 })}
              min={0}
              max={600}
              step={10}
              style={{ width: 120 }}
              size='small'
              suffix='秒'
            />
            <span className='sf-agent-system-hint'>0 = 禁用自动刷新</span>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>虚拟滚动阈值</label>
            <InputNumber
              value={performancePrefs.virtualScrollThreshold}
              onChange={(v) => updatePerformancePrefs({ virtualScrollThreshold: Number(v) || 100 })}
              min={20}
              max={5000}
              step={50}
              style={{ width: 120 }}
              size='small'
            />
            <span className='sf-agent-system-hint'>列表超过此条数时启用虚拟滚动</span>
          </div>
        </div>
      </section>
    </div>
  );
};

export default AgentSystemView;
