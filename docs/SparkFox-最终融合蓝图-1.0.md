# SparkFox 最终融合蓝图 v1.0

> 生成时间：2026-07-18
> 基于文档：SparkFox-重组优化方案-1.0.md + RFC-001~005 + SparkFox-四项目深度分析与融合拆解报告.md
> 基于第 1 步（四项目技术栈分析）和第 2 步（深度功能拆解）的结论
> 项目硬约束：AGPL 合规 / Phase -1 PoC 前置 / automerge-rs CRDT / Double Ratchet E2EE / 6 层记忆 L0-L5 / DAG 编排 / 3-4 并行 / Apple 系统风格

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 总览
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

本蓝图把四项目（Pangu Nebula / NomiFun / OpenAkita / BaiLongma）的精华融合为 SparkFox 的最终设计：

- **NomiFun（28 项主导）**：前端 UI 框架、Arco Design + UnoCSS 样式、50 crate Rust workspace 组织方式、Tauri 2 进程内后端、Companion 桌宠、Titlebar/Sider/Content 三栏布局、ThemeContext + themeControlContract 主题契约
- **OpenAkita（22 项主导）**：Agent 菜单系统（AgentProfile 22 字段 + Sheet 侧滑编辑）、监视面板（TokenStatsView 6 周期 5 维度 + AgentDashboard 力导向图）、三层记忆体系（Scratchpad + Core+Dynamic + Persona+Identity）+ MemoryExtractor + MemoryConsolidator + LLM 审查 ReviewProgress、22 lazy views 路由模式、Pixel Office
- **BaiLongma（18 项主导）**：对话展示（liveEl 流式气泡 + 双重去重 + 激活预热锁 + 自适应高度）、思考过程可视化（ThoughtStream 57 工具中文映射 + 三区联动）、信息热点追踪（4 平台热榜 + HotspotEarth Three.js + 中性上下文构建）、Tick 心跳 + Scene Protocol、Thread 线索模型
- **Pangu Nebula（4 项保留）**：6 层记忆 L0-L5 蓝图（RFC-003 SoT）、蜂群编排（与 OpenAkita 组织编排合并为 DAG）、双引擎、11 安全栈

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第一部分：整体架构选型
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 决策 1：桌面框架

→ **选择：Tauri 2（进程内后端模式）**

→ **理由**：
1. **四项目中有三个（Pangu Nebula / NomiFun / OpenAkita）已采用 Tauri 2**，融合路径最短，迁移成本最低；只有 BaiLongma 用 Electron 33
2. **体积与内存优势决定性**：Tauri 2 安装包 <10MB、运行内存 50-80MB；Electron 33 安装包 80-150MB、运行内存 200-400MB。SparkFox 是长期运行的"数字意识框架"，内存占用直接影响 24/7 体验
3. **安全模型更严**：Tauri 2 默认无 Node.js 集成、能力（capability）系统细粒度授权，与 SparkFox 的 11 安全栈（RFC-安全）天然契合；Electron 的 Node 集成是历史包袱
4. **Rust 后端与 automerge-rs / Double Ratchet 原生集成**：CRDT 与 E2EE 都需要 Rust 生态，Tauri 2 进程内后端（axum on 127.0.0.1）模式（NomiFun 已验证）让 IPC 延迟 <1ms
5. **Apple 系统风格更纯粹**：Tauri 2 在 macOS 上使用 WKWebView，原生渲染管道；Electron 用 Chromium，字体渲染与系统有差异

→ **放弃**：Electron 33（BaiLongma 方案）、纯 Web（PWA）、Flutter、.NET MAUI

→ **放弃理由**：
- **Electron**：体积大、内存高、Node.js 集成与 AGPL 清洁室流程冲突（BaiLongma MIT 部分需重写）
- **纯 Web PWA**：无法访问本地文件系统、无法运行 Rust 后端、不能 24/7 常驻、无系统托盘
- **Flutter**：与四项目都无血缘，从零学习成本高；Dart 生态弱于 Rust
- **.NET MAUI**：仅 Windows 优势，跨平台弱；与四项目技术栈完全不兼容

→ **现有影响**：
1. **BaiLongma 全部 Electron 代码需清洁室重写**：`electron-builder` 配置 → `tauri.conf.json`；`BrowserWindow` → `WebviewWindow`；`ipcMain/ipcRenderer` → `ipcBridge`；`better-sqlite3` 同步调用 → `rusqlite` 异步
2. **NomiFun 的 Tauri 2 配置可直接迁移**：`apps/desktop/tauri.conf.json` 作为 SparkFox 桌面壳蓝本
3. **OpenAkita 的 Tauri Setup Center 模式放弃**：SparkFox 不需要 Setup Center 独立安装器，主应用即 Setup
4. **Pangu Nebula 的 Python sidecar 模式放弃**：改用 NomiFun 验证过的"进程内 axum 后端"模式，消除 sidecar 启动延迟

---

### 决策 2：前端框架

→ **选择：React 19.1 + TypeScript 5.x**

→ **理由**：
1. **NomiFun + OpenAkita 都用 React 19.1**，是四项目中占比最大的前端栈，融合成本最低
2. **React 19 的并发渲染（Suspense + useTransition）对实时流式对话+思考过程+热点推送三路并发场景至关重要**：BaiLongma 的 ThoughtStream 三区联动（thinkingEl/lastToolEl/statusEl）在 React 19 下可用 `useTransition` 平滑处理
3. **Arco Design 官方支持 React**：NomiFun 用的 `@arco-design/web-react` 是 React 专用库
4. **生态与人才池最大**：shadcn/ui（OpenAkita 用）、lucide-react、TanStack Query、React Router 7 等关键依赖都是 React 生态
5. **Tauri 2 官方 `@tauri-apps/api` 与 React 集成最成熟**

→ **放弃**：Preact 10（Pangu Nebula 方案）、Vue 3、Svelte 5、SolidJS

→ **放弃理由**：
- **Preact**：API 兼容 React 但缺少 `useTransition`/`useDeferredValue` 等并发原语，无法支撑 ThoughtStream 三路并发；`@arco-design/web-react` 不保证 Preact 兼容
- **Vue 3**：与 NomiFun/OpenAkita 代码完全无法复用；Arco Design Vue 是另一套独立库
- **Svelte 5**：编译时框架，与 NomiFun 的 50 crate 类型同步（ts-rs）链条冲突
- **SolidJS**：生态小、Arco 无官方适配

→ **现有影响**：
1. **Pangu Nebula 的 Preact 代码全部迁移**：`preact/hooks` → `react/hooks`；`preact/compat` 移除；`h()` JSX 运行时差异需审查
2. **BaiLongma 的原生 JS UI 全部 React 化**：`chat.js`/`thought-stream.js`/`hotspot.js` 重写为 React 组件（清洁室流程，因为 BaiLongma MIT）
3. **NomiFun React 19 代码可直接复用**：Layout.tsx / Titlebar.tsx / InstantHoverTooltip.tsx / themeControlContract.ts 等核心 UI 几乎零迁移
4. **OpenAkita 的 22 lazy views 需要从 shadcn/ui 适配为 Arco Design**：Sheet → Arco Drawer、Checkbox → Arco Checkbox 等

---

### 决策 3：前端 UI / 样式方案

→ **选择：Arco Design React + UnoCSS + Apple 系统风格主题**

→ **理由**：
1. **NomiFun 已用 Arco Design + UnoCSS 66**，是用户明确喜欢的 UI 设计来源，迁移路径最短
2. **Arco Design 提供完整企业级组件库**（60+ 组件），覆盖 SparkFox 所有视图需求：Layout、Drawer、Table、Form、Tree、Tabs、Modal、Drawer、Badge
3. **UnoCSS 原子化样式 + 主题定制能力强**：通过 `themeControlContract` 契约机制（NomiFun 已实现）可动态切换 Apple 系统风格主题（macOS Light/Dark）
4. **Apple 系统风格可通过 CSS 变量+UnoCSS preset 实现**：圆角 8px、阴影柔和、毛玻璃 backdrop-filter、SF Pro 字体栈、系统强调色
5. **shadcn/ui（OpenAkita 方案）放弃**：与 NomiFun 不兼容，需要重写 OpenAkita 全部 22 views 的样式

→ **放弃**：shadcn/ui（OpenAkita 方案）、Ant Design 5、Material UI、Tailwind CSS 单独使用、Chakra UI

→ **放弃理由**：
- **shadcn/ui**：基于 Radix UI + Tailwind，与 Arco Design 双重依赖冲突；OpenAkita 用 shadcn 是因为 shadcn 不绑死组件库，但 NomiFun 已锁定 Arco，引入 shadcn 会产生两套设计语言
- **Ant Design 5**：与 Arco 高度同质化，且 Ant Design 更偏后台风；Arco 更现代
- **Material UI**：Google Material 风格与 Apple 系统风格冲突
- **Tailwind 单独使用**：缺乏复杂组件（如 Tree、Cascader），需要自己实现；Arco + UnoCSS 已经覆盖
- **Chakra UI**：社区活跃度下降，企业级组件少

→ **现有影响**：
1. **OpenAkita 22 lazy views 全部从 shadcn/ui 适配为 Arco**：`@/components/ui/button` → `@arco-design/web-react/Button`；`Sheet` → `Drawer`；`ToggleGroup` → `Radio.Group`；样式 className 全部重写为 UnoCSS
2. **BaiLongma 自研 ACUI 卡片 + Scene Protocol UI 重写为 Arco**：保留 Scene Protocol 的"场景驱动"语义，但视觉层换 Arco
3. **Pangu Nebula 的 Tailwind 3 + ReactFlow + @antv/g6 替换**：Tailwind → UnoCSS；ReactFlow → `@xyflow/react`（NomiFun 用的版本）；@antv/g6 → D3 7.9（BaiLongma 用的版本，用于记忆图）
4. **新增 Apple 系统风格主题预设**：macOS Light / macOS Dark / macOS Auto（跟随系统），通过 `themeControlContract` 注入

---

### 决策 4：后端 / 逻辑层语言

→ **选择：Rust 2024 edition（单一后端语言）**

→ **理由**：
1. **NomiFun 50 crate workspace 已验证 Rust 全栈可行性**：进程内 axum 后端 + tokio 异步 + sqlx/rusqlite 存储，是 SparkFox 后端的最佳蓝本
2. **automerge-rs（CRDT）与 Double Ratchet（E2EE）都是 Rust 原生库**，Python/Node.js 都需要 FFI 绑定，引入额外复杂度
3. **性能基线**：NomiFun release profile（thin LTO + codegen-units=1 + strip）启动 <500ms、内存 50MB；Python（Pangu Nebula）启动 2-3s、内存 200MB+
4. **AGPL 清洁室流程对 Rust 代码更友好**：BaiLongma（MIT）+ NomiFun（Apache-2.0）的 Rust 实现可作为"参考实现"由 B 组重写，Python 实现需要完全翻译
5. **Tauri 2 进程内后端模式天然集成 Rust**：无需 sidecar，IPC 走 axum HTTP/SSE，延迟 <1ms
6. **类型安全 + 零成本抽象**：14 crate 边界（RFC-001）需要强类型接口，Rust 的 trait + serde 是最佳选择

→ **放弃**：Python 3.11（Pangu Nebula + OpenAkita 方案）、Node.js ESM（BaiLongma 方案）、Go、混合方案（Rust + Python sidecar）

→ **放弃理由**：
- **Python**：性能差（GIL 限制并发）；与 automerge-rs/Double Ratchet 集成需要 PyO3 FFI，复杂度高；GIL 阻塞 SSE 流式；sidecar 启动慢
- **Node.js**：单线程 + 异步回调模型与 Rust 类型系统不兼容；better-sqlite3 同步调用会阻塞事件循环；与 Tauri 进程内后端模式不匹配
- **Go**：四项目无 Go 血缘；GC 暂停对实时流式对话有影响；泛型弱
- **混合方案**：sidecar 模式（Pangu Nebula）启动慢、IPC 序列化开销大、调试困难

→ **现有影响**：
1. **Pangu Nebula 全部 Python 后端代码 Rust 重写**（清洁室）：
   - `server/main.py` + FastAPI → `crates/sparkfox-core/src/lib.rs` + axum
   - `server/services/memory_service.py` → `crates/sparkfox-memory/`
   - `server/services/swarm_orchestrator.py` → `crates/sparkfox-orchestrator/`
2. **OpenAkita 全部 Python 后端代码 Rust 重写**（清洁室）：
   - `src/openakita/memory/manager.py` → `crates/sparkfox-memory/src/manager.rs`
   - `src/openakita/memory/retrieval.py` → `crates/sparkfox-memory/src/retrieval.rs`
   - `src/openakita/memory/extractor.py` → `crates/sparkfox-memory/src/extractor.rs`
   - `src/openakita/memory/consolidator.py` → `crates/sparkfox-memory/src/consolidator.rs`
3. **BaiLongma 全部 Node.js 后端代码 Rust 重写**（清洁室，因 MIT）：
   - `src/llm.js` → `crates/sparkfox-llm/`
   - `src/memory/injector.js` + `src/memory/threads.js` → `crates/sparkfox-memory/src/threads.rs`
   - `src/scene/scene-store.js` → `crates/sparkfox-core/src/scene.rs`
   - `src/capabilities/executor.js` → `crates/sparkfox-agent/src/executor.rs`
   - `src/runtime/consciousness-loop.js` → `crates/sparkfox-agent/src/consciousness.rs`（L5 元认知横向平面基础）
   - `src/runtime/tick-policy.js` → `crates/sparkfox-agent/src/tick.rs`
4. **NomiFun Rust 代码可直接复用**：50 crate 部分模块（如 `nomi-memory`、`nomi-agent`）作为 SparkFox 对应 crate 的起点
5. **新增 Rust crate**：`sparkfox-e2ee`（Double Ratchet）、`sparkfox-crdt`（automerge-rs 封装）—— 这是四项目都没有的

---

### 决策 5：数据存储方案

→ **选择：SQLite（rusqlite + sqlx）+ sqlite-vec 向量检索 + 文件系统 + automerge-rs CRDT 同步层**

→ **理由**：
1. **四项目中有三个（NomiFun / OpenAkita / BaiLongma）都用 SQLite**，融合路径最短
2. **sqlite-vec 是 BaiLongma 已验证的本地向量方案**：与 `@huggingface/transformers` 本地嵌入配合，无需 ChromaDB/Qdrant 等外部服务
3. **本地优先（Local-First）是 SparkFox 核心价值观**：用户数据主权（"别把第二大脑租给别人"），所有数据必须在本地
4. **automerge-rs CRDT 同步层解决跨设备一致性**：RFC-004 已确定，SQLite 存储文档快照，automerge 存储操作日志
5. **文件系统存储大对象**：图片、语音、PDF 附件走文件系统，SQLite 仅存元数据 + 路径
6. **FTS5 全文检索**：SQLite 内置 FTS5（BaiLongma 已用），无需 Elasticsearch

→ **放弃**：ChromaDB（OpenAkita 方案）、IndexedDB、PostgreSQL、LanceDB、Redis、纯文件系统

→ **放弃理由**：
- **ChromaDB**：需要 Python 进程，与 Rust 单语言后端冲突；额外服务依赖
- **IndexedDB**：仅浏览器，无法被 Rust 后端访问；容量受限
- **PostgreSQL**：需要服务器进程，违反本地优先原则
- **LanceDB**：Rust 原生但生态尚浅；sqlite-vec 已足够
- **Redis**：内存数据库，不适合持久化；24/7 运行内存压力大
- **纯文件系统**：无事务、无查询、无向量检索

→ **现有影响**：
1. **Pangu Nebula 的 SQLite（aiosqlite + SQLAlchemy）迁移**：Schema 直接迁移到 `rusqlite` + `sqlx`；SQLAlchemy ORM 移除，改用 `sqlx::query!` 宏
2. **OpenAkita 的 ChromaDB 向量存储迁移**：所有向量数据迁移到 sqlite-vec；`VectorStore` 抽象层保留，实现替换
3. **BaiLongma 的 better-sqlite3 同步调用迁移**：所有 SQL 调用改为 `rusqlite` 异步；22 个 memory 子模块的 Schema 合并到 `sparkfox-store`
4. **新增 automerge-rs 同步层**：每个记忆条目都附带 `automerge::ChangeHash`，跨设备同步时合并
5. **新增加密层**：`sparkfox-e2ee` crate 使用 Double Ratchet 加密 SQLite 数据库中敏感字段（用户身份、记忆内容）

---

### 决策 6：记忆体系方案

→ **选择：以 OpenAkita 三层记忆为表层实现起点 + 改造扩展为 Pangu Nebula 6 层 L0-L5 蓝图（RFC-003 SoT）**

→ **改造增强点**：

**改造 1：从 3 层扩展为 6 层（L0-L5）**
- 改造理由：OpenAkita 三层（Scratchpad + Core+Dynamic + Persona+Identity）覆盖 L1-L4，缺少 L0 感官层和 L5 元认知横向平面；Pangu Nebula 6 层架构是 RFC-003 的硬约束蓝图
- 实现思路：
  - **L0 感官层**（新增）：捕获原始多模态输入（文本/图像/语音/热点），基于 BaiLongma 的 `consciousness-loop.js` 用 Rust 重写为 `crates/sparkfox-memory/src/sensory.rs`，做轻量预处理后丢弃
  - **L1 Scratchpad**（保留 OpenAkita）：`contextvars` 6 个 ContextVar 迁移为 Rust `tokio::task_local!`
  - **L2 情景记忆**（新增，OpenAkita Dynamic 扩展）：对话片段 + 时间戳 + 上下文标签
  - **L3 语义记忆**（保留 OpenAkita Core）：事实/偏好/技能/规则/错误/经验/人格特质/上下文 8 类型
  - **L4 程序记忆**（OpenAkita Persona+Identity 扩展）：SOUL.md / AGENT.md / USER.md / MEMORY.md / POLICIES.yaml + 8 personas + 工具调用模式
  - **L5 元认知横向平面**（新增，BaiLongma consciousness-loop Rust 化）：跨层监控 + 反思 + 自我修正 + 遗忘决策

**改造 2：增加 LLM 审查 ReviewProgress（OpenAkita 已有，保留并强化）**
- 改造理由：OpenAkita 的 `ReviewProgress` 状态机（idle/running/done/error/cancelled + phase: llm_calling/batch_done/done）是记忆质量控制的核心；需保留并扩展为 L5 元认知的一部分
- 实现思路：Rust 重写为 `crates/sparkfox-memory/src/review.rs`，状态机用 `enum` + `tokio::sync::watch`；UI 侧 MemoryView 的 ReviewProgress 类型直接复用

**改造 3：集成 automerge-rs CRDT 实现跨设备同步**
- 改造理由：RFC-004 硬约束；OpenAkita 无 CRDT，多设备会冲突
- 实现思路：每条记忆附带 `automerge::ChangeHash`；`crates/sparkfox-crdt` 提供 `MemoryDoc` 类型，封装 `automerge::AutoCommit`；同步时合并 doc，冲突策略：importance_score 取大、content 取最新、tags 取并集

**改造 4：增加 5 策略遗忘机制（优化方案 v1.0 要求）**
- 改造理由：OpenAkita 仅有 `apply_retention`，策略单一；BaiLongma Thread 温度窗口（warm 6h/cool 48h/cold）有遗忘雏形但不完整
- 实现思路：`crates/sparkfox-memory/src/forgetting.rs` 实现 5 策略：
  1. **时间衰减**：`importance_score *= exp(-Δt/τ)`，τ 按类型不同（fact 90d / context 7d / experience 30d）
  2. **访问频率**：access_count < 3 且 age > 30d → 降级
  3. **重要性阈值**：score < 0.3 → 归档到冷存储
  4. **重复合并**：MemoryConsolidator 已有，扩展为 LLM 辅助合并
  5. **用户主动遗忘**：UI 提供"忘记这条"按钮，硬删除 + CRDT tombstone

**改造 5：L5 元认知横向平面基于 BaiLongma consciousness-loop 用 Rust 重写**
- 改造理由：L5 是 Pangu Nebula 6 层架构的最高层，但 Pangu Nebula 实现不完整；BaiLongma 的 `consciousness-loop.js`（Tick 心跳 + ACI 预判注入 + 委托发现）是现成的元认知雏形
- 实现思路：
  - `crates/sparkfox-agent/src/consciousness.rs`：Tick 心跳（默认 100ms）驱动元认知循环
  - 每个 Tick：扫描 L1-L4 状态 → 评估是否需要反思 → 触发 ReviewProgress 或 Consolidator
  - ACI 预判注入：基于历史 turn 模式预判下一步工具调用，提前注入到 L1 Scratchpad
  - 委托发现（delegationDiscovery）：L5 监控蜂群 Worker 负载，自动委托子任务

**改造 6：Thread 线索模型融合（BaiLongma → L2 情景记忆）**
- 改造理由：BaiLongma 的 Thread（多并发线索 + 前台指针 + 承诺 + 温度窗口）是情景记忆的天然实现，OpenAkita Dynamic Memory 缺少并发线索能力
- 实现思路：`crates/sparkfox-memory/src/threads.rs` 实现 Thread 结构体（id/topic/state/warm_until/cool_until/commitments），L2 情景记忆按 Thread 组织

**改造 7：增加可视化记忆图（OpenAkita MemoryGraph3D + BaiLongma D3）**
- 改造理由：OpenAkita 已有 `MemoryGraph3D` lazy 组件（3D 图谱），BaiLongma 有 D3 7.9 记忆图；用户喜欢 OpenAkita 的记忆管理 UI
- 实现思路：复用 OpenAkita `MemoryGraph3D`，适配 Arco Design；D3 用于二级视图（力导向图）

→ **总体架构**：
```
L5 元认知横向平面（consciousness.rs + review.rs + forgetting.rs）  ← BaiLongma + 新增
L4 程序记忆（persona.rs + identity.rs + procedure.rs）              ← OpenAkita Layer 3
L3 语义记忆（semantic.rs + 8 types）                                ← OpenAkita Layer 2 Core
L2 情景记忆（episodic.rs + threads.rs）                             ← BaiLongma Thread + OpenAkita Dynamic
L1 工作记忆（scratchpad.rs + contextvars）                          ← OpenAkita Layer 1
L0 感官层（sensory.rs + multimodal capture）                        ← 新增
```

---

### 决策 7：状态管理方案

→ **选择：Zustand 5 + React Context（分层）**

→ **理由**：
1. **轻量（<1KB）+ TypeScript 友好**：与 NomiFun 的 ts-rs 类型同步链条天然契合
2. **NomiFun 已用 React Context + ts-rs 类型同步**，但 Context 在跨模块更新时性能差（全树重渲染）；Zustand 用 selector 订阅，仅订阅组件重渲染
3. **OpenAkita 22 lazy views 需要跨视图共享 Agent 状态、Memory 状态、Token 统计**：Zustand 的 `create` + `subscribeWithSelector` 是最佳方案
4. **BaiLongma 的全局 `state` 对象 + sceneStore 需要迁移到类型安全的状态管理**：Zustand + TypeScript 是最小迁移成本
5. **支持中间件**：`persist`（localStorage 持久化）、`devtools`（Redux DevTools 集成）、`immer`（不可变更新）
6. **与 React 19 并发渲染兼容**：Zustand 5 已适配 `useSyncExternalStore`

→ **放弃**：Redux Toolkit、Pinia、Vuex、MobX、Jotai、Recoil、Valtio、单一 Context 方案

→ **放弃理由**：
- **Redux Toolkit**：模板代码多、bundle 大（~15KB）；Zustand 已够用
- **Pinia/Vuex**：Vue 专用，与 React 不兼容
- **MobX**：observable 装饰器与 TypeScript 严格模式冲突；学习曲线陡
- **Jotai/Recoil**：atom 模型适合细粒度但跨模块聚合弱；Zustand store 更直观
- **Valtio**：proxy 模型与 React 19 StrictMode 有兼容问题
- **单一 Context**：NomiFun 已用但性能瓶颈明显（每次更新全树重渲染）

→ **现有影响**：
1. **NomiFun 的 LayoutContext / NavigationHistoryContext / WebuiServerContext 保留**（这些是低频更新场景，Context 合适）
2. **新增 Zustand stores**：
   - `chatStore`：当前会话、消息列表、流式状态（替代 BaiLongma chatHistory/chatMessages 全局变量）
   - `agentStore`：当前选中 Agent、Agent 列表、运行状态（替代 OpenAkita AgentManagerView local state）
   - `memoryStore`：记忆列表、筛选条件、ReviewProgress（替代 OpenAkita MemoryView local state）
   - `monitorStore`：Token 统计、Timeline、Sessions（替代 OpenAkita TokenStatsView local state）
   - `thinkingStore`：ThoughtStream 状态、当前工具、错误状态（替代 BaiLongma ThoughtStream 类内部状态）
   - `hotspotStore`：热点列表、平台筛选、3D 地球状态（替代 BaiLongma hotspotLists 全局变量）
   - `settingsStore`：主题、快捷键、模型配置（替代 NomiFun configService 部分）
3. **BaiLongma 的 sceneStore（Scene Protocol 单一真相源）保留并 Rust 化**：`crates/sparkfox-core/src/scene.rs`，通过 IPC 同步到前端 `sceneStore`

---

### 决策 8：构建工具

→ **选择：Vite 6（前端）+ cargo workspace（Rust）+ bun scripts（任务编排）**

→ **理由**：
1. **NomiFun 已用 cargo workspace（resolver="3"）+ bun scripts + thin LTO + codegen-units=1 + strip**，是 SparkFox 后端构建的蓝本
2. **Vite 6 是 React 19 最佳搭档**：ESM 原生、HMR <100ms、Rolldown 生产构建快
3. **bun 作为 task runner 比 npm/yarn 快 10x**：NomiFun 已验证
4. **Tauri 2 官方 CLI 与 cargo + Vite 深度集成**：`tauri build` 一键打包
5. **TypeScript 5 + ts-rs 类型同步**：Rust struct → TypeScript type 自动生成，避免手写

→ **放弃**：webpack 5、Turbopack、Rollup 单独使用、esbuild 单独使用、SWC、Parcel

→ **放弃理由**：
- **webpack 5**：慢（HMR 500ms+）、配置复杂、与 ESM 兼容差
- **Turbopack**：仍 beta，Next.js 绑定
- **Rollup 单独使用**：缺乏 HMR、配置冗长
- **esbuild 单独使用**：无 HMR、无插件生态
- **SWC**：仅转译，无打包
- **Parcel**：生态弱、企业级场景少

→ **现有影响**：
1. **Pangu Nebula 的 Vite 6 + hatchling + cargo 配置迁移**：保留 Vite 6；hatchling（Python）移除；cargo 配置参考 NomiFun release profile
2. **OpenAkita 的 Vite + hatchling + ruff + pytest 配置迁移**：hatchling/ruff/pytest 移除；Vite 保留
3. **BaiLongma 的 electron-builder + electron/rebuild 配置全部移除**：改为 `tauri build`
4. **NomiFun 的 `package.json` scripts + `Cargo.toml` workspace 配置可直接复用**：改名为 sparkfox-*
5. **新增 ts-rs 类型同步脚本**：`bun run sync-types` → 触发 `cargo test --features export-types`

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第二部分：模块拓扑（SparkFox 最终模块划分）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 模块 A：前端 UI 框架

- **来源**：NomiFun
- **来源文件路径**：
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\components\layout\Layout.tsx`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\components\layout\Titlebar.tsx`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\components\layout\PwaPullToRefresh.tsx`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\components\base\InstantHoverTooltip.tsx`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\utils\theme\themeControlContract.ts`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\utils\theme\customCssProcessor.ts`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\utils\theme\themeCssSync.ts`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\utils\theme\themeBroadcast.ts`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\hooks\context\LayoutContext.tsx`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\styles\layout.css`
- **功能**：提供整体布局（Titlebar + Sider 184px 可拖拽 + Content 三栏）、主题配色（ThemeContext + themeControlContract 契约机制）、Apple 系统风格主题预设、移动端检测、自定义 CSS 注入
- **改造点**：
  1. **侧边栏导航项重写**：NomiFun 原始导航是"会话/终端/知识/陪伴"等，改为 SparkFox 的 6 大路由（对话/Agent/监视/热点/记忆/设置），保留 `useConversationShortcuts` 快捷键机制
  2. **Titlebar 工作区切换按钮对接 monitorStore**：原 `workspaceAvailable` 仅判断 `/conversation/` 和 `/terminal/`，扩展到 `/agents/`、`/monitor/` 等
  3. **新增 Apple 系统风格主题预设**：在 `presets/` 下新增 `macosLight.ts`、`macosDark.ts`、`macosAuto.ts`（跟随系统），通过 `themeControlContract` 注入；圆角 8px、毛玻璃 backdrop-filter、SF Pro 字体栈、系统强调色
  4. **Sider 宽度存储 key 改名**：`nomifun:rail-width` → `sparkfox:rail-width`；DEFAULT_SIDER_WIDTH 从 184 调整为 200（为 Agent 头像预留空间）
  5. **移除 NomiFun 特有逻辑**：`useUpdateAvailability`、`UpdateModal`、`useDeepLink`、`PwaPullToRefresh` 等 NomiFun 业务逻辑移除或重写
  6. **IPC 桥改名**：`ipcBridge` → `sparkfox.ipc`，所有 `ipcBridge.application.*` / `ipcBridge.config.*` 调用重写为 SparkFox IPC 协议
- **依赖**：无（基础模块）
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\components\layout\`

---

### 模块 B：Agent 菜单系统

- **来源**：OpenAkita
- **来源文件路径**：
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\AgentManagerView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\AgentDashboardView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\AgentStoreView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\AgentSystemView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\components\AgentIcon.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\components\pixel-avatar\*`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\components\pixel-office\*`
- **功能**：Agent 的选择、配置、创建、管理（AgentProfile 22 字段）+ Agent Dashboard 力导向图可视化（TopoNode + TopoEdge + Pulse 边脉冲 + ToolSat 工具卫星 + Mote 环境粒子）+ Agent Store 市场 + Pixel Office 像素办公室
- **改造点**：
  1. **shadcn/ui → Arco Design 适配**：`Sheet`（侧滑编辑面板）→ `Arco Drawer`；`Checkbox`/`Switch`/`Select`/`Badge` → Arco 同名组件；`lucide-react` 图标保留（与 Arco 兼容）；`Input`/`Textarea`/`Label`/`Button` 全部换 Arco
  2. **AgentProfile 类型扩展**：在 22 字段基础上新增 `crdt_doc_id: string`（automerge 文档 ID）、`e2ee_public_key: string`（Double Ratchet 公钥）、`pangu_nebula_layer: L0|L1|L2|L3|L4|L5`（标记 Agent 主要活动层）、`scene_protocol_enabled: boolean`（BaiLongma Scene Protocol 集成开关）
  3. **对接 Zustand agentStore**：所有 local state（agentList/selectedId/editingProfile）迁移到 `useAgentStore`；Sheet 编辑面板通过 `agentStore.updateProfile()` 提交
  4. **AgentDashboard 力导向图对接 sparkfox-orchestrator**：`TopoNode` 数据源从 OpenAkita 本地 mock 改为 `crates/sparkfox-orchestrator` 实时 DAG 推送（SSE）；Pulse 边脉冲 / ToolSat 卫星 / Mote 粒子保留但数据源换成 Rust 后端
  5. **新增 L5 元认知 Agent 类型**：在 `AgentProfile.type` 枚举中新增 `"meta_cognitive"`，专门用于 L5 横向平面 Agent（基于 BaiLongma consciousness-loop）
  6. **Pixel Office 适配 Apple 系统风格**：原 Pixel Office 是 8-bit 像素风，与 Apple 风格冲突；改为毛玻璃 + 系统圆角的"办公室"视觉，但保留像素 Avatar 作为 Agent 头像
  7. **Agent Store 集成 AGPL 合规标识**：每个 Agent 卡片显示 License 标签（AGPL/MIT/Apache），清洁室重写后的 Agent 标"SparkFox Original"
- **依赖**：模块 A（嵌入到侧边栏与主内容区）、模块 D（调度 Agent 执行）、模块 I（agentStore 状态管理）
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\components\agent\` + `D:\xin kaifa\SparkFox\ui\src\renderer\views\AgentView\`

---

### 模块 C：监视面板

- **来源**：OpenAkita
- **来源文件路径**：
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\TokenStatsView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\StatusView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\InboxView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\TroubleshootPanel.tsx`（如存在）
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\ProgressLedgerTimeline.tsx`（如存在）
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\RuntimeEnvironmentPanel.tsx`（如存在）
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\OrgDashboard.tsx`（如存在）
- **功能**：Token 用量统计（6 周期 1d/3d/1w/1m/6m/1y × 5 维度 input/output/total/requests/cost）+ Timeline 柱状图 + Sessions 明细 + Records 流水 + Agent 运行状态总览 + 故障排查面板 + 进度账本时间线 + 收件箱
- **改造点**：
  1. **shadcn/ui → Arco Design 适配**：`Card`/`Table`/`Button`/`Switch`/`Label`/`Badge` 全部换 Arco；MiniBar 自定义组件保留但样式重写为 UnoCSS
  2. **API 端点改名**：`/api/stats/tokens/*` → `/api/sparkfox/monitor/tokens/*`；`safeFetch` 改为 `sparkfox.ipc.monitor.*` 走 Tauri IPC（性能更好）
  3. **新增 CRDT 同步状态面板**：显示 automerge-rs 同步状态（peer 连接数/同步延迟/冲突解决次数/最后同步时间），数据源 `crates/sparkfox-crdt`
  4. **新增 E2EE 安全面板**：显示 Double Ratchet 状态（会话密钥轮换次数/加密消息数/解密失败次数），数据源 `crates/sparkfox-e2ee`
  5. **新增 6 层记忆健康面板**：L0-L5 每层条目数、平均重要性、ReviewProgress 状态、遗忘机制执行统计，数据源 `crates/sparkfox-memory`
  6. **新增蜂群 DAG 可视化面板**：复用 AgentDashboard 的 TopoNode 力导向图，但只读模式，专门展示当前蜂群编排状态
  7. **对接 Zustand monitorStore**：`period`/`total`/`byEndpoint`/`byOp`/`timeline`/`sessions`/`records` 全部迁移到 `useMonitorStore`，避免组件卸载后丢失状态
  8. **新增 Apple 系统风格图表样式**：Timeline 柱状图颜色改为 macOS 系统蓝（#007AFF）/ 绿（#34C759）；Card 阴影改为柔和毛玻璃
- **依赖**：模块 A（UI 框架）、模块 B（获取 Agent 状态数据）、模块 I（monitorStore）、后端 `crates/sparkfox-monitor` + `sparkfox-crdt` + `sparkfox-e2ee` + `sparkfox-memory` + `sparkfox-orchestrator`
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\components\monitor\` + `D:\xin kaifa\SparkFox\ui\src\renderer\views\MonitorView\`

---

### 模块 D：三层记忆体系（改造为 6 层 L0-L5）

- **来源**：OpenAkita（基础）+ Pangu Nebula（6 层蓝图）+ BaiLongma（Thread + consciousness-loop）+ 改造增强
- **来源文件路径**：
  - `D:\xin kaifa\_reference\openakita\src\openakita\memory\manager.py`（MemoryManager v2 架构）
  - `D:\xin kaifa\_reference\openakita\src\openakita\memory\retrieval.py`（RetrievalEngine）
  - `D:\xin kaifa\_reference\openakita\src\openakita\memory\extractor.py`（MemoryExtractor）
  - `D:\xin kaifa\_reference\openakita\src\openakita\memory\consolidator.py`（MemoryConsolidator）
  - `D:\xin kaifa\_reference\openakita\src\openakita\memory\unified_store.py`（UnifiedStore）
  - `D:\xin kaifa\_reference\openakita\src\openakita\memory\types.py`（Memory/Attachment/ConversationTurn 等）
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\views\MemoryView.tsx`
  - `D:\xin kaifa\_reference\openakita\apps\setup-center\src\components\MemoryGraph3D.tsx`
  - `D:\xin kaifa\_reference\BaiLongma\src\memory\injector.js`（Thread 线索模型）
  - `D:\xin kaifa\_reference\BaiLongma\src\memory\threads.js`（Thread 温度窗口）
  - `D:\xin kaifa\_reference\BaiLongma\src\runtime\consciousness-loop.js`（L5 元认知基础）
  - `D:\Pangu Nebula\server\services\memory_service.py`（6 层 L0-L5 蓝图）
- **功能**：6 层记忆管理（L0 感官 / L1 工作记忆 / L2 情景 / L3 语义 / L4 程序 / L5 元认知）+ 自动提取（MemoryExtractor）+ 后台整合（MemoryConsolidator）+ LLM 审查（ReviewProgress）+ 5 策略遗忘 + Thread 线索模型 + 跨设备 CRDT 同步 + 可视化记忆图
- **改造点**：
  1. **增强长期记忆的语义检索能力**：OpenAkita 已有 `RetrievalEngine` 但仅基于 sqlite-vec；新增混合检索（向量 + FTS5 全文 + 图谱多跳），通过 `crates/sparkfox-memory/src/retrieval.rs` 实现；权重：向量 0.5 + FTS5 0.3 + 图谱 0.2
  2. **增加记忆的可视化管理界面**：复用 OpenAkita `MemoryGraph3D`（3D 图谱）+ 新增 D3 力导向 2D 视图（BaiLongma 风格）；UI 适配 Arco Design；新增 L0-L5 层级筛选 Tab
  3. **优化记忆自动提取的逻辑**：OpenAkita `MemoryExtractor` 是规则驱动；改造为 LLM 辅助提取（每 5 个 turn 触发一次 LLM 提取，避免每 turn 调用浪费 token）；新增"重要性预判"（LLM 在提取时同时打分 0-1）
  4. **新增 5 策略遗忘机制**：见决策 6 改造点 4
  5. **新增 Thread 线索模型**：见决策 6 改造点 6
  6. **新增 L5 元认知横向平面**：见决策 6 改造点 5
  7. **集成 automerge-rs CRDT**：见决策 6 改造点 3
  8. **集成 Double Ratchet E2EE**：敏感字段（用户身份、L4 Persona）加密存储；`crates/sparkfox-e2ee` 提供 `encrypt_memory_field()` / `decrypt_memory_field()`
  9. **MemoryView UI 适配**：shadcn → Arco；`TYPE_LABEL_KEYS` 8 类型保留；`TYPE_COLORS` 改为 macOS 系统色板；新增 L0-L5 层级筛选；新增"忘记这条"按钮（硬删除 + CRDT tombstone）
  10. **MigrationStatus 保留并扩展**：新增 `crdt_sync_status` 字段（已同步/冲突/未同步）、`e2ee_encrypted` 字段（是否加密）
- **依赖**：模块 E（对话数据输入）、模块 A（管理界面 UI）、模块 I（memoryStore）、后端 `crates/sparkfox-memory` + `sparkfox-crdt` + `sparkfox-e2ee` + `sparkfox-store`
- **目标文件路径**：
  - 后端：`D:\xin kaifa\SparkFox\crates\sparkfox-memory\src\`（manager.rs / retrieval.rs / extractor.rs / consolidator.rs / unified_store.rs / types.rs / sensory.rs / scratchpad.rs / episodic.rs / semantic.rs / procedure.rs / meta_cognitive.rs / threads.rs / forgetting.rs / review.rs）
  - UI：`D:\xin kaifa\SparkFox\ui\src\renderer\components\memory\` + `D:\xin kaifa\SparkFox\ui\src\renderer\views\MemoryView\`

---

### 模块 E：对话展示组件

- **来源**：BaiLongma
- **来源文件路径**：
  - `D:\xin kaifa\_reference\BaiLongma\src\ui\brain-ui\chat.js`（initChat 核心）
  - `D:\xin kaifa\_reference\BaiLongma\src\ui\brain-ui\app.js`（Brain UI 主入口）
  - `D:\xin kaifa\_reference\BaiLongma\src\api.js`（API 调用）
  - `D:\xin kaifa\_reference\BaiLongma\src\db.js`（SQLite 调用）
- **功能**：对话消息渲染（liveEl 流式气泡）+ 对话流布局 + 双重去重（renderedMessageIds + recentRenderedKeys + RENDER_DEDUPE_TTL_MS 2min）+ 激活预热锁（applyActivationWarmupLock）+ 自适应高度（autoGrowInput）+ 按住空格说话（PUSH_TO_TALK_PLACEHOLDER）+ 多渠道消息（friendlyChannelLabel）+ 图片粘贴（MAX_PASTED_IMAGES 8 / MAX_PASTED_IMAGE_BYTES 12MB）+ 音频上下文（ensureAudioContext）
- **改造点**：
  1. **适配 NomiFun 的整体布局框架**：BaiLongma 原生 HTML/CSS/JS 全部重写为 React 19 + TypeScript + Arco Design（清洁室流程，因 BaiLongma MIT）；DOM 操作（`chatArea.appendChild`）改为 React 组件树；`liveEl` 改为 `useRef<HTMLDivElement>` + 流式 state
  2. **对接记忆体系，支持记忆注入**：对话开始前调用 `crates/sparkfox-memory` RetrievalEngine 检索相关记忆，注入到 system prompt；UI 侧在消息气泡上方显示"使用了 N 条记忆"徽章，点击展开查看
  3. **对接 Zustand chatStore**：`chatHistory`/`chatMessages`/`msgInput`/`chatArea` 全局变量迁移到 `useChatStore`；`claimRenderedMessage` 双重去重逻辑保留但用 `Set` + `Map` 存储在 store
  4. **保留并强化双重去重**：`renderedMessageIds: Set<string>` + `recentRenderedKeys: Map<string, number>` + `RENDER_DEDUPE_TTL_MS = 2*60*1000` 全部迁移到 `chatStore.dedupe` 字段
  5. **保留激活预热锁**：`applyActivationWarmupLock`（sessionStorage）逻辑保留，但改为 `chatStore.warmupLock` 字段；激活期间显示"AI 预热中..."占位
  6. **保留自适应高度**：`autoGrowInput` 改为 `useAutoGrowInput` hook；MAX_PASTED_IMAGES 8 / MAX_PASTED_IMAGE_BYTES 12MB 保留
  7. **保留按住空格说话**：`PUSH_TO_TALK_PLACEHOLDER = "按住空格键开始说话"` 保留；新增"按住空格"事件监听 hook `usePushToTalk`
  8. **保留多渠道标签**：`friendlyChannelLabel`（WeChat/WeCom/Discord/Feishu）保留，扩展支持 Slack/Telegram/WhatsApp
  9. **保留音频上下文**：`ensureAudioContext` + `unlockAudioOnFirstGesture` 保留，用于语音播报
  10. **集成思考过程展示**：在每条 AI 消息气泡内嵌入 ThoughtStream 组件（模块 F）
  11. **集成热点上下文**：`buildHotspotContext` 构建的中性上下文注入到对话 system prompt（仅在相关时）
  12. **Apple 系统风格气泡**：用户气泡 macOS 系统蓝（#007AFF）右对齐；AI 气泡毛玻璃浅灰左对齐；圆角 12px；字体 SF Pro
- **依赖**：模块 A（布局）、模块 D（记忆注入）、模块 F（思考过程嵌入）、模块 I（chatStore）、后端 `crates/sparkfox-chat` + `sparkfox-llm`
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\components\chat\` + `D:\xin kaifa\SparkFox\ui\src\renderer\views\ChatView\`

---

### 模块 F：思考过程可视化

- **来源**：BaiLongma
- **来源文件路径**：
  - `D:\xin kaifa\_reference\BaiLongma\src\ui\brain-ui\thought-stream.js`（ThoughtStream 类）
- **功能**：展示 AI 推理链（thinkingEl）+ 工具调用（lastToolEl）+ 状态（statusEl）三区联动 + 57 工具中文映射（TOOL_ZH）+ 57 工具 emoji 图标（TOOL_ICON）+ 失败检测（isFailureResult）+ 时间戳（tStamp）+ reasoning_effort=high
- **改造点**：
  1. **ThoughtStream 类 → React 组件 + hook**：`class ThoughtStream` → `useThoughtStream` hook + `<ThoughtStreamPanel>` 组件；`thinkingEl`/`lastToolEl`/`statusEl` 改为 React state + ref
  2. **TOOL_ZH 57 工具中文映射保留并扩展**：新增 SparkFox 工具（`crdt_sync` → "CRDT 同步" / `e2ee_encrypt` → "加密记忆" / `l5_metacognition` → "元认知反思" / `forget_memory` → "遗忘记忆" / `hotspot_track` → "追踪热点"）
  3. **TOOL_ICON 57 工具图标保留并扩展**：新增 SparkFox 工具 emoji（`crdt_sync` → 🔄 / `e2ee_encrypt` → 🔐 / `l5_metacognition` → 🧠 / `forget_memory` → 🗑️ / `hotspot_track` → 📰）
  4. **isFailureResult 失败检测保留**：正则匹配"错误/失败/异常/Error/ERROR" + JSON `ok:false` 检测；扩展为 Rust 后端返回的 `Result<T, E>` 类型自动判断
  5. **三区联动设计保留**：`thinkingEl`（思考中...）+ `lastToolEl`（当前工具）+ `statusEl`（时间戳 + 状态）三区在对话气泡内垂直布局
  6. **新增 Turn Trace 折叠面板**：BaiLongma 的 turn-trace 回合级轨迹保留，改为可折叠的 `<TurnTraceAccordion>` 组件，默认折叠
  7. **对接 Zustand thinkingStore**：`hadToolCall`/`toolFailed`/`curLine`/`startedAt` 迁移到 `useThinkingStore`
  8. **Apple 系统风格**：思考中用 macOS 系统灰（#8E8E93）+ 斜体；工具调用用系统蓝（#007AFF）；失败用系统红（#FF3B30）；成功用系统绿（#34C759）
  9. **集成 reasoning_effort=high**：所有 LLM 调用默认 `reasoning_effort=high`，可在设置中调整
- **依赖**：模块 E（嵌入到对话展示中）、模块 I（thinkingStore）、后端 `crates/sparkfox-thinking`
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\components\thinking\`

---

### 模块 G：信息热点追踪

- **来源**：BaiLongma
- **来源文件路径**：
  - `D:\xin kaifa\_reference\BaiLongma\src\ui\brain-ui\hotspot.js`（4 平台热榜 + MOCK_FEED + TICKER_ITEMS + buildHotspotContext + renderList）
  - `D:\xin kaifa\_reference\BaiLongma\src\ui\brain-ui\trending.js`（国内外分区 trending）
  - `D:\xin kaifa\_reference\BaiLongma\src\ui\brain-ui\hotspot-earth.js`（如存在，Three.js 3D 地球）
- **功能**：4 平台热榜（douyin/xiaohongshu/wechat/weibo）+ 8 类别事件流（自然灾害/科技/财经/体育/社会/政策/旅游/其他）+ 跑马灯（TICKER_ITEMS）+ 中性上下文构建（buildHotspotContext）+ 国内外分区（trending.js：CN→微博+知乎 / 其他→HN+Reddit）+ 3D 地球可视化（HotspotEarth Three.js）+ 30min 自动刷新
- **改造点**：
  1. **原生 JS → React + TypeScript**：`hotspotLists`/`MOCK_FEED`/`TICKER_ITEMS` 全局变量迁移到 `useHotspotStore`；`renderList` 函数改为 `<HotspotList>` 组件
  2. **平台扩展**：4 平台（douyin/xiaohongshu/wechat/weibo）→ 8 平台（新增 zhihu/bilibili/hn/reddit）；`PLATFORM_CONFIG` 扩展
  3. **数据源后端化**：原 BaiLongma 是前端直接抓取；改为后端 `crates/sparkfox-hotspot` 统一抓取（Rust reqwest + 30min cron），前端通过 SSE 订阅推送
  4. **buildHotspotContext 中性上下文构建保留**：`"不要求主动总结，不要把它当成用户消息"` + `"仅在相关时提及"` 保留；注入到 LLM system prompt
  5. **HotspotEarth Three.js 3D 地球保留**：3D 地球可视化迁移到 React 组件 `<HotspotEarth>`；保留 Three.js 依赖；点击地球上的热点标记触发对话讨论
  6. **TICKER_ITEMS 跑马灯保留**：顶部跑马灯样式适配 Apple 系统风格（毛玻璃背景 + 系统字体）
  7. **新增热点存入记忆**：用户点击热点 → 自动写入 L0 感官层 + L2 情景记忆（Thread "热点讨论"）；下次相关讨论时 RetrievalEngine 自动召回
  8. **新增热点趋势分析**：`TREND_ICONS`（↑↓—）+ `TREND_CLASSES` 保留；新增 7 天趋势图（基于历史数据）
  9. **Apple 系统风格**：热点列表用 Arco `List` 组件；3D 地球保留但 UI 控件用 Arco；类别颜色用 macOS 系统色板
  10. **AGPL 清洁室**：BaiLongma hotspot.js 全部 React 化重写，不复制原代码
- **依赖**：模块 A（UI 框架）、模块 D（热点信息存入记忆）、模块 I（hotspotStore）、后端 `crates/sparkfox-hotspot`
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\components\hotspot\` + `D:\xin kaifa\SparkFox\ui\src\renderer\views\HotspotView\`

---

### 模块 H：路由与页面管理

- **来源**：全新设计（基于 React Router 7）
- **功能**：页面路由、导航、页面切换、深链接、快捷键路由、历史记录
- **设计要点**：
  1. **React Router 7 data mode**：使用 `createBrowserRouter` + `RouterProvider`
  2. **路由结构**：
     - `/` → 主对话页（ChatView）
     - `/agents` → Agent 管理页（AgentView）
     - `/monitor` → 监视面板页（MonitorView）
     - `/hotspot` → 热点追踪页（HotspotView）
     - `/memory` → 记忆管理页（MemoryView）
     - `/settings` → 设置页（SettingsView）
     - `/settings/*` → 设置子页（模型/主题/快捷键/数据管理/E2EE/CRDT）
  3. **懒加载**：所有非首页路由用 `React.lazy` + `Suspense`（参考 OpenAkita 22 lazy views 模式）
  4. **快捷键路由**：参考 NomiFun `useConversationShortcuts`，扩展为：
     - `Cmd/Ctrl+1` → 对话
     - `Cmd/Ctrl+2` → Agent
     - `Cmd/Ctrl+3` → 监视
     - `Cmd/Ctrl+4` → 热点
     - `Cmd/Ctrl+5` → 记忆
     - `Cmd/Ctrl+,` → 设置
  5. **历史记录**：参考 NomiFun `NavigationHistoryProvider`，支持前进/后退
  6. **深链接**：参考 NomiFun `useDeepLink`，支持 `sparkfox://chat/<session_id>` / `sparkfox://memory/<memory_id>` 等
- **依赖**：模块 A（Layout 嵌套）
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\router\`

---

### 模块 I：状态管理

- **来源**：全新设计（Zustand 5 + React Context 分层）
- **功能**：全局状态管理、跨模块数据流通、状态持久化、状态订阅
- **设计要点**：
  1. **Zustand stores 划分**（见决策 7）：
     - `chatStore`：会话列表 / 当前会话 / 消息列表 / 流式状态 / 双重去重 / 激活预热锁
     - `agentStore`：Agent 列表 / 当前选中 / 编辑中 Profile / Dashboard 拓扑
     - `memoryStore`：记忆列表 / 筛选 / ReviewProgress / MigrationStatus / 视图模式（list/graph）
     - `monitorStore`：Token 统计 / Timeline / Sessions / Records / CRDT 同步状态 / E2EE 状态 / 6 层健康 / 蜂群 DAG
     - `thinkingStore`：ThoughtStream 状态 / 当前工具 / 错误状态 / Turn Trace
     - `hotspotStore`：热点列表 / 平台筛选 / 3D 地球状态 / 跑马灯
     - `settingsStore`：主题 / 快捷键 / 模型配置 / E2EE 设置 / CRDT 设置
     - `sceneStore`：Scene Protocol 单一真相源（与后端 `crates/sparkfox-core/src/scene.rs` 同步）
  2. **Context 保留**（低频更新场景）：
     - `LayoutContext`：Sider 宽度 / 折叠状态 / 移动端检测
     - `NavigationHistoryContext`：路由历史
     - `WebuiServerContext`：WebUI 服务状态（如启用）
  3. **中间件**：
     - `persist`：settingsStore（localStorage）、sceneStore（localStorage）
     - `devtools`：所有 store（开发环境）
     - `immer`：chatStore / memoryStore（深嵌套）
     - `subscribeWithSelector`：所有 store（精确订阅）
  4. **ts-rs 类型同步**：所有 store 的 state 类型从 Rust `crates/sparkfox-core/src/types.rs` 自动生成
- **依赖**：所有模块
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\store\`

---

### 模块 J：配置与设置

- **来源**：NomiFun（基础）+ 增强
- **来源文件路径**：
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\pages\settings\`（整个目录）
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\renderer\components\settings\UpdateModal.tsx`
  - `D:\xin kaifa\_reference\nomifun-tauri\ui\src\common\config\configService.ts`
- **功能**：模型配置（LLM Provider / API Key / 模型选择 / 温度 / reasoning_effort）+ 主题设置（macOS Light/Dark/Auto + 自定义 CSS）+ 快捷键设置 + 数据管理（导入/导出/清理）+ E2EE 设置（Double Ratchet 密钥管理）+ CRDT 设置（同步开关 / Peer 管理）+ 更新检查
- **改造点**：
  1. **NomiFun 设置页结构保留**：左侧 Tab 导航 + 右侧内容区；Tab 改为 SparkFox 6 大设置类别（模型/外观/快捷键/数据/安全/同步）
  2. **模型配置扩展**：新增 `reasoning_effort` 滑块（low/medium/high，默认 high）；新增 Agent 默认模型配置；新增 LLM Provider 测试连通性按钮
  3. **外观设置扩展**：新增 Apple 系统风格主题预设（macOS Light/Dark/Auto）；保留自定义 CSS；新增字体大小 / 行高 / 圆角微调
  4. **新增 E2EE 安全设置**：Double Ratchet 公钥/私钥对管理；会话密钥轮换周期；加密字段范围选择（全量/仅 L4 Persona/仅用户身份）
  5. **新增 CRDT 同步设置**：同步开关；Peer 节点管理（添加/删除/连接状态）；冲突解决策略选择（importance_score 取大 / content 取最新 / tags 取并集 / 手动解决）
  6. **新增 6 层记忆设置**：每层容量上限；遗忘策略开关（5 策略独立开关）；ReviewProgress 频率
  7. **数据管理扩展**：导入/导出 SQLite 数据库；导入/导出 CRDT 文档；清理 L0 感官层（保留 N 天）；归档冷存储
  8. **UpdateModal 保留**：NomiFun 的更新检查逻辑保留，改为 SparkFox 更新源
  9. **configService 改名**：`configService` → `sparkfox.config`；存储 key 前缀 `nomifun:` → `sparkfox:`
- **依赖**：模块 A（UI 框架）、模块 I（settingsStore）、后端 `crates/sparkfox-e2ee` + `sparkfox-crdt` + `sparkfox-memory`
- **目标文件路径**：`D:\xin kaifa\SparkFox\ui\src\renderer\views\SettingsView\`

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第三部分：关键取舍决策表
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

| 决策项 | 选择了什么 | 来自哪个项目 | 放弃了什么 | 放弃了哪个项目的 | 选择理由 | 放弃理由 |
|--------|-----------|-------------|-----------|-----------------|----------|----------|
| 桌面框架 | Tauri 2 进程内后端 | NomiFun + OpenAkita + Pangu Nebula | Electron 33 | BaiLongma | 体积小（<10MB）/ 内存低 / 安全模型严 / Rust 原生集成 / macOS WKWebView 纯粹 | 体积大（80-150MB）/ 内存高 / Node.js 集成与 AGPL 冲突 / Chromium 渲染与 Apple 风格有差异 |
| 前端框架 | React 19.1 + TypeScript | NomiFun + OpenAkita | Preact 10 | Pangu Nebula | 并发渲染支撑三路并发 / Arco Design 官方支持 / 生态最大 / ts-rs 集成成熟 | 缺并发原语 / Arco 不保证兼容 / 无法支撑 ThoughtStream 三区联动 |
| 前端 UI 库 | Arco Design React | NomiFun | shadcn/ui | OpenAkita | NomiFun 已用 / 60+ 企业组件 / 用户明确喜欢 NomiFun UI / 与 Apple 风格可定制兼容 | 与 Arco 双重依赖冲突 / 需要重写 OpenAkita 全部 22 views / 两套设计语言 |
| 样式方案 | UnoCSS 66 + Apple 主题 | NomiFun | Tailwind 3 单独 | Pangu Nebula | NomiFun 已用 / 原子化 + 主题定制强 / themeControlContract 契约机制 / preset 丰富 | 需要自己实现复杂组件 / Arco + UnoCSS 已覆盖 |
| 前端可视化 | @xyflow/react + D3 7.9 + Three.js | NomiFun + BaiLongma | ReactFlow 11 + @antv/g6 5 | Pangu Nebula | NomiFun 用 @xyflow（ReactFlow 升级版）/ BaiLongma 用 D3 + Three.js / 生态成熟 | ReactFlow 11 已升级为 @xyflow / @antv/g6 与 Apple 风格不搭 |
| 后端语言 | Rust 2024 edition | NomiFun | Python 3.11 | Pangu Nebula + OpenAkita | 性能好 / automerge-rs + Double Ratchet 原生 / 类型安全 / Tauri 进程内集成 / AGPL 清洁室友好 | 性能差（GIL）/ sidecar 启动慢 / 与 Rust CRDT/E2EE 需 FFI |
| 后端运行时 | axum + tokio（进程内） | NomiFun | FastAPI + uvicorn sidecar | Pangu Nebula + OpenAkita | 进程内无 IPC 序列化 / 延迟 <1ms / 启动 <500ms / 内存 50MB | sidecar 启动 2-3s / IPC 序列化开销 / 内存 200MB+ |
| 后端框架放弃 | Node.js ESM | - | Node.js + better-sqlite3 | BaiLongma | Rust 单语言 / Tauri 集成 / 类型安全 | 单线程回调 / better-sqlite3 阻塞事件循环 / 与 Tauri 不匹配 |
| 数据库 | SQLite + rusqlite + sqlx | NomiFun + BaiLongma | ChromaDB | OpenAkita | 本地优先 / 四项目三家用 SQLite / 无外部服务 / 事务 + FTS5 | 需要 Python 进程 / 额外服务依赖 / 与 Rust 单语言冲突 |
| 向量检索 | sqlite-vec | BaiLongma | ChromaDB | OpenAkita | 本地嵌入 / 无外部服务 / BaiLongma 已验证 / 与 SQLite 一体 | 需要 Python 进程 / 额外服务 / 与 Rust 单语言冲突 |
| 全文检索 | SQLite FTS5 | BaiLongma | Elasticsearch | - | SQLite 内置 / 无外部服务 / 性能足够 | 需要服务器进程 / 违反本地优先 / 资源占用大 |
| CRDT | automerge-rs | RFC-004 硬约束 | 自研方案 | Pangu Nebula | RFC-004 硬约束 / 成熟库 / Rust 原生 / 冲突解决完善 | 不可靠 / 维护成本高 / 无社区支持 |
| E2EE | Double Ratchet | RFC-004 硬约束 | 无 | 四项目均无 | RFC-004 硬约束 / 四项目都无 / 前向保密 / 信号协议成熟 | 四项目都没有 / 需要新实现 |
| 记忆体系 | 6 层 L0-L5（OpenAkita 三层扩展） | Pangu Nebula 蓝图 + OpenAkita 实现 | OpenAkita 原生 3 层 | OpenAkita | RFC-003 硬约束 / 6 层更完整 / L0 感官 + L5 元认知是独有能力 / 用户要求以 OpenAkita 为基础改造 | 3 层缺少 L0 感官 + L5 元认知 / 无法支撑元认知反思 / 不符合 RFC-003 |
| 记忆实现起点 | OpenAkita MemoryManager v2 | OpenAkita | NomiFun YAML frontmatter | NomiFun | 用户明确不喜欢 NomiFun 记忆 / OpenAkita 已有 UnifiedStore + RetrievalEngine + MemoryExtractor + MemoryConsolidator + ReviewProgress / 三层架构完整 | 用户明确不喜欢 / 仅 1 层 / 无自动提取 / 无 LLM 审查 |
| 元认知实现 | BaiLongma consciousness-loop Rust 化 | BaiLongma | 无 | - | BaiLongma 独有 Tick 心跳 + ACI 预判 + 委托发现 / L5 元认知最佳雏形 / 用户喜欢 BaiLongma 思考过程 | 四项目仅 BaiLongma 有 / 需 Rust 重写 |
| 线索模型 | BaiLongma Thread 温度窗口 | BaiLongma | OpenAkita Dynamic Memory | OpenAkita | 多并发线索 / 前台指针 / 承诺 / warm 6h cool 48h cold / L2 情景记忆天然实现 | 无并发线索 / 无温度窗口 / 无承诺机制 |
| 对话展示 | BaiLongma liveEl 流式气泡 | BaiLongma | NomiFun Chat 面板 | NomiFun | 双重去重（ID+内容+TTL 2min）/ 激活预热锁 / 自适应高度 / 按住空格说话 / 多渠道标签 / 用户明确喜欢 | 传统流式 / 无去重 / 无预热锁 / 无多渠道 |
| 思考过程 | BaiLongma ThoughtStream 三区联动 | BaiLongma | NomiFun plan mode | NomiFun | 57 工具中文映射 + 57 工具图标 / 三区联动（思考+工具+状态）/ Turn Trace / reasoning_effort=high / 用户明确喜欢 | 仅 plan mode / 无工具映射 / 无三区联动 / 无 Turn Trace |
| 热点追踪 | BaiLongma 4 平台 + 3D 地球 | BaiLongma | OpenAkita Proactive Engine | OpenAkita | 4 平台热榜 + 8 类别 + 3D 地球 + 跑马灯 + 中性上下文 / 用户明确喜欢 | 仅 trending 集成 / 无 3D 可视化 / 无跑马灯 |
| Agent 菜单 | OpenAkita AgentProfile 22 字段 + Sheet | OpenAkita | NomiFun Agent 引擎 | NomiFun | 22 字段最全 / Sheet 侧滑编辑 / AgentDashboard 力导向图 / Pixel Office / Agent Store / 用户明确喜欢 | 字段少 / 无侧滑编辑 / 无 Dashboard / 无 Store |
| 监视面板 | OpenAkita TokenStatsView 6 周期 5 维度 | OpenAkita | NomiFun Agent Dashboard | NomiFun | 6 周期 × 5 维度 / Timeline 柱状图 / Sessions 明细 / Records 流水 / 22 lazy views / 用户明确喜欢 | 仅 Agent 状态 / 无 Token 统计 / 无 Timeline |
| Agent 配置类型 | AgentProfile 22 字段扩展 | OpenAkita | NomiFun Agent 配置 | NomiFun | 字段最全 / 已验证 / 扩展成本低 | 字段少 / 不支持复杂配置 |
| 状态管理 | Zustand 5 + React Context 分层 | 全新设计 | 单一 Context | NomiFun | selector 订阅性能好 / 中间件丰富（persist/devtools/immer）/ TypeScript 友好 / 与 React 19 兼容 | 全树重渲染 / 无中间件 / 性能瓶颈 |
| 路由 | React Router 7 data mode | 全新设计 | React Router 6 | OpenAkita | data mode / lazy + Suspense / 深链接 / 历史记录 / 快捷键路由 | 非 data mode / API 旧 |
| 构建工具 | Vite 6 + cargo workspace + bun | NomiFun | webpack 5 / hatchling | OpenAkita + Pangu Nebula | HMR <100ms / ESM 原生 / cargo workspace / bun 快 10x / Tauri 官方集成 | 慢 / 配置复杂 / 与 ESM 兼容差 |
| 主题机制 | ThemeContext + themeControlContract | NomiFun | BaiLongma 3 主题硬编码 | BaiLongma | 契约机制 / 动态切换 / 自定义 CSS / 跨窗口同步 | 硬编码 / 无契约 / 无跨窗口同步 |
| 编排系统 | DAG（蜂群 + 组织编排合并） | Pangu Nebula + OpenAkita | 单一编排 | - | RFC-002 硬约束 / DAG 表达力强 / 蜂群动态扩缩容 / 组织编排角色清晰 | 单一编排表达力弱 / 无法支撑复杂任务 |
| 桌宠 | NomiFun Companion | NomiFun | BaiLongma 语音球 | BaiLongma | NomiFun 已实现 / 与 Arco 集成 / 用户喜欢 NomiFun UI | 仅语音球 / 无桌宠形态 |
| 类型同步 | ts-rs | NomiFun | 手写 TypeScript 类型 | OpenAkita + BaiLongma | Rust struct → TypeScript type 自动 / 避免手写错误 / NomiFun 已验证 | 手写易错 / 维护成本高 |
| 日志系统 | tracing + tracing-subscriber | NomiFun | log4rs / env_logger | - | tokio 原生 / 结构化日志 / span 链路追踪 / NomiFun 已用 | 功能弱 / 无 span / 无链路追踪 |
| 测试框架 | cargo test + pytest（仅 PoC）+ vitest | NomiFun + OpenAkita | 仅 cargo test | - | Rust 单元测试 + 前端 vitest + PoC 阶段 pytest / 覆盖全 | 覆盖不全 |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第四部分：数据流设计
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 数据流 1：用户输入 → AI 对话 → 展示

- **数据从哪到哪**：用户在 ChatView 输入框 → chatStore → IPC → Rust 后端 sparkfox-chat → LLM Provider → SSE 流式回传 → chatStore → ChatView 渲染
- **传递什么数据**：
  - 输入：`{ session_id, content, attachments, agent_id, hotspot_context? }`
  - 输出：SSE 事件流（`thinking` / `tool_call` / `tool_result` / `text_delta` / `done` / `error`）
- **通过什么方式传递**：
  - 前端 → chatStore：Zustand `useChatStore.send()`
  - chatStore → 后端：Tauri IPC `sparkfox.ipc.chat.send()` → axum POST `/api/chat/sessions/<id>/messages`
  - 后端 → LLM：HTTP SSE（OpenAI 兼容协议）
  - 后端 → 前端：SSE event stream（`EventSource` 或 `fetch` + ReadableStream）
- **涉及哪些文件**：
  - 前端：`ui/src/renderer/components/chat/SendBox.tsx` / `ChatMessages.tsx` / `useChatStream.ts`
  - store：`ui/src/renderer/store/chatStore.ts`
  - IPC：`ui/src/common/ipcBridge.ts`
  - 后端：`crates/sparkfox-chat/src/handler.rs` / `crates/sparkfox-llm/src/provider.rs`
- **伪代码示例**：
```typescript
// ChatView.tsx
const send = useChatStore(s => s.send);
const messages = useChatStore(s => s.messages);

const handleSend = async (content: string, attachments: Attachment[]) => {
  const sessionId = useChatStore.getState().currentSessionId;
  const agentId = useAgentStore.getState().currentAgentId;
  const hotspotCtx = useHotspotStore.getState().neutralContext; // 可选

  // 1. 乐观更新：立即显示用户消息
  useChatStore.getState().appendMessage({ role: 'user', content, attachments });

  // 2. 调用后端
  const stream = await sparkfox.ipc.chat.send({
    session_id: sessionId,
    content,
    attachments,
    agent_id: agentId,
    hotspot_context: hotspotCtx,
  });

  // 3. 流式渲染
  for await (const event of stream) {
    switch (event.type) {
      case 'thinking':
        useThinkingStore.getState().appendThinking(event.text);
        break;
      case 'tool_call':
        useThinkingStore.getState().appendToolCall(event.tool, event.args);
        break;
      case 'tool_result':
        useThinkingStore.getState().appendToolResult(event.tool, event.result, event.is_failure);
        break;
      case 'text_delta':
        useChatStore.getState().appendDelta(event.delta);
        break;
      case 'done':
        useChatStore.getState().finalizeMessage();
        break;
      case 'error':
        useChatStore.getState().setError(event.error);
        break;
    }
  }
};
```

```rust
// crates/sparkfox-chat/src/handler.rs
pub async fn send_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): ChatSendRequest,
) -> impl IntoResponse {
    let stream = async_stream::stream! {
        // 1. L1 Scratchpad 注入
        let scratchpad = state.memory.scratchpad_snapshot(&session_id).await;

        // 2. L2-L4 记忆检索
        let memories = state.memory.retrieval_engine
            .retrieve(&req.content, &req.agent_id, 10)
            .await;

        // 3. 构造 system prompt
        let system = build_system_prompt(&scratchpad, &memories, &req.hotspot_context);

        // 4. LLM 调用（reasoning_effort=high）
        let mut llm_stream = state.llm.provider
            .chat_stream(&system, &req.content, ReasoningEffort::High)
            .await;

        // 5. 流式转发
        while let Some(chunk) = llm_stream.next().await {
            match chunk {
                LlmChunk::Thinking(t) => yield Event::thinking(t),
                LlmChunk::ToolCall(tool, args) => {
                    yield Event::tool_call(&tool, &args);
                    let result = state.agent.executor.execute(&tool, &args).await;
                    yield Event::tool_result(&tool, &result, result.is_failure());
                }
                LlmChunk::TextDelta(delta) => yield Event::text_delta(delta),
                LlmChunk::Done => {
                    // 6. 对话完成 → 触发记忆提取
                    state.memory.extractor.extract_async(&session_id).await;
                    yield Event::done();
                }
                LlmChunk::Error(e) => yield Event::error(e.to_string()),
            }
        }
    };

    Sse::new(stream)
}
```

---

### 数据流 2：对话 → 记忆存储

- **数据从哪到哪**：对话 turn 完成 → MemoryExtractor 自动提取 → UnifiedStore 写入 → MemoryConsolidator 后台整合 → sqlite-vec 向量化 → MemoryView 可视化
- **传递什么数据**：
  - 输入：`{ session_id, turns: [ConversationTurn], agent_id }`
  - 输出：新记忆条目（L2 情景 / L3 语义 / L4 程序）+ ReviewProgress 状态
- **通过什么方式传递**：
  - 后端内部：`tokio::spawn` 异步任务（不阻塞对话流）
  - 后端 → 前端：SSE 推送 `memory_extracted` 事件（可选订阅）
  - 前端：`useMemoryStore` 监听并刷新
- **涉及哪些文件**：
  - 后端：`crates/sparkfox-memory/src/extractor.rs` / `consolidator.rs` / `unified_store.rs`
  - 前端：`ui/src/renderer/store/memoryStore.ts`
- **伪代码示例**：
```rust
// crates/sparkfox-memory/src/extractor.rs
pub async fn extract_async(&self, session_id: &str) {
    let turns = self.store.get_recent_turns(session_id, 5).await;

    // 1. LLM 辅助提取（每 5 个 turn 触发一次）
    let prompt = build_extraction_prompt(&turns);
    let extracted: Vec<ExtractedMemory> = self.llm.extract(&prompt).await;

    // 2. 重要性预判（LLM 同时打分 0-1）
    for mut mem in extracted {
        mem.importance_score = self.llm.score_importance(&mem).await;

        // 3. 写入 UnifiedStore（按类型路由到 L2/L3/L4）
        let layer = match mem.r#type {
            MemoryType::Context => MemoryLayer::L2Episodic,
            MemoryType::Fact | MemoryType::Preference | MemoryType::Skill |
            MemoryType::Rule | MemoryType::Error | MemoryType::Experience |
            MemoryType::PersonaTrait => MemoryLayer::L3Semantic,
        };
        self.store.write(mem, layer).await;

        // 4. sqlite-vec 向量化
        let embedding = self.embedder.embed(&mem.content).await;
        self.store.write_vector(mem.id, &embedding).await;

        // 5. CRDT 同步
        self.crdt.record_change(mem.id, Change::Insert(mem)).await;

        // 6. 推送前端
        self.event_bus.emit(MemoryEvent::Extracted(mem));
    }

    // 7. 触发后台整合
    self.consolidator.consolidate_async().await;
}
```

```typescript
// memoryStore.ts
export const useMemoryStore = create<MemoryState>((set, get) => ({
  memories: [],
  reviewProgress: { status: 'idle' },

  subscribeEvents: () => {
    // 订阅后端 SSE 事件
    sparkfox.ipc.memory.subscribe((event) => {
      if (event.type === 'extracted') {
        set((s) => ({ memories: [event.memory, ...s.memories] }));
      } else if (event.type === 'review_progress') {
        set({ reviewProgress: event.progress });
      }
    });
  },
}));
```

---

### 数据流 3：记忆 → 对话注入

- **数据从哪到哪**：新 turn 开始 → RetrievalEngine 检索（L1 Scratchpad + L2-L4 Core+Dynamic）→ builder 构造 system prompt → LLM 调用
- **传递什么数据**：
  - 输入：`{ user_query, session_id, agent_id, top_k: 10 }`
  - 输出：`{ scratchpad, core_memories: [Memory], dynamic_memories: [Memory], persona: SOUL.md }`
- **通过什么方式传递**：
  - 后端内部：函数调用（同步，在 LLM 调用前）
  - 前端展示：`useMemoryStore.getInjectedMemories()` 读取最近注入的记忆
- **涉及哪些文件**：
  - 后端：`crates/sparkfox-memory/src/retrieval.rs` / `builder.rs`
  - 前端：`ui/src/renderer/components/chat/InjectedMemoriesBadge.tsx`
- **伪代码示例**：
```rust
// crates/sparkfox-memory/src/retrieval.rs
pub async fn retrieve(
    &self,
    query: &str,
    agent_id: &str,
    top_k: usize,
) -> RetrievedMemories {
    // 1. L1 Scratchpad（contextvars 等价物）
    let scratchpad = self.scratchpad.snapshot().await;

    // 2. 混合检索（向量 0.5 + FTS5 0.3 + 图谱 0.2）
    let (vec_results, fts_results, graph_results) = tokio::join!(
        self.vector_search(query, top_k * 2),
        self.fts_search(query, top_k * 2),
        self.graph_search(query, top_k * 2),
    );

    let merged = self.merge_results(vec_results, fts_results, graph_results, top_k);

    // 3. 按层分类
    let core: Vec<_> = merged.iter().filter(|m| m.layer == L3Semantic).collect();
    let dynamic: Vec<_> = merged.iter().filter(|m| m.layer == L2Episodic).collect();

    // 4. L4 Persona
    let persona = self.persona.get(agent_id).await;

    RetrievedMemories { scratchpad, core, dynamic, persona }
}
```

```typescript
// InjectedMemoriesBadge.tsx
export function InjectedMemoriesBadge() {
  const injectedCount = useChatStore(s => s.currentInjectedCount);

  if (injectedCount === 0) return null;

  return (
    <ArcoBadge count={injectedCount} offset={[-4, 0]}>
      <ArcoTag color="arc-green" bordered>
        使用 {injectedCount} 条记忆
      </ArcoTag>
    </ArcoBadge>
  );
}
```

---

### 数据流 4：Agent 选择 → Agent 执行 → 监视面板

- **数据从哪到哪**：AgentManagerView 选择 Agent → agentStore → Orchestrator 调度 → DAG 编排 → TokenStats 实时记录 → MonitorView 显示
- **传递什么数据**：
  - 选择：`{ agent_id, task }`
  - 执行：DAG 节点状态（idle/running/completed/error/dormant）+ Token 用量
  - 监视：`{ TopoNode[], TopoEdge[], TokenStats, SessionRow[] }`
- **通过什么方式传递**：
  - 前端 → 后端：IPC `sparkfox.ipc.agent.dispatch()`
  - 后端内部：`Orchestrator` + 蜂群 Worker
  - 后端 → 前端：SSE 推送 `topo_update` / `token_stats_update`
- **涉及哪些文件**：
  - 前端：`ui/src/renderer/views/AgentView/AgentManagerView.tsx` / `ui/src/renderer/views/MonitorView/AgentDashboardPanel.tsx` / `ui/src/renderer/store/agentStore.ts` / `monitorStore.ts`
  - 后端：`crates/sparkfox-orchestrator/src/dag.rs` / `crates/sparkfox-monitor/src/collector.rs`
- **伪代码示例**：
```typescript
// AgentManagerView.tsx
const dispatch = useAgentStore(s => s.dispatch);

const handleDispatch = async (agentId: string, task: string) => {
  await sparkfox.ipc.agent.dispatch({ agent_id: agentId, task });

  // 监视面板自动订阅 SSE
  useMonitorStore.getState().subscribeTopo(agentId);
};
```

```rust
// crates/sparkfox-orchestrator/src/dag.rs
pub async fn dispatch(&self, agent_id: &str, task: &str) {
    // 1. 解析任务为 DAG
    let dag = self.planner.plan(task, agent_id).await;

    // 2. 蜂群 Worker 分配
    let workers = self.swarm.assign(&dag).await;

    // 3. 实时推送 TopoNode 状态
    for node in dag.nodes {
        self.event_bus.emit(TopoEvent::NodeUpdated(TopoNode {
            id: node.id,
            profile_id: agent_id.to_string(),
            status: NodeStatus::Running,
            // ...
        }));

        // 4. 执行节点
        let result = self.swarm.execute(&node).await;

        // 5. Token 统计
        self.monitor.record_token_usage(&result.token_usage).await;

        // 6. 推送完成
        self.event_bus.emit(TopoEvent::NodeUpdated(TopoNode {
            status: NodeStatus::Completed,
            // ...
        }));
    }
}
```

---

### 数据流 5：思考过程 → 对话展示

- **数据从哪到哪**：LLM 流式输出 → SSE event:thinking → thinkingStore → ThoughtStreamPanel 渲染 → 嵌入到 ChatView 对话气泡内
- **传递什么数据**：
  - thinking 事件：`{ text, timestamp }`
  - tool_call 事件：`{ tool, args, timestamp }`
  - tool_result 事件：`{ tool, result, is_failure, timestamp }`
- **通过什么方式传递**：
  - 后端 → 前端：SSE 事件流（与数据流 1 共享）
  - 前端：thinkingStore 接收 + ThoughtStreamPanel 渲染
- **涉及哪些文件**：
  - 前端：`ui/src/renderer/components/thinking/ThoughtStreamPanel.tsx` / `useThoughtStream.ts` / `ui/src/renderer/store/thinkingStore.ts`
  - 常量：`ui/src/renderer/components/thinking/toolMap.ts`（TOOL_ZH + TOOL_ICON）
- **伪代码示例**：
```typescript
// thinkingStore.ts
export const useThinkingStore = create<ThinkingState>((set) => ({
  thinkingLines: [],
  currentTool: null,
  toolHistory: [],
  hadToolCall: false,
  toolFailed: false,
  startedAt: 0,

  appendThinking: (text: string) =>
    set((s) => ({
      thinkingLines: [...s.thinkingLines, { text, ts: Date.now() }],
      startedAt: s.startedAt || Date.now(),
    })),

  appendToolCall: (tool: string, args: any) =>
    set((s) => ({
      currentTool: { tool, args, ts: Date.now() },
      hadToolCall: true,
    })),

  appendToolResult: (tool: string, result: any, isFailure: boolean) =>
    set((s) => ({
      currentTool: null,
      toolHistory: [
        ...s.toolHistory,
        { tool, result, isFailure, ts: Date.now() },
      ],
      toolFailed: s.toolFailed || isFailure,
    })),
}));

// ThoughtStreamPanel.tsx
export function ThoughtStreamPanel() {
  const { thinkingLines, currentTool, toolHistory, startedAt } = useThinkingStore();

  return (
    <div className="thought-stream">
      {/* 思考区 */}
      <div className="thinking-el">
        {thinkingLines.map((line, i) => (
          <div key={i} className="thinking-line">
            <span className="ts">{formatTs(line.ts)}</span>
            <span className="text">{line.text}</span>
          </div>
        ))}
      </div>

      {/* 工具区 */}
      <div className="last-tool-el">
        {currentTool && (
          <div className="tool-call">
            <span className="icon">{TOOL_ICON[currentTool.tool]}</span>
            <span className="name">{TOOL_ZH[currentTool.tool]}</span>
            <span className="args">{JSON.stringify(currentTool.args).slice(0, 160)}</span>
          </div>
        )}
      </div>

      {/* 状态区 */}
      <div className="status-el">
        <span className={toolFailed ? 'failed' : 'ok'}>
          {toolFailed ? '❌ 失败' : '✓ 完成'}
        </span>
        <span className="elapsed">{Date.now() - startedAt}ms</span>
      </div>
    </div>
  );
}
```

---

### 数据流 6：热点获取 → 热点展示 → 对话讨论

- **数据从哪到哪**：后端定时器 30min → sparkfox-hotspot 抓取（8 平台）→ hotspotStore → HotspotView 渲染（3D 地球 + 列表 + 跑马灯）→ 用户点击讨论 → buildHotspotContext 构建中性上下文 → 注入到 ChatView
- **传递什么数据**：
  - 热点列表：`HotspotItem[]`（`{ id, platform, rank, title, hot_value, trend, url, category, fetched_at }`）
  - 中性上下文：`{ context: string, source: string, fetched_at: string }`
- **通过什么方式传递**：
  - 后端定时器 → 后端：`tokio::time::interval`（30min）
  - 后端 → 前端：SSE 推送 `hotspot_update`
  - 前端：hotspotStore 接收 + HotspotView 渲染
  - 用户点击 → 后端：IPC `sparkfox.ipc.hotspot.discuss()`
  - 后端 → 前端：注入到 chatStore
- **涉及哪些文件**：
  - 后端：`crates/sparkfox-hotspot/src/fetcher.rs` / `crates/sparkfox-hotspot/src/context.rs`
  - 前端：`ui/src/renderer/views/HotspotView/HotspotView.tsx` / `ui/src/renderer/components/hotspot/HotspotEarth.tsx` / `ui/src/renderer/components/hotspot/HotspotList.tsx` / `ui/src/renderer/store/hotspotStore.ts`
- **伪代码示例**：
```rust
// crates/sparkfox-hotspot/src/fetcher.rs
pub async fn start_fetcher(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));

    loop {
        interval.tick().await;

        // 1. 并发抓取 8 平台
        let platforms = ["douyin", "xiaohongshu", "wechat", "weibo",
                         "zhihu", "bilibili", "hn", "reddit"];

        let results = futures::future::join_all(
            platforms.iter().map(|p| fetch_platform(p))
        ).await;

        // 2. 合并 + 去重 + 分类
        let items = merge_and_classify(results);

        // 3. 写入 L0 感官层 + L2 情景记忆
        for item in &items {
            state.memory.sensory.capture_hotspot(item).await;
        }

        // 4. 推送前端
        state.event_bus.emit(HotspotEvent::Updated(items));

        // 5. 更新元数据
        state.hotspot.update_meta(HotspotMeta {
            source: "multi".into(),
            fetched_at: Utc::now(),
            stale: false,
            refresh_minutes: 30,
            status: "ok".into(),
        });
    }
}

// crates/sparkfox-hotspot/src/context.rs
pub fn build_neutral_context(items: &[HotspotItem], _agent_id: &str) -> String {
    // BaiLongma 原始逻辑保留：不要求主动总结，不要把它当成用户消息，仅在相关时提及
    let top = items.iter().take(5).map(|i| format!("- {}", i.title)).collect::<Vec<_>>().join("\n");

    format!(
        "【热点参考上下文】\n以下是当前热点，仅供你了解上下文，不要主动总结，不要把它当成用户消息。仅在用户问题相关时提及。\n{}\n",
        top
    )
}
```

```typescript
// HotspotView.tsx
export function HotspotView() {
  const items = useHotspotStore(s => s.items);
  const discuss = useHotspotStore(s => s.discuss);

  const handleDiscuss = (item: HotspotItem) => {
    // 1. 构建中性上下文
    const ctx = buildNeutralContext([item]);

    // 2. 注入到 chatStore
    useChatStore.getState().setHotspotContext(ctx);

    // 3. 跳转到对话页
    navigate('/');
  };

  return (
    <div className="hotspot-view">
      <HotspotEarth items={items} onSelect={handleDiscuss} />
      <HotspotList items={items} onDiscuss={handleDiscuss} />
      <HotspotTicker items={items} />
    </div>
  );
}
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第五部分：页面 / 视图规划
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

| 页面名称 | 路由路径 | 主要功能 | 核心组件来源 | 依赖模块 | 优先级 |
|---------|---------|---------|-------------|---------|--------|
| 主对话页 | `/` | 对话 + 思考过程 + 记忆注入 + 热点上下文 | BaiLongma E + F + OpenAkita D 注入 | A, D, E, F, I | P0 |
| Agent 管理页 | `/agents` | Agent 菜单 + 配置 22 字段 + 创建 + Dashboard 力导向图 + Pixel Office + Agent Store | OpenAkita B | A, B, I | P1 |
| 监视面板页 | `/monitor` | Token 统计 6 周期 5 维度 + Timeline + Sessions + CRDT 同步 + E2EE 安全 + 6 层记忆健康 + 蜂群 DAG | OpenAkita C + 新增面板 | A, B, C, I | P1 |
| 热点追踪页 | `/hotspot` | 8 平台热榜 + 3D 地球 + 列表 + 跑马灯 + 中性上下文 + 一键讨论 | BaiLongma G | A, D, G, I | P2 |
| 记忆管理页 | `/memory` | 查看 / 编辑 / 搜索 / 删除记忆 + L0-L5 层级筛选 + MemoryGraph3D + ReviewProgress + MigrationStatus | OpenAkita D UI | A, D, I | P2 |
| 设置页 | `/settings` | 模型 / 外观 / 快捷键 / 数据 / E2EE 安全 / CRDT 同步 / 6 层记忆 / 更新检查 | NomiFun J + 新增 | A, J, I | P1 |
| 设置-模型子页 | `/settings/models` | LLM Provider / API Key / 模型选择 / 温度 / reasoning_effort / 连通性测试 | NomiFun J | A, J | P1 |
| 设置-外观子页 | `/settings/appearance` | macOS Light/Dark/Auto 主题 / 自定义 CSS / 字体 / 圆角 | NomiFun J + Apple 预设 | A, J | P1 |
| 设置-安全子页 | `/settings/security` | Double Ratchet 密钥管理 / 会话密钥轮换 / 加密字段范围 | 全新设计 | A, J | P2 |
| 设置-同步子页 | `/settings/sync` | CRDT 同步开关 / Peer 节点管理 / 冲突解决策略 | 全新设计 | A, J | P2 |
| 设置-数据子页 | `/settings/data` | 导入 / 导出 SQLite / 导入 / 导出 CRDT 文档 / 清理 L0 / 归档冷存储 | 全新设计 | A, J | P2 |

**优先级定义**：
- **P0**：Phase 0 MVP 必须完成（主对话页）
- **P1**：Phase 1 必须完成（Agent 管理 / 监视面板 / 设置）
- **P2**：Phase 2 完成（热点 / 记忆管理 / 安全 / 同步 / 数据）

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第六部分：最终目录结构规划
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

基于 RFC-001（14 crate 边界）+ NomiFun 50 crate workspace 组织模式，SparkFox 最终目录结构如下：

```
D:\xin kaifa\SparkFox\
├── apps/                                        → 应用层
│   └── desktop/                                 → Tauri 2 桌面应用
│       ├── src/
│       │   ├── main.rs                          → Tauri 进程入口（NomiFun 蓝本）
│       │   └── lib.rs                           → 应用初始化 + axum 路由挂载
│       ├── tauri.conf.json                      → Tauri 配置（NomiFun 蓝本）
│       ├── Cargo.toml
│       └── icons/                               → 应用图标
│
├── crates/                                      → Rust workspace（14 crates，RFC-001）
│   ├── sparkfox-core/                           → 核心类型与接口
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types.rs                         → 全局类型（ts-rs 导出）
│   │   │   ├── scene.rs                         → Scene Protocol（BaiLongma 蓝本）
│   │   │   ├── error.rs                         → 统一错误类型
│   │   │   └── config.rs                        → 配置定义
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-memory/                         → 6 层记忆系统（L0-L5）
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── manager.rs                       → MemoryManager（OpenAkita 蓝本）
│   │   │   ├── sensory.rs                       → L0 感官层（新增）
│   │   │   ├── scratchpad.rs                    → L1 工作记忆（OpenAkita Layer 1）
│   │   │   ├── episodic.rs                      → L2 情景记忆（OpenAkita Dynamic 扩展）
│   │   │   ├── threads.rs                       → Thread 线索模型（BaiLongma 蓝本）
│   │   │   ├── semantic.rs                      → L3 语义记忆（OpenAkita Core）
│   │   │   ├── procedure.rs                     → L4 程序记忆（OpenAkita Persona+Identity）
│   │   │   ├── persona.rs                       → SOUL.md/AGENT.md/USER.md/MEMORY.md/POLICIES.yaml
│   │   │   ├── meta_cognitive.rs                → L5 元认知横向平面（BaiLongma consciousness-loop 蓝本）
│   │   │   ├── retrieval.rs                     → RetrievalEngine（混合检索）
│   │   │   ├── extractor.rs                     → MemoryExtractor（LLM 辅助）
│   │   │   ├── consolidator.rs                  → MemoryConsolidator（后台整合）
│   │   │   ├── unified_store.rs                 → UnifiedStore（路由到 L0-L5）
│   │   │   ├── forgetting.rs                    → 5 策略遗忘机制（新增）
│   │   │   ├── review.rs                        → ReviewProgress 状态机（OpenAkita 蓝本）
│   │   │   └── types.rs                         → Memory/Attachment/ConversationTurn 等
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-orchestrator/                   → DAG 编排（蜂群 + 组织编排）
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── dag.rs                           → DAG 编排（RFC-002）
│   │   │   ├── swarm.rs                         → 蜂群 Worker（Pangu Nebula 蓝本）
│   │   │   ├── org.rs                           → 组织编排 CEO/CTO/CFO（OpenAkita 蓝本）
│   │   │   ├── planner.rs                       → 任务分解为 DAG
│   │   │   └── topo.rs                          → TopoNode/TopoEdge 推送
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-agent/                          → Agent 引擎
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── engine.rs                        → Agent 引擎（NomiFun nomi-agent 蓝本）
│   │   │   ├── executor.rs                      → 工具执行器（BaiLongma executor 蓝本）
│   │   │   ├── consciousness.rs                 → Tick 心跳 + ACI 预判（BaiLongma 蓝本）
│   │   │   ├── tick.rs                          → Tick 策略（BaiLongma 蓝本）
│   │   │   └── profile.rs                       → AgentProfile 22+ 字段（OpenAkita 蓝本）
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-chat/                           → 对话引擎
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── handler.rs                       → 对话处理（SSE 流式）
│   │   │   ├── session.rs                       → 会话管理
│   │   │   ├── dedupe.rs                        → 双重去重（BaiLongma 蓝本）
│   │   │   ├── warmup.rs                        → 激活预热锁（BaiLongma 蓝本）
│   │   │   └── builder.rs                       → system prompt 构造（记忆注入）
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-thinking/                       → 思考过程流
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── stream.rs                        → ThoughtStream（BaiLongma 蓝本）
│   │   │   ├── tool_map.rs                      → TOOL_ZH + TOOL_ICON（BaiLongma 57 + SparkFox 扩展）
│   │   │   └── trace.rs                         → Turn Trace 回合级轨迹
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-hotspot/                        → 热点追踪
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── fetcher.rs                       → 8 平台抓取（BaiLongma 4 平台扩展）
│   │   │   ├── context.rs                       → buildNeutralContext（BaiLongma 蓝本）
│   │   │   ├── trending.rs                      → 国内外分区（BaiLongma 蓝本）
│   │   │   └── meta.rs                          → HotspotMeta
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-monitor/                        → 监视数据收集
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── collector.rs                     → Token 统计（OpenAkita 蓝本）
│   │   │   ├── token_stats.rs                   → 6 周期 5 维度
│   │   │   ├── session_stats.rs                 → Sessions 明细
│   │   │   └── health.rs                        → 6 层记忆健康 + CRDT + E2EE 状态
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-crdt/                           → automerge-rs CRDT 封装
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── doc.rs                           → MemoryDoc 封装
│   │   │   ├── sync.rs                          → 跨设备同步
│   │   │   └── conflict.rs                      → 冲突解决策略
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-e2ee/                           → Double Ratchet E2EE
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ratchet.rs                       → Double Ratchet 实现
│   │   │   ├── session.rs                       → 加密会话管理
│   │   │   └── field_encrypt.rs                 → 字段级加密
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-store/                          → SQLite + sqlite-vec 存储
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── sqlite.rs                        → rusqlite + sqlx 封装
│   │   │   ├── vector.rs                        → sqlite-vec 向量检索
│   │   │   ├── fts.rs                           → FTS5 全文检索
│   │   │   ├── migrations/                      → 数据库迁移脚本
│   │   │   │   ├── 001_init.sql
│   │   │   │   ├── 002_memory_layers.sql
│   │   │   │   └── 003_crdt_e2ee.sql
│   │   │   └── schema.rs                        → Schema 定义
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-ipc/                            → Tauri IPC 桥
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── bridge.rs                        → IPC 命令注册
│   │   │   ├── sse.rs                           → SSE 事件流
│   │   │   └── commands/                        → IPC 命令模块
│   │   │       ├── chat.rs
│   │   │       ├── agent.rs
│   │   │       ├── memory.rs
│   │   │       ├── monitor.rs
│   │   │       ├── hotspot.rs
│   │   │       └── settings.rs
│   │   └── Cargo.toml
│   │
│   ├── sparkfox-llm/                            → LLM Provider 抽象
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── provider.rs                      → Provider trait
│   │   │   ├── openai.rs                        → OpenAI 兼容
│   │   │   ├── anthropic.rs                     → Claude
│   │   │   ├── gemini.rs                        → Gemini
│   │   │   ├── bedrock.rs                       → AWS Bedrock
│   │   │   └── reasoning.rs                     → reasoning_effort 抽象
│   │   └── Cargo.toml
│   │
│   └── sparkfox-security/                       → 11 安全栈
│       ├── src/
│       │   ├── lib.rs
│       │   ├── sandbox.rs                       → 6 层沙箱（OpenAkita 蓝本）
│       │   ├── capability.rs                    → 能力授权
│       │   ├── audit.rs                         → 审计日志
│       │   └── rate_limit.rs                    → 速率限制
│       └── Cargo.toml
│
├── ui/                                          → React 19 前端
│   ├── src/
│   │   ├── main.tsx                             → React 入口
│   │   ├── App.tsx                              → 应用根组件 + RouterProvider
│   │   ├── common/                              → 通用类型与 IPC 桥（NomiFun 蓝本）
│   │   │   ├── ipcBridge.ts                     → Tauri IPC 桥
│   │   │   ├── types/                           → ts-rs 自动生成类型
│   │   │   │   └── generated.ts
│   │   │   ├── config/
│   │   │   │   └── configService.ts             → 配置服务（NomiFun 蓝本）
│   │   │   └── utils/
│   │   ├── platform/                            → 平台适配（NomiFun 蓝本）
│   │   │   └── apiUrl.ts
│   │   ├── renderer/
│   │   │   ├── components/
│   │   │   │   ├── layout/                      → 模块 A：前端 UI 框架
│   │   │   │   │   ├── Layout.tsx               → 主布局（NomiFun 蓝本）
│   │   │   │   │   ├── Titlebar.tsx             → 顶栏（NomiFun 蓝本）
│   │   │   │   │   ├── Sider.tsx                → 侧边栏（NomiFun 蓝本 + SparkFox 导航）
│   │   │   │   │   ├── PwaPullToRefresh.tsx     → PWA 下拉刷新
│   │   │   │   │   └── InstantHoverTooltip.tsx  → 即时悬停提示
│   │   │   │   ├── base/                        → 基础组件
│   │   │   │   │   ├── Button.tsx
│   │   │   │   │   ├── Card.tsx
│   │   │   │   │   ├── Input.tsx
│   │   │   │   │   └── ...
│   │   │   │   ├── chat/                        → 模块 E：对话展示组件
│   │   │   │   │   ├── ChatMessages.tsx         → 消息列表（BaiLongma 蓝本）
│   │   │   │   │   ├── SendBox.tsx              → 输入框（BaiLongma autoGrowInput）
│   │   │   │   │   ├── MessageBubble.tsx        → 消息气泡（Apple 风格）
│   │   │   │   │   ├── LiveStream.tsx           → 流式气泡（BaiLongma liveEl）
│   │   │   │   │   ├── InjectedMemoriesBadge.tsx
│   │   │   │   │   ├── MultiChannelLabel.tsx    → 多渠道标签
│   │   │   │   │   └── hooks/
│   │   │   │   │       ├── useChatStream.ts     → SSE 流式 hook
│   │   │   │   │       ├── useAutoGrowInput.ts  → 自适应高度
│   │   │   │   │       ├── usePushToTalk.ts     → 按住空格说话
│   │   │   │   │       └── useAudioContext.ts   → 音频上下文
│   │   │   │   ├── thinking/                    → 模块 F：思考过程可视化
│   │   │   │   │   ├── ThoughtStreamPanel.tsx   → 三区联动面板
│   │   │   │   │   ├── TurnTraceAccordion.tsx   → Turn Trace 折叠
│   │   │   │   │   ├── toolMap.ts               → TOOL_ZH + TOOL_ICON
│   │   │   │   │   └── hooks/
│   │   │   │   │       └── useThoughtStream.ts
│   │   │   │   ├── agent/                       → 模块 B：Agent 菜单系统
│   │   │   │   │   ├── AgentManagerPanel.tsx    → Agent 管理面板（OpenAkita 蓝本）
│   │   │   │   │   ├── AgentProfileEditor.tsx   → Sheet 侧滑编辑（OpenAkita 蓝本）
│   │   │   │   │   ├── AgentDashboard.tsx       → 力导向图（OpenAkita 蓝本）
│   │   │   │   │   ├── AgentStore.tsx           → Agent 市场
│   │   │   │   │   ├── PixelOffice.tsx          → 像素办公室
│   │   │   │   │   ├── AgentIcon.tsx            → Agent 图标
│   │   │   │   │   └── types.ts                 → AgentProfile 22+ 字段
│   │   │   │   ├── monitor/                     → 模块 C：监视面板
│   │   │   │   │   ├── TokenStatsPanel.tsx      → Token 统计（OpenAkita 蓝本）
│   │   │   │   │   ├── TimelineChart.tsx        → Timeline 柱状图
│   │   │   │   │   ├── SessionsTable.tsx        → Sessions 明细
│   │   │   │   │   ├── CrdtSyncPanel.tsx        → CRDT 同步状态（新增）
│   │   │   │   │   ├── E2eeSecurityPanel.tsx    → E2EE 安全面板（新增）
│   │   │   │   │   ├── MemoryHealthPanel.tsx    → 6 层记忆健康（新增）
│   │   │   │   │   └── SwarmDagPanel.tsx        → 蜂群 DAG 可视化
│   │   │   │   ├── hotspot/                     → 模块 G：热点追踪
│   │   │   │   │   ├── HotspotEarth.tsx         → 3D 地球（BaiLongma 蓝本）
│   │   │   │   │   ├── HotspotList.tsx          → 热点列表
│   │   │   │   │   ├── HotspotTicker.tsx        → 跑马灯
│   │   │   │   │   └── HotspotTrendChart.tsx    → 7 天趋势图
│   │   │   │   ├── memory/                      → 模块 D UI：记忆管理
│   │   │   │   │   ├── MemoryList.tsx           → 记忆列表（OpenAkita 蓝本）
│   │   │   │   │   ├── MemoryGraph3D.tsx        → 3D 图谱（OpenAkita 蓝本）
│   │   │   │   │   ├── MemoryGraph2D.tsx        → D3 力导向 2D（BaiLongma 风格）
│   │   │   │   │   ├── LayerFilter.tsx          → L0-L5 层级筛选
│   │   │   │   │   ├── ReviewProgressIndicator.tsx
│   │   │   │   │   └── MigrationBanner.tsx
│   │   │   │   ├── settings/                    → 模块 J：配置与设置
│   │   │   │   │   ├── SettingsLayout.tsx       → 设置布局（NomiFun 蓝本）
│   │   │   │   │   ├── ModelSettings.tsx        → 模型配置
│   │   │   │   │   ├── AppearanceSettings.tsx   → 外观（Apple 主题）
│   │   │   │   │   ├── ShortcutSettings.tsx     → 快捷键
│   │   │   │   │   ├── DataSettings.tsx         → 数据管理
│   │   │   │   │   ├── SecuritySettings.tsx     → E2EE 安全
│   │   │   │   │   ├── SyncSettings.tsx         → CRDT 同步
│   │   │   │   │   ├── MemorySettings.tsx       → 6 层记忆设置
│   │   │   │   │   └── UpdateModal.tsx          → 更新检查（NomiFun 蓝本）
│   │   │   │   └── companion/                   → 桌宠（NomiFun 蓝本）
│   │   │   │       └── Companion.tsx
│   │   │   ├── views/                           → 页面视图
│   │   │   │   ├── ChatView/
│   │   │   │   │   ├── index.tsx                → 主对话页
│   │   │   │   │   └── ChatView.tsx
│   │   │   │   ├── AgentView/
│   │   │   │   │   ├── index.tsx                → Agent 管理页
│   │   │   │   │   └── AgentView.tsx
│   │   │   │   ├── MonitorView/
│   │   │   │   │   ├── index.tsx                → 监视面板页
│   │   │   │   │   └── MonitorView.tsx
│   │   │   │   ├── HotspotView/
│   │   │   │   │   ├── index.tsx                → 热点追踪页
│   │   │   │   │   └── HotspotView.tsx
│   │   │   │   ├── MemoryView/
│   │   │   │   │   ├── index.tsx                → 记忆管理页
│   │   │   │   │   └── MemoryView.tsx
│   │   │   │   └── SettingsView/
│   │   │   │       ├── index.tsx                → 设置页
│   │   │   │       └── SettingsView.tsx
│   │   │   ├── store/                           → 模块 I：状态管理（Zustand）
│   │   │   │   ├── chatStore.ts
│   │   │   │   ├── agentStore.ts
│   │   │   │   ├── memoryStore.ts
│   │   │   │   ├── monitorStore.ts
│   │   │   │   ├── thinkingStore.ts
│   │   │   │   ├── hotspotStore.ts
│   │   │   │   ├── settingsStore.ts
│   │   │   │   └── sceneStore.ts
│   │   │   ├── router/                          → 模块 H：路由
│   │   │   │   ├── index.tsx                    → createBrowserRouter
│   │   │   │   ├── routes.tsx                   → 路由定义
│   │   │   │   └── shortcuts.ts                 → 快捷键路由
│   │   │   ├── hooks/                           → 通用 hooks
│   │   │   │   ├── context/
│   │   │   │   │   ├── LayoutContext.tsx        → 布局 Context（NomiFun 蓝本）
│   │   │   │   │   └── NavigationHistoryContext.tsx
│   │   │   │   ├── system/
│   │   │   │   │   ├── useDeepLink.ts
│   │   │   │   │   └── useNotificationClick.ts
│   │   │   │   └── ui/
│   │   │   │       └── useConversationShortcuts.ts
│   │   │   ├── utils/                           → 工具函数
│   │   │   │   ├── theme/
│   │   │   │   │   ├── themeControlContract.ts  → 主题契约（NomiFun 蓝本）
│   │   │   │   │   ├── customCssProcessor.ts    → 自定义 CSS 处理
│   │   │   │   │   ├── themeCssSync.ts          → 主题同步
│   │   │   │   │   ├── themeBroadcast.ts        → 跨窗口广播
│   │   │   │   │   └── presets/
│   │   │   │   │       ├── macosLight.ts        → Apple 系统风格-亮色（新增）
│   │   │   │   │       ├── macosDark.ts         → Apple 系统风格-暗色（新增）
│   │   │   │   │       └── macosAuto.ts         → 跟随系统（新增）
│   │   │   │   ├── platform.ts
│   │   │   │   └── ...
│   │   │   └── styles/
│   │   │       ├── layout.css
│   │   │       ├── apple-system.css             → Apple 系统风格基础样式
│   │   │       └── variables.css                → CSS 变量
│   │   └── platform/                            → 平台适配
│   ├── public/                                  → 静态资源
│   │   └── icons/
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── unocss.config.ts                         → UnoCSS 配置
│   └── index.html
│
├── docs/                                        → 文档（已存在）
│   ├── SparkFox-重组优化方案-1.0.md
│   ├── SparkFox-四项目深度分析与融合拆解报告.md
│   ├── SparkFox-最终融合蓝图-1.0.md              → 本文档
│   ├── poc-report.md
│   ├── rfc/
│   │   ├── RFC-001-crate-boundaries.md
│   │   ├── RFC-002-orchestration-coordination.md
│   │   ├── RFC-003-memory-source-of-truth.md
│   │   ├── RFC-004-crdt-selection.md
│   │   └── RFC-005-parallelism-and-target-isolation.md
│   └── architecture/
│       └── ...
│
├── scripts/                                     → 构建/工具脚本
│   ├── sync-types.sh                            → ts-rs 类型同步
│   ├── clean-room-check.sh                      → 清洁室流程检查
│   └── release.sh                               → 发布打包
│
├── .github/
│   └── workflows/
│       ├── ci.yml                               → CI 流水线
│       ├── clean-room-audit.yml                 → 清洁室审计
│       └── release.yml                          → 发布流水线
│
├── Cargo.toml                                   → Rust workspace 根（resolver="3"）
├── package.json                                 → workspace 根（bun scripts）
├── bunfig.toml                                  → bun 配置
├── rust-toolchain.toml                          → Rust 工具链（nightly + edition 2024）
├── .gitignore
├── LICENSE                                      → AGPL-3.0
├── README.md
└── CHANGELOG.md
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 附录：与 RFC 对应关系
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

| RFC | 标题 | 蓝图对应部分 |
|-----|------|-------------|
| RFC-001 | crate 边界重划 | 第二部分 14 crate + 第六部分 crates/ 目录 |
| RFC-002 | 编排协调（DAG） | 决策 4 + 模块 B AgentDashboard + 数据流 4 + crates/sparkfox-orchestrator |
| RFC-003 | 记忆 SoT（6 层 L0-L5） | 决策 6 + 模块 D + 数据流 2/3 + crates/sparkfox-memory |
| RFC-004 | CRDT 选型（automerge-rs + Double Ratchet） | 决策 1/4 + 模块 D 改造点 3 + crates/sparkfox-crdt + sparkfox-e2ee |
| RFC-005 | 并行度（3-4 并行 + target 隔离） | 第六部分 cargo workspace + scripts/clean-room-check.sh |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 附录：硬约束合规检查
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

| 硬约束 | 蓝图合规设计 |
|--------|-------------|
| Phase -1 PoC 前置 | 蓝图为基础，PoC 验收后再动工（poc-report.md 模板已就绪） |
| MVP 14 crate 不削减 | 第六部分 crates/ 完整列出 14 crate |
| CRDT 用 automerge-rs | 决策 5 + crates/sparkfox-crdt |
| Double Ratchet E2EE | 决策 4 + crates/sparkfox-e2ee + 模块 J 安全设置 |
| 6 层记忆 L0-L5 | 决策 6 + 模块 D + crates/sparkfox-memory 完整 6 层 |
| DAG 编排（蜂群+组织编排） | 决策 4 + crates/sparkfox-orchestrator（dag.rs + swarm.rs + org.rs） |
| 前端 NomiFun Arco + BaiLongma Scene Protocol | 决策 3 + 模块 A + crates/sparkfox-core/src/scene.rs |
| AGPL 清洁室（BaiLongma MIT 重写） | 决策 1/4 + 所有 BaiLongma 来源均标注"清洁室重写" |
| Apple 系统风格 | 决策 3 + 模块 A 改造点 3 + styles/apple-system.css + 主题预设 |
| 文档双备份（SparkFox/docs + D:\Pangu Nebula\docs） | 本文档保存到 SparkFox/docs（按用户指示） |
| 3-4 并行 + target 隔离 | RFC-005 已约束，蓝图不冲突 |
| 7 专家评审 28 高危问题 | 蓝图已对应 7 P0 共识问题的解决方案 |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

**蓝图版本**：v1.0
**生成时间**：2026-07-18
**下一步**：基于本蓝图编写《SparkFox 实施方案-1.0.md》（分 Phase -1 → 0 → 1 → 1.5 → 2 → 3 → 4 → 5 → 6 的详细实施步骤）
