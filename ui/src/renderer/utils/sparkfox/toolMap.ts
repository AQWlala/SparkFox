/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox 工具映射表 + 工具详情格式化器
 *
 * 来源：BaiLongma src/ui/brain-ui/thought-stream.js（清洁室重写为 TS 纯函数）
 *
 * 保留 BaiLongma 特性：
 * - 57 个工具名中文映射（TOOL_ZH）
 * - 52 个工具 emoji 映射（TOOL_ICON）
 * - isFailureResult 失败结果识别
 * - 14 个工具详情格式化器（formatWebSearchDetail / formatFetchUrlDetail 等）
 * - formatToolSubject（22 类工具的 subject 提取）
 * - formatToolDetail（工具详情总调度）
 *
 * 改造点：
 * - JS → TypeScript（添加类型注解）
 * - class 方法 → 纯函数（无 this 依赖，便于 tree-shaking）
 * - BaiLongma CSS 变量颜色 → 由调用方注入（保持纯函数特性）
 */

// ─── 工具名中文映射（57 个） ───

export const TOOL_ZH: Record<string, string> = {
  send_message: '发送消息',
  express: '表达',
  read_file: '读取文件',
  write_file: '写入文件',
  delete_file: '删除文件',
  make_dir: '创建目录',
  list_dir: '查看目录',
  exec_command: '执行命令',
  exec_quick_command: '快速命令',
  exec_task_command: '任务命令',
  exec_background_command: '后台命令',
  download_file: '下载文件',
  kill_process: '终止进程',
  list_processes: '列出进程',
  web_search: '搜索网页',
  fetch_url: '抓取网页',
  browser_read: '浏览器读取网页',
  search_memory: '检索记忆',
  probe_memory: '探测记忆',
  upsert_memory: '写入记忆',
  merge_memories: '合并记忆',
  downgrade_memory: '降级记忆',
  recall_memory: '唤起记忆',
  skip_recognition: '跳过识别',
  skip_consolidation: '跳过整理',
  set_tick_interval: '调整节奏',
  speak: '朗读',
  generate_lyrics: '生成歌词',
  generate_music: '生成音乐',
  generate_image: '生成图片',
  ui_set: '投影界面',
  focus_banner: '专注横幅',
  set_task: '启动任务',
  complete_task: '完成任务',
  update_task_step: '推进任务',
  schedule_reminder: '安排提醒',
  manage_reminder: '管理提醒',
  manage_prefetch_task: '预抓任务',
  set_location: '设置定位',
  set_agent_name: '设置代号',
  set_security: '设置权限',
  delegate_to_agent: '委派代理',
  grant_agent_delegation: '授权代理',
  complete_startup_self_check: '完成自检',
  install_tool: '安装工具',
  install_software: '安装软件',
  uninstall_tool: '卸载工具',
  list_tools: '列出工具',
  connect_wechat: '连接微信',
  connect_feishu: '连接飞书',
  media_mode: '媒体模式',
  hotspot_mode: '热点模式',
  open_doc_panel: '打开文档',
  person_card_mode: '人物名片',
  music: '播放音乐',
};

// ─── 工具 emoji 映射（52 个） ───

export const TOOL_ICON: Record<string, string> = {
  send_message: '💬',
  express: '🗣️',
  read_file: '📄',
  write_file: '✏️',
  delete_file: '🗑️',
  make_dir: '📁',
  list_dir: '📂',
  exec_command: '⚡',
  kill_process: '🛑',
  list_processes: '📋',
  web_search: '🔎',
  fetch_url: '🌐',
  browser_read: '🧭',
  search_memory: '🔍',
  probe_memory: '🩺',
  upsert_memory: '🧠',
  merge_memories: '🧬',
  downgrade_memory: '🌫️',
  recall_memory: '💭',
  skip_recognition: '⏭️',
  skip_consolidation: '⏭️',
  set_tick_interval: '⏱️',
  speak: '🔊',
  generate_lyrics: '🎵',
  generate_music: '🎼',
  generate_image: '🎨',
  ui_set: '🎴',
  focus_banner: '🎯',
  set_task: '📋',
  complete_task: '✅',
  update_task_step: '↳',
  schedule_reminder: '⏰',
  manage_reminder: '⏰',
  manage_prefetch_task: '📡',
  set_location: '📍',
  set_agent_name: '🪪',
  set_security: '🔐',
  delegate_to_agent: '🤝',
  grant_agent_delegation: '🤝',
  complete_startup_self_check: '🩺',
  install_tool: '🔧',
  install_software: '⬇️',
  uninstall_tool: '🔧',
  list_tools: '🧰',
  connect_wechat: '🔗',
  connect_feishu: '🪶',
  media_mode: '🎬',
  hotspot_mode: '🔥',
  open_doc_panel: '📖',
  person_card_mode: '🪪',
  music: '🎶',
};

// ─── AI 活动状态 8 大工具分组（用于派生"正在扫描文件"等标签） ───

export const AI_TOOL_GROUPS: Record<string, Set<string>> = {
  扫描文件: new Set(['read_file', 'list_dir']),
  改动文件: new Set(['write_file', 'make_dir', 'delete_file']),
  执行命令: new Set([
    'exec_command',
    'exec_quick_command',
    'exec_task_command',
    'exec_background_command',
    'download_file',
    'kill_process',
    'list_processes',
  ]),
  上网: new Set(['fetch_url', 'web_search', 'browser_read']),
  调取记忆: new Set([
    'search_memory',
    'recall_memory',
    'probe_memory',
    'upsert_memory',
    'merge_memories',
    'downgrade_memory',
  ]),
  推送界面: new Set(['ui_set', 'focus_banner']),
  处理多媒体: new Set([
    'speak',
    'generate_lyrics',
    'generate_music',
    'generate_image',
    'music',
    'media_mode',
  ]),
  回复用户: new Set(['send_message', 'express']),
};

/** 默认分组（未匹配以上 8 类时） */
export const AI_ACTIVITY_DEFAULT_GROUP = '处理事务';

/**
 * 工具归类（用于 AI 活动状态派生）
 */
export function classifyTool(name: string): string {
  for (const [label, set] of Object.entries(AI_TOOL_GROUPS)) {
    if (set.has(name)) return label;
  }
  return AI_ACTIVITY_DEFAULT_GROUP;
}

// ─── 工具状态类型 ───

export type ToolStatus = 'success' | 'failed' | 'ended';

// ─── 失败结果识别 ───

export function isFailureResult(resultStr: string | null | undefined): boolean {
  const t = (resultStr || '').trim();
  if (!t) return false;
  if (/^(错误|失败|异常)[：:]/.test(t) || /^Error\b/i.test(t) || /^ERROR\b/.test(t)) return true;
  try {
    const parsed = JSON.parse(t);
    if (parsed && typeof parsed === 'object' && parsed.ok === false) return true;
  } catch {
    // ignore
  }
  return false;
}

// ─── 通用工具函数 ───

/** 紧凑文本（去多余空白 + 超长截断） */
export function compactText(text: string | null | undefined, max = 180): string {
  const compact = String(text || '').replace(/\s+/g, ' ').trim();
  return compact.length > max ? `${compact.slice(0, max)}…` : compact;
}

/** 从 URL 提取主域名 */
export function hostFromUrl(url: string | null | undefined): string {
  try {
    return new URL(String(url || '')).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

/** 短路径（长路径折叠为 head/…/tail） */
export function shortPath(p: string | null | undefined, max = 48): string {
  const s = String(p || '').trim();
  if (!s) return '';
  if (s.length <= max) return s;
  const norm = s.replace(/\\/g, '/');
  const segs = norm.split('/').filter(Boolean);
  if (segs.length >= 3) {
    const tail = segs.slice(-2).join('/');
    const head = segs[0];
    const candidate = `${head}/…/${tail}`;
    if (candidate.length <= max) return candidate;
    return `…/${tail.slice(max - 2)}`;
  }
  return `${s.slice(0, max - 1)}…`;
}

/** 短命令 */
export function shortCommand(cmd: string | null | undefined, max = 60): string {
  return compactText(String(cmd || '').replace(/\s+/g, ' ').trim(), max);
}

/** JSON 结果解析 */
export function parseJsonResult(result: string | null | undefined): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(String(result || ''));
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

// ─── 14 个工具详情格式化器 ───

interface ToolPayload {
  ok?: boolean;
  error?: string;
  status?: number;
  hint?: string;
  query?: string;
  url?: string;
  title?: string;
  content?: string;
  final_url?: string;
  results?: Array<{ url?: string; title?: string; snippet?: string }>;
  stdout?: string;
  stderr?: string;
  exit_code?: number | null;
  timed_out?: boolean;
  aborted?: boolean;
  mode?: string;
  pid?: number;
  timeout_ms?: number;
  policy?: { risk?: string; reason?: string };
  hits?: Array<{ summary?: string; content?: string; text?: string }>;
  results_hits?: Array<{ summary?: string; content?: string; text?: string }>;
  memories?: Array<{ summary?: string; content?: string; text?: string }>;
  entries?: Array<string | { name?: string }>;
  items?: Array<string | { name?: string }>;
  files?: Array<string | { name?: string }>;
  summary?: string;
  message?: string;
  detail?: string;
  tool?: string;
}

/** web_search 详情 */
export function formatWebSearchDetail(payload: ToolPayload): string {
  const results = Array.isArray(payload.results) ? payload.results : [];
  if (payload.ok === false) {
    return `搜索失败：${payload.error || '没有拿到结果'}。关键词：${payload.query || '未提供'}`;
  }
  const lines = [`关键词：${payload.query || '未提供'}；找到 ${results.length} 条结果。`];
  results.slice(0, 3).forEach((item, index) => {
    const host = hostFromUrl(item.url);
    const title = compactText(item.title || item.url || '未命名结果', 70);
    const snippet = compactText(item.snippet || '', 90);
    lines.push(`${index + 1}. ${title}${host ? `（${host}）` : ''}${snippet ? `：${snippet}` : ''}`);
  });
  return lines.join(' ');
}

/** fetch_url 详情 */
export function formatFetchUrlDetail(payload: ToolPayload): string {
  const host = hostFromUrl(payload.url);
  if (payload.ok === false) {
    const status = payload.status ? `HTTP ${payload.status}` : payload.error || '请求失败';
    if (payload.error === 'no readable content extracted') {
      return `未读到正文：页面能打开${host ? `（${host}）` : ''}，但只拿到空白、等待页或反爬验证内容。建议换一个可直接访问的来源。`;
    }
    return `读取失败：${status}${host ? `；来源：${host}` : ''}。${payload.hint ? compactText(payload.hint, 90) : '可以换一个可访问来源。'}`;
  }
  const title = compactText(payload.title || host || payload.url || '网页', 80);
  const content = compactText(payload.content || '', 220);
  return `已读取：${title}${host ? `（${host}）` : ''}。${content || '页面能打开，但没有提取到可用正文。'}`;
}

/** browser_read 详情 */
export function formatBrowserReadDetail(payload: ToolPayload): string {
  const host = hostFromUrl(payload.final_url || payload.url);
  if (payload.ok === false) {
    if (payload.error === 'no readable content rendered') {
      return `浏览器已打开页面${host ? `（${host}）` : ''}，但仍未读到正文；可能需要登录、验证码或阻止自动化访问。建议换来源。`;
    }
    return `浏览器读取失败${host ? `（${host}）` : ''}：${compactText(payload.error || '页面无法渲染', 120)}`;
  }
  const title = compactText(payload.title || host || payload.final_url || payload.url || '网页', 80);
  const content = compactText(payload.content || '', 240);
  return `浏览器已读取：${title}${host ? `（${host}）` : ''}。${content || '页面已渲染，但没有提取到可用正文。'}`;
}

/** exec_command 详情 */
export function formatExecCommandDetail(payload: ToolPayload): string {
  if (payload.ok === false) {
    if (payload.error === 'permission denied') {
      const risk = payload.policy?.risk;
      const reason = payload.policy?.reason || '策略拒绝';
      const riskLabel = risk === 'high' ? '高风险' : risk === 'medium' ? '中风险' : risk === 'low' ? '低风险' : '受限';
      return `权限被拒绝（${riskLabel}）：${reason}`;
    }
    if (payload.timed_out) {
      return `命令超时（${Math.round((payload.timeout_ms || 0) / 1000)}s）${payload.stderr ? '；stderr：' + compactText(payload.stderr, 120) : ''}`;
    }
    if (payload.aborted) return '命令已被中断。';
    const code = payload.exit_code != null ? `退出码 ${payload.exit_code}` : '执行失败';
    const errOut = payload.stderr || payload.stdout || payload.error || '';
    return `命令失败（${code}）${errOut ? '：' + compactText(errOut.replace(/\s+/g, ' '), 160) : ''}`;
  }
  if (payload.mode === 'background') {
    return `已转入后台运行，pid ${payload.pid}。可用 list_processes 查看，kill_process 停止。`;
  }
  if (payload.mode === 'promoted_to_background') {
    return `前台超时，已转入后台，pid ${payload.pid}。`;
  }
  const stdout = String(payload.stdout || '').trim();
  if (stdout) {
    const preview = compactText(stdout.replace(/\s+/g, ' '), 180);
    return `输出：${preview}`;
  }
  if (payload.stderr) {
    return `stderr：${compactText(payload.stderr.replace(/\s+/g, ' '), 160)}`;
  }
  return `命令完成（退出码 ${payload.exit_code ?? 0}）。`;
}

/** 通用 permission denied */
export function formatGenericPermissionDenied(payload: ToolPayload): string {
  const risk = payload.policy?.risk;
  const reason = payload.policy?.reason || '策略拒绝';
  const riskLabel = risk === 'high' ? '高风险' : risk === 'medium' ? '中风险' : risk === 'low' ? '低风险' : '受限';
  return `权限被拒绝（${riskLabel}）：${reason}`;
}

/** search_memory 详情 */
export function formatSearchMemoryDetail(payload: ToolPayload): string {
  if (payload?.ok === false) return compactText(payload.error || '检索失败', 120);
  const hits = Array.isArray(payload?.hits)
    ? payload.hits
    : Array.isArray(payload?.results_hits)
      ? payload.results_hits
      : Array.isArray(payload?.memories)
        ? payload.memories
        : null;
  if (hits) {
    if (hits.length === 0) return '没有命中记忆。';
    const preview = hits
      .slice(0, 2)
      .map((h) => compactText(h.summary || h.content || h.text || '', 50))
      .filter(Boolean)
      .join(' ｜ ');
    return `命中 ${hits.length} 条${preview ? '：' + preview : ''}`;
  }
  return '';
}

/** read_file 详情 */
export function formatFileReadDetail(result: string | null | undefined): string {
  const s = String(result || '').trim();
  if (!s) return '（空文件）';
  if (s.startsWith('错误')) return compactText(s, 160);
  return `内容预览：${compactText(s.replace(/\s+/g, ' '), 160)}`;
}

/** 通用 OK 详情 */
export function formatGenericOkDetail(
  payload: ToolPayload | null,
  raw: string | null | undefined,
  toolDetailLength = 160
): string {
  if (payload?.ok === false) {
    return compactText(payload.error || '执行失败', 160);
  }
  if (payload?.ok === true) {
    const meaningful = payload.summary || payload.message || payload.detail || payload.hint;
    if (meaningful) return compactText(String(meaningful), 160);
    return '';
  }
  const trimmed = String(raw ?? '').trim();
  if (!trimmed) return '';
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
    return '结果过长未展开。';
  }
  return compactText(trimmed.replace(/\s+/g, ' '), toolDetailLength);
}

// ─── 22 类工具的 subject 提取 ───

export function formatToolSubject(
  name: string,
  args: Record<string, unknown> = {},
  parsed: Record<string, unknown> | null = null
): string {
  const a = args || {};
  const p = (parsed || {}) as Record<string, unknown>;
  switch (name) {
    case 'read_file':
    case 'write_file':
    case 'delete_file':
    case 'make_dir':
    case 'list_dir':
      return shortPath(a.path as string);
    case 'exec_command':
      return shortCommand(a.command as string || (p.command as string));
    case 'kill_process':
      return a.pid ? `pid ${a.pid}` : '';
    case 'web_search':
      return compactText(a.query as string || (p.query as string) || '', 60);
    case 'fetch_url':
    case 'browser_read':
      return hostFromUrl(a.url as string || (p.url as string)) || compactText(a.url as string || '', 60);
    case 'search_memory':
      return Array.isArray(a.keywords) ? (a.keywords as string[]).slice(0, 4).join(' / ') : '';
    case 'upsert_memory':
    case 'merge_memories':
    case 'downgrade_memory':
    case 'recall_memory':
      return compactText(a.summary as string || a.note as string || a.reason as string || '', 50);
    case 'send_message':
      return compactText(a.content as string || '', 60);
    case 'speak':
      return compactText(a.text as string || '', 50);
    case 'generate_lyrics':
    case 'generate_music':
    case 'generate_image':
      return compactText(a.prompt as string || '', 50);
    case 'set_tick_interval':
      return a.seconds ? `${a.seconds}s · ttl ${a.ttl || 10}` : '';
    case 'ui_set':
      return compactText(String(a.id || (a.surface as { kind?: string })?.kind || ''), 30);
    case 'focus_banner':
      return a.action
        ? `${a.action}${a.task ? ' · ' + compactText(a.task as string, 30) : ''}`
        : '';
    case 'set_task':
    case 'complete_task':
    case 'update_task_step':
      return compactText(a.description as string || a.step as string || a.note as string || '', 50);
    case 'schedule_reminder':
    case 'manage_reminder':
      return compactText(a.content as string || a.action as string || '', 50);
    case 'set_location':
      return compactText(a.location as string || a.city as string || '', 40);
    case 'set_agent_name':
      return compactText(a.name as string || '', 30);
    case 'delegate_to_agent':
    case 'grant_agent_delegation':
      return compactText(a.agent_id as string || a.target_id as string || '', 30);
    case 'install_tool':
    case 'uninstall_tool':
      return compactText(a.tool_name as string || a.name as string || '', 40);
    case 'install_software':
      return compactText(a.software as string || a.brew_name as string || a.url as string || '', 50);
    case 'music':
      return compactText(a.title as string || a.action as string || '', 40);
    case 'media_mode':
    case 'hotspot_mode':
    case 'person_card_mode':
      return compactText(a.mode as string || a.action as string || '', 30);
    default:
      return '';
  }
}

// ─── 工具详情总调度 ───

export function formatToolDetail(
  name: string,
  args: Record<string, unknown>,
  result: string | null | undefined,
  toolDetailLength = 160
): string {
  const parsed = parseJsonResult(result);
  const parsedTool = parsed?.tool as string | undefined;

  // Web tools 保留原人类化格式器
  if (parsedTool === 'web_search' || name === 'web_search') return formatWebSearchDetail((parsed || {}) as ToolPayload);
  if (parsedTool === 'fetch_url' || name === 'fetch_url') return formatFetchUrlDetail((parsed || {}) as ToolPayload);
  if (parsedTool === 'browser_read' || name === 'browser_read') return formatBrowserReadDetail((parsed || {}) as ToolPayload);

  // 通用 permission denied
  if (parsed?.ok === false && parsed.error === 'permission denied') {
    return formatGenericPermissionDenied(parsed as ToolPayload);
  }

  if (name === 'exec_command') {
    if (parsed) return formatExecCommandDetail(parsed as ToolPayload);
    return '命令已执行（结果过长未展开）。';
  }

  if (name === 'search_memory') {
    return formatSearchMemoryDetail((parsed || {}) as ToolPayload);
  }

  if (name === 'read_file') {
    return formatFileReadDetail(result);
  }

  if (name === 'write_file' || name === 'delete_file' || name === 'make_dir') {
    if (parsed?.ok === false) return compactText((parsed.error as string) || '操作失败', 160);
    const raw = String(result || '').trim();
    if (raw.startsWith('错误')) return compactText(raw, 160);
    return '';
  }

  if (name === 'list_dir') {
    if (parsed?.ok === false) return compactText((parsed.error as string) || '查看失败', 160);
    const items = Array.isArray(parsed?.entries)
      ? parsed.entries
      : Array.isArray(parsed?.items)
        ? parsed.items
        : Array.isArray(parsed?.files)
          ? parsed.files
          : null;
    if (items) {
      if (items.length === 0) return '（空目录）';
      const sample = items
        .slice(0, 6)
        .map((it) => (typeof it === 'string' ? it : it.name || ''))
        .filter(Boolean)
        .join(' · ');
      return `${items.length} 项：${compactText(sample, 160)}`;
    }
    return '';
  }

  if (name === 'send_message') {
    if (parsed?.ok === false) return compactText((parsed.error as string) || '发送失败', 160);
    return '';
  }

  return formatGenericOkDetail(parsed, result, toolDetailLength);
}

// ─── 工具标签（icon + 中文名） ───

export function toolLabel(name: string): string {
  const zh = TOOL_ZH[name] || name;
  const icon = TOOL_ICON[name] || '🔧';
  return `${icon} ${zh}`;
}
