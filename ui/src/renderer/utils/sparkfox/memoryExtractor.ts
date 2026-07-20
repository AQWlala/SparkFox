/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox 对话记忆提取工具
 *
 * 来源：SparkFox 原创设计（基于 OpenAkita extractor.py 思路清洁室重写）
 *
 * 功能：
 * - 从用户消息中提取偏好（"我喜欢/不喜欢/擅长/用xx"）
 * - 从用户消息中提取事实（"我叫xx/我在xx公司"）
 * - 从对话中提取关键信息（长文本摘要）
 * - 判断消息是否值得记忆（避免无意义消息入库）
 *
 * PoC 策略：基于关键词模式匹配（生产环境应改为 LLM 提取）
 */

// ============================================================================
// 关键词模式（PoC：规则匹配）
// ============================================================================

/** 偏好触发词 */
const PREFERENCE_TRIGGERS = [
  '我喜欢', '我偏好', '我习惯', '我擅长', '我爱用', '我常用',
  '我不喜欢', '我讨厌', '我反感', '我避免',
  '我喜欢用', '我习惯用', '我倾向于', '我偏好用',
  '我擅长', '我精通', '我熟悉',
];

/** 事实触发词 */
const FACT_TRIGGERS = [
  '我叫', '我的名字', '我是', '我在', '我住', '我工作',
  '我的公司', '我的团队', '我的项目', '我的工作',
  '我正在', '我准备', '我打算', '我计划',
];

/** 无意义消息过滤（不存入长期记忆） */
const NOISE_PATTERNS = [
  /^(你好|嗨|hi|hello|hey|在吗|在不在|？|\?)+$/i,
  /^(谢谢|感谢|thx|thanks|多谢)+$/i,
  /^(好的|嗯|ok|okay|好|行|可以)+$/i,
  /^(再见|拜拜|bye|88)+$/i,
  /^[\s\S]{0,3}$/, // 少于 3 字符
];

// ============================================================================
// 类型定义
// ============================================================================

export interface ExtractedMemory {
  /** 记忆类型 */
  type: 'preference' | 'fact' | 'episode';
  /** 记忆层级 */
  layer: 'L1' | 'L2' | 'L3';
  /** 提取的内容 */
  content: string;
  /** 主体（L3 实体-属性结构用） */
  subject?: string;
  /** 谓词 */
  predicate?: string;
  /** 客体 */
  object?: string;
  /** 置信度 0-1 */
  confidence: number;
}

// ============================================================================
// 提取函数
// ============================================================================

/** 判断消息是否为噪音（不值得记忆） */
export function isNoiseMessage(message: string): boolean {
  const trimmed = message.trim();
  if (!trimmed) return true;
  return NOISE_PATTERNS.some((p) => p.test(trimmed));
}

/** 从用户消息中提取偏好 */
export function extractPreferences(message: string): ExtractedMemory[] {
  const results: ExtractedMemory[] = [];
  const trimmed = message.trim();

  for (const trigger of PREFERENCE_TRIGGERS) {
    const idx = trimmed.indexOf(trigger);
    if (idx === -1) continue;

    // 提取触发词后到句号/逗号/换行前的内容
    const after = trimmed.slice(idx + trigger.length);
    const endMatch = after.match(/[。，,；;\n。]/);
    const content = endMatch ? after.slice(0, endMatch.index) : after;

    if (content && content.length >= 2 && content.length <= 80) {
      const isNegative = trigger.includes('不') || trigger.includes('讨厌') || trigger.includes('反感') || trigger.includes('避免');
      results.push({
        type: 'preference',
        layer: 'L3',
        content: `${isNegative ? '不喜欢' : '喜欢'}${content}`,
        subject: '用户',
        predicate: isNegative ? 'dislikes' : 'likes',
        object: content,
        confidence: 0.85,
      });
    }
  }

  return results;
}

/** 从用户消息中提取事实 */
export function extractFacts(message: string): ExtractedMemory[] {
  const results: ExtractedMemory[] = [];
  const trimmed = message.trim();

  for (const trigger of FACT_TRIGGERS) {
    const idx = trimmed.indexOf(trigger);
    if (idx === -1) continue;

    const after = trimmed.slice(idx + trigger.length);
    const endMatch = after.match(/[。，,；;\n。]/);
    const content = endMatch ? after.slice(0, endMatch.index) : after;

    if (content && content.length >= 2 && content.length <= 80) {
      // 避免重复匹配"我是"触发词下的无意义内容
      if (/^(一个|那个|这|那|谁|什么|怎么)/.test(content)) continue;

      results.push({
        type: 'fact',
        layer: 'L3',
        content: `${trigger.replace('我', '用户')}${content}`,
        subject: '用户',
        predicate: trigger.includes('叫') || trigger.includes('名字')
          ? 'name'
          : trigger.includes('工作') || trigger.includes('公司')
          ? 'organization'
          : trigger.includes('住')
          ? 'location'
          : 'attribute',
        object: content,
        confidence: 0.8,
      });
    }
  }

  return results;
}

/** 从完整对话（用户问 + AI 答）中提取情节记忆 */
export function extractEpisode(
  userMessage: string,
  aiReply: string,
): ExtractedMemory | null {
  if (!userMessage.trim() || !aiReply.trim()) return null;

  // 截断过长的内容
  const userPreview = userMessage.length > 100 ? `${userMessage.slice(0, 100)}…` : userMessage;
  const aiPreview = aiReply.length > 200 ? `${aiReply.slice(0, 200)}…` : aiReply;

  return {
    type: 'episode',
    layer: 'L2',
    content: `用户问：${userPreview}\nSparkFox 答：${aiPreview}`,
    confidence: 0.7,
  };
}

/** 完整提取：从单条用户消息中提取所有值得记忆的信息 */
export function extractFromUserMessage(message: string): ExtractedMemory[] {
  if (isNoiseMessage(message)) return [];

  const memories: ExtractedMemory[] = [];

  // 1. 偏好
  memories.push(...extractPreferences(message));

  // 2. 事实
  memories.push(...extractFacts(message));

  return memories;
}

/** 检查消息是否包含编程语言/技术栈偏好（增强提取） */
export function extractTechPreferences(message: string): ExtractedMemory[] {
  const results: ExtractedMemory[] = [];
  const trimmed = message.toLowerCase();

  const techKeywords: Record<string, string> = {
    python: 'Python',
    javascript: 'JavaScript',
    typescript: 'TypeScript',
    rust: 'Rust',
    golang: 'Go',
    'go语言': 'Go',
    java: 'Java',
    'c++': 'C++',
    react: 'React',
    vue: 'Vue',
    angular: 'Angular',
    svelte: 'Svelte',
    node: 'Node.js',
    deno: 'Deno',
    bun: 'Bun',
    tauri: 'Tauri',
    electron: 'Electron',
    fastapi: 'FastAPI',
    django: 'Django',
    flask: 'Flask',
  };

  // 检测"用xx编程/开发/写"模式
  for (const [keyword, name] of Object.entries(techKeywords)) {
    if (trimmed.includes(keyword)) {
      // 检查是否在偏好上下文中
      const hasPreferenceContext = PREFERENCE_TRIGGERS.some((t) =>
        trimmed.includes(t.toLowerCase().replace('我', '')),
      ) || /用.{0,8}(编程|开发|写|做|实现)/.test(trimmed);

      if (hasPreferenceContext) {
        // 避免重复（如果 extractPreferences 已经抓到）
        const already = results.some((r) => r.object === name);
        if (!already) {
          results.push({
            type: 'preference',
            layer: 'L3',
            content: `使用 ${name}`,
            subject: '用户',
            predicate: 'uses',
            object: name,
            confidence: 0.75,
          });
        }
      }
    }
  }

  return results;
}
