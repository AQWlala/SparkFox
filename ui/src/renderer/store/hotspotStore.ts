/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox hotspotStore — 热点追踪状态管理
 *
 * 来源：BaiLongma src/hotspots.js + src/ui/brain-ui/hotspot.js
 *       （清洁室重写为 TypeScript Zustand store，PoC mock 数据）
 *
 * 功能：
 * - 4 平台热榜（抖音 / 小红书 / 微信 / 微博）
 * - 实时事件流（8 类别：自然灾害 / 科技 / 财经 / 体育 / 社会 / 政策 / 旅游）
 * - 底部跑马灯
 * - 实时时钟
 * - 中性上下文构建（不强制 Agent 回复）
 * - 用户消息匹配热点 → 自动归档到 memoryStore
 * - 模式开关（panelActive）
 *
 * 改造点：
 * - BaiLongma 后端 Node.js + 前端原生 DOM → Zustand store + React
 * - HTTP /hotspots → PoC mock 数据（无后端依赖）
 * - document.getElementById → React state
 * - 3D 地球懒加载 → PoC 不引入（保留接口）
 * - 自动归档：调用 memoryStore.injectFromConversation
 *
 * AGPL 合规：BaiLongma MIT 部分已清洁室重写
 */

import { create } from 'zustand';
// Tauri 2 IPC：hotspot_track / hotspot_list 失败时静默回退到 mock 数据
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

export type Platform = 'douyin' | 'xiaohongshu' | 'wechat' | 'weibo';

/** 单条热点 */
export interface HotspotItem {
  id: string;
  platform: Platform;
  rank: number;
  title: string;
  /** 热度数值（格式化后字符串，如 "1.2亿" / "850万"） */
  heat: string;
  /** 标签：热 / 荐 / 新 / 辟谣 / 活动 */
  tag: string;
  /** 趋势：up / down / same */
  trend: 'up' | 'down' | 'same';
  /** 是否新上榜 */
  isNew: boolean;
  /** 来源 URL */
  url: string;
  /** 数据来源 */
  source: string;
}

/** 实时事件流卡片 */
export interface FeedItem {
  id: string;
  /** HH:mm 格式 */
  time: string;
  /** 类别：自然灾害 / 科技 / 财经 / 体育 / 社会 / 政策 / 旅游 */
  cat: string;
  /** 类别颜色 */
  catColor: string;
  title: string;
  desc: string;
  /** 地点 */
  loc: string;
}

/** 跑马灯项 */
export interface TickerItem {
  id: string;
  time: string;
  text: string;
}

/** 平台状态 */
export interface PlatformStatus {
  ok: boolean;
  count: number;
  source: string;
  error?: string;
}

/** 热点元信息 */
export interface HotspotMeta {
  source: 'loading' | 'hotspot-api' | 'mock';
  fetchedAt: string | null;
  stale: boolean;
  refreshMinutes: number;
  status: Partial<Record<Platform, PlatformStatus>>;
}

/** 用户消息与热点的匹配结果 */
export interface HotspotMatch {
  item: HotspotItem;
  keywords: string[];
  direct: boolean;
  rankRef: boolean;
  hitCount: number;
}

// ============================================================================
// 常量
// ============================================================================

export const PLATFORM_ORDER: Platform[] = ['douyin', 'xiaohongshu', 'wechat', 'weibo'];

export const PLATFORM_LABELS: Record<Platform, string> = {
  douyin: '抖音',
  xiaohongshu: '小红书',
  wechat: '微信热点',
  weibo: '微博',
};

export const PLATFORM_COLORS: Record<Platform, string> = {
  douyin: '#000000',
  xiaohongshu: '#FF2442',
  wechat: '#07C160',
  weibo: '#E6162D',
};

export const TREND_ICONS: Record<HotspotItem['trend'], string> = {
  up: '↑',
  down: '↓',
  same: '—',
};

/** 实时事件流类别颜色 */
export const CAT_COLORS: Record<string, string> = {
  自然灾害: '#FF3B30',
  科技: '#007AFF',
  财经: '#FF9500',
  体育: '#34C759',
  社会: '#5856D6',
  政策: '#5AC8FA',
  旅游: '#FFCC00',
};

/** 默认刷新间隔（分钟） */
export const DEFAULT_REFRESH_MINUTES = 30;

/** 上下文 TTL（分钟） */
export const HOTSPOT_CONTEXT_TTL_MINUTES = 60;

// ============================================================================
// Mock 数据
// ============================================================================

const _hid = (p: string, n: number) => `${p}_${String(n).padStart(3, '0')}`;

/** Mock：抖音热榜 */
const mockDouyin: HotspotItem[] = [
  { id: _hid('dy', 1), platform: 'douyin', rank: 1, title: '神舟十八号航天员太空过大年', heat: '1.2亿', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('dy', 2), platform: 'douyin', rank: 2, title: '春运高峰期多地迎雨雪天气', heat: '9854万', tag: '热', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('dy', 3), platform: 'douyin', rank: 3, title: '华为发布全新AI芯片昇腾910C', heat: '8721万', tag: '荐', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('dy', 4), platform: 'douyin', rank: 4, title: '国产大模型DeepSeek-V3开源', heat: '7632万', tag: '热', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('dy', 5), platform: 'douyin', rank: 5, title: '巴黎奥运会圣火采集仪式', heat: '6541万', tag: '热', trend: 'down', isNew: false, url: '', source: 'mock' },
  { id: _hid('dy', 6), platform: 'douyin', rank: 6, title: '欧盟通过AI监管法案', heat: '5432万', tag: '新', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('dy', 7), platform: 'douyin', rank: 7, title: 'A股春节后开门红沪指涨1.5%', heat: '4321万', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('dy', 8), platform: 'douyin', rank: 8, title: '马斯克称Neuralink完成首例人脑植入', heat: '3210万', tag: '热', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('dy', 9), platform: 'douyin', rank: 9, title: '多地景区迎春节客流高峰', heat: '2876万', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('dy', 10), platform: 'douyin', rank: 10, title: 'OpenAI发布GPT-5预告', heat: '2543万', tag: '新', trend: 'up', isNew: true, url: '', source: 'mock' },
];

/** Mock：小红书热榜 */
const mockXiaohongshu: HotspotItem[] = [
  { id: _hid('xhs', 1), platform: 'xiaohongshu', rank: 1, title: '春节后减肥食谱大公开', heat: '523万', tag: '荐', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('xhs', 2), platform: 'xiaohongshu', rank: 2, title: '2026早春穿搭灵感', heat: '421万', tag: '热', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('xhs', 3), platform: 'xiaohongshu', rank: 3, title: '居家收纳神器测评', heat: '387万', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('xhs', 4), platform: 'xiaohongshu', rank: 4, title: '职场新人通勤穿搭', heat: '312万', tag: '热', trend: 'down', isNew: false, url: '', source: 'mock' },
  { id: _hid('xhs', 5), platform: 'xiaohongshu', rank: 5, title: '春节旅游避坑指南', heat: '287万', tag: '新', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('xhs', 6), platform: 'xiaohongshu', rank: 6, title: '家居改造前后对比', heat: '254万', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('xhs', 7), platform: 'xiaohongshu', rank: 7, title: '国货美妆品牌排行', heat: '221万', tag: '荐', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('xhs', 8), platform: 'xiaohongshu', rank: 8, title: '一周快手菜谱合集', heat: '198万', tag: '热', trend: 'down', isNew: false, url: '', source: 'mock' },
];

/** Mock：微信热点 */
const mockWechat: HotspotItem[] = [
  { id: _hid('wx', 1), platform: 'wechat', rank: 1, title: '国务院发布2026年1号文件', heat: '荐', tag: '荐', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('wx', 2), platform: 'wechat', rank: 2, title: '全国两会3月在北京召开', heat: '热', tag: '热', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('wx', 3), platform: 'wechat', rank: 3, title: '央行调整LPR利率', heat: '热', tag: '热', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('wx', 4), platform: 'wechat', rank: 4, title: '教育部公布新高考方案', heat: '热', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('wx', 5), platform: 'wechat', rank: 5, title: '医保跨省结算全面推行', heat: '荐', tag: '荐', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('wx', 6), platform: 'wechat', rank: 6, title: '新能源汽车补贴延续', heat: '热', tag: '热', trend: 'down', isNew: false, url: '', source: 'mock' },
];

/** Mock：微博热搜 */
const mockWeibo: HotspotItem[] = [
  { id: _hid('wb', 1), platform: 'weibo', rank: 1, title: '#神舟十八号太空过年#', heat: '987万', tag: '热', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('wb', 2), platform: 'weibo', rank: 2, title: '#DeepSeek-V3开源#', heat: '854万', tag: '热', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('wb', 3), platform: 'weibo', rank: 3, title: '#华为昇腾910C发布#', heat: '765万', tag: '热', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('wb', 4), platform: 'weibo', rank: 4, title: '#欧盟AI法案生效#', heat: '654万', tag: '新', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('wb', 5), platform: 'weibo', rank: 5, title: '#A股开门红#', heat: '543万', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('wb', 6), platform: 'weibo', rank: 6, title: '#巴黎奥运倒计时#', heat: '432万', tag: '热', trend: 'down', isNew: false, url: '', source: 'mock' },
  { id: _hid('wb', 7), platform: 'weibo', rank: 7, title: '#马斯克Neuralink#', heat: '321万', tag: '热', trend: 'up', isNew: false, url: '', source: 'mock' },
  { id: _hid('wb', 8), platform: 'weibo', rank: 8, title: '#春节旅游高峰#', heat: '287万', tag: '热', trend: 'same', isNew: false, url: '', source: 'mock' },
  { id: _hid('wb', 9), platform: 'weibo', rank: 9, title: '#OpenAI GPT-5预告#', heat: '254万', tag: '新', trend: 'up', isNew: true, url: '', source: 'mock' },
  { id: _hid('wb', 10), platform: 'weibo', rank: 10, title: '#多地雨雪天气#', heat: '221万', tag: '热', trend: 'down', isNew: false, url: '', source: 'mock' },
];

/** Mock：实时事件流 */
const mockFeed: FeedItem[] = [
  { id: 'feed_1', time: '19:25', cat: '自然灾害', catColor: CAT_COLORS['自然灾害'], title: '四川宜宾县发生6.0级地震', desc: '震源深度10公里，暂无人员伤亡报告，救援力量已巡查到达震源周边', loc: '中国·四川' },
  { id: 'feed_2', time: '19:24', cat: '科技', catColor: CAT_COLORS['科技'], title: '神舟十八号发射任务圆满成功', desc: '载人飞船与空间站组合体成功对接，状态良好。', loc: '酒泉卫星发射中心' },
  { id: 'feed_3', time: '19:23', cat: '财经', catColor: CAT_COLORS['财经'], title: '特斯拉全球召回超110万辆汽车', desc: '涉及安全带及软件问题，特斯拉免费修复。', loc: '全球' },
  { id: 'feed_4', time: '19:22', cat: '体育', catColor: CAT_COLORS['体育'], title: '巴黎奥运圣火抵达马赛港', desc: '开幕式倒计时启动，法国全境传递沿线盛况空前，7月26日开幕。', loc: '法国·马赛' },
  { id: 'feed_5', time: '19:21', cat: '社会', catColor: CAT_COLORS['社会'], title: '台风"玛莉亚"逼近东南沿海', desc: '预计26日凌晨在浙江登陆，多地发布台风橙色预警，船只回港避险。', loc: '中国·东南沿海' },
  { id: 'feed_6', time: '19:19', cat: '科技', catColor: CAT_COLORS['科技'], title: '华为发布全新 AI 芯片', desc: '性能较上代提升60%，将首批搭载于旗舰产品线，引发行业广泛关注。', loc: '中国·深圳' },
  { id: 'feed_7', time: '19:18', cat: '政策', catColor: CAT_COLORS['政策'], title: '欧盟正式通过 AI 监管法案', desc: '《人工智能法案》生效，将对高风险AI系统实施强制合规审查。', loc: '比利时·布鲁塞尔' },
  { id: 'feed_8', time: '19:17', cat: '旅游', catColor: CAT_COLORS['旅游'], title: '多地景区迎来客流高峰', desc: '暑期旅游热度持续攀升，热门景区单日接待游客超历史纪录。', loc: '中国多地' },
];

/** Mock：跑马灯 */
const mockTicker: TickerItem[] = [
  { id: 'tk_1', time: '19:20', text: '上海发布高温红色预警，气温预计突破40℃' },
  { id: 'tk_2', time: '19:19', text: '全球芯片市场半年报告发布，亚太份额持续上升' },
  { id: 'tk_3', time: '19:18', text: '欧盟通过 AI 法案，将对高风险系统强制审查' },
  { id: 'tk_4', time: '19:17', text: '多地景区迎来客流高峰，暑运旅游市场表现亮眼' },
  { id: 'tk_5', time: '19:16', text: '国际油价小幅上涨，布伦特原油突破85美元/桶' },
  { id: 'tk_6', time: '19:15', text: 'A股午后强势拉升，沪指收涨1.24%，科技板块领涨' },
  { id: 'tk_7', time: '19:14', text: '北京时间明日凌晨2点：欧洲杯决赛，全球直播' },
  { id: 'tk_8', time: '19:13', text: '研究显示：今夏北半球平均气温创历史新高' },
];

/** 全部 mock 平台数据 */
const MOCK_PLATFORM_DATA: Record<Platform, HotspotItem[]> = {
  douyin: mockDouyin,
  xiaohongshu: mockXiaohongshu,
  wechat: mockWechat,
  weibo: mockWeibo,
};

// ============================================================================
// 工具函数
// ============================================================================

/** 格式化热度数值 */
export function formatHeat(value: number | string): string {
  const n = Number(value);
  if (!Number.isFinite(n)) return String(value || '');
  if (n >= 100000000) return `${(n / 100000000).toFixed(n >= 1000000000 ? 1 : 2).replace(/\.0+$/, '')}亿`;
  if (n >= 10000) return `${Math.round(n / 10000)}万`;
  return String(n);
}

/** 标签文本 */
export function labelText(label: string | number): string {
  const value = String(label ?? '').trim();
  if (!value || value === '0') return '';
  const labels: Record<string, string> = {
    '1': '热',
    '3': '热',
    '5': '荐',
    '8': '新',
    '16': '辟谣',
    '17': '活动',
  };
  return labels[value] || value;
}

/** 标准化搜索文本 */
export function normalizeSearchText(text = ''): string {
  return String(text || '')
    .toLowerCase()
    .replace(/[^\p{Script=Han}a-z0-9]+/gu, '');
}

/** 提取热点关键词 */
export function extractHotspotKeywords(title = ''): string[] {
  const cleaned = String(title || '').replace(/[^\p{Script=Han}a-zA-Z0-9]+/gu, ' ').trim();
  const words = new Set<string>();
  for (const part of cleaned.split(/\s+/).filter(Boolean)) {
    if (/^[a-zA-Z0-9]{3,}$/.test(part)) words.add(part.toLowerCase());
  }
  const compact = cleaned.replace(/\s+/g, '');
  for (let i = 0; i < compact.length - 1; i++) {
    for (let len = 2; len <= 5 && i + len <= compact.length; len++) {
      const token = compact.slice(i, i + len);
      if (/[\p{Script=Han}]/u.test(token)) words.add(token);
    }
  }
  return [...words].slice(0, 24);
}

/** 用户消息匹配热点 */
export function matchHotspots(
  message: string,
  items: HotspotItem[],
): HotspotMatch[] {
  const normalizedMessage = normalizeSearchText(message);
  if (!normalizedMessage) return [];
  const rawMessage = String(message || '');

  const matches: HotspotMatch[] = [];
  for (const item of items) {
    const title = item.title;
    const normalizedTitle = normalizeSearchText(title);
    if (!normalizedTitle) continue;
    const rank = Number(item.rank || 0);
    const platformLabel = PLATFORM_LABELS[item.platform];
    const rankRef =
      rank > 0 &&
      (new RegExp(`(热搜|热点|榜单|${platformLabel}).{0,4}(第\\s*${rank}|${rank}\\s*(条|名|位))`).test(
        rawMessage,
      ) ||
        (rank === 1 &&
          new RegExp(`(热搜|热点|榜单|${platformLabel}).{0,4}(第一|榜一|第\\s*1|1\\s*(条|名|位))`).test(
            rawMessage,
          )));

    const direct =
      normalizedMessage.includes(normalizedTitle) ||
      (normalizedTitle.length >= 4 && normalizedTitle.includes(normalizedMessage));

    const keywords = extractHotspotKeywords(title);
    const hitCount = keywords.filter((k) => normalizedMessage.includes(normalizeSearchText(k))).length;

    if (direct || rankRef || hitCount >= 2) {
      matches.push({ item, keywords: keywords.slice(0, 8), direct, rankRef, hitCount });
    }
  }

  return matches.slice(0, 5);
}

/** 格式化时间 */
function fmtFetchedAt(value: string | null): string {
  if (!value) return '未知';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return '未知';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

// ============================================================================
// Store 接口
// ============================================================================

interface HotspotState {
  initialized: boolean;
  loading: boolean;
  panelActive: boolean;
  panelActiveUntilMs: number;
  hotspotLists: Record<Platform, HotspotItem[]>;
  feedItems: FeedItem[];
  tickerItems: TickerItem[];
  meta: HotspotMeta;
  feedIndex: number;
  feedAuto: boolean;
  clock: string;
  selectedHotspot: HotspotItem | null;
  /** 用户消息匹配到的热点列表 */
  matchedHotspots: HotspotMatch[];
  /** 待注入到对话的热点（数据流5：HotspotView 点击"发送到对话" → ChatPanel 消费） */
  pendingChatInjection: HotspotItem | null;

  // Actions
  initialize: () => Promise<void>;
  refresh: () => Promise<void>;
  setPanelActive: (active: boolean, source?: string) => void;
  setFeedIndex: (idx: number) => void;
  nextFeed: () => void;
  prevFeed: () => void;
  setFeedAuto: (auto: boolean) => void;
  updateClock: () => void;
  selectHotspot: (item: HotspotItem | null) => void;
  /** 用户消息 → 匹配热点 */
  matchUserMessage: (message: string) => HotspotMatch[];
  /** 构建中性上下文（用于注入 Agent） */
  buildContext: (message?: string) => string;
  /** 关键词搜索热点 */
  search: (query: string) => HotspotItem[];
  /** 数据流5：将热点发送到对话页讨论（设置 pendingChatInjection，ChatPanel 监听后调用 injectHotspotToChat） */
  sendToChat: (item: HotspotItem) => void;
  /** 清除待注入的热点（ChatPanel 消费后调用） */
  clearPendingChatInjection: () => void;

  // ─── Tauri IPC 对接（渐进增强：失败时回退到 mock，不抛异常） ───
  /**
   * 调用后端追踪指定关键词的热点（Tauri IPC: hotspot_track）
   * @param keyword 关键词；后端返回追踪结果并存储
   */
  trackFromBackend: (keyword: string) => Promise<void>;
  /**
   * 从后端拉取热点列表（Tauri IPC: hotspot_list）
   * @param limit 最大条数；失败时回退到本地 mock 数据
   */
  listFromBackend: (limit?: number) => Promise<void>;
}

// ============================================================================
// Store 实现
// ============================================================================

export const useHotspotStore = create<HotspotState>((set, get) => ({
  initialized: false,
  loading: false,
  panelActive: false,
  panelActiveUntilMs: 0,
  hotspotLists: { douyin: [], xiaohongshu: [], wechat: [], weibo: [] },
  feedItems: mockFeed,
  tickerItems: mockTicker,
  meta: {
    source: 'loading',
    fetchedAt: null,
    stale: true,
    refreshMinutes: DEFAULT_REFRESH_MINUTES,
    status: {},
  },
  feedIndex: 0,
  feedAuto: true,
  clock: '',
  selectedHotspot: null,
  matchedHotspots: [],
  pendingChatInjection: null,

  initialize: async () => {
    if (get().initialized) return;
    set({ loading: true });
    await new Promise((r) => setTimeout(r, 200));
    set({
      hotspotLists: MOCK_PLATFORM_DATA,
      meta: {
        source: 'mock',
        fetchedAt: new Date().toISOString(),
        stale: false,
        refreshMinutes: DEFAULT_REFRESH_MINUTES,
        status: {
          douyin: { ok: true, count: mockDouyin.length, source: 'mock' },
          xiaohongshu: { ok: true, count: mockXiaohongshu.length, source: 'mock' },
          wechat: { ok: true, count: mockWechat.length, source: 'mock' },
          weibo: { ok: true, count: mockWeibo.length, source: 'mock' },
        },
      },
      initialized: true,
      loading: false,
    });
  },

  refresh: async () => {
    set({ loading: true });
    await new Promise((r) => setTimeout(r, 200));
    set((state) => ({
      hotspotLists: MOCK_PLATFORM_DATA,
      meta: {
        ...state.meta,
        source: 'mock',
        fetchedAt: new Date().toISOString(),
        stale: false,
      },
      loading: false,
    }));
  },

  setPanelActive: (active, _source = 'user') => {
    set({
      panelActive: active,
      panelActiveUntilMs: active
        ? Date.now() + HOTSPOT_CONTEXT_TTL_MINUTES * 60 * 1000
        : 0,
    });
  },

  setFeedIndex: (idx) => {
    const total = get().feedItems.length;
    if (total === 0) return;
    set({ feedIndex: ((idx % total) + total) % total });
  },

  nextFeed: () => get().setFeedIndex(get().feedIndex + 1),
  prevFeed: () => get().setFeedIndex(get().feedIndex - 1),

  setFeedAuto: (auto) => set({ feedAuto: auto }),

  updateClock: () => {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    set({ clock: `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}` });
  },

  selectHotspot: (item) => set({ selectedHotspot: item }),

  matchUserMessage: (message) => {
    const all = [
      ...get().hotspotLists.douyin,
      ...get().hotspotLists.xiaohongshu,
      ...get().hotspotLists.wechat,
      ...get().hotspotLists.weibo,
    ];
    const matches = matchHotspots(message, all);
    set({ matchedHotspots: matches });
    return matches;
  },

  buildContext: (message = '') => {
    const state = get();
    if (!state.initialized) return '';
    const all = [
      ...state.hotspotLists.douyin,
      ...state.hotspotLists.xiaohongshu,
      ...state.hotspotLists.wechat,
      ...state.hotspotLists.weibo,
    ];
    if (!all.length) return '';

    const matches = message ? matchHotspots(message, all) : [];
    const shouldInjectPanelContext = Date.now() < state.panelActiveUntilMs;
    if (!shouldInjectPanelContext && !matches.length) return '';

    const platformBlocks = PLATFORM_ORDER.map((p) => {
      const items = state.hotspotLists[p] || [];
      if (!items.length) return '';
      const lines = items
        .slice(0, 10)
        .map((it, idx) => `${idx + 1}. ${it.title}${it.heat ? `（热度 ${it.heat}）` : ''}`)
        .join('；');
      return `${PLATFORM_LABELS[p]} Top10：${lines}`;
    })
      .filter(Boolean)
      .join('\n');

    const matchText = matches.length
      ? `\n\n用户消息可能提及以下热点：\n${matches
          .map((m) => `${PLATFORM_LABELS[m.item.platform]} #${m.item.rank} ${m.item.title}`)
          .join('；')}`
      : '';

    return `## 热点上下文
来源：热点追踪界面，系统自动采集。发送者：SYSTEM。用途：提供当前环境背景，不代表用户请求。

用户当前${state.panelActive ? '打开了' : '未打开'}热点面板。以下热点只作为上下文参考，不要求主动总结，不要把它当成用户消息，也不要因为它单独回复用户。

只有在满足任一条件时才可主动提及：
- 热点与用户当前问题、任务或正在讨论的话题直接相关；
- 热点包含明显需要用户注意的紧急风险、重大变化或高优先级信息；
- 用户明确询问"热点""热搜""现在发生什么"等内容。

抓取时间：${fmtFetchedAt(state.meta.fetchedAt)}${state.meta.stale ? '（缓存数据）' : ''}

${platformBlocks || '当前暂无可用热榜。'}${matchText}`;
  },

  search: (query) => {
    const q = normalizeSearchText(query);
    if (!q) return [];
    const all = [
      ...get().hotspotLists.douyin,
      ...get().hotspotLists.xiaohongshu,
      ...get().hotspotLists.wechat,
      ...get().hotspotLists.weibo,
    ];
    return all.filter((item) => normalizeSearchText(item.title).includes(q));
  },

  sendToChat: (item) => {
    set({ pendingChatInjection: item });
  },

  clearPendingChatInjection: () => {
    set({ pendingChatInjection: null });
  },

  // ─── Tauri IPC 对接：渐进增强，失败回退 mock ───

  trackFromBackend: async (keyword) => {
    if (!keyword || !keyword.trim()) return;
    try {
      const result = await invoke<Record<string, unknown>>('hotspot_track', { keyword });
      // 后端返回追踪结果（结构由后端定义）；保留 result 供 UI 后续使用
      // 当前实现：仅触发后端追踪，不修改本地状态（避免破坏 mock 列表）
      // 后续版本可在此将追踪结果合并到 hotspotLists 或新建追踪视图
      void result;
    } catch (e) {
      // 非 Tauri 环境或 command 未注册，静默回退
      console.warn(`[hotspotStore] trackFromBackend(${keyword}) failed:`, e);
    }
  },

  listFromBackend: async (limit = 20) => {
    try {
      const items = await invoke<HotspotItem[]>('hotspot_list', { limit });
      if (Array.isArray(items) && items.length > 0) {
        // 后端有数据 → 按 platform 分组覆盖到 hotspotLists
        const grouped: Record<Platform, HotspotItem[]> = {
          douyin: [],
          xiaohongshu: [],
          wechat: [],
          weibo: [],
        };
        for (const item of items) {
          if (item && item.platform && grouped[item.platform]) {
            grouped[item.platform].push(item);
          }
        }
        set((state) => ({
          hotspotLists: grouped,
          meta: {
            ...state.meta,
            source: 'hotspot-api',
            fetchedAt: new Date().toISOString(),
            stale: false,
          },
        }));
      }
      // 后端返回空 → 保留现有 mock 数据
    } catch (e) {
      // 非 Tauri 环境或 command 未注册，静默回退
      console.warn(`[hotspotStore] listFromBackend(${limit}) failed, using mock:`, e);
    }
  },
}));

// ============================================================================
// 选择器
// ============================================================================

/** 全部热点（4 平台合并） */
export function selectAllHotspots(state: HotspotState): HotspotItem[] {
  return [
    ...state.hotspotLists.douyin,
    ...state.hotspotLists.xiaohongshu,
    ...state.hotspotLists.wechat,
    ...state.hotspotLists.weibo,
  ];
}

/** 总数 */
export function selectTotalCount(state: HotspotState): number {
  return selectAllHotspots(state).length;
}
