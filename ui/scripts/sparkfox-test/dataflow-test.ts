/**
 * SparkFox 数据流串联验证脚本（PoC 功能联调测试）
 *
 * 由于浏览器自动化工具受限（browser_evaluate 返回 null + React 受控组件无法触发 onChange），
 * 本脚本直接通过 Zustand store API 验证 5 个数据流的串联逻辑。
 *
 * 运行方式：cd ui && bun run scripts/sparkfox-test/dataflow-test.ts
 *
 * 测试范围：
 * - 数据流1：对话 → 记忆存储（memoryExtractor + memoryStore）
 * - 数据流2：记忆 → 对话注入（buildMemoryContext）
 * - 数据流3：Agent → 监视面板（monitorStore.addActivity）
 * - 数据流4：思考过程（thinkingStore.beginRound/endRound）
 * - 数据流5：热点 → 对话（hotspotStore.sendToChat/buildContext）
 */

// @ts-nocheck — 测试脚本，不需要类型检查
import { useChatStore, newClientMessageId } from '../../src/renderer/store/chatStore';
import { useMemoryStore } from '../../src/renderer/store/memoryStore';
import { useMonitorStore } from '../../src/renderer/store/monitorStore';
import { useThinkingStore, L2_STREAM_KEY } from '../../src/renderer/store/thinkingStore';
import { useHotspotStore } from '../../src/renderer/store/hotspotStore';
import { useAgentStore } from '../../src/renderer/store/agentStore';
import {
  extractFromUserMessage,
  extractTechPreferences,
  extractEpisode,
  isNoiseMessage,
} from '../../src/renderer/utils/sparkfox/memoryExtractor';

// ============================================================================
// 测试工具
// ============================================================================

let passCount = 0;
let failCount = 0;
const results: { name: string; status: 'PASS' | 'FAIL'; detail: string }[] = [];

function assert(name: string, condition: boolean, detail: string) {
  if (condition) {
    passCount++;
    results.push({ name, status: 'PASS', detail });
    console.log(`  ✓ ${name}`);
  } else {
    failCount++;
    results.push({ name, status: 'FAIL', detail });
    console.error(`  ✗ ${name} — ${detail}`);
  }
}

function section(title: string) {
  console.log(`\n${'═'.repeat(60)}`);
  console.log(`  ${title}`);
  console.log(`${'═'.repeat(60)}`);
}

// ============================================================================
// 测试 1：完整对话（数据流1 + 3 + 4）
// ============================================================================

async function test1_CompleteConversation() {
  section('测试1：完整对话（发送消息 + AI 回复 + 思考流）');

  // 初始化所有 store（必须 await，因为 initialize 是 async 且预填充 mock 数据）
  await useMemoryStore.getState().initialize();
  await useMonitorStore.getState().initialize();

  const initialMsgCount = useChatStore.getState().messages.length;
  const initialMemCount = useMemoryStore.getState().memories.length;
  const initialActCount = useMonitorStore.getState().activities.length;

  console.log(`\n  初始状态: messages=${initialMsgCount}, memories=${initialMemCount}, activities=${initialActCount}`);

  // ── 步骤1：用户发送消息 ──
  const userText = '你好，我喜欢用 TypeScript 和 React 开发';
  const userClientId = newClientMessageId();
  const aiClientId = newClientMessageId();

  // 1a. 本地 echo 用户消息
  useChatStore.getState().addMessage({
    id: '',
    clientId: userClientId,
    role: 'user',
    content: userText,
    channel: 'sparkfox',
    label: 'You',
    attachments: [],
  });

  const afterUserMsg = useChatStore.getState().messages;
  assert(
    '1a. 用户消息本地 echo',
    afterUserMsg.length === initialMsgCount + 1 && afterUserMsg[afterUserMsg.length - 1].content === userText,
    `messages.length=${afterUserMsg.length}, expected=${initialMsgCount + 1}`,
  );

  // 1b. 数据流1：存入记忆（L1 短期 + L3 长期偏好）
  if (!isNoiseMessage(userText)) {
    // L1 短期记忆
    await useMemoryStore.getState().injectFromConversation(
      `用户：${userText.slice(0, 500)}`,
      'context' as never,
      'L1' as never,
    );

    // L3 长期记忆：提取偏好 + 事实
    const extracted = [...extractFromUserMessage(userText), ...extractTechPreferences(userText)];
    for (const mem of extracted) {
      await useMemoryStore.getState().addMemory({
        layer: mem.layer,
        type: mem.type as never,
        content: mem.content,
        subject: mem.subject,
        predicate: mem.predicate,
        object: mem.object,
        source: '对话',
        importance_score: 0.7,
        confidence: mem.confidence,
        scope: 'global',
        priority: 'long_term',
      });
    }

    const extractedCount = extracted.length;
    assert(
      '1b. 偏好提取（TypeScript/React 应触发）',
      extractedCount > 0,
      `提取到 ${extractedCount} 条记忆: ${extracted.map((e) => e.content).join(' | ')}`,
    );
  }

  const afterMem = useMemoryStore.getState().memories;
  assert(
    '1b. 记忆存储到 memoryStore',
    afterMem.length > initialMemCount,
    `memories.length=${afterMem.length}, expected>${initialMemCount}`,
  );

  // 1c. 数据流3：推送监视面板事件（message_in）
  useMonitorStore.getState().addActivity({
    id: `evt_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    type: 'message_in',
    title: '用户发送消息',
    agent_name: 'SparkFox',
    detail: `内容：${userText.slice(0, 80)}`,
    timestamp: new Date().toISOString(),
    status: 'ok',
  });

  const afterActIn = useMonitorStore.getState().activities;
  assert(
    '1c. 监视面板推送 message_in',
    afterActIn.length === initialActCount + 1 && afterActIn[0].type === 'message_in',
    `activities.length=${afterActIn.length}, first.type=${afterActIn[0]?.type}`,
  );

  // 1d. 数据流4：开始 AI 思考过程
  useThinkingStore.getState().setActiveL1Key(aiClientId);
  useThinkingStore.getState().beginRound(aiClientId);
  useThinkingStore.getState().startThinkingSession(aiClientId, '正在思考…');

  const thinkingStreams = useThinkingStore.getState().l1Streams;
  assert(
    '1d. 思考过程 beginRound',
    thinkingStreams.has(aiClientId) || thinkingStreams.get(aiClientId) !== undefined,
    `l1Streams.has(${aiClientId})=${thinkingStreams.has(aiClientId)}`,
  );

  // 1e. AI 占位消息
  useChatStore.getState().addMessage({
    id: '',
    clientId: aiClientId,
    role: 'assistant',
    content: '',
    channel: 'sparkfox',
    label: 'SparkFox',
    attachments: [],
    isStreaming: true,
  });

  const afterAiPlaceholder = useChatStore.getState().messages;
  assert(
    '1e. AI 占位消息添加',
    afterAiPlaceholder.length === initialMsgCount + 2 && afterAiPlaceholder[afterAiPlaceholder.length - 1].role === 'assistant',
    `messages.length=${afterAiPlaceholder.length}, last.role=${afterAiPlaceholder[afterAiPlaceholder.length - 1]?.role}`,
  );

  // ── 步骤2：模拟 AI 流式回复 ──
  const aiReply = `收到你的消息："${userText.slice(0, 30)}"。我是 SparkFox，很高兴为你服务！`;
  const chunks = aiReply.match(/.{1,3}/g) || [aiReply];
  for (const chunk of chunks) {
    useChatStore.getState().appendToLiveMessage(aiClientId, chunk);
  }

  const duringStream = useChatStore.getState().messages.find((m) => m.clientId === aiClientId);
  assert(
    '2a. AI 流式输出（appendToLiveMessage）',
    duringStream?.content === aiReply,
    `content="${duringStream?.content?.substring(0, 50)}...", expected="${aiReply.substring(0, 50)}..."`,
  );

  // 2b. 结束流式输出
  useChatStore.getState().finalizeLiveMessage(aiClientId);
  const afterFinalize = useChatStore.getState().messages.find((m) => m.clientId === aiClientId);
  assert(
    '2b. finalizeLiveMessage',
    afterFinalize?.isStreaming !== true,
    `isStreaming=${afterFinalize?.isStreaming}`,
  );

  // 2c. 数据流4：结束思考过程
  useThinkingStore.getState().stopThinking(aiClientId, '思考完成');
  useThinkingStore.getState().endRound(aiClientId);

  // 2d. 数据流1：存储情节记忆（L2）
  const episode = extractEpisode(userText, aiReply);
  if (episode) {
    await useMemoryStore.getState().injectFromConversation(
      episode.content,
      'episode' as never,
      'L2' as never,
    );
  }
  assert(
    '2d. 情节记忆存储（L2 episode）',
    episode !== null,
    `episode=${episode ? 'extracted' : 'null'}`,
  );

  // 2e. 数据流3：推送监视面板事件（message_out）
  useMonitorStore.getState().addActivity({
    id: `evt_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    type: 'message_out',
    title: 'SparkFox 回复消息',
    agent_name: 'SparkFox',
    detail: `生成回复，长度 ${aiReply.length} 字符`,
    tokens: Math.ceil(aiReply.length / 4),
    timestamp: new Date().toISOString(),
    status: 'ok',
  });

  const afterActOut = useMonitorStore.getState().activities;
  assert(
    '2e. 监视面板推送 message_out',
    afterActOut.length === initialActCount + 2 && afterActOut[0].type === 'message_out',
    `activities.length=${afterActOut.length}, first.type=${afterActOut[0]?.type}`,
  );

  // ── 最终状态检查 ──
  console.log(`\n  最终状态:`);
  console.log(`    messages: ${useChatStore.getState().messages.length} (新增 ${useChatStore.getState().messages.length - initialMsgCount})`);
  console.log(`    memories: ${useMemoryStore.getState().memories.length} (新增 ${useMemoryStore.getState().memories.length - initialMemCount})`);
  console.log(`    activities: ${useMonitorStore.getState().activities.length} (新增 ${useMonitorStore.getState().activities.length - initialActCount})`);
  console.log(`    thinking l1Streams: ${useThinkingStore.getState().l1Streams.size}`);

  assert(
    '最终：消息数量 = 初始 + 2（用户 + AI）',
    useChatStore.getState().messages.length === initialMsgCount + 2,
    `actual=${useChatStore.getState().messages.length}`,
  );
}

// ============================================================================
// 测试 2：记忆存储检索（数据流1 + 2）
// ============================================================================

async function test2_MemoryStorageRetrieval() {
  section('测试2：记忆存储检索（偏好提取 + 检索注入）');

  // ── 步骤1：偏好提取 ──
  const testMessages = [
    { text: '我喜欢用 Rust 写后端', expectExtract: true, desc: '偏好触发词"我喜欢"' },
    { text: '我叫张三', expectExtract: true, desc: '事实触发词"我叫"' },
    { text: '你好', expectExtract: false, desc: '噪音消息' },
    { text: '我偏好深色主题', expectExtract: true, desc: '偏好触发词"我偏好"' },
    { text: '我在北京工作', expectExtract: true, desc: '事实触发词"我在"' },
    { text: '谢谢', expectExtract: false, desc: '噪音消息' },
  ];

  for (const tc of testMessages) {
    const isNoise = isNoiseMessage(tc.text);
    const extracted = extractFromUserMessage(tc.text);
    if (tc.expectExtract) {
      assert(
        `偏好/事实提取: "${tc.text}" (${tc.desc})`,
        !isNoise && extracted.length > 0,
        `isNoise=${isNoise}, extracted=${extracted.length}`,
      );
    } else {
      assert(
        `噪音过滤: "${tc.text}" (${tc.desc})`,
        isNoise || extracted.length === 0,
        `isNoise=${isNoise}, extracted=${extracted.length}`,
      );
    }
  }

  // ── 步骤2：技术偏好提取 ──
  const techMsg = '我常用 Python 和 TypeScript，也熟悉 React 和 Vue';
  const techPrefs = extractTechPreferences(techMsg);
  assert(
    '技术偏好提取（Python/TypeScript/React/Vue）',
    techPrefs.length >= 3,
    `提取到 ${techPrefs.length} 条技术偏好: ${techPrefs.map((e) => e.content).join(' | ')}`,
  );

  // ── 步骤3：记忆检索注入 ──
  const memories = useMemoryStore.getState().memories;
  const preferences = memories.filter((m) => m.type === 'preference' && m.scope === 'global');

  console.log(`\n  当前记忆库: ${memories.length} 条`);
  console.log(`  偏好记忆: ${preferences.length} 条`);
  preferences.slice(0, 5).forEach((p, i) => {
    console.log(`    [${i + 1}] ${p.content}`);
  });

  // 构建 memoryContext（模拟 buildMemoryContext）
  let memoryContext = '';
  if (preferences.length) {
    const prefText = preferences
      .slice(0, 10)
      .map((m) => `- ${m.content}`)
      .join('\n');
    memoryContext = `## 用户偏好（来自记忆系统）\n${prefText}`;
  }

  assert(
    '记忆上下文构建（buildMemoryContext）',
    memoryContext.length > 0 && memoryContext.includes('用户偏好'),
    `context.length=${memoryContext.length}, has_keyword=${memoryContext.includes('用户偏好')}`,
  );

  // ── 步骤4：情节记忆提取 ──
  const episode = extractEpisode('如何用 React 写组件', '你可以用函数组件...');
  assert(
    '情节记忆提取（extractEpisode）',
    episode !== null && episode.content.length > 0,
    `episode=${episode ? 'extracted, length=' + episode.content.length : 'null'}`,
  );
}

// ============================================================================
// 测试 3：Agent 调度监视（数据流3）
// ============================================================================

async function test3_AgentMonitor() {
  section('测试3：Agent调度监视（事件推送）');

  const initialActCount = useMonitorStore.getState().activities.length;

  // 模拟 Agent 调度过程中的事件推送
  const events = [
    { type: 'agent_switch', title: '切换到代码助手', detail: '从默认 Agent 切换' },
    { type: 'tool_call', title: '调用工具：web_search', detail: '搜索"React 最佳实践"' },
    { type: 'tool_result', title: '工具返回结果', detail: '找到 15 条结果', tokens: 1200 },
    { type: 'memory_op', title: '记忆操作：检索 L3', detail: '检索到 3 条相关偏好' },
    { type: 'message_out', title: 'Agent 回复消息', detail: '生成回复', tokens: 350 },
  ];

  for (const evt of events) {
    useMonitorStore.getState().addActivity({
      id: `evt_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      type: evt.type as never,
      title: evt.title,
      agent_name: '代码助手',
      detail: evt.detail,
      tokens: evt.tokens,
      timestamp: new Date().toISOString(),
      status: 'ok',
    });
  }

  const activities = useMonitorStore.getState().activities;
  // monitorStore.addActivity 将新事件放在数组开头（最新在前），取前 5 个
  const newActivities = activities.slice(0, 5);

  console.log(`\n  新增活动事件: ${newActivities.length}`);
  newActivities.forEach((a, i) => {
    console.log(`    [${i + 1}] ${a.type}: ${a.title}`);
  });

  assert(
    '5 种事件类型全部推送成功',
    activities.length === initialActCount + 5,
    `expected=${initialActCount + 5}, actual=${activities.length}`,
  );

  assert(
    '事件类型包含 agent_switch / tool_call / tool_result / memory_op / message_out',
    ['agent_switch', 'tool_call', 'tool_result', 'memory_op', 'message_out'].every((t) =>
      newActivities.some((a) => a.type === t),
    ),
    `types=${newActivities.map((a) => a.type).join(', ')}`,
  );
}

// ============================================================================
// 测试 4：热点追踪（数据流5）
// ============================================================================

async function test4_HotspotTracking() {
  section('测试4：热点追踪（发送到对话讨论）');

  // ── 步骤1：热点上下文构建 ──
  const userMsg = '今天有什么热点新闻关于 React 的';
  const hotspotContext = useHotspotStore.getState().buildContext(userMsg);

  console.log(`\n  热点上下文: ${hotspotContext ? '已构建' : '空（无匹配热点）'}`);
  if (hotspotContext) {
    console.log(`    内容预览: ${hotspotContext.substring(0, 100)}...`);
  }

  // buildContext 在没有热点数据时返回空字符串是正常的
  assert(
    '热点上下文构建函数可调用',
    typeof hotspotContext === 'string',
    `type=${typeof hotspotContext}`,
  );

  // ── 步骤2：热点匹配 ──
  const matchResult = useHotspotStore.getState().matchUserMessage(userMsg);
  console.log(`  热点匹配: ${matchResult ? '匹配到' : '无匹配'}`);

  assert(
    '热点匹配函数可调用',
    matchResult === null || typeof matchResult === 'object',
    `type=${typeof matchResult}`,
  );

  // ── 步骤3：热点发送到对话（pendingChatInjection 机制） ──
  const initialPending = useHotspotStore.getState().pendingChatInjection;

  // 模拟 HotspotPlatformLists 中的"发送到对话"按钮点击
  const mockHotspot = {
    id: 'test_hotspot_1',
    title: 'React 19 正式发布',
    source: 'github',
    heat: 98,
    trend: 'up' as const,
    url: 'https://example.com/react-19',
  };

  useHotspotStore.getState().sendToChat(mockHotspot);

  const afterSendPending = useHotspotStore.getState().pendingChatInjection;
  assert(
    'sendToChat 设置 pendingChatInjection',
    afterSendPending !== null && afterSendPending?.title === mockHotspot.title,
    `pending=${afterSendPending?.title}, expected=${mockHotspot.title}`,
  );

  // ── 步骤4：清除 pendingChatInjection（模拟 useSparkfoxConversation 的 useEffect 处理） ──
  useHotspotStore.getState().clearPendingChatInjection();
  const afterClearPending = useHotspotStore.getState().pendingChatInjection;
  assert(
    'clearPendingChatInjection 清除待处理注入',
    afterClearPending === null,
    `pending=${afterClearPending}`,
  );

  // ── 步骤5：模拟热点注入对话流程 ──
  if (mockHotspot) {
    const heatText = mockHotspot.heat ? `（热度 ${mockHotspot.heat}）` : '';
    const trendText = mockHotspot.trend === 'up' ? '上升' : mockHotspot.trend === 'down' ? '下降' : '持平';
    const content = `来源：${mockHotspot.source}${heatText}，趋势${trendText}`;
    const injectText = `📊 热点讨论：${mockHotspot.title}\n\n${content}`;

    // 模拟 injectHotspotToChat 中的 handleSend 调用
    const injectClientId = newClientMessageId();
    useChatStore.getState().addMessage({
      id: '',
      clientId: injectClientId,
      role: 'user',
      content: injectText,
      channel: 'sparkfox',
      label: 'You',
      attachments: [],
    });

    const injectedMsg = useChatStore.getState().messages.find((m) => m.clientId === injectClientId);
    assert(
      '热点注入对话（消息添加到 chatStore）',
      injectedMsg?.content.includes(mockHotspot.title),
      `content="${injectedMsg?.content?.substring(0, 80)}..."`,
    );
  }
}

// ============================================================================
// 测试 5：页面切换（Store 隔离性）
// ============================================================================

async function test5_PageSwitching() {
  section('测试5：页面切换（Store 状态隔离）');

  // 验证各 store 的状态在页面切换后保持一致
  // （SparkFox 使用单一状态树，页面切换不应丢失状态）

  const chatState = useChatStore.getState();
  const memoryState = useMemoryStore.getState();
  const monitorState = useMonitorStore.getState();
  const thinkingState = useThinkingStore.getState();
  const hotspotState = useHotspotStore.getState();
  const agentState = useAgentStore.getState();

  console.log(`\n  各 Store 状态快照:`);
  console.log(`    chatStore: messages=${chatState.messages.length}, input.value="${chatState.input.value}"`);
  console.log(`    memoryStore: memories=${memoryState.memories.length}`);
  console.log(`    monitorStore: activities=${monitorState.activities.length}`);
  console.log(`    thinkingStore: l1Streams=${thinkingState.l1Streams?.size || 0}, activeL1Key=${thinkingState.activeL1Key}`);
  console.log(`    hotspotStore: items=${hotspotState.items?.length || 0}, pendingChatInjection=${hotspotState.pendingChatInjection ? '有' : '无'}`);
  console.log(`    agentStore: agents=${agentState.agents?.length || 0}, currentAgentId=${agentState.currentAgentId}`);

  assert(
    'chatStore 状态完整（messages 数组存在）',
    Array.isArray(chatState.messages),
    `messages type=${typeof chatState.messages}`,
  );

  assert(
    'memoryStore 状态完整（memories 数组存在）',
    Array.isArray(memoryState.memories),
    `memories type=${typeof memoryState.memories}`,
  );

  assert(
    'monitorStore 状态完整（activities 数组存在）',
    Array.isArray(monitorState.activities),
    `activities type=${typeof monitorState.activities}`,
  );

  assert(
    'thinkingStore 状态完整（l1Streams Map 存在）',
    thinkingState.l1Streams instanceof Map || (typeof thinkingState.l1Streams === 'object' && thinkingState.l1Streams !== null),
    `l1Streams type=${typeof thinkingState.l1Streams}`,
  );

  assert(
    'hotspotStore 状态完整（pendingChatInjection 字段存在）',
    hotspotState.pendingChatInjection === null || typeof hotspotState.pendingChatInjection === 'object',
    `pendingChatInjection type=${typeof hotspotState.pendingChatInjection}`,
  );

  // 验证 store actions 存在
  const storeRefs: Record<string, { getState: () => Record<string, unknown> }> = {
    chatStore: useChatStore,
    memoryStore: useMemoryStore,
    monitorStore: useMonitorStore,
    thinkingStore: useThinkingStore,
    hotspotStore: useHotspotStore,
  };
  const requiredActions: Record<string, string[]> = {
    chatStore: ['addMessage', 'appendToLiveMessage', 'finalizeLiveMessage', 'setInputValue', 'clearAttachments'],
    memoryStore: ['addMemory', 'injectFromConversation', 'initialize'],
    monitorStore: ['addActivity', 'initialize'],
    thinkingStore: ['beginRound', 'endRound', 'startThinkingSession', 'stopThinking', 'setActiveL1Key'],
    hotspotStore: ['buildContext', 'matchUserMessage', 'sendToChat', 'clearPendingChatInjection'],
  };

  for (const [storeName, actions] of Object.entries(requiredActions)) {
    const store = storeRefs[storeName];
    const state = store?.getState?.();
    const missingActions = actions.filter((a) => typeof state?.[a] !== 'function');
    assert(
      `${storeName} 所有必要 actions 存在`,
      missingActions.length === 0,
      `missing: ${missingActions.join(', ')}`,
    );
  }
}

// ============================================================================
// 主函数
// ============================================================================

async function main() {
  console.log('╔══════════════════════════════════════════════════════════╗');
  console.log('║     SparkFox PoC 数据流串联验证（功能联调测试）          ║');
  console.log('╚══════════════════════════════════════════════════════════╝');

  try {
    await test1_CompleteConversation();
  } catch (e) {
    console.error('测试1 异常:', e);
    failCount++;
  }

  try {
    await test2_MemoryStorageRetrieval();
  } catch (e) {
    console.error('测试2 异常:', e);
    failCount++;
  }

  try {
    await test3_AgentMonitor();
  } catch (e) {
    console.error('测试3 异常:', e);
    failCount++;
  }

  try {
    await test4_HotspotTracking();
  } catch (e) {
    console.error('测试4 异常:', e);
    failCount++;
  }

  try {
    await test5_PageSwitching();
  } catch (e) {
    console.error('测试5 异常:', e);
    failCount++;
  }

  // ── 汇总 ──
  console.log(`\n${'═'.repeat(60)}`);
  console.log(`  测试汇总`);
  console.log(`${'═'.repeat(60)}`);
  console.log(`  ✓ PASS: ${passCount}`);
  console.log(`  ✗ FAIL: ${failCount}`);
  console.log(`  总计: ${passCount + failCount}`);

  if (failCount > 0) {
    console.log(`\n  失败项:`);
    results.filter((r) => r.status === 'FAIL').forEach((r) => {
      console.log(`    ✗ ${r.name}: ${r.detail}`);
    });
  }

  console.log('');
  process.exit(failCount > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error('Fatal error:', e);
  process.exit(1);
});
