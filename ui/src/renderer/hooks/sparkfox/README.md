# SparkFox Hooks

> AGPL-3.0-only · Copyright 2026 SparkFox Contributors

SparkFox 专用 React Hooks 目录。提供消息流 IPC 订阅、对话编排、AI 活动状态派生三类核心能力，是 UI 组件与 Zustand store 之间的"业务编排层"。

---

## 目录

- [用途说明](#用途说明)
- [Hooks 一览](#hooks-一览)
- [useAiActivity](#useaiaactivity)
- [useSparkfoxMessage](#usesparkfoxmessage)
- [useSparkfoxConversation](#usesparkfoxconversation)
- [5 大数据流](#5-大数据流)
- [与 Store 的关系](#与-store-的关系)
- [设计约束](#设计约束)

---

## 用途说明

SparkFox 的 React 组件层遵循 **"组件 → hook → store → IPC"** 的单向数据流：

- **组件层**：只负责渲染，不直接调用 store actions，也不直接订阅 IPC
- **Hook 层（本目录）**：编排多个 store 之间的联动，订阅 IPC 事件流，提供业务语义化 API
- **Store 层**（`@renderer/store/*`）：Zustand store，持有状态与原子 actions
- **IPC 层**（`@common/ipcBridge` / Tauri `invoke` / `listen`）：与 Rust 后端通信

本目录的 3 个 hook 是 SparkFox 核心业务逻辑的入口：

| Hook | 职责 | 调用方 |
| --- | --- | --- |
| `useAiActivity` | 派生 AI 活动状态（idle/busy + 工具分组） | AiActivityBadge、ChatPanel 状态栏 |
| `useSparkfoxMessage` | 订阅 IPC 消息流，把后端事件路由到 chatStore + thinkingStore | ChatPanel（消息收发） |
| `useSparkfoxConversation` | 编排 5 大数据流（记忆 / Agent / 监视 / 思考 / 热点） | ChatPanel 主流程 |

---

## Hooks 一览

```
hooks/sparkfox/
├── useAiActivity.ts             # AI 活动状态派生
├── useSparkfoxMessage.ts        # 消息流 IPC 订阅
├── useSparkfoxConversation.ts   # 对话编排器（5 大数据流）
└── README.md                    # 本文件
```

---

## useAiActivity

派生 AI 当前活动状态。基于 `thinkingStore.aiActivityLog` 的 60s 滑动窗口 + 15s 空闲阈值，输出语义化标签。

**来源**：BaiLongma `src/ui/brain-ui/app.js` 的 `recordAiActivity` / `refreshAiActivity`（清洁室重写）。

### 参数

无参数。

### 返回值

```ts
interface AiActivityState {
  state: 'idle' | 'busy';
  label: string;        // '空闲' | '刚完成' | '正在扫描文件' | '正在处理事务' ...
  detail: string;       // '· 3 次工具 · 12s' | '5s 前停止' | ''
  count: number;        // 60s 窗口内工具调用次数
  elapsedSec: number;   // 持续秒数
}
```

### 使用示例

```tsx
import { useAiActivity } from '@renderer/hooks/sparkfox/useAiActivity';

function AiStatusBar() {
  const activity = useAiActivity();
  return (
    <div className="ai-status">
      <span className="dot" data-busy={activity.state === 'busy'} />
      <span>{activity.label}</span>
      {activity.detail && <small>{activity.detail}</small>}
    </div>
  );
}
```

### 实现细节

- 每秒刷新一次（`setInterval(1000)`）以更新 `elapsedSec` 与空闲判定
- 通过 `useMemo` 派生，避免不必要的重渲染
- 内部调用 `deriveAiActivity(log, firstTs, now)`（定义在 `thinkingStore` 中）

---

## useSparkfoxMessage

订阅 SparkFox 消息流 IPC（`ipcBridge.conversation.responseStream`），把后端推送的 `IResponseMessage` 事件路由到 `chatStore` 与 `thinkingStore` 的 actions。

**来源**：NomiFun `useNomiMessage.ts` 仿写，结合 BaiLongma `ThoughtStream` 的 8 大方法。

### 参数

```ts
interface UseSparkfoxMessageOptions {
  /** 仅监听该会话的事件；不传则监听全部（PoC 阶段便于演示） */
  conversationId?: ConversationId | string | null;
  /** 错误回调 */
  onError?: (message: IResponseMessage) => void;
  /** 是否只读模式（不发送消息，仅接收） */
  readOnly?: boolean;
}
```

### 返回值

```ts
interface UseSparkfoxMessageResult {
  /** 发送消息到 IPC（POST /api/conversations/{id}/messages） */
  sendMessage: (text: string, attachments: ChatAttachment[]) => Promise<void>;
  /** 当前是否正在流式输出 */
  isStreaming: boolean;
}
```

### 事件路由表

| 后端事件 type | L1（用户消息流） | L2（Tick 心跳流） |
| --- | --- | --- |
| `start` | `addMessage(assistant)` + `beginRound` + `startThinkingSession` | `beginRound(L2)` + `addLine('Tick')` |
| `thought` | `setStatus('思考中: ' + subject)` | `recordAiActivity('tick_thought')` |
| `content` / `text` | `appendToLiveMessage(chunk)` | `addLine(L2, '内容', chunk)` |
| `tool_group` | `addToolCall` × N + `recordAiActivity` × N | 同左 |
| `finish` | `finalizeLiveMessage` + `stopThinking` + `endRound` | 同左（L2） |
| `turn_completed` | （仅指标，无 action） | 同左 |
| `error` | `setStatus('failed')` + `finalizeLiveMessage` + `endRound` | 同左（L2） |

### 使用示例

```tsx
import { useSparkfoxMessage } from '@renderer/hooks/sparkfox/useSparkfoxMessage';

function ChatPanel({ conversationId }: { conversationId: string }) {
  const { sendMessage, isStreaming } = useSparkfoxMessage({
    conversationId,
    onError: (msg) => console.error('SparkFox error:', msg),
  });

  const handleSend = async (text: string) => {
    if (isStreaming) return;
    await sendMessage(text, []);
  };

  return <MessageInput onSend={handleSend} disabled={isStreaming} />;
}
```

### 实现细节

- 订阅在 `useEffect` 中完成，`unsubscribe` 在卸载时调用
- 通过 `conversationId` 在回调内过滤，避免会话切换时重订阅
- `sendMessage` 失败时调用 `onError` 回调并解锁输入框
- `isStreaming` 派生自 `chatStore.liveMessageId !== null`

---

## useSparkfoxConversation

SparkFox 对话编排器 —— 统一管理 5 大数据流，是 ChatPanel 的主入口。

**核心职责**：在用户发送消息时，串联本地 echo、记忆存储、上下文构建、Agent 调度、思考过程、AI 回复 6 个步骤。

### 参数

```ts
interface SparkfoxConversationOptions {
  /** 是否启用记忆存储（默认 true） */
  enableMemoryStorage?: boolean;
  /** 是否启用记忆注入（默认 true） */
  enableMemoryInjection?: boolean;
  /** 是否启用思考过程记录（默认 true） */
  enableThinkingStream?: boolean;
  /** 是否启用监视面板事件推送（默认 true） */
  enableMonitorPush?: boolean;
}
```

### 返回值

```ts
{
  // 数据流1：对话 → 记忆
  handleSend: (text: string, attachments: ChatAttachment[]) => {
    userClientId: string;
    aiClientId: string;
    context: string;     // 注入上下文（记忆 + 热点）
  };
  handleAIReply: (userMessage: string, aiClientId: string, aiReply: string) => void;
  simulateAIReply: (userMessage: string, aiClientId: string) => void;  // PoC mock
  storeUserMessageToMemory: (userMessage: string) => void;
  storeEpisodeToMemory: (userMessage: string, aiReply: string) => void;

  // 数据流2：记忆 → 对话
  buildMemoryContext: (userMessage: string) => string;

  // 数据流3：Agent → 监视
  pushMonitorEvent: (params: { type?: ...; title: string; ... }) => void;

  // 数据流4：思考过程
  beginThinkingRound: (aiClientId: string) => void;
  endThinkingRound: (aiClientId: string) => void;

  // 数据流5：热点 → 对话
  injectHotspotToChat: (hotspotTitle: string, hotspotContent: string) => void;
  buildHotspotContext: (userMessage: string) => string;
}
```

### 使用示例

```tsx
import { useSparkfoxConversation } from '@renderer/hooks/sparkfox/useSparkfoxConversation';

function ChatPanel() {
  const {
    handleSend,
    handleAIReply,
    simulateAIReply,
  } = useSparkfoxConversation({
    enableMemoryStorage: true,
    enableMonitorPush: true,
  });

  const onSubmit = (text: string) => {
    const { aiClientId, context } = handleSend(text, []);
    // 把 context 传给 IPC 层；PoC 阶段直接 mock 回复
    simulateAIReply(text, aiClientId);
  };

  // IPC 层收到完整 AI 回复后调用
  const onAIReplyComplete = (userMsg: string, aiClientId: string, reply: string) => {
    handleAIReply(userMsg, aiClientId, reply);
  };

  return <MessageInput onSend={onSubmit} />;
}
```

### 实现细节

- 在 mount 时自动初始化 `memoryStore` + `monitorStore`
- 监听 `hotspotStore.pendingChatInjection`，自动调用 `injectHotspotToChat`
- `buildMemoryContext` 从 L3 语义记忆中检索偏好 / 事实 / 相关情节，构建注入提示
- `buildHotspotContext` 委托给 `hotspotStore.buildContext`
- `simulateAIReply` 是 PoC mock，生产环境由 `useSparkfoxMessage` 接管

---

## 5 大数据流

`useSparkfoxConversation` 串联的 5 条数据流：

```
┌─────────────────────────────────────────────────────────────────────┐
│                          ChatPanel (用户发送消息)                    │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
   ┌───────────────────────────────┼───────────────────────────────┐
   │                               │                               │
   ▼                               ▼                               ▼
┌─────────────┐             ┌──────────────┐              ┌──────────────┐
│ 数据流1     │             │ 数据流3      │              │ 数据流4      │
│ 对话→记忆   │             │ Agent→监视    │              │ 思考→对话    │
│             │             │              │              │              │
│ memoryStore │             │ monitorStore │              │ thinkingStore│
│ .injectFrom │             │ .addActivity │              │ .beginRound  │
│ Conversation│             │              │              │ .startThink  │
└──────┬──────┘             └──────────────┘              └──────────────┘
       │
       ▼
┌─────────────┐
│ 数据流2     │
│ 记忆→对话   │
│             │
│ buildMemory │
│ Context()   │
│             │
│ ↓ 注入到    │
│ system      │
│ prompt      │
└─────────────┘

       ┌──────────────────────────────────────┐
       │ 数据流5：热点→对话                    │
       │                                      │
       │ HotspotView ─→ hotspotStore          │
       │              .sendToChat(item)       │
       │              .pendingChatInjection    │
       │                   │                  │
       │                   ▼                  │
       │ useSparkfoxConversation 监听         │
       │ → injectHotspotToChat(title, desc)   │
       │ → handleSend(text, [])                │
       └──────────────────────────────────────┘
```

| # | 数据流 | 方向 | 起点 | 终点 | 涉及 Store |
| --- | --- | --- | --- | --- | --- |
| 1 | 对话 → 记忆 | 用户消息 → 记忆存储 | `handleSend` | `memoryStore.addMemory` / `injectFromConversation` | `memoryStore` |
| 2 | 记忆 → 对话 | 新对话开始 → 检索记忆注入上下文 | `buildMemoryContext` | IPC system prompt | `memoryStore` |
| 3 | Agent 调度 → 监视 | Agent 选择 → 执行 → 状态推送 | `pushMonitorEvent` | `monitorStore.addActivity` | `agentStore`, `monitorStore` |
| 4 | 思考过程 → 对话 | AI 响应 → 思考流 → ChatMessage | `beginThinkingRound` | `thinkingStore` + `chatStore` | `thinkingStore`, `chatStore` |
| 5 | 热点 → 对话 | HotspotView 点击 → 发送到对话页 | `hotspotStore.sendToChat` | `injectHotspotToChat` → `handleSend` | `hotspotStore`, `chatStore` |

---

## 与 Store 的关系

```
┌──────────────────────────────────────────────────────────────────────┐
│                          React 组件层                                │
│  ChatPanel / HotspotView / MemoryView / MonitorView / AgentManagerView│
└──────────────────────────────────┬───────────────────────────────────┘
                                   │ 调用
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│                       hooks/sparkfox（本目录）                       │
│  useAiActivity · useSparkfoxMessage · useSparkfoxConversation        │
└──────────────────────────────────┬───────────────────────────────────┘
                                   │ 读写
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│                         store/（Zustand）                            │
│  agentStore · chatStore · hotspotStore · memoryStore ·               │
│  monitorStore · sceneStore · settingsStore · thinkingStore           │
└──────────────────────────────────┬───────────────────────────────────┘
                                   │ invoke / listen
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│                  Tauri IPC（sparkfox-ipc crate）                     │
│  agent_list · memory_put/list · knowledge_search ·                  │
│  monitor_stats/ack · hotspot_track/list · thought_pushed (event)     │
└──────────────────────────────────────────────────────────────────────┘
```

每个 hook 的 store 依赖：

| Hook | 依赖的 Store |
| --- | --- |
| `useAiActivity` | `thinkingStore`（读 `aiActivityLog` / `aiActivityFirstTs`） |
| `useSparkfoxMessage` | `chatStore`（写消息流） + `thinkingStore`（写思考流） |
| `useSparkfoxConversation` | `chatStore` + `memoryStore` + `agentStore` + `monitorStore` + `hotspotStore` + `thinkingStore`（全部 6 个核心 store） |

### Store 的 IPC 对接（Task 7.8）

下列 store 已对接 Tauri IPC（渐进增强：失败时回退到 mock，不抛异常）：

| Store | 新增 Action | 对应 IPC |
| --- | --- | --- |
| `agentStore` | `syncFromBackend()` | `invoke('agent_list')` |
| `memoryStore` | `syncFromBackend(layer)` / `putToBackend(layer, entry)` | `invoke('memory_list')` / `invoke('memory_put')` |
| `chatStore` | `searchKnowledge(query, mode)` | `invoke('knowledge_search')` |
| `monitorStore` | `fetchStatsFromBackend(period)` / `ackEvent(eventId)` | `invoke('monitor_stats')` / `invoke('monitor_ack')` |
| `hotspotStore` | `trackFromBackend(keyword)` / `listFromBackend(limit)` | `invoke('hotspot_track')` / `invoke('hotspot_list')` |
| `thinkingStore` | `subscribeToBackendEvents()` / `unsubscribeFromBackendEvents()` | `listen('thought_pushed')` |
| `sceneStore` | （无，v1.1.0+ 计划） | — |
| `settingsStore` | （无，v1.1.0+ 计划） | — |

---

## 设计约束

1. **AGPL-3.0 合规**：BaiLongma / NomiFun 来源代码均经清洁室重写
2. **PoC 兼容**：所有 IPC 调用 try/catch，非 Tauri 环境静默回退到 mock
3. **单向数据流**：组件 → hook → store → IPC，不反向
4. **不破坏现有 API**：所有现有 actions 签名保持兼容，仅新增 IPC 对接 action
5. **中文注释**：所有新增代码使用中文注释
6. **幂等订阅**：`subscribeToBackendEvents` 可重复调用，内部去重

---

## 相关文档

- SparkFox v1.0.0 spec：`docs/SparkFox-v1.0.0-spec-1.0.md`
- Task 7.8（store IPC 对接）：spec L.2500-2529
- Task 7.9（本 README）：spec L.2533-2545
- Store 实现目录：`ui/src/renderer/store/`
- 后端 IPC 命令：`crates/sparkfox/sparkfox-ipc/src/commands.rs`
