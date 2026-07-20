/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox agentStore — Agent 菜单系统状态管理
 *
 * 来源：OpenAkita apps/setup-center/src/views/AgentManagerView.tsx 类型定义 + API 调用
 *       （清洁室重写为 Zustand store + mock 数据）
 *
 * 保留 OpenAkita 特性：
 * - AgentProfile 22 字段完整定义（id/name/description/icon/color/type/skills/skills_mode/
 *   tools/tools_mode/mcp_servers/mcp_mode/custom_prompt/preferred_endpoint/endpoint_policy/
 *   category/hidden/user_customized/identity_mode/memory_mode/memory_inherit_global/
 *   name_i18n/description_i18n）
 * - CategoryInfo 分类系统（id/label/color/builtin/agent_count）
 * - SkillItem / ToolCategoryItem / McpServerItem / ModelInfo 子类型
 * - 隔离身份（SOUL.md/USER.md/AGENT.md）+ 隔离记忆统计
 * - 12 个 API 端点对应 actions（PoC 阶段内置 mock 数据）
 *
 * 改造点：
 * - safeFetch HTTP → Zustand store actions（PoC: mock 数据，无后端依赖）
 * - React useState 散落状态 → 集中 store 管理
 * - 新增 installFromStore / searchStoreAgents 商店 actions
 * - 新增 currentAgentId + setCurrentAgent（对话页可切换当前 Agent）
 *
 * PoC mock 数据：5 个示例 Agent + 2 个分类
 */

import { create } from 'zustand';
// Tauri 2 IPC：invoke 调用后端 command，失败时静默回退到 mock 数据
import { invoke } from '@tauri-apps/api/core';

// ─── 类型定义（对应 OpenAkita AgentManagerView.tsx L.19-103） ───

/** Agent 配置（22 字段，完整迁移自 OpenAkita） */
export interface AgentProfile {
  id: string;
  name: string;
  description: string;
  /** 图标：emoji（"🤖"）或 svg:key（"svg:bot"）或自定义 URL（"/agents/x/icon.png"） */
  icon: string;
  /** 主题色（#RRGGBB） */
  color: string;
  /** 类型：builtin / custom */
  type: string;
  /** 已启用技能 ID 列表 */
  skills: string[];
  /** 技能模式：all（全部）/ whitelist（白名单） */
  skills_mode: string;
  /** 已启用工具 ID 列表 */
  tools: string[];
  /** 工具模式：all / whitelist */
  tools_mode: string;
  /** 已启用 MCP 服务列表 */
  mcp_servers: string[];
  /** MCP 模式：all / whitelist */
  mcp_mode: string;
  /** 自定义提示词（覆盖默认 SOUL.md） */
  custom_prompt: string;
  /** 偏好模型端点 ID */
  preferred_endpoint?: string | null;
  /** 端点策略：prefer（优先）/ require（强制） */
  endpoint_policy?: 'prefer' | 'require';
  /** 所属分类 ID */
  category?: string;
  /** 是否隐藏 */
  hidden?: boolean;
  /** 是否用户自定义 */
  user_customized?: boolean;
  /** 身份模式：shared（共享）/ isolated（隔离） */
  identity_mode?: string;
  /** 记忆模式：shared / isolated */
  memory_mode?: string;
  /** 隔离模式下是否继承全局记忆 */
  memory_inherit_global?: boolean;
  /** 多语言名称 */
  name_i18n?: Record<string, string>;
  /** 多语言描述 */
  description_i18n?: Record<string, string>;
}

/** Agent 分类 */
export interface CategoryInfo {
  id: string;
  label: string;
  color: string;
  builtin: boolean;
  agent_count: number;
}

/** 技能项 */
export interface SkillItem {
  skillId: string;
  name: string;
  enabled: boolean;
  name_i18n?: Record<string, string> | null;
}

/** 工具分类项 */
export interface ToolCategoryItem {
  id: string;
  tools: string[];
}

/** MCP 服务项 */
export interface McpServerItem {
  name: string;
  description?: string;
  connected?: boolean;
  enabled?: boolean;
  tool_count?: number;
  catalog_tool_count?: number;
}

/** 模型端点信息 */
export interface ModelInfo {
  name: string;
  provider: string;
  model: string;
  status: string;
  has_api_key: boolean;
}

/** 身份文件内容（SOUL.md / USER.md / AGENT.md） */
export interface IdentityFile {
  filename: string;
  content: string;
  /** 来源：global（继承全局）/ profile（Agent 独有） */
  source: 'global' | 'profile';
}

/** Agent 记忆统计 */
export interface AgentMemoryStats {
  exists: boolean;
  semantic_count: number;
  db_size_bytes: number;
}

/** 商店 Agent（与本地 AgentProfile 不同，简化字段） */
export interface StoreAgent {
  id: string;
  name: string;
  description: string;
  category: string;
  authorName?: string;
  downloads: number;
  avgRating?: number;
  ratingCount?: number;
  latestVersion?: string;
  tags?: string[];
  isFeatured?: boolean;
  license?: string;
}

// ─── 默认值 ───

export const EMPTY_PROFILE: AgentProfile = {
  id: '',
  name: '',
  description: '',
  icon: '🤖',
  color: '#6b7280',
  type: 'custom',
  skills: [],
  skills_mode: 'all',
  tools: [],
  tools_mode: 'all',
  mcp_servers: [],
  mcp_mode: 'all',
  custom_prompt: '',
  preferred_endpoint: null,
  endpoint_policy: 'prefer',
  category: '',
  hidden: false,
  user_customized: false,
  identity_mode: 'shared',
  memory_mode: 'shared',
  memory_inherit_global: true,
};

// ─── Mock 数据（PoC 阶段无后端） ───

const MOCK_CATEGORIES: CategoryInfo[] = [
  { id: 'builtin', label: '内置', color: '#007AFF', builtin: true, agent_count: 3 },
  { id: 'custom', label: '自定义', color: '#34C759', builtin: false, agent_count: 2 },
];

const MOCK_AGENTS: AgentProfile[] = [
  {
    id: 'sparkfox-default',
    name: 'SparkFox 默认助手',
    description: '通用对话助手，擅长代码、文档、问答',
    icon: '🦊',
    color: '#FF9500',
    type: 'builtin',
    skills: [],
    skills_mode: 'all',
    tools: ['read_file', 'write_file', 'list_dir', 'exec_command', 'web_search'],
    tools_mode: 'whitelist',
    mcp_servers: [],
    mcp_mode: 'all',
    custom_prompt: '你是 SparkFox 默认助手，友好、简洁、专业。',
    preferred_endpoint: null,
    endpoint_policy: 'prefer',
    category: 'builtin',
    hidden: false,
    user_customized: false,
    identity_mode: 'shared',
    memory_mode: 'shared',
    memory_inherit_global: true,
  },
  {
    id: 'code-reviewer',
    name: '代码审查员',
    description: '专注于代码审查，识别 bug、性能、安全问题',
    icon: '🔍',
    color: '#007AFF',
    type: 'builtin',
    skills: ['code-review'],
    skills_mode: 'whitelist',
    tools: ['read_file', 'list_dir', 'exec_command'],
    tools_mode: 'whitelist',
    mcp_servers: [],
    mcp_mode: 'all',
    custom_prompt: '你是资深代码审查员，逐行分析代码，给出可执行的改进建议。',
    preferred_endpoint: null,
    endpoint_policy: 'prefer',
    category: 'builtin',
    hidden: false,
    user_customized: false,
    identity_mode: 'shared',
    memory_mode: 'shared',
    memory_inherit_global: true,
  },
  {
    id: 'research-assistant',
    name: '研究助理',
    description: '网络搜索 + 资料整理 + 报告撰写',
    icon: '📚',
    color: '#5856D6',
    type: 'builtin',
    skills: ['baidu-search', 'baidu-scholar', 'webapp-testing'],
    skills_mode: 'whitelist',
    tools: ['web_search', 'fetch_url', 'browser_read', 'write_file'],
    tools_mode: 'whitelist',
    mcp_servers: [],
    mcp_mode: 'all',
    custom_prompt: '你是研究助理，深入调研、交叉验证、结构化输出报告。',
    preferred_endpoint: null,
    endpoint_policy: 'prefer',
    category: 'builtin',
    hidden: false,
    user_customized: false,
    identity_mode: 'shared',
    memory_mode: 'shared',
    memory_inherit_global: true,
  },
  {
    id: 'ppt-maker',
    name: 'PPT 制作师',
    description: '根据大纲生成精美 PPT，支持自定义模板',
    icon: '📊',
    color: '#FF2D55',
    type: 'builtin',
    skills: ['ppt-creator', 'pptx', 'theme-factory'],
    skills_mode: 'whitelist',
    tools: ['write_file', 'read_file'],
    tools_mode: 'whitelist',
    mcp_servers: [],
    mcp_mode: 'all',
    custom_prompt: '你是 PPT 设计师，结构清晰、视觉精炼、内容有力。',
    preferred_endpoint: null,
    endpoint_policy: 'prefer',
    category: 'builtin',
    hidden: false,
    user_customized: false,
    identity_mode: 'shared',
    memory_mode: 'shared',
    memory_inherit_global: true,
  },
  {
    id: 'my-assistant',
    name: '我的助理',
    description: '个人定制助手（用户自定义示例）',
    icon: '🧑‍💼',
    color: '#34C759',
    type: 'custom',
    skills: [],
    skills_mode: 'all',
    tools: [],
    tools_mode: 'all',
    mcp_servers: [],
    mcp_mode: 'all',
    custom_prompt: '你是我的私人助理，帮我管理日程、回复消息。',
    preferred_endpoint: null,
    endpoint_policy: 'prefer',
    category: 'custom',
    hidden: false,
    user_customized: true,
    identity_mode: 'isolated',
    memory_mode: 'isolated',
    memory_inherit_global: true,
  },
];

const MOCK_SKILLS: SkillItem[] = [
  { skillId: 'code-review', name: '代码审查', enabled: true },
  { skillId: 'baidu-search', name: '百度搜索', enabled: true },
  { skillId: 'baidu-scholar', name: '百度学术', enabled: true },
  { skillId: 'ppt-creator', name: 'PPT 创建', enabled: true },
  { skillId: 'pptx', name: 'PPTX 编辑', enabled: true },
  { skillId: 'theme-factory', name: '主题工厂', enabled: true },
  { skillId: 'webapp-testing', name: 'Web 应用测试', enabled: true },
  { skillId: 'docx', name: 'Word 文档', enabled: true },
  { skillId: 'xlsx', name: 'Excel 表格', enabled: true },
  { skillId: 'pdf', name: 'PDF 处理', enabled: true },
];

const MOCK_TOOL_CATEGORIES: ToolCategoryItem[] = [
  { id: 'file', tools: ['read_file', 'write_file', 'delete_file', 'make_dir', 'list_dir'] },
  { id: 'exec', tools: ['exec_command', 'exec_quick_command', 'exec_task_command', 'kill_process', 'list_processes'] },
  { id: 'web', tools: ['web_search', 'fetch_url', 'browser_read', 'download_file'] },
  { id: 'memory', tools: ['search_memory', 'probe_memory', 'upsert_memory', 'merge_memories'] },
  { id: 'media', tools: ['speak', 'generate_image', 'generate_music'] },
];

const MOCK_MCP_SERVERS: McpServerItem[] = [
  { name: 'filesystem', description: '文件系统访问', connected: true, enabled: true, tool_count: 8, catalog_tool_count: 8 },
  { name: 'github', description: 'GitHub 仓库管理', connected: false, enabled: false, tool_count: 0, catalog_tool_count: 12 },
];

const MOCK_MODELS: ModelInfo[] = [
  { name: 'GPT-4o', provider: 'openai', model: 'gpt-4o', status: 'ok', has_api_key: true },
  { name: 'Claude 3.5 Sonnet', provider: 'anthropic', model: 'claude-3-5-sonnet', status: 'ok', has_api_key: true },
  { name: 'GLM-4.6', provider: 'zhipu', model: 'glm-4.6', status: 'ok', has_api_key: true },
];

const MOCK_STORE_AGENTS: StoreAgent[] = [
  {
    id: 'store-deep-researcher',
    name: '深度研究员',
    description: '多轮检索 + 交叉验证 + 结构化报告输出',
    category: 'research',
    authorName: 'OpenAkita Team',
    downloads: 12834,
    avgRating: 4.8,
    ratingCount: 256,
    latestVersion: '1.2.0',
    tags: ['研究', '搜索', '报告'],
    isFeatured: true,
    license: 'MIT',
  },
  {
    id: 'store-meeting-assistant',
    name: '会议助理',
    description: '会议纪要 + 待办提取 + 行动项追踪',
    category: 'productivity',
    authorName: 'Community',
    downloads: 8921,
    avgRating: 4.6,
    ratingCount: 178,
    latestVersion: '0.9.3',
    tags: ['会议', '纪要', '待办'],
    isFeatured: false,
    license: 'Apache-2.0',
  },
  {
    id: 'store-data-analyst',
    name: '数据分析师',
    description: 'Excel/CSV/SQL 数据清洗 + 可视化 + 洞察',
    category: 'data',
    authorName: 'OpenAkita Team',
    downloads: 6543,
    avgRating: 4.7,
    ratingCount: 132,
    latestVersion: '2.0.1',
    tags: ['数据', 'Excel', '可视化'],
    isFeatured: true,
    license: 'MIT',
  },
];

// ─── Store 接口 ───

interface AgentState {
  initialized: boolean;

  // ─── 数据 ───
  agents: AgentProfile[];
  categories: CategoryInfo[];
  currentAgentId: string | null;

  // ─── 可选项 ───
  availableSkills: SkillItem[];
  availableToolCategories: ToolCategoryItem[];
  availableMcpServers: McpServerItem[];
  availableModels: ModelInfo[];

  // ─── 商店 ───
  storeAgents: StoreAgent[];
  storeLoading: boolean;
  storeTotal: number;

  // ─── UI 状态 ───
  loading: boolean;
  editorOpen: boolean;
  editingProfile: AgentProfile;
  isCreating: boolean;
  showHidden: boolean;
  activeCategory: string; // '' = 全部
  batchSelected: Set<string>;

  // ─── Actions ───
  initialize: () => void;
  /**
   * 从后端拉取 Agent 列表（Tauri IPC: agent_list）
   * 失败时静默回退到现有 mock 数据，保证 PoC 环境可用
   */
  syncFromBackend: () => Promise<void>;
  fetchManagerState: () => Promise<void>;
  setCurrentAgent: (id: string) => void;
  setActiveCategory: (category: string) => void;
  setShowHidden: (show: boolean) => void;
  setBatchSelected: (ids: Set<string>) => void;

  // 编辑器
  openEditor: (profile?: AgentProfile) => void;
  closeEditor: () => void;
  updateEditingField: <K extends keyof AgentProfile>(key: K, value: AgentProfile[K]) => void;
  saveEditing: () => Promise<void>;
  deleteAgent: (id: string) => Promise<void>;

  // 隔离身份 / 记忆
  fetchIdentityFile: (profileId: string, filename: string) => Promise<IdentityFile>;
  saveIdentityFile: (profileId: string, filename: string, content: string) => Promise<void>;
  fetchMemoryStats: (profileId: string) => Promise<AgentMemoryStats>;

  // 分类管理
  addCategory: (label: string, color: string) => Promise<void>;

  // 商店
  searchStoreAgents: (params: { q?: string; category?: string; sort?: string; page?: number }) => Promise<void>;
  installFromStore: (agentId: string) => Promise<void>;
}

// ─── 工具：生成新 ID ───
function newAgentId(): string {
  return `agent-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
}

// ─── Store 实现 ───

export const useAgentStore = create<AgentState>((set, get) => ({
  initialized: false,

  agents: [],
  categories: [],
  currentAgentId: null,

  availableSkills: [],
  availableToolCategories: [],
  availableMcpServers: [],
  availableModels: [],

  storeAgents: [],
  storeLoading: false,
  storeTotal: 0,

  loading: false,
  editorOpen: false,
  editingProfile: EMPTY_PROFILE,
  isCreating: false,
  showHidden: false,
  activeCategory: '',
  batchSelected: new Set(),

  initialize: () => {
    if (get().initialized) return;
    // 1. 先用 mock 数据填充，保证 UI 立即可用（不阻塞）
    set({
      agents: MOCK_AGENTS,
      categories: MOCK_CATEGORIES,
      currentAgentId: MOCK_AGENTS[0]?.id || null,
      availableSkills: MOCK_SKILLS,
      availableToolCategories: MOCK_TOOL_CATEGORIES,
      availableMcpServers: MOCK_MCP_SERVERS,
      availableModels: MOCK_MODELS,
      storeAgents: MOCK_STORE_AGENTS,
      storeTotal: MOCK_STORE_AGENTS.length,
      initialized: true,
    });
    // 2. 渐进增强：异步从后端拉取覆盖 mock（失败则保留 mock）
    get().syncFromBackend();
  },

  syncFromBackend: async () => {
    try {
      const agents = await invoke<AgentProfile[]>('agent_list');
      if (Array.isArray(agents) && agents.length > 0) {
        // 后端有数据 → 覆盖 mock；保持当前 currentAgentId 不变（如存在）
        const currentId = get().currentAgentId;
        set({
          agents,
          currentAgentId:
            (currentId && agents.some((a) => a.id === currentId) ? currentId : agents[0]?.id) || null,
        });
      }
      // 后端返回空数组时，保留现有 mock 数据
    } catch (e) {
      // 非 Tauri 环境或 command 未注册，静默回退到 mock
      console.warn('[agentStore] syncFromBackend failed, using mock:', e);
    }
  },

  fetchManagerState: async () => {
    set({ loading: true });
    // PoC 阶段：mock 数据直接返回；生产环境对接 ipcBridge.agent.managerState
    await new Promise((resolve) => setTimeout(resolve, 200));
    set((state) => ({
      agents: state.agents.length > 0 ? state.agents : MOCK_AGENTS,
      categories: state.categories.length > 0 ? state.categories : MOCK_CATEGORIES,
      loading: false,
    }));
  },

  setCurrentAgent: (id) => set({ currentAgentId: id }),

  setActiveCategory: (category) => set({ activeCategory: category }),
  setShowHidden: (show) => set({ showHidden: show }),
  setBatchSelected: (ids) => set({ batchSelected: ids }),

  openEditor: (profile) =>
    set({
      editorOpen: true,
      isCreating: !profile,
      editingProfile: profile ? { ...profile } : { ...EMPTY_PROFILE, id: newAgentId() },
    }),

  closeEditor: () =>
    set({
      editorOpen: false,
      editingProfile: EMPTY_PROFILE,
      isCreating: false,
    }),

  updateEditingField: (key, value) =>
    set((state) => ({
      editingProfile: { ...state.editingProfile, [key]: value },
    })),

  saveEditing: async () => {
    const { editingProfile, isCreating, agents } = get();
    if (!editingProfile.name.trim()) {
      throw new Error('Agent 名称不能为空');
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
    if (isCreating) {
      set({ agents: [...agents, { ...editingProfile, user_customized: true }], editorOpen: false });
    } else {
      set({
        agents: agents.map((a) => (a.id === editingProfile.id ? { ...editingProfile } : a)),
        editorOpen: false,
      });
    }
  },

  deleteAgent: async (id) => {
    await new Promise((resolve) => setTimeout(resolve, 200));
    set((state) => ({
      agents: state.agents.filter((a) => a.id !== id),
      currentAgentId: state.currentAgentId === id ? state.agents[0]?.id || null : state.currentAgentId,
    }));
  },

  fetchIdentityFile: async (_profileId, filename) => {
    await new Promise((resolve) => setTimeout(resolve, 100));
    // PoC mock：返回示例内容
    const samples: Record<string, string> = {
      'SOUL.md': '# 灵魂\n\n我是 SparkFox 助手，友好、专业、简洁。\n',
      'USER.md': '# 用户\n\n偏好中文交流，喜欢简洁回答。\n',
      'AGENT.md': '# Agent 配置\n\n- 默认模型：gpt-4o\n- 温度：0.7\n',
    };
    return {
      filename,
      content: samples[filename] || '',
      source: 'global',
    };
  },

  saveIdentityFile: async (_profileId, _filename, _content) => {
    await new Promise((resolve) => setTimeout(resolve, 200));
    // PoC: no-op
  },

  fetchMemoryStats: async (_profileId) => {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return {
      exists: true,
      semantic_count: 128,
      db_size_bytes: 256 * 1024,
    };
  },

  addCategory: async (label, color) => {
    if (!label.trim()) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
    set((state) => ({
      categories: [
        ...state.categories,
        {
          id: `cat-${Date.now()}`,
          label,
          color,
          builtin: false,
          agent_count: 0,
        },
      ],
    }));
  },

  searchStoreAgents: async (params) => {
    const { q, category, sort = 'downloads', page = 1 } = params;
    set({ storeLoading: true });
    await new Promise((resolve) => setTimeout(resolve, 200));
    let filtered = [...MOCK_STORE_AGENTS];
    if (q) {
      const lower = q.toLowerCase();
      filtered = filtered.filter(
        (a) =>
          a.name.toLowerCase().includes(lower) ||
          a.description.toLowerCase().includes(lower) ||
          (a.tags || []).some((t) => t.toLowerCase().includes(lower))
      );
    }
    if (category) {
      filtered = filtered.filter((a) => a.category === category);
    }
    if (sort === 'downloads') {
      filtered.sort((a, b) => b.downloads - a.downloads);
    } else if (sort === 'rating') {
      filtered.sort((a, b) => (b.avgRating || 0) - (a.avgRating || 0));
    }
    const pageSize = 20;
    const start = (page - 1) * pageSize;
    set({
      storeAgents: filtered.slice(start, start + pageSize),
      storeTotal: filtered.length,
      storeLoading: false,
    });
  },

  installFromStore: async (agentId) => {
    await new Promise((resolve) => setTimeout(resolve, 500));
    // PoC：从商店 mock 数据找到 Agent，转换为本地 AgentProfile
    const storeAgent = MOCK_STORE_AGENTS.find((a) => a.id === agentId);
    if (!storeAgent) return;
    const newProfile: AgentProfile = {
      id: newAgentId(),
      name: storeAgent.name,
      description: storeAgent.description,
      icon: '🤖',
      color: '#007AFF',
      type: 'custom',
      skills: [],
      skills_mode: 'all',
      tools: [],
      tools_mode: 'all',
      mcp_servers: [],
      mcp_mode: 'all',
      custom_prompt: '',
      category: 'custom',
      user_customized: true,
      identity_mode: 'shared',
      memory_mode: 'shared',
      memory_inherit_global: true,
    };
    set((state) => ({ agents: [...state.agents, newProfile] }));
  },
}));
