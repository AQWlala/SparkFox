/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentEditorSheet — Agent 编辑器抽屉
 *
 * 来源：OpenAkita AgentManagerView.tsx 右侧编辑面板（清洁室重写为独立组件）
 *
 * 保留 OpenAkita 特性：
 * - 22 字段 AgentProfile 完整编辑表单
 * - 图标选择器（emoji + SVG + 自定义 URL）
 * - 技能 / 工具 / MCP 三栏白名单编辑
 * - 身份文件标签页（SOUL.md / USER.md / AGENT.md）
 * - 记忆统计展示
 * - 导入 / 导出 / 删除操作
 *
 * 改造点：
 * - shadcn Sheet/Select/Badge → Arco Drawer/Tabs/Tag
 * - useTranslation → 硬编码中文
 * - safeFetch → useAgentStore actions
 * - Apple 风格：圆角 8px + 系统蓝 + SF Pro
 */

import React from 'react';
import { Drawer, Tabs, Input, Select, Tag, Button, Switch, Message } from '@arco-design/web-react';
import { useAgentStore, type AgentProfile } from '@renderer/store/agentStore';
import { AgentIcon } from './AgentIcon';
import { AgentIconPicker } from './AgentIconPicker';

const { TextArea } = Input;
const { TabPane } = Tabs;
const { Option } = Select;

interface AgentEditorSheetProps {
  /** 受控显示（与 store.editorOpen 同步） */
  visible?: boolean;
  /** 关闭回调 */
  onClose?: () => void;
}

export const AgentEditorSheet: React.FC<AgentEditorSheetProps> = ({ visible, onClose }) => {
  const storeVisible = useAgentStore((s) => s.editorOpen);
  const editingProfile = useAgentStore((s) => s.editingProfile);
  const isCreating = useAgentStore((s) => s.isCreating);
  const updateEditingField = useAgentStore((s) => s.updateEditingField);
  const closeEditor = useAgentStore((s) => s.closeEditor);
  const saveEditing = useAgentStore((s) => s.saveEditing);
  const deleteAgent = useAgentStore((s) => s.deleteAgent);

  const availableSkills = useAgentStore((s) => s.availableSkills);
  const availableToolCategories = useAgentStore((s) => s.availableToolCategories);
  const availableMcpServers = useAgentStore((s) => s.availableMcpServers);
  const availableModels = useAgentStore((s) => s.availableModels);
  const categories = useAgentStore((s) => s.categories);
  const fetchIdentityFile = useAgentStore((s) => s.fetchIdentityFile);
  const saveIdentityFile = useAgentStore((s) => s.saveIdentityFile);
  const fetchMemoryStats = useAgentStore((s) => s.fetchMemoryStats);

  const isOpen = visible ?? storeVisible;
  const handleClose = () => {
    closeEditor();
    onClose?.();
  };

  // ─── 图标选择器状态 ───
  const [iconPickerOpen, setIconPickerOpen] = React.useState(false);

  // ─── 身份文件标签页状态 ───
  const [identityTab, setIdentityTab] = React.useState('SOUL.md');
  const [identityContent, setIdentityContent] = React.useState('');
  const [identitySource, setIdentitySource] = React.useState<'global' | 'profile'>('global');
  const [identityLoading, setIdentityLoading] = React.useState(false);

  // ─── 记忆统计 ───
  const [memoryStats, setMemoryStats] = React.useState<{ exists: boolean; semantic_count: number; db_size_bytes: number } | null>(null);

  // ─── 保存中 ───
  const [saving, setSaving] = React.useState(false);

  // 加载身份文件
  const loadIdentity = React.useCallback(
    async (profileId: string, filename: string) => {
      if (!profileId) return;
      setIdentityLoading(true);
      try {
        const file = await fetchIdentityFile(profileId, filename);
        setIdentityContent(file.content);
        setIdentitySource(file.source);
      } catch {
        setIdentityContent('');
        setIdentitySource('global');
      }
      setIdentityLoading(false);
    },
    [fetchIdentityFile]
  );

  // 加载记忆统计
  const loadMemoryStats = React.useCallback(
    async (profileId: string) => {
      if (!profileId) return;
      try {
        const stats = await fetchMemoryStats(profileId);
        setMemoryStats(stats);
      } catch {
        setMemoryStats(null);
      }
    },
    [fetchMemoryStats]
  );

  // 打开时加载身份文件 + 记忆统计
  React.useEffect(() => {
    if (isOpen && editingProfile.id && !isCreating) {
      loadIdentity(editingProfile.id, identityTab);
      loadMemoryStats(editingProfile.id);
    }
  }, [isOpen, editingProfile.id, isCreating, identityTab, loadIdentity, loadMemoryStats]);

  // ─── 保存 ───
  const handleSave = async () => {
    setSaving(true);
    try {
      await saveEditing();
      // 若身份文件有修改，一并保存
      if (!isCreating && identityContent) {
        await saveIdentityFile(editingProfile.id, identityTab, identityContent);
      }
      Message.success(isCreating ? 'Agent 创建成功' : 'Agent 已保存');
    } catch (err) {
      Message.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  // ─── 删除 ───
  const handleDelete = async () => {
    if (!editingProfile.id) return;
    if (!window.confirm(`确定要删除 Agent "${editingProfile.name}" 吗？此操作不可撤销。`)) return;
    try {
      await deleteAgent(editingProfile.id);
      Message.success('Agent 已删除');
    } catch (err) {
      Message.error(err instanceof Error ? err.message : '删除失败');
    }
  };

  // ─── 工具/技能/MCP 选择处理 ───
  const handleSkillsChange = (values: (string | number | undefined)[]) => {
    updateEditingField('skills', values.filter((v): v is string => typeof v === 'string'));
  };
  const handleToolsChange = (values: (string | number | undefined)[]) => {
    updateEditingField('tools', values.filter((v): v is string => typeof v === 'string'));
  };
  const handleMcpChange = (values: (string | number | undefined)[]) => {
    updateEditingField('mcp_servers', values.filter((v): v is string => typeof v === 'string'));
  };

  // 所有可用工具的扁平列表（用于 Select options）
  const allTools = React.useMemo(() => {
    return availableToolCategories.flatMap((cat) => cat.tools);
  }, [availableToolCategories]);

  return (
    <Drawer
      width={520}
      title={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <AgentIcon icon={editingProfile.icon} color={editingProfile.color} size={20} />
          <span>{isCreating ? '新建 Agent' : `编辑: ${editingProfile.name}`}</span>
        </div>
      }
      visible={isOpen}
      onCancel={handleClose}
      footer={
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
          <div>
            {!isCreating && (
              <Button status='danger' onClick={handleDelete}>
                删除
              </Button>
            )}
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button onClick={handleClose}>取消</Button>
            <Button type='primary' loading={saving} onClick={handleSave}>
              {isCreating ? '创建' : '保存'}
            </Button>
          </div>
        </div>
      }
    >
      <Tabs>
        {/* ─── 基础信息 ─── */}
        <TabPane key='basic' title='基础信息'>
          <div className='sf-agent-editor-section'>
            {/* 图标 + 颜色 */}
            <div className='sf-agent-editor-row'>
              <label className='sf-agent-editor-label'>图标</label>
              <div className='sf-agent-editor-icon-row'>
                <button
                  type='button'
                  className='sf-agent-editor-icon-btn'
                  onClick={() => setIconPickerOpen(true)}
                  style={{ background: `${editingProfile.color}1A` }}
                >
                  <AgentIcon icon={editingProfile.icon} color={editingProfile.color} size={32} />
                </button>
                <input
                  type='color'
                  value={editingProfile.color}
                  onChange={(e) => updateEditingField('color', e.target.value)}
                  className='sf-agent-editor-color'
                  title='主题色'
                />
                <input
                  type='text'
                  value={editingProfile.icon}
                  onChange={(e) => updateEditingField('icon', e.target.value)}
                  placeholder='emoji 或 svg:key 或 URL'
                  className='sf-agent-editor-icon-input'
                />
              </div>
            </div>

            {/* 名称 */}
            <div className='sf-agent-editor-row'>
              <label className='sf-agent-editor-label'>名称 *</label>
              <Input
                value={editingProfile.name}
                onChange={(v) => updateEditingField('name', v)}
                placeholder='Agent 名称'
              />
            </div>

            {/* 描述 */}
            <div className='sf-agent-editor-row'>
              <label className='sf-agent-editor-label'>描述</label>
              <TextArea
                value={editingProfile.description}
                onChange={(v) => updateEditingField('description', v)}
                placeholder='Agent 描述'
                autoSize={{ minRows: 2, maxRows: 4 }}
              />
            </div>

            {/* 分类 */}
            <div className='sf-agent-editor-row'>
              <label className='sf-agent-editor-label'>分类</label>
              <Select
                value={editingProfile.category || ''}
                onChange={(v) => updateEditingField('category', v)}
                placeholder='选择分类'
                allowClear
              >
                {categories.map((cat) => (
                  <Option key={cat.id} value={cat.id}>
                    <span style={{ color: cat.color }}>●</span> {cat.label}
                  </Option>
                ))}
              </Select>
            </div>

            {/* 自定义提示词 */}
            <div className='sf-agent-editor-row'>
              <label className='sf-agent-editor-label'>自定义提示词</label>
              <TextArea
                value={editingProfile.custom_prompt}
                onChange={(v) => updateEditingField('custom_prompt', v)}
                placeholder='覆盖默认 SOUL.md 的系统提示词...'
                autoSize={{ minRows: 4, maxRows: 10 }}
              />
            </div>

            {/* 隐藏 / 用户自定义 */}
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>隐藏此 Agent</label>
              <Switch
                checked={!!editingProfile.hidden}
                onChange={(v) => updateEditingField('hidden', v)}
              />
            </div>
          </div>
        </TabPane>

        {/* ─── 能力配置（技能/工具/MCP） ─── */}
        <TabPane key='capabilities' title='能力配置'>
          {/* 技能 */}
          <div className='sf-agent-editor-section'>
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>技能模式</label>
              <Select
                value={editingProfile.skills_mode}
                onChange={(v) => updateEditingField('skills_mode', v)}
                style={{ width: 160 }}
              >
                <Option value='all'>全部技能</Option>
                <Option value='whitelist'>白名单</Option>
              </Select>
            </div>
            {editingProfile.skills_mode === 'whitelist' && (
              <div className='sf-agent-editor-row'>
                <label className='sf-agent-editor-label'>已启用技能</label>
                <Select
                  mode='multiple'
                  value={editingProfile.skills}
                  onChange={handleSkillsChange}
                  placeholder='选择技能'
                >
                  {availableSkills.map((s) => (
                    <Option key={s.skillId} value={s.skillId}>
                      {s.name}
                    </Option>
                  ))}
                </Select>
              </div>
            )}

            {/* 工具 */}
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>工具模式</label>
              <Select
                value={editingProfile.tools_mode}
                onChange={(v) => updateEditingField('tools_mode', v)}
                style={{ width: 160 }}
              >
                <Option value='all'>全部工具</Option>
                <Option value='whitelist'>白名单</Option>
              </Select>
            </div>
            {editingProfile.tools_mode === 'whitelist' && (
              <div className='sf-agent-editor-row'>
                <label className='sf-agent-editor-label'>已启用工具</label>
                <Select
                  mode='multiple'
                  value={editingProfile.tools}
                  onChange={handleToolsChange}
                  placeholder='选择工具'
                >
                  {allTools.map((t) => (
                    <Option key={t} value={t}>
                      {t}
                    </Option>
                  ))}
                </Select>
              </div>
            )}

            {/* MCP 服务 */}
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>MCP 模式</label>
              <Select
                value={editingProfile.mcp_mode}
                onChange={(v) => updateEditingField('mcp_mode', v)}
                style={{ width: 160 }}
              >
                <Option value='all'>全部 MCP</Option>
                <Option value='whitelist'>白名单</Option>
              </Select>
            </div>
            {editingProfile.mcp_mode === 'whitelist' && (
              <div className='sf-agent-editor-row'>
                <label className='sf-agent-editor-label'>已启用 MCP 服务</label>
                <Select
                  mode='multiple'
                  value={editingProfile.mcp_servers}
                  onChange={handleMcpChange}
                  placeholder='选择 MCP 服务'
                >
                  {availableMcpServers.map((m) => (
                    <Option key={m.name} value={m.name}>
                      {m.name} {m.connected ? '✓' : '○'} ({m.tool_count}/{m.catalog_tool_count})
                    </Option>
                  ))}
                </Select>
              </div>
            )}

            {/* 偏好模型端点 */}
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>偏好模型</label>
              <Select
                value={editingProfile.preferred_endpoint || ''}
                onChange={(v) => updateEditingField('preferred_endpoint', v || null)}
                placeholder='无偏好'
                allowClear
                style={{ flex: 1 }}
              >
                {availableModels.map((m) => (
                  <Option key={m.name} value={m.name}>
                    {m.name} ({m.provider})
                  </Option>
                ))}
              </Select>
            </div>
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>端点策略</label>
              <Select
                value={editingProfile.endpoint_policy || 'prefer'}
                onChange={(v) => updateEditingField('endpoint_policy', v as 'prefer' | 'require')}
                style={{ width: 160 }}
              >
                <Option value='prefer'>优先（prefer）</Option>
                <Option value='require'>强制（require）</Option>
              </Select>
            </div>
          </div>
        </TabPane>

        {/* ─── 隔离身份 / 记忆 ─── */}
        <TabPane key='isolation' title='身份与记忆'>
          <div className='sf-agent-editor-section'>
            {/* 身份模式 */}
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>身份模式</label>
              <Select
                value={editingProfile.identity_mode || 'shared'}
                onChange={(v) => updateEditingField('identity_mode', v)}
                style={{ width: 200 }}
              >
                <Option value='shared'>共享（继承全局 SOUL.md）</Option>
                <Option value='isolated'>隔离（独立 SOUL.md）</Option>
              </Select>
            </div>

            {/* 记忆模式 */}
            <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
              <label className='sf-agent-editor-label'>记忆模式</label>
              <Select
                value={editingProfile.memory_mode || 'shared'}
                onChange={(v) => updateEditingField('memory_mode', v)}
                style={{ width: 200 }}
              >
                <Option value='shared'>共享（全局记忆）</Option>
                <Option value='isolated'>隔离（独立记忆库）</Option>
              </Select>
            </div>

            {/* 隔离模式下是否继承全局记忆 */}
            {editingProfile.memory_mode === 'isolated' && (
              <div className='sf-agent-editor-row sf-agent-editor-row-inline'>
                <label className='sf-agent-editor-label'>继承全局记忆</label>
                <Switch
                  checked={!!editingProfile.memory_inherit_global}
                  onChange={(v) => updateEditingField('memory_inherit_global', v)}
                />
              </div>
            )}

            {/* 身份文件编辑器（仅隔离模式或查看时显示） */}
            {editingProfile.identity_mode === 'isolated' && !isCreating && (
              <div className='sf-agent-editor-row'>
                <label className='sf-agent-editor-label'>
                  身份文件
                  <Tag size='small' color={identitySource === 'profile' ? 'green' : 'gray'} style={{ marginLeft: 8 }}>
                    {identitySource === 'profile' ? 'Agent 独有' : '继承全局'}
                  </Tag>
                </label>
                <div className='sf-agent-editor-identity-tabs'>
                  {['SOUL.md', 'USER.md', 'AGENT.md'].map((f) => (
                    <button
                      key={f}
                      type='button'
                      className={`sf-agent-editor-identity-tab${identityTab === f ? ' active' : ''}`}
                      onClick={() => setIdentityTab(f)}
                    >
                      {f}
                    </button>
                  ))}
                </div>
                <TextArea
                  value={identityContent}
                  onChange={setIdentityContent}
                  disabled={identityLoading}
                  autoSize={{ minRows: 8, maxRows: 20 }}
                  style={{ fontFamily: 'SF Mono, Menlo, Monaco, Consolas, monospace', fontSize: 12 }}
                />
              </div>
            )}

            {/* 记忆统计 */}
            {!isCreating && memoryStats && (
              <div className='sf-agent-editor-row'>
                <label className='sf-agent-editor-label'>记忆统计</label>
                <div className='sf-agent-editor-memory-stats'>
                  <div className='sf-agent-editor-memory-stat'>
                    <div className='sf-agent-editor-memory-stat-label'>已建立</div>
                    <div className='sf-agent-editor-memory-stat-value'>
                      {memoryStats.exists ? '✓ 是' : '✗ 否'}
                    </div>
                  </div>
                  <div className='sf-agent-editor-memory-stat'>
                    <div className='sf-agent-editor-memory-stat-label'>语义记忆数</div>
                    <div className='sf-agent-editor-memory-stat-value'>
                      {memoryStats.semantic_count}
                    </div>
                  </div>
                  <div className='sf-agent-editor-memory-stat'>
                    <div className='sf-agent-editor-memory-stat-label'>数据库大小</div>
                    <div className='sf-agent-editor-memory-stat-value'>
                      {(memoryStats.db_size_bytes / 1024).toFixed(1)} KB
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </TabPane>
      </Tabs>

      {/* 图标选择器弹层 */}
      {iconPickerOpen && (
        <div className='sf-agent-icon-picker-overlay' onClick={() => setIconPickerOpen(false)}>
          <div onClick={(e) => e.stopPropagation()}>
            <AgentIconPicker
              value={editingProfile.icon}
              color={editingProfile.color}
              onChange={(icon) => {
                updateEditingField('icon', icon);
                setIconPickerOpen(false);
              }}
              onClose={() => setIconPickerOpen(false)}
            />
          </div>
        </div>
      )}
    </Drawer>
  );
};

export default AgentEditorSheet;
