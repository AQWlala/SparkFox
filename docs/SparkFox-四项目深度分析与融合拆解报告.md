# SparkFox 四项目深度分析与融合拆解报告

> 生成时间：2026-07-18
> 项目路径：D:\Pangu Nebula（主体）+ D:\xin kaifa\_reference\{nomifun-tauri, openakita, BaiLongma}（参考）
> 基于文档：SparkFox-重组优化方案-1.0.md + RFC-001~005

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 目录
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

- 第一部分：四项目技术栈汇总对比
- 第二部分：四项目详细分析
  - 2.1 Pangu Nebula
  - 2.2 NomiFun
  - 2.3 OpenAkita
  - 2.4 BaiLongma
- 第三部分：深度差异分析
- 第四部分：融合重组可行性分析
- 第五部分：与 SparkFox 优化方案 v1.0 契合度分析
- 第六部分：风险评估与建议
- 第七部分：深度功能拆解（基于用户偏好）
  - 7.1 Pangu Nebula 深度拆解
  - 7.2 NomiFun 深度拆解（重点 UI + 功能模块 + 记忆体系）
  - 7.3 OpenAkita 深度拆解（重点 Agent 菜单 + 监视面板 + 三层记忆）
  - 7.4 BaiLongma 深度拆解（重点对话展示 + 思考过程 + 热点追踪）
- 第八部分：功能重叠对比矩阵（72 功能点）
- 第九部分：融合决策汇总

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第一部分：四项目技术栈汇总对比
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

| 维度 | Pangu Nebula | NomiFun | OpenAkita | BaiLongma |
|------|----------|---------|-----------|-----------|
| **一句话定位** | 元认知多 Agent Runtime（6 层记忆 L0-L5 + 蜂群 + 双引擎 + 11 安全栈 + CRDT） | 无限制全开源本地优先超级 AI 工作站（50 crate workspace 全栈 Rust） | 开源全能自进化多 Agent AI 助手（Ralph 永不放弃 + 6 层沙箱 + 组织编排） | 持续运行的桌面 AI Agent 数字意识框架（Tick 心跳 + ACI 预判注入 + Scene Protocol + Thread 线索模型） |
| **桌面框架** | Tauri 2（薄壳 + Python sidecar） | Tauri 2（进程内后端，axum on 127.0.0.1） | Tauri 2.x Setup Center + Capacitor Mobile + Web | Electron 33（electron-builder 25 + NSIS/dmg + 单实例 + 焦点横幅） |
| **前端框架** | Preact 10 | React 19.1 | React 19 + TypeScript | 原生 HTML/CSS/JS（无框架）+ D3 7.9 + Three.js |
| **前端 UI 库** | Tailwind 3 + ReactFlow 11 + @antv/g6 5 | Arco Design + UnoCSS 66 + @xyflow/react + xterm + Monaco + CodeMirror + mermaid + KaTeX | shadcn/ui + Tailwind + lucide-react | 自研 ACUI 卡片 + Scene Protocol 驱动 + 3 主题 + 记忆图物理控制 |
| **后端/逻辑层** | Python 3.11 + FastAPI + PyWebView sidecar | Rust 2024（edition）+ axum + tokio + 50 crate workspace | Python 3.11+ + FastAPI + Typer + asyncio + Pydantic v2 | Node.js（ESM）+ 本地 HTTP 服务（端口 3721）+ SSE + WebSocket + better-sqlite3 同步 |
| **数据存储** | SQLite（aiosqlite + SQLAlchemy）+ CRDT + 文件系统 | SQLite（sqlx + rusqlite）+ 文件系统 + ts-rs 类型同步 | SQLite（aiosqlite）+ ChromaDB/向量 + MEMORY.md/USER.md/SOUL.md/AGENT.md | SQLite（better-sqlite3 同步）+ sqlite-vec 向量 + @huggingface/transformers 本地嵌入 + FTS5 + sandbox/ |
| **状态管理** | Preact Context + useReducer（7 类 state）+ SSE 断点续传 + 指数退避 | React 19 Context + ts-rs 类型同步 | React Context + lazy Suspense（22 lazy views） | sceneStore（Scene Protocol 单一真相源）+ Thread State（焦点/承诺）+ state 全局对象 |
| **构建工具** | Vite 6 + hatchling + cargo（Tauri） | cargo（workspace resolver="3"）+ bun scripts + thin LTO + codegen-units=1 + strip | Vite + hatchling + ruff + pytest + cargo（Tauri） | electron-builder + electron/rebuild（better-sqlite3 native）+ Swift（macOS 语音） |
| **是否 Tauri 应用** | ✅ 是（Tauri 2 + 10 plugin + sidecar 模式） | ✅ 是（Tauri 2 进程内后端，无 sidecar） | ✅ 是（Tauri 2.x Setup Center） | ❌ 否（Electron 33） |
| **是否有记忆系统** | ✅ 是（6 层 L0-L5 + 双向链接 + 图查询） | ✅ 是（YAML frontmatter + 文件存储 + distill 提炼） | ✅ 是（双模式：碎片 3 层 7 类型 + MDRM 关系图 5 维 3D 可视化） | ✅ 是（SQLite 12 表 + FTS5 + sqlite-vec 向量 + 22 子模块 + ACI 预判注入） |
| **记忆系统层级** | **6 层**（L0 感官/L1 工作/L2 情景/L3 语义/L4 程序/L5 元认知横向平面） | **1 层**（长期记忆 YAML frontmatter + distill 提炼） | **2 模式**（Mode 1 碎片 3 层 7 类型 + Mode 2 MDRM 关系图 5 维多跳） | **Thread 线索模型**（多并发线索 + 前台指针 + 承诺 + 温度窗口 warm 6h/cool 48h/cold） |
| **是否有多 Agent** | ✅ 是（SwarmOrchestrator 2-8 Worker 动态扩缩容 + 共识验证 + 任务分解） | ✅ 是（Agent 引擎 + plan mode + compact + cache 诊断，MAX_PROVIDER_TURN_TOOL_CALLS=128） | ✅ 是（多 Agent 并行 + 自动接管 + 故障转移 + **组织编排 CEO/CTO/CFO/Marketing/Finance**） | ✅ 是（本地 Agent 注册 + 委托 + consciousness-loop 意识循环 + delegationDiscovery） |
| **是否有监视面板** | ✅ 是（Sidebar + StatusBar + MascotAssistant + ReactFlow 蜂群可视化 + @antv/g6 图） | ✅ 是（Agent Dashboard + 神经网络可视化 + 实时多 Agent 状态） | ✅ 是（11 面板：Chat/AgentDashboard/AgentManager/IM/Skills/MCP/Memory/Scheduler/TokenStats/Config/Feedback + Org/PixelOffice） | ✅ 是（Brain UI：聊天/思考流/记忆图 D3/焦点线程/热点地球 Three.js/人物卡片/文档/语音/设置/ACUI + Scene Shell 12 kind） |
| **是否有思考过程展示** | ✅ 是（SSE 单连接 + 流式输出） | ✅ 是（LLM 流式 + plan mode + compact + cache 诊断） | ✅ 是（ReAct 推理引擎 思考→行动→观察 + 检查点回滚 + 实时链式思考 + IM 流式） | ✅ 是（实时思考流 thought-stream + Turn Trace 回合级轨迹 + reasoning_effort=high） |
| **是否有热点追踪** | ❌ 否（未发现独立热点模块） | ❌ 否（未发现独立热点模块） | ✅ 是（Proactive Engine 问候/任务跟进/闲聊/晚安 + trending 集成） | ✅ 是（trending.js：CN→微博+知乎 / 其他→HN+Reddit，1h 缓存 + 热点地球 Three.js 可视化） |
| **是否有对话界面** | ✅ 是（Titlebar+Sidebar+Content+StatusBar+MascotAssistant） | ✅ 是（Chat 面板 + 流式输出 + 拖拽上传 + 图片灯箱） | ✅ 是（Chat 流式 + Thinking + 拖拽上传 + 图片灯箱 + 6 IM 平台） | ✅ 是（Brain UI 聊天 + 多渠道 + 实时思考流 + 语音球 + 焦点横幅） |
| **是否有设置页面** | ✅ 是（3 主题 warm-orange/soft-pink/cream-beige + CSS 变量） | ✅ 是（主题契约 check + i18n + 图标检查 + 进程运行时边界检查） | ✅ 是（Config 面板 + LLM 端点 + 系统设置 + 高级选项 + 主题 + Onboarding 向导 + 双语） | ✅ 是（Brain UI 设置页 + 激活页 + 模型/语音/社交/嵌入/搜索/安全配置 + UI 缩放 0.8-1.8 + 3 主题） |
| **代码量估算** | **~50K+ 行**（Python 35K + TS/TSX 15K + Rust 1.5K，223+41+8 文件） | **~680K+ 行**（Rust 500K + TS/TSX 180K + Rust apps 3K，1235+1162+8 文件，50 crate） | **~400K+ 行**（Python 300K + TS/TSX 100K，747+227 文件） | **~140K+ 行**（JS 120K + HTML 12K + CSS 3K + Python 4K，323+25+2+15 文件，本地实测） |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第二部分：四项目详细分析
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 2.1 Pangu Nebula

**项目路径**：D:\Pangu Nebula
**一句话定位**：元认知多 Agent Runtime（6 层记忆 L0-L5 + 蜂群 + 双引擎 + 11 安全栈 + CRDT）

**核心功能清单**：
1. Tauri 2 薄壳 + Python sidecar 模式（10 plugin）
2. Preact 10 + Vite 6 + Tailwind 3 + ReactFlow 11 + @antv/g6 5
3. FastAPI + 30+ router + EventBus + lifespan
4. 6 层记忆 L0-L5（L0 感官/L1 工作/L2 情景/L3 语义/L4 程序/L5 元认知横向平面）+ 双向链接 + 图查询
5. SwarmOrchestrator 蜂群编排（2-8 Worker 动态扩缩容 + 任务分解 + 共识验证）
6. 11 安全栈
7. CRDT 多端同步
8. 3 主题 CSS 变量（warm-orange/soft-pink/cream-beige）
9. SSE 断点续传 + 指数退避
10. Settings (NEBULA_*) + APP_DIR 解析（frozen/dev）

**代码量**：~50K+ 行（Python 35K + TS/TSX 15K + Rust 1.5K）

### 2.2 NomiFun

**项目路径**：D:\xin kaifa\_reference\nomifun-tauri
**版本**：v0.2.28，Apache-2.0
**一句话定位**：无限制全开源本地优先超级 AI 工作站（50 crate workspace 全栈 Rust）

**核心功能清单**：
1. Tauri 2 进程内后端（axum on 127.0.0.1，无 sidecar）
2. React 19.1 + Arco Design + UnoCSS 66 + @xyflow/react + xterm + Monaco + CodeMirror + mermaid + KaTeX
3. 50 crate workspace（32 backend + 16 agent + 2 shared），resolver="3"，edition 2024
4. SQLite 数据层（sqlx + rusqlite + 20+ repository trait + ts-rs 类型同步）
5. Agent 引擎（LLM 流式 + 工具执行 + plan mode + compact + cache 诊断，MAX_PROVIDER_TURN_TOOL_CALLS=128）
6. 长期记忆（YAML frontmatter + 文件存储 + distill 提炼 + FRONTMATTER_MAX_LINES=30 + MAX_MEMORY_FILES=200）
7. Arco Layout 三栏布局（Titlebar + Sider 可拖拽 184px + Content）
8. Companion 桌宠系统（Bolt/Ink/Mochi/CustomFigure + 15+ 子模块）
9. DagCanvas DAG 可视化编排（StepNode + StepConfigBar）
10. 26+ LLM SDK + IM SDK（grammy/lark/dingtalk/wecom）
11. release profile 优化（opt-level=3 + thin LTO + codegen-units=1 + strip）
12. 12 通道 logos + 完善的检查脚本（i18n/theme/icons/process-runtime-boundary/agent-vocabulary）

**代码量**：~680K+ 行（Rust 500K + TS/TSX 180K + Rust apps 3K，50 crate）

### 2.3 OpenAkita

**项目路径**：D:\xin kaifa\_reference\openakita
**版本**：v1.27.28，AGPL-3.0-only
**一句话定位**：开源全能自进化多 Agent AI 助手（Ralph 永不放弃 + 6 层沙箱 + 组织编排）

**核心功能清单**：
1. Python 3.11+ + FastAPI + Typer + asyncio + Pydantic v2
2. Tauri 2.x Setup Center + Capacitor Mobile + Web 三端
3. 30+ LLMs + 6 IM 平台 + 89+ 工具 + 8 类插件 + 3 层权限 + 10 生命周期钩子
4. Ralph 永不放弃循环（max_attempts=10/max_iterations=100 + StopHook 拦截退出）
5. 6 层沙箱（路径分区/确认门/命令拦截/文件快照/自保护/OS 级 bwrap/seatbelt/MIC）
6. 组织编排（CEO/CTO/CFO/Marketing/Finance + 黑板共享 + 消息路由 + 死锁检测 + 心跳 + 自动扩缩容）
7. 双模式记忆（Mode 1 碎片 3 层 7 类型 + Mode 2 MDRM 关系图 5 维 3D 可视化）
8. MemoryManager v2（UnifiedStore + RetrievalEngine + MemoryExtractor + MemoryConsolidator + VectorStore）
9. 8 类型记忆（fact/preference/skill/rule/error/experience/persona_trait/context）
10. 身份系统（SOUL.md/AGENT.md/USER.md/MEMORY.md/POLICIES.yaml + 8 personas）
11. ReAct 推理引擎（思考→行动→观察 + 检查点回滚 + 循环检测 + 策略切换）
12. 11 监视面板（Chat/AgentDashboard/AgentManager/AgentStore/AgentSystem/IM/Identity/Memory/Skills/MCP/Tools/Plugins/LLM/Security/Scheduler/OrgEditor/PixelOffice/TokenStats/Status/Inbox/Feedback/Setup）
13. Pixel Office 像素办公室（Phaser + OfficeScene + AgentSprite + ActivitySystem）
14. 40+ 技能 + 12 插件（avatar-studio/clip-sense/excel-maker/fin-pulse/finance-auto/footage-gate/idea-research/manga-studio/media-post/omni-post/ppt-maker/tongyi-image/word-maker）

**代码量**：~400K+ 行（Python 300K + TS/TSX 100K）

### 2.4 BaiLongma

**项目路径**：D:\xin kaifa\_reference\BaiLongma（gitcode 镜像 clone 完整成功，v2.1.515，246 commits）
**License**：MIT
**一句话定位**：持续运行的桌面 AI Agent 数字意识框架（Tick 心跳 + ACI 预判注入 + Scene Protocol + Thread 线索模型）

**核心功能清单**（基于本地源码深度阅读 11 个核心文件）：
1. Electron 33（electron-builder 25 + NSIS/dmg + 单实例 + 焦点横幅 + 语音球 + 唤醒探测 + 开发板灯效）
2. Node.js ESM + 本地 HTTP 服务（端口 3721）+ SSE + WebSocket + better-sqlite3 同步
3. Tick 心跳主循环（consciousness-loop + watchdog 600s + 优先级抢占 user=100/background=50/tick=10 + Awakening 前 10 tick 固定 10s）
4. LLM-directed Tick 策略（心跳方向由模型自主判断：silence/内部状态/工具/任务/节奏/联系人）
5. 流式 LLM 调用（7 Provider + STREAM_IDLE_TIMEOUT_MS 45s + streamOnceWithRetry 重试 + reasoning_effort=high + thinking_enabled）
6. Thread 线索模型（多并发线索 + 前台指针 + 承诺 + 温度窗口 warm 6h/cool 48h/cold + 指代-就近回指标记 + MAX_THREADS_IN_MEMORY 12）
7. 动态上下文注入器（多路召回 + confidence 调参 low×1.5/medium×1.0/high×0.7 + prev_recall + activePolicies + prefetchCache）
8. Scene Protocol v1（UI = f(scene) 单一真相源 + 幂等 set + 单调递增 rev + ALLOWED_INTENTS ambient/inform/confront）
9. 工具执行器（30+ 工具 + 8 类 Schema + 工具市场 + 能力注册）
10. SQLite 数据层（better-sqlite3 同步 + 12 表 + 4 repository + FTS5 + sqlite-vec 向量）
11. ACI 预判注入（3 类预判：A 语义记忆/B 工具链模式/C 定时预热 + 置信度分级 >0.85 直接注入/0.5-0.85 轻提示/<0.5 不注入）
12. 记忆子系统（22 子模块：injector/threads/recognizer/consolidation/refresh/self-evolution/self-perception/focus/concept-extractor/...）
13. 自进化引擎 + 自感知
14. 本地资源感知（系统/桌面/已安装软件/本地 Agent Claude Code/Codex/Hermes/OpenClaw/SSH/Git 扫描）
15. Brain UI（30+ 文件：聊天/思考流/记忆图 D3/焦点线程/热点地球 Three.js/人物卡片/文档/语音/设置/ACUI）
16. Scene Shell（12 kind：awakening/choice/dom/image/layout/metric/progress/selfcheck/text/weather）
17. 语音系统（云端 ASR + macOS 原生 + 多 TTS + 本地 Whisper + 唤醒词 sherpa-onnx）
18. 社交连接器（Discord + 飞书 WebSocket + 微信 iLink Bot + Webhook）
19. 热点追踪（4 平台热榜 + 3D 地球 + 实时事件流 8 类别 + 跑马灯 + 1h 缓存）
20. 专题面板（台风预警 + 世界杯 + 天气 + 地图 + 人物卡片）
21. 60+ 测试套件（覆盖 threads/injector/focus/recognizer/section-gate/self-evolution/strict-evaluation/tool-router/turn-trace 等）

**代码量**：~140K+ 行（JS 120K + HTML 12K + CSS 3K + Python 4K，323+25+2+15 文件，本地实测）

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第三部分：深度差异分析
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 3.1 技术栈路线分化

**Pangu Nebula（Python + Tauri 薄壳）**：Python 主导 + Tauri 2 作为薄壳通过 sidecar 模式调用 Python 后端。优势：Python 生态丰富，AI/ML 库齐全；劣势：sidecar IPC 延迟，Python GIL 限制并发。

**NomiFun（全栈 Rust + React 19）**：50 crate workspace 全栈 Rust + Tauri 2 进程内后端。优势：进程内后端无 sidecar 延迟，Rust 性能 + 内存安全，类型安全（ts-rs）；劣势：开发门槛高，编译时间长。

**OpenAkita（Python + Tauri Setup Center + 三端齐发）**：Python 3.11+ FastAPI 后端 + Tauri 2.x Setup Center + Capacitor Mobile + Web。优势：三端覆盖，功能最全；劣势：架构复杂度高，Python 性能限制。

**BaiLongma（Node.js ESM + Electron + 原生 JS）**：Node.js ESM 后端 + Electron 33 + 原生 HTML/CSS/JS 前端。优势：无框架依赖，better-sqlite3 同步驱动性能优；劣势：Electron 体积大，内存占用高。

### 3.2 记忆系统设计哲学分化

- **Pangu Nebula**：6 层架构（L0-L5 含元认知横向平面）— 最分层
- **NomiFun**：YAML frontmatter 文件存储 — 最简洁（用户不喜欢，认为落后）
- **OpenAkita**：双模式（碎片 3 层 7 类型 + MDRM 关系图 3D 可视化）— 最可视化（用户喜欢三层体系）
- **BaiLongma**：Thread 线索模型（多并发 + 前台指针 + 承诺 + 温度窗口 + ACI 预判注入）— 最主动预判 + 最接近人类对话

### 3.3 多 Agent 协同深度分化

- **Pangu Nebula**：Swarm 蜂群 2-8 Worker + 共识验证
- **NomiFun**：单 Agent 深度优化（plan mode + compact）
- **OpenAkita**：组织编排（CEO/CTO/CFO/Marketing/Finance AI 公司）— 最复杂
- **BaiLongma**：consciousness-loop 意识循环（持续运行数字意识）— 最独特

### 3.4 独特亮点

| 项目 | 核心亮点 | 技术创新点 |
|------|---------|-----------|
| **Pangu Nebula** | 6 层记忆 L5 元认知 + CRDT + 11 安全栈 | 元认知横向平面是唯一覆盖元认知层面的设计；11 安全栈最全面 |
| **NomiFun** | 50 crate workspace 全栈 Rust + 进程内后端 | 进程内后端无 sidecar 延迟；ts-rs 类型安全；release profile 优化 |
| **OpenAkita** | Ralph 永不放弃循环 + 6 层沙箱 + 8 类插件 + 6 IM 平台 + 30+ LLM | Ralph 循环独树一帜；6 层沙箱最全面；组织编排最复杂 |
| **BaiLongma** | Tick 心跳 LLM-directed + ACI 预判注入 + Scene Protocol v1 + Thread 线索模型 | Tick 心跳方向由模型自主判断；ACI 预判注入（学界 PASTE 对标）；Scene Protocol UI=f(scene)；Thread 线索模型最接近人类对话；stream idle timeout 45s + watchdog 600s |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第四部分：融合重组可行性分析
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 4.1 代码量与融合难度

| 项目 | 代码量 | 融合难度 | 原因 |
|------|--------|---------|------|
| Pangu Nebula | ~50K+ 行 | 低 | 代码量最小，作为 SparkFox 主体框架基础 |
| NomiFun | ~680K+ 行 | 高 | 代码量最大，50 crate workspace 需要选择性吸收 |
| OpenAkita | ~400K+ 行 | 中 | 功能最全，但需选择性吸收（Ralph 循环 + 6 层沙箱 + 插件系统） |
| BaiLongma | ~140K+ 行 | 中 | Thread 模型 + Scene Protocol + consciousness-loop 需要深度重构 |

### 4.2 主体技术栈选择

基于优化方案 v1.0 的 8 项关键决策：

- **桌面框架**：Tauri 2（统一，放弃 Electron）
- **后端/逻辑层**：Rust（以 NomiFun 50 crate workspace 为基础）
- **前端**：React 19 + TypeScript（以 NomiFun Arco Design 为基础）
- **数据存储**：SQLite（sqlx + rusqlite）+ automerge-rs CRDT + sqlite-vec 向量

### 4.3 选择性吸收策略

| 来源项目 | 吸收模块 | 融合方式 |
|---------|---------|---------|
| Pangu Nebula | 6 层记忆 L0-L5 架构 | 作为 SoT 蓝图（RFC-003），用 Rust 重写 |
| Pangu Nebula | 11 安全栈 | 选择性吸收，用 Rust 重写 |
| NomiFun | 50 crate workspace 结构 | 作为 SparkFox 主体框架基础 |
| NomiFun | Arco Design + UnoCSS UI | 作为前端基础（用户偏好） |
| NomiFun | ts-rs 类型同步 | 直接复用 |
| OpenAkita | Ralph 永不放弃循环 | 用 Rust 重写，作为 Agent 执行策略插件（RFC-002） |
| OpenAkita | 6 层沙箱 | 用 Rust 重写 |
| OpenAkita | 8 类插件系统 | 用 Rust 重写 |
| OpenAkita | 6 IM 平台 | 选择性吸收 |
| BaiLongma | Thread 线索模型 | 用 Rust 重写，作为 L2 情景层实现参考 |
| BaiLongma | ACI 预判注入 | 用 Rust 重写 |
| BaiLongma | Scene Protocol v1 | 直接复用规范 |
| BaiLongma | consciousness-loop | 用 Rust 重写，作为 L5 元认知运行时基础 |
| BaiLongma | Tick 心跳 LLM-directed | 用 Rust 重写 |

### 4.4 AGPL 合规风险

| 项目 | License | 合规风险 | 清洁室流程 |
|------|---------|---------|-----------|
| Pangu Nebula | （未明确） | 低 | 作为内部参考，直接重写 |
| NomiFun | Apache-2.0 | 中 | 关键模块需清洁室重写 |
| OpenAkita | AGPL-3.0-only | 低 | 已是 AGPL-3.0，可直接吸收 |
| BaiLongma | MIT | 中 | 需清洁室重写关键模块 |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第五部分：与 SparkFox 优化方案 v1.0 契合度分析
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 5.1 五个 RFC 对应关系

**RFC-001 crate 边界重划**：NomiFun（50 crate workspace）> Pangu Nebula > OpenAkita > BaiLongma

**RFC-002 编排协调**：OpenAkita（组织编排）> Pangu Nebula（蜂群）> BaiLongma（consciousness-loop）> NomiFun（单 Agent）

**RFC-003 记忆 SoT**：Pangu Nebula（6 层 L0-L5）> BaiLongma（Thread 线索模型）> OpenAkita（双模式）> NomiFun（1 层）

**RFC-004 CRDT 选型**：NomiFun（Rust）> 其他；四项目均未实现 Double Ratchet，需新建

**RFC-005 并行度**：BaiLongma consciousness-loop 单线程意识流可作为 target 隔离参考

### 5.2 L5 元认知横向平面的运行时基础

BaiLongma 的 consciousness-loop + Tick 心跳 LLM-directed 是四项目中唯一实现"持续运行数字意识"的设计，可作为 L5 元认知横向平面的运行时基础。

### 5.3 UI/UX 融合策略

基于用户偏好（nomifun 界面 + Apple 系统风格）：

- **前端基础**：NomiFun Arco Design + UnoCSS（用户明确偏好）
- **Agent 驱动 UI**：BaiLongma Scene Protocol v1（UI = f(scene) 单一真相源）
- **可视化**：BaiLongma D3 记忆图 + Pangu Nebula ReactFlow 蜂群 + OpenAkita 神经网络可视化 + BaiLongma Three.js 热点地球

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第六部分：风险评估与建议
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 6.1 高风险项

| 风险项 | 等级 | 原因 | 缓解措施 |
|--------|------|------|---------|
| BaiLongma Thread 模型长期稳定性 | 高 | Thread 模型新颖，温度窗口 + 承诺机制未经长期验证 | Phase -1 PoC 必须包含 Thread 模型稳定性测试 |
| NomiFun 50 crate workspace 融合复杂度 | 高 | 680K+ 行代码，50 crate 边界重划复杂 | 选择性吸收，非全量融合；RFC-001 优先 |
| OpenAkita 组织编排复杂度 | 高 | OrgRuntime 6355 LOC | 作为 DAG 策略插件参考，非全量吸收 |
| AGPL 清洁室流程耗时 | 中 | BaiLongma（MIT）+ NomiFun（Apache-2.0）需清洁室重写 | 团队 A/B 分离，法律审查前置 |
| automerge-rs + Double Ratchet 集成 | 中 | 四项目均未实现，需新建 | Phase -1 PoC 必须验证 CRDT 可行性 |
| Electron → Tauri 2 迁移 | 中 | BaiLongma 用 Electron，需用 Rust 重写 | 放弃 Electron，BaiLongma 模块用 Rust 重写 |

### 6.2 建议下一步行动

**优先级 1：Phase -1 PoC 启动（4-6 周）**
- PoC 1：L5 元认知横向平面可行性（基于 BaiLongma consciousness-loop 用 Rust 重写）
- PoC 2：automerge-rs CRDT 集成（基于 NomiFun Rust 栈）
- PoC 3：bge 嵌入性能基线（基于 BaiLongma sqlite-vec + @huggingface/transformers）
- PoC 4：性能基线（基于 NomiFun release profile 优化）

**优先级 2：RFC 文档细化**
- RFC-001 crate 边界重划：基于 NomiFun 50 crate workspace 划分 SparkFox 14 crate
- RFC-002 编排协调：基于 OpenAkita 组织编排 + Pangu Nebula 蜂群 + BaiLongma consciousness-loop 设计 DAG
- RFC-003 记忆 SoT：基于 Pangu Nebula 6 层 + BaiLongma Thread 线索模型设计 6 层 SoT

**优先级 3：清洁室流程规划**
- 团队 A/B 分离方案
- BaiLongma（MIT）+ NomiFun（Apache-2.0）清洁室重写清单
- 法律审查前置

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第七部分：深度功能拆解（基于用户偏好）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

**用户偏好声明**：
- ✅ 喜欢 NomiFun 的：前端 UI 设计、功能模块设计
- ✅ 喜欢 OpenAkita 的：Agent 菜单设计、监视面板设计、三层长期记忆体系
- ✅ 喜欢 BaiLongma 的：对话展示方式、思考过程可视化、信息热点追踪功能
- ❌ 不喜欢 NomiFun 的：记忆体系较落后

### 7.1 Pangu Nebula 深度拆解

#### 功能清单（最细粒度）

| 功能模块 | 子功能 | 详细说明 | 实现文件路径 | 我是否喜欢 | 原因 |
|----------|--------|---------|-------------|-----------|------|
| 桌面框架 | Tauri 2 薄壳 | Tauri 2 + 10 plugin（shell/dialog/notification/clipboard/global-shortcut/fs/single-instance/updater/process/autostart）+ keepawake | src-tauri/src/lib.rs | ✅ | Tauri 轻量，符合 Apple 风格 |
| 桌面框架 | Python sidecar 模式 | Tauri spawn Python 子进程 + supervisor + handshake | launch.py | ❌ | sidecar IPC 延迟，NomiFun 进程内后端更优 |
| 前端框架 | Preact 10 + Vite 6 | 轻量 React 替代 | frontend/package.json | 中立 | 可迁移到 React 19（NomiFun） |
| 前端 UI | Tailwind 3 | 原子化 CSS | frontend/package.json | 中立 | Arco Design 更现代 |
| 前端 UI | 3 主题 CSS 变量 | warm-orange/soft-pink/cream-beige | frontend/src/styles/variables.css | ✅ | 多主题机制可保留 |
| 前端 UI | ReactFlow 蜂群可视化 | 蜂群节点拓扑图 | frontend/package.json | ✅ | 多 Agent 可视化基础 |
| 前端 UI | @antv/g6 5 图可视化 | 通用图可视化 | frontend/package.json | ✅ | 记忆图可视化基础 |
| 前端布局 | 5 区域布局 | Titlebar+Sidebar+Content+StatusBar+MascotAssistant | frontend/src/app.tsx | ✅ | 布局合理 |
| 状态管理 | SSE 断点续传 + 指数退避 | 网络断开自动重连 | frontend/src/lib/store.tsx | ✅ | 健壮性设计 |
| 后端 | FastAPI + 30+ router | chat/swarm/memory/dag/cu/kb 等 | server/main.py | ❌ | 需用 Rust 重写 |
| 后端 | EventBus 事件总线 | 跨模块事件通信 | server/main.py | ✅ | 架构合理 |
| 记忆系统 | 6 层 L0-L5 架构 | L0 感官/L1 工作/L2 情景/L3 语义/L4 程序/L5 元认知横向平面 | server/services/memory_service.py | ✅ | 核心优势，作为 SoT 蓝本 |
| 记忆系统 | 双向链接 + 图查询 | 记忆节点双向关联 + 多跳关系查询 | server/services/memory_service.py | ✅ | 图查询基础 |
| 多 Agent | SwarmOrchestrator | 2-8 Worker 动态扩缩容 | server/services/swarm_orchestrator.py | ✅ | 蜂群编排基础 |
| 多 Agent | 共识验证 | Worker 结果共识 | server/services/swarm_orchestrator.py | ✅ | 质量保证 |
| 安全栈 | 11 安全栈 | 11 层安全机制 | server/services/ | ✅ | 最全面 |
| 数据存储 | CRDT | 多端冲突无关数据类型 | server/ | ✅ | 需用 automerge-rs 重写 |

#### 现有优势（值得保留）

1. **6 层记忆 L0-L5 架构**：四项目中唯一的元认知 L5 横向平面设计，作为 SparkFox 记忆 SoT 蓝本（RFC-003）
2. **11 安全栈**：四项目中最全面的安全机制，远超 OpenAkita 6 层沙箱
3. **蜂群编排 + 共识验证**：2-8 Worker 动态扩缩容 + 共识机制保证质量
4. **CRDT 多端同步意识**：四项目中唯一有 CRDT 设计的项目
5. **3 主题 CSS 变量机制**：多主题可保留

#### 现有劣势（需要替换/改进）

1. **Python sidecar 模式**：IPC 延迟，需迁移到 NomiFun 进程内后端
2. **Python 后端**：需用 Rust 重写
3. **Preact 10**：需迁移到 React 19
4. **Tailwind 单一**：需补充 Arco Design 组件库
5. **无 Agent 菜单设计**：缺乏 OpenAkita 式的 Agent 管理界面
6. **无监视面板**：缺乏 OpenAkita 式的 AgentDashboard + TokenStats
7. **无对话展示创新**：缺乏 BaiLongma 式的实时思考流
8. **无热点追踪**：缺乏 BaiLongma 式的热点地球

#### 与三个参考项目相比的差距清单

| 差距点 | SparkFox 现状 | 参考项目已有 | 需要从哪个项目补 |
|--------|-------------|-------------|----------------|
| 进程内后端 | Python sidecar | NomiFun 进程内 axum | NomiFun |
| Arco Design UI | Tailwind 单一 | NomiFun Arco + UnoCSS | NomiFun |
| 50 crate workspace | 单体 | NomiFun 50 crate | NomiFun |
| Agent 管理界面 | 无 | OpenAkita AgentManagerView | OpenAkita |
| Agent 仪表盘 | ReactFlow 蜂群 | OpenAkita TopoNode 力导向图 | OpenAkita |
| Token 统计 | 无 | OpenAkita TokenStatsView | OpenAkita |
| 组织编排 | 蜂群 2-8 Worker | OpenAkita CEO/CTO/CFO | OpenAkita |
| 6 层沙箱 | 11 安全栈 | OpenAkita bwrap/seatbelt/MIC | OpenAkita |
| Ralph 永不放弃 | 无 | OpenAkita Ralph 循环 | OpenAkita |
| 8 类插件系统 | 无 | OpenAkita 8 类插件 | OpenAkita |
| 6 IM 平台 | 无 | OpenAkita 6 IM | OpenAkita |
| 对话展示创新 | SSE 流式 | BaiLongma 实时思考流 | BaiLongma |
| 思考过程可视化 | 无 | BaiLongma ThoughtStream | BaiLongma |
| 热点追踪 | 无 | BaiLongma HotspotEarth 3D 地球 | BaiLongma |
| Scene Protocol | 无 | BaiLongma UI=f(scene) | BaiLongma |
| Thread 线索模型 | 无 | BaiLongma 多并发 + 温度窗口 | BaiLongma |
| ACI 预判注入 | 无 | BaiLongma 3 类预判 | BaiLongma |
| Tick 心跳 LLM-directed | 无 | BaiLongma 模型自主决策 | BaiLongma |
| consciousness-loop | 无 | BaiLongma 持续意识循环 | BaiLongma |
| 60+ 测试套件 | 无 | BaiLongma 完整测试 | BaiLongma |

### 7.2 NomiFun 深度拆解（重点 UI + 功能模块 + 记忆体系）

#### 功能清单（最细粒度）

| 功能模块 | 子功能 | 详细说明 | 实现文件路径 | 我是否喜欢 | 原因 |
|----------|--------|---------|-------------|-----------|------|
| 前端 UI | 整体布局 | Arco Layout 三栏：Titlebar（顶）+ Sider（左侧可拖拽 184px）+ Content（主）+ 可折叠 | ui/src/renderer/components/layout/Layout.tsx | ✅喜欢 | 三栏布局清晰，侧边栏可拖拽 |
| 前端 UI | 主题配色 | Arco Design 默认主题 + 自定义 CSS 主题 + ThemeContext + themeControlContract + 主题契约检查 | ui/src/renderer/hooks/context/ThemeContext.tsx | ✅喜欢 | 主题契约机制严谨 |
| 前端 UI | 侧边栏设计 | SiderNav 导航 + SiderFooter 底部 + SiderItem 项 + SiderThemeControl 主题控制 + WebuiControlPanel WebUI 控制 + 可拖拽调整宽度 | ui/src/renderer/components/layout/Sider/ | ✅喜欢 | 功能丰富，可拖拽 |
| 前端 UI | 顶栏设计 | Titlebar + WindowControls 窗口控制按钮 + 自定义拖拽区 | ui/src/renderer/components/layout/Titlebar/ | ✅喜欢 | 标准桌面应用顶栏 |
| 前端 UI | 卡片样式 | NomiCollapse 折叠卡片 + NomiModal 模态框 + NomiScrollArea 滚动区 + NomiSelect 选择器 + NomiSteps 步骤 | ui/src/renderer/components/base/ | ✅喜欢 | 组件库完整 |
| 前端 UI | 动效/转场 | InstantHoverTooltip 即时悬停 + useTypingAnimation 打字动画 + useAutoScroll 自动滚动 + PwaPullToRefresh 下拉刷新 | ui/src/renderer/components/base/InstantHoverTooltip.tsx | ✅喜欢 | 动效细腻 |
| 前端 UI | 响应式适配 | useContainerWidth + useResizableSplit + isDesktopShell/isElectronDesktop + PWA 模式 + MobileActionSheet | ui/src/renderer/hooks/ui/ | ✅喜欢 | 多端适配完善 |
| 前端 UI | i18n 国际化 | i18n-config.json + i18n.ts + useExtI18n + i18n 类型检查 | ui/src/common/config/i18n.ts | ✅喜欢 | 类型安全的 i18n |
| 前端 UI | 图标系统 | IconParkHOC + 12 通道 logos + icons + logos/brand/app.png | ui/src/renderer/components/IconParkHOC.tsx | ✅喜欢 | 图标资源丰富 |
| 聊天 | SendBox 输入框 | sendbox.css + useSendBoxDraft 草稿 + useSendBoxFiles 文件 + useCompositionInput 组合输入 + useInputFocusRing 聚焦环 + SlashCommandMenu 斜杠命令 + EmojiPicker 表情 + SpeechInputButton 语音 + AtFileMenu @文件 | ui/src/renderer/components/chat/SendBox/ | ✅喜欢 | 输入框功能完备 |
| 聊天 | 消息列表 | MessageList + messages.css + useAutoScroll + processTipModel + turnProcessState + typography | ui/src/renderer/pages/conversation/Messages/ | ✅喜欢 | 消息渲染成熟 |
| 聊天 | Markdown 渲染 | Markdown 组件 + CodeBlock 代码块 + MermaidBlock 流程图 + ShadowView | ui/src/renderer/components/Markdown/ | ✅喜欢 | 支持 Mermaid 流程图 |
| 聊天 | 文件预览 | Preview 系统 + FilePreview + LocalImageView + HorizontalFileList + UploadProgressBar + WebviewHost + Diff2Html 代码差异 | ui/src/renderer/components/media/ | ✅喜欢 | 预览能力强 |
| 聊天 | 会话列表 | SessionList + TerminalRow + Workspace 工作区 | ui/src/renderer/pages/conversation/SessionList/ | ✅喜欢 | 多会话管理 |
| 聊天 | DAG 画布 | DagCanvas + StepNode + StepConfigBar + StepModelPill + StepPresetPill + dag-canvas.css + participantLabel + useLeadThinking | ui/src/renderer/pages/conversation/execution/ | ✅喜欢 | DAG 可视化编排 |
| 聊天 | ACP 平台 | AcpChat + AcpSendBox + acpTurnState | ui/src/renderer/pages/conversation/platforms/acp/ | ✅喜欢 | 多协议支持 |
| 聊天 | Nomi 平台 | NomiChat + turnMetrics | ui/src/renderer/pages/conversation/platforms/nomi/ | ✅喜欢 | 原生平台 |
| 伴侣 | Companion 桌宠 | CompanionAvatar + characters（Bolt/Ink/Mochi/CustomFigure）+ 15+ 子模块 + companionHitMask + companionHitTarget + useCompanionClickThrough + useDetachedMemoryPanel | ui/src/renderer/pages/companion/ | ✅喜欢 | 桌宠系统完整 |
| 定时任务 | Cron 管理 | ScheduledTasksPage + CronStatusTag + TaskDetailPage + cronJobSearch + CronJobIndicator + CronJobManager + cronJobConversationMap + cronUtils + repairCronJobTimeZone + useCronJobs | ui/src/renderer/pages/cron/ | ✅喜欢 | 定时任务管理完善 |
| 知识库 | Knowledge | CreateStudio（SourceConfig/TagPicker/TeachingCard/TypeRail）+ KnowledgeDetailPage + InboxReviewPanel + KnowledgeCard + KnowledgeConnectorDrawer + KnowledgeConsumersSection + KnowledgeEmptyState + treeModel | ui/src/renderer/pages/knowledge/ | ✅喜欢 | 知识库管理完整 |
| Guid 引导 | GuidPage | AgentPillBar + ComposerEntryStrip + DrawerPresetCard + DrawerSkillCard + GuidActionRow + GuidInputCard + GuidModelSelector + GuidPresetEditorHost + GuidResourceCards + GuidSkeleton + MentionDropdown + PresetAgentTag + PresetPickerDrawer + QuickActionButtons + autoWorkEntry + 15+ hooks | ui/src/renderer/pages/guid/ | ✅喜欢 | 引导页设计精细 |
| 后端 | 50 crate workspace | 32 backend + 16 agent + 2 shared，resolver="3"，edition 2024 | Cargo.toml | ✅喜欢 | 架构清晰 |
| 后端 | 进程内后端 | axum on 127.0.0.1，无 sidecar | apps/desktop/src/main.rs | ✅喜欢 | 无 IPC 延迟 |
| 后端 | webui_init_script | 注入信任头（fetch/XHR 拦截） | apps/desktop/src/main.rs | ✅喜欢 | 安全的 WebUI 集成 |
| Agent | Agent 引擎 | LLM 流式 + 工具执行 + plan mode + compact + cache 诊断 + MAX_PROVIDER_TURN_TOOL_CALLS=128 | crates/agent/nomi-agent/src/engine.rs | ✅喜欢 | 单 Agent 深度优化 |
| 数据层 | SQLite 数据层 | sqlx + rusqlite + 20+ repository trait + ts-rs 类型同步 + backup_bundle + validate_id_schema_contract | crates/backend/nomifun-db/src/lib.rs | ✅喜欢 | repository trait 设计优雅 |
| 记忆体系 | 长期记忆 | YAML frontmatter + 文件存储 + distill 提炼 + FRONTMATTER_MAX_LINES=30 + MAX_MEMORY_FILES=200 | crates/agent/nomi-memory/src/store.rs | ❌不喜欢 | 记忆体系落后，单层文件存储 |
| 记忆体系 | 记忆模块 | distill/error/index/paths/prompt/store/types 子模块 | crates/agent/nomi-memory/src/lib.rs | ❌不喜欢 | 缺乏层级和关系图 |
| API 客户端 | 多 Provider | AnthropicRotatingClient + GeminiRotatingClient + OpenAIRotatingClient + RotatingApiClient + OpenAI2AnthropicConverter + OpenAI2GeminiConverter + ProtocolConverter + ApiKeyManager + ClientFactory | ui/src/common/api/ | ✅喜欢 | 多 Provider 轮换 + 协议转换 |
| 配置 | 配置服务 | configService + configMigration + configKeys + constants + storage + storageKeys + appEnv | ui/src/common/config/ | ✅喜欢 | 配置管理完善 |
| 平台适配 | IPlatformServices | NodePlatformServices 接口 + bridge + logger + storage + theme | ui/src/common/platform/ | ✅喜欢 | 平台抽象层 |
| 协议绑定 | AgentExecution 事件 | AgentExecutionEventKind + FinishEventData + SessionAssignedEventData + StartEventData + TurnCompletedEventData + TurnStopReason | ui/src/common/protocolBindings/ | ✅喜欢 | 事件协议类型安全 |
| 构建 | release profile | opt-level=3 + thin LTO + codegen-units=1 + strip | Cargo.toml | ✅喜欢 | 发布优化到位 |
| 构建 | bun scripts | dev/build/release/test/check + typecheck + check:i18n + check:theme + check:icons + check:process-runtime-boundary + check:agent-vocabulary | package.json | ✅喜欢 | 检查脚本完善 |
| 通道 | 12 通道 logos | dingtalk/discord/lark/matrix/mattermost/nostr/qqbot/slack/telegram/twitch/wecom/weixin | ui/src/renderer/assets/channel-logos/ | ✅喜欢 | 通道覆盖广 |
| 设置 | 设置组件 | DirectorySelectionModal + FontSizeControl + LanguageSwitcher + ThemeSwitcher + UpdateModal | ui/src/renderer/components/settings/ | ✅喜欢 | 设置组件完备 |

#### UI 设计特色深度描述

**布局方式**：

```
┌─────────────────────────────────────────────────────────────┐
│ Titlebar（顶栏 + 窗口控制按钮 + 自定义拖拽区）              │
├──────────┬──────────────────────────────────────────────────┤
│          │                                                  │
│  Sider   │              Content（主内容区）                 │
│  184px   │                                                  │
│ 可拖拽   │   - DagCanvas（DAG 编排画布）                    │
│ 可折叠   │   - MessageList（消息列表）                      │
│          │   - SendBox（输入框 + 斜杠命令 + 表情 + 语音）   │
│ SiderNav │                                                  │
│ SiderFoot│                                                  │
│ ThemeCtrl│                                                  │
│ WebuiCtrl│                                                  │
├──────────┴──────────────────────────────────────────────────┤
│ (无独立状态栏，融入 Content)                                │
└─────────────────────────────────────────────────────────────┘
```

- 左侧 Sider 默认 184px，可拖拽调整（RAIL_MIN/RAIL_MAX），窄于 RAIL_COLLAPSE_THRESHOLD 自动折叠
- Sider 分三层：SiderNav（导航）/ SiderFooter（底部）/ SiderThemeControl + WebuiControlPanel
- 主内容区支持 DagCanvas + MessageList + SendBox 三层叠加
- PwaPullToRefresh 支持 PWA 模式下拉刷新
- 移动端通过 MobileActionSheet 适配

**配色方案**：
- 主色：Arco Design 默认主题（科技蓝 #165DFF 系列）
- 辅色：通过 ThemeContext 动态切换 + 自定义 CSS 主题
- 背景：Arco 默认亮色 #FFFFFF / 暗色 #1D2129
- 文字：Arco 默认 #1D2129 / #4E5969 / #86909C（三级灰度）
- 强调色：通过 themeControlContract 约束主题切换一致性
- 支持 DEFAULT_THEME_ID 预设主题

**交互细节**：
- 按钮悬停：InstantHoverTooltip 即时显示（无延迟）
- 输入框：useInputFocusRing 聚焦环动画 + useCompositionInput 组合输入处理（中文输入法友好）
- 列表：useAutoScroll 自动滚动到底部 + useSmoothReveal 平滑揭示新消息
- 拖拽：Sider 可拖拽调整宽度 + useResizableSplit 分割面板可调
- 草稿：useSendBoxDraft 输入草稿自动保存
- 文件：useDragUpload 拖拽上传 + usePasteService 粘贴上传
- 打字：useTypingAnimation 打字机动画效果
- 命令面板：BtwOverlay + useBtwCommand 命令覆盖层
- 斜杠命令：SlashCommandMenu + useSlashCommandController
- 表情：EmojiPicker 表情选择器

**动效/转场**：
- 页面切换：React Router + Suspense 懒加载（22+ lazy 组件）
- 列表展开：NomiCollapse 折叠动画
- 模态框：NomiModal + ModalWrapper 模态动画
- 滚动：NomiScrollArea 自定义滚动条
- 桌宠：CompanionAvatar + companionBarReveal 桌宠显示动画 + companionHitMask 点击穿透
- 提示：PwaPullToRefresh 下拉刷新动画
- 加载：AppLoader 应用加载动画 + GuidSkeleton 骨架屏

### 7.3 OpenAkita 深度拆解（重点 Agent 菜单 + 监视面板 + 三层记忆）

#### 功能清单（最细粒度）

| 功能模块 | 子功能 | 详细说明 | 实现文件路径 | 我是否喜欢 | 原因 |
|----------|--------|---------|-------------|-----------|------|
| 前端 UI | 整体布局 | Sidebar + Topbar + PanelShell + 22 lazy views + shadcn/ui + Tailwind | apps/setup-center/src/components/Sidebar.tsx | ✅喜欢 | 22 视图懒加载 |
| 前端 UI | shadcn/ui 组件库 | alert-dialog/badge/button/card/checkbox/dialog/dropdown-menu/input/label/select/sheet/slider/sonner/switch/table/textarea/toggle-group/toggle/tooltip | apps/setup-center/src/components/ui/ | ✅喜欢 | 组件库完整 |
| 前端 UI | Pixel Office 像素办公室 | PhaserGame + OfficeScene + AgentSprite + ActivitySystem + EventBus + PixelOfficeAgentList + PixelOfficeEventLog + PixelOfficeThemeSelector + RoomGenerator + SceneTheme + StatusMapping + TilesetManager | apps/setup-center/src/components/pixel-office/ | ✅喜欢 | 像素风办公室可视化 Agent |
| 前端 UI | Pixel Avatar 像素头像 | AvatarCache + CharacterComposer + PixelAvatar + appearance-types | apps/setup-center/src/components/pixel-avatar/ | ✅喜欢 | 像素头像生成 |
| Agent 菜单 | Agent 管理器 | AgentManagerView + AgentProfile（id/name/description/icon/color/type/skills/skills_mode/tools/tools_mode/mcp_servers/mcp_mode/custom_prompt/preferred_endpoint/endpoint_policy/category/hidden/user_customized/identity_mode/memory_mode/memory_inherit_global/name_i18n/description_i18n）+ Sheet 编辑面板 + 分类管理 + 导入导出 | apps/setup-center/src/views/AgentManagerView.tsx | ✅喜欢 | Agent 配置最全面 |
| Agent 菜单 | Agent 图标 | AgentIcon + AGENT_SVG_ICONS + isCustomAgentIcon + 自定义图标上传 | apps/setup-center/src/components/AgentIcon.tsx | ✅喜欢 | 图标系统灵活 |
| Agent 菜单 | Agent 仪表盘 | AgentDashboardView + TopoNode/TopoEdge/TopoStats + SimNode 力导向 + Pulse 边脉冲 + ToolSat 工具卫星 + Mote 环境粒子 + hexToRgb 颜色缓存 | apps/setup-center/src/views/AgentDashboardView.tsx | ✅喜欢 | 力导向图 + 粒子效果 |
| Agent 菜单 | Agent 系统 | AgentSystemView | apps/setup-center/src/views/AgentSystemView.tsx | ✅喜欢 | 系统级 Agent |
| Agent 菜单 | Agent 商店 | AgentStoreView | apps/setup-center/src/views/AgentStoreView.tsx | ✅喜欢 | Agent 市场 |
| 监视面板 | Token 统计 | TokenStatsView + 6 周期（1d/3d/1w/1m/6m/1y）+ SummaryRow/TimelineRow/TotalRow/SessionRow/UsageRecordRow 5 维度 + total_input/total_output/total_tokens/total_cache_creation/total_cache_read/request_count/total_cost | apps/setup-center/src/views/TokenStatsView.tsx | ✅喜欢 | Token 统计最全面 |
| 监视面板 | Org 仪表盘 | OrgDashboard + OrgMonitorPanel + OrgProjectBoard + OrgInboxSidebar + OrgBlackboardPanel | apps/setup-center/src/components/OrgDashboard.tsx | ✅喜欢 | 组织仪表盘 |
| 监视面板 | Org 聊天面板 | OrgChatPanel + 8 测试用例 | apps/setup-center/src/components/OrgChatPanel.tsx | ✅喜欢 | 组织聊天 |
| 监视面板 | 进度账本时间线 | ProgressLedgerTimeline + orderTasksForGantt + filterDeliverables | apps/setup-center/src/components/ProgressLedgerTimeline.tsx | ✅喜欢 | 甘特图式进度 |
| 监视面板 | 状态视图 | StatusView | apps/setup-center/src/views/StatusView.tsx | ✅喜欢 | 系统状态 |
| 监视面板 | 故障排查 | TroubleshootPanel + LinkDiagnosticsPanel + DegradedBanner + StaleBundleBanner | apps/setup-center/src/components/TroubleshootPanel.tsx | ✅喜欢 | 故障诊断完善 |
| 监视面板 | 运行时环境 | RuntimeEnvironmentPanel | apps/setup-center/src/components/RuntimeEnvironmentPanel.tsx | ✅喜欢 | 运行时监控 |
| 监视面板 | Inbox 收件箱 | InboxView + InboxBadge + PendingApprovalsView | apps/setup-center/src/views/InboxView.tsx | ✅喜欢 | 任务收件箱 |
| 记忆体系 | 记忆视图 | MemoryView + MemoryItem（id/type/priority/content/source/subject/predicate/tags/importance_score/confidence/access_count/created_at/updated_at/last_accessed_at/expires_at）+ Stats + MigrationStatus + ReviewResult/ReviewProgress + 8 类型 + TYPE_COLORS + MemoryGraph3D 3D 可视化 | apps/setup-center/src/views/MemoryView.tsx | ✅喜欢 | 三层记忆 + 3D 图 + LLM 审查 |
| 记忆体系 | 记忆管理器 v2 | MemoryManager + UnifiedStore（SQLite + SearchBackend）+ RetrievalEngine 多路召回 + MemoryExtractor（工具感知/实体-属性）+ MemoryConsolidator（JSONL 双写）+ VectorStore + 三层注入（Scratchpad + Core Memory + Dynamic Memories） | src/openakita/memory/manager.py | ✅喜欢 | 三层记忆架构清晰 |
| 记忆体系 | 记忆类型 | 8 类型：fact/preference/skill/rule/error/experience/persona_trait/context + MemoryPriority + SemanticMemory + Attachment + ConversationTurn | src/openakita/memory/types.py | ✅喜欢 | 类型分类细 |
| 记忆体系 | 记忆保留 | apply_retention + retention.py | src/openakita/memory/retention.py | ✅喜欢 | 保留策略 |
| 记忆体系 | 记忆整合 | MemoryConsolidator + consolidator.py + JSONL 双写 | src/openakita/memory/consolidator.py | ✅喜欢 | 整合机制 |
| 记忆体系 | 记忆提取 | MemoryExtractor + extractor.py + 工具感知 + 实体-属性 | src/openakita/memory/extractor.py | ✅喜欢 | 自动提取 |
| 记忆体系 | 记忆检索 | RetrievalEngine + retrieval.py + 多路召回 | src/openakita/memory/retrieval.py | ✅喜欢 | 多路召回 |
| 记忆体系 | 统一存储 | UnifiedStore + unified_store.py + SQLite + SearchBackend | src/openakita/memory/unified_store.py | ✅喜欢 | 统一存储抽象 |
| 记忆体系 | 向量存储 | VectorStore + vector_store.py | src/openakita/memory/vector_store.py | ✅喜欢 | 向量检索 |
| 记忆体系 | 记忆遥测 | emit_memory_health_event + telemetry.py + record_health_event | src/openakita/memory/telemetry.py | ✅喜欢 | 健康监控 |
| 聊天 | 聊天视图 | ChatView + MessageList + MessageBubble + MessageParts + FlatMessageItem + MarkdownContent + LightboxOverlay + Artifacts + AttachmentPreview + ContextMenu + SourceStrip + SourceBadge + SpinnerTipDisplay + SubAgentCards + SlashCommandPanel | apps/setup-center/src/views/chat/ | ✅喜欢 | 聊天功能完整 |
| 聊天 | 思考链 | ThinkingChain + FloatingPlanBar + PlanCard + PlanApprovalPanel | apps/setup-center/src/views/chat/components/ThinkingChain.tsx | ✅喜欢 | 思考链可视化 |
| 聊天 | 安全确认 | SecurityConfirmModal + useSecurityPolicy + useFrictionDetector + useQueryGuard + useCircuitBreaker | apps/setup-center/src/views/chat/components/SecurityConfirmModal.tsx | ✅喜欢 | 安全机制完善 |
| 聊天 | 工具调用 | MCPCallStrip + AskUser + AskUserSummary | apps/setup-center/src/views/chat/components/MCPCallStrip.tsx | ✅喜欢 | MCP 调用展示 |
| 聊天 | 子 Agent | SubAgentCards + OrgTimeline | apps/setup-center/src/views/chat/components/SubAgentCards.tsx | ✅喜欢 | 子 Agent 可视化 |
| 安全 | 安全视图 | SecurityView + PolicyV2MatrixView | apps/setup-center/src/views/SecurityView.tsx | ✅喜欢 | 安全策略矩阵 |
| 组织 | 组织编辑器 | OrgEditorView + orgEditorConstants + orgStatus + orgStructureEvents | apps/setup-center/src/views/OrgEditorView.tsx | ✅喜欢 | 可视化组织编辑 |
| 组织 | Pixel Office 视图 | PixelOfficeView | apps/setup-center/src/views/PixelOfficeView.tsx | ✅喜欢 | 像素办公室 |
| 身份 | 身份视图 | IdentityView + SOUL.md/AGENT.md/USER.md/MEMORY.md + POLICIES.yaml + 8 personas（boyfriend/business/butler/default/family/girlfriend/jarvis/tech_expert） | apps/setup-center/src/views/IdentityView.tsx | ✅喜欢 | 身份系统完整 |
| 技能 | 技能管理 | SkillManager + SkillStoreView + SkillUsageView + SkillConflictsPanel | apps/setup-center/src/views/SkillManager.tsx | ✅喜欢 | 技能管理完善 |
| 工具 | 工具视图 | ToolsView | apps/setup-center/src/views/ToolsView.tsx | ✅喜欢 | 工具管理 |
| MCP | MCP 视图 | MCPView | apps/setup-center/src/views/MCPView.tsx | ✅喜欢 | MCP 服务器管理 |
| 插件 | 插件管理 | PluginManagerView + PluginAppHost + PluginOnboardModal | apps/setup-center/src/views/PluginManagerView.tsx | ✅喜欢 | 插件系统 |
| LLM | LLM 视图 | LLMView + ImageEndpointsSection + ProviderIcon + ProviderSearchSelect + SearchSelect | apps/setup-center/src/views/LLMView.tsx | ✅喜欢 | LLM 配置 |
| IM | IM 视图 | IMView + IMConfigView + FeishuQRModal + QQBotQRModal + WechatQRModal + WecomQRModal + WebPasswordManager | apps/setup-center/src/views/IMView.tsx | ✅喜欢 | IM 二维码绑定 |
| 反馈 | 反馈视图 | FeedbackModal + MyFeedbackView + PublicFeedbackList | apps/setup-center/src/views/FeedbackModal.tsx | ✅喜欢 | 反馈系统 |
| 调度 | 调度视图 | SchedulerView | apps/setup-center/src/views/SchedulerView.tsx | ✅喜欢 | 定时任务 |
| 通道 logos | 14 平台 | dingtalk/discord/kook/lark/line/misskey/onebot/qq/satori/slack/telegram/vocechat/wechat/wecom | apps/setup-center/src/assets/platform_logos/ | ✅喜欢 | 通道覆盖最广 |
| 国际化 | i18n | en.json + zh.json + index.ts | apps/setup-center/src/i18n/ | ✅喜欢 | 双语支持 |
| 更新 | 应用更新 | AppUpdateDialog + useVersionCheck + ReleaseNotesDialog | apps/setup-center/src/components/AppUpdateDialog.tsx | ✅喜欢 | 更新提示 |
| 流事件 | streamEvents | streamEvents.ts + sseStateMachine | apps/setup-center/src/streamEvents.ts | ✅喜欢 | SSE 状态机 |

#### Agent 菜单设计深度描述

**菜单结构**：
- 一级菜单：Sidebar 侧边栏导航，22 个一级视图
- 二级菜单：Agent 管理器内部分类（CategoryInfo：id/label/color），支持按分类筛选 Agent
- 三级子菜单：Agent 编辑面板内部分组（基本信息/技能/工具/MCP/身份/记忆/端点策略）

**交互方式**：
- 切换 Agent：通过 ChatView 顶部 Agent 选择器切换 + AgentPillBar 显示当前 Agent
- 配置 Agent 参数：Sheet 侧滑面板（shadcn/ui Sheet 组件），从右侧滑出，包含：
  - 基本信息（name/description/icon/color/type/category）
  - 技能配置（skills/skills_mode：all/whitelist/blacklist）
  - 工具配置（tools/tools_mode：all/whitelist/blacklist）
  - MCP 配置（mcp_servers/mcp_mode）
  - 身份配置（identity_mode：shared/isolated）
  - 记忆配置（memory_mode：shared/isolated + memory_inherit_global）
  - 端点策略（preferred_endpoint + endpoint_policy：prefer/require）
  - 自定义提示词（custom_prompt）
  - 多语言（name_i18n/description_i18n）
- 创建新 Agent：IconPlus 按钮 + EMPTY_PROFILE 模板 + 自由配置
- 导入导出：IconDownload 导出 + IconUpload 导入 + IconImage 自定义图标上传
- 删除：IconTrash + ConfirmDialog 确认
- 隐藏：hidden 字段 + 隐藏/显示切换
- 用户自定义标记：user_customized 字段区分系统/用户 Agent

**视觉呈现**：
- 菜单样式：Sidebar 侧边栏列表 + 分类筛选 + 网格/列表切换
- Agent 展示：AgentIcon + AGENT_SVG_ICONS（SVG 图标库）+ 自定义图标上传 + 颜色标识 + Badge 标签 + PresetAgentTag 预设标签
- 状态标识：AgentDashboard 中 TopoNode.status（idle/running/completed/error/dormant）+ 颜色映射 + 粒子动画
- 分类颜色：CategoryInfo.color + TYPE_COLORS 8 类型颜色

**实现文件**：
- apps/setup-center/src/views/AgentManagerView.tsx - Agent 管理器主视图
- apps/setup-center/src/views/AgentDashboardView.tsx - Agent 仪表盘
- apps/setup-center/src/views/AgentStoreView.tsx - Agent 商店
- apps/setup-center/src/views/AgentSystemView.tsx - Agent 系统
- apps/setup-center/src/components/AgentIcon.tsx - Agent 图标
- apps/setup-center/src/components/ProviderIcon.tsx - Provider 图标
- apps/setup-center/src/components/pixel-office/AgentSprite.tsx - 像素 Agent 精灵
- apps/setup-center/src/components/OrgAvatars.tsx - 组织头像

#### 监视面板设计深度描述

**监视维度**：
- Token 消耗：total_input/total_output/total_tokens/total_cache_creation/total_cache_read 5 个 token 维度
- 调用次数：request_count
- 成本：total_cost/estimated_cost
- 响应时间：avg_latency_ms（AgentDashboard TopoStats）
- 成功率：successful/failed/total_requests
- Agent 运行状态：idle/running/completed/error/dormant 5 状态
- 任务执行进度：iteration/tools_executed/tools_total/elapsed_s
- 会话统计：SessionRow（first_call/last_call/operation_types/endpoints）
- 组织监控：OrgMonitorPanel + OrgProjectBoard + OrgBlackboardPanel
- 进度账本：ProgressLedgerTimeline + 甘特图 + filterDeliverables
- 运行时环境：RuntimeEnvironmentPanel
- 故障诊断：TroubleshootPanel + LinkDiagnosticsPanel
- 系统状态：StatusView
- 收件箱：InboxView + PendingApprovalsView（待审批）

**呈现方式**：
- 数据展示：Table 表格 + Card 卡片 + Badge 徽章 + 折线图 + 力导向图 + 甘特图 + 3D 图（MemoryGraph3D）
- 布局方式：网格布局（Card + CardHeader + CardTitle + CardContent）+ 仪表盘布局（OrgDashboard）+ 侧边栏+主区（OrgInboxSidebar + OrgChatPanel）
- 刷新方式：useEffect 定时轮询 + onWsEvent WebSocket 实时推送 + 手动 IconRefresh 按钮
- 6 周期切换：1d/3d/1w/1m/6m/1y（PERIOD_KEYS + PERIOD_I18N）

**交互方式**：
- 钻入详情：点击 SessionRow 可展开 UsageRecordRow 明细
- 筛选/过滤：PeriodKey 周期筛选 + 分类筛选 + orgStatus 状态筛选 + orderTasksForGantt 任务排序
- 导出：saveFileDialog 文件下载（IS_TAURI 平台）
- 切换显示：Switch 开关 + ToggleGroup 切换组
- 悬停提示：Tooltip + TooltipTrigger + TooltipContent + TooltipProvider

**实现文件**：
- apps/setup-center/src/views/TokenStatsView.tsx - Token 统计
- apps/setup-center/src/views/AgentDashboardView.tsx - Agent 仪表盘
- apps/setup-center/src/views/StatusView.tsx - 状态视图
- apps/setup-center/src/views/InboxView.tsx - 收件箱
- apps/setup-center/src/views/PendingApprovalsView.tsx - 待审批
- apps/setup-center/src/components/OrgDashboard.tsx - 组织仪表盘
- apps/setup-center/src/components/OrgMonitorPanel.tsx - 组织监控
- apps/setup-center/src/components/OrgProjectBoard.tsx - 项目看板
- apps/setup-center/src/components/OrgInboxSidebar.tsx - 组织收件箱
- apps/setup-center/src/components/OrgBlackboardPanel.tsx - 黑板面板
- apps/setup-center/src/components/ProgressLedgerTimeline.tsx - 进度时间线
- apps/setup-center/src/components/RuntimeEnvironmentPanel.tsx - 运行时环境
- apps/setup-center/src/components/TroubleshootPanel.tsx - 故障排查
- apps/setup-center/src/components/LinkDiagnosticsPanel.tsx - 链路诊断
- apps/setup-center/src/components/DegradedBanner.tsx - 降级横幅
- apps/setup-center/src/components/StaleBundleBanner.tsx - 过期横幅
- apps/setup-center/src/components/InboxBadge.tsx - 收件箱徽章
- apps/setup-center/src/components/MemoryGraph3D.tsx - 3D 记忆图

#### 三层长期记忆体系深度描述

**第一层 - 短期记忆（工作记忆 / Scratchpad）**：
- 存什么：当前对话上下文 + 最近 N 条消息（_recent_messages）+ ConversationTurn + 临时变量（_session_turns）
- 存在哪：contextvars.ContextVar 内存（_current_session_id_var / _current_user_id_var / _current_workspace_id_var / _session_turns_var / _recent_messages_var / _session_cited_memories_var）
- 过期策略：会话结束即清除（contextvars 随协程生命周期）+ 默认值 "default" 兜底
- 容量限制：由 _session_turns 和 _recent_messages 列表动态管理
- 实现文件：src/openakita/memory/manager.py（_ensure_context_vars 方法，6 个 ContextVar）

**第二层 - 长期记忆（知识记忆 / Core Memory + Dynamic Memories）**：
- 存什么：
  - 8 类型记忆：fact（事实）/ preference（偏好）/ skill（技能）/ rule（规则）/ error（错误）/ experience（经验）/ persona_trait（人格特质）/ context（上下文）
  - Memory 字段：id/type/priority/content/source/subject/predicate/tags/importance_score/confidence/access_count/created_at/updated_at/last_accessed_at/expires_at
  - SemanticMemory（语义记忆）+ Attachment（附件，AttachmentDirection 方向）
- 存在哪：
  - UnifiedStore（SQLite + SearchBackend 统一存储抽象）
  - VectorStore（向量存储，可选，由 SearchBackend 封装）
  - JSONL 双写（MemoryConsolidator 保留）
  - 数据库字段：by_type/by_scope/by_owner（user_id/workspace_id 双维度）
- 检索方式：
  - RetrievalEngine 多路召回（关键词检索 + 语义检索 + 时间检索 + 实体检索）
  - MigrationStatus 显示 semantic.by_scope/by_owner + graph.total_nodes/by_owner
  - last_accessed_at 访问时间追踪 + access_count 访问次数
- 积累方式：
  - MemoryExtractor 自动提取（工具感知 + 实体-属性提取）
  - MemoryConsolidator 自动整合（JSONL 双写 + 去重 + 合并）
  - apply_retention 保留策略（过期清理 + 重要性降级）
  - LLM 审查（ReviewResult：deleted/updated/merged/kept/errors + ReviewProgress 状态机：idle/running/done/error/cancelled + phase：llm_calling/batch_done/done）
- 实现文件：
  - src/openakita/memory/manager.py - 核心协调器
  - src/openakita/memory/unified_store.py - 统一存储
  - src/openakita/memory/vector_store.py - 向量存储
  - src/openakita/memory/retrieval.py - 检索引擎
  - src/openakita/memory/extractor.py - 记忆提取
  - src/openakita/memory/consolidator.py - 记忆整合
  - src/openakita/memory/retention.py - 保留策略
  - src/openakita/memory/types.py - 类型定义
  - apps/setup-center/src/views/MemoryView.tsx - 前端视图

**第三层 - 用户偏好（个性记忆 / Persona + Identity）**：
- 存什么：
  - 身份文件：SOUL.md（灵魂）/ AGENT.md（Agent 设定）/ USER.md（用户画像）/ MEMORY.md（记忆策略）/ POLICIES.yaml（策略）
  - 8 personas：boyfriend（男友）/ business（商务）/ butler（管家）/ default（默认）/ family（家人）/ girlfriend（女友）/ jarvis（贾维斯）/ tech_expert（技术专家）+ user_custom.md.example（用户自定义）
  - persona_trait 类型记忆（人格特质）
  - preference 类型记忆（偏好）
- 存在哪：
  - identity/ 目录（Markdown + YAML 文件）
  - 数据库 persona_trait/preference 类型记忆
  - identity/runtime/ 运行时状态
- 积累方式：
  - 从对话中自动推断（MemoryExtractor 实体-属性提取）
  - 用户手动设置（IdentityView 编辑 SOUL.md/AGENT.md/USER.md）
  - SYSTEM_TASKS.yaml.template 系统任务模板
- 应用方式：
  - identity_mode（shared/isolated）控制 Agent 是否共享身份
  - 自动注入系统提示（builder.py 调用，不再在 manager.py 组装）
  - 影响 Agent 选择（personas 切换）
  - 影响回复风格（persona_trait 记忆注入）
- 实现文件：
  - identity/SOUL.md.example - 灵魂模板
  - identity/AGENT.md.example - Agent 模板
  - identity/USER.md.example - 用户模板
  - identity/MEMORY.md.example - 记忆策略
  - identity/POLICIES.yaml - 策略
  - identity/personas/ - 8 personas
  - apps/setup-center/src/views/IdentityView.tsx - 前端视图

**记忆间串联机制**：
- 短期 → 长期：触发条件（MemoryExtractor 在对话回合结束时自动提取）+ 实现逻辑（MemoryManager 调用 extractor.extract() → UnifiedStore.store() → JSONL 双写）+ 重要性评估（importance_score + confidence + priority 三维度）
- 长期 → 偏好：触发条件（MemoryConsolidator 定期整合 + apply_retention）+ 实现逻辑（高 access_count + 高 importance_score 的 fact/preference 记忆被升级为 persona_trait）+ LLM 审查（ReviewProgress 状态机批量审查）
- 偏好 → 对话：实现逻辑（builder.py 调用 MemoryManager → 三层注入：Scratchpad + Core Memory + Dynamic Memories → 注入系统提示）+ identity_mode 控制注入范围 + persona_trait 记忆直接影响回复风格
- 长期 → 对话：实现逻辑（RetrievalEngine 多路召回 → Dynamic Memories 注入）+ _session_cited_memories_var 追踪本会话已引用记忆（避免重复注入）+ last_accessed_at 更新 + access_count 递增 + tags 标签匹配 + importance_score 加权

### 7.4 BaiLongma 深度拆解（重点对话展示 + 思考过程 + 热点追踪）

#### 功能清单（最细粒度）

| 功能模块 | 子功能 | 详细说明 | 实现文件路径 | 我是否喜欢 | 原因 |
|----------|--------|---------|-------------|-----------|------|
| 桌面框架 | Electron 33 | electron-builder 25 + electron-updater 6 + NSIS/dmg + 单实例 + 焦点横幅 | electron/main.cjs | ❌ | 体积大，Tauri 更优 |
| 桌面框架 | 焦点横幅 | focus-banner-preload + focus-banner.html | electron/focus-banner-preload.cjs | ✅喜欢 | 焦点提示创新 |
| 桌面框架 | 语音球 | voice-orb-preload + VoicePanel | electron/voice-orb-preload.cjs | ✅喜欢 | 语音球创新 |
| 桌面框架 | 唤醒探测 | wake-probe-preload + kws-process + wake-word + kws-model（sherpa-onnx） | electron/wake-probe-preload.cjs | ✅喜欢 | 唤醒词模型 |
| 桌面框架 | 开发板灯效 | dev-board-light | electron/dev-board-light.cjs | ✅喜欢 | 硬件联动 |
| 前端 UI | 整体布局 | Brain UI 单页应用 + 多面板切换（聊天/热点/人物/文档/语音）+ 模式切换 | src/ui/brain-ui/app.js | ✅喜欢 | 面板切换流畅 |
| 前端 UI | 3 主题 | jarvis-brain-ui-theme + CSS 变量 + UI 缩放 0.8-1.8 | src/ui/brain-ui/styles.css | ✅喜欢 | 缩放功能贴心 |
| 前端 UI | ACUI 卡片 | 自研 ACUI 卡片系统 + Remix 版本 | ACUI (Remix)/ | ✅喜欢 | 自研卡片创新 |
| 前端 UI | D3 记忆图 | D3 7.9 + 记忆图物理控制（gravity/repulsion/node-size） | src/ui/brain-ui/hotspot.js | ✅喜欢 | 记忆图可视化 |
| 前端 UI | Three.js 地球 | Three.js + hotspot-earth.js 3D 地球热点 | src/ui/brain-ui/hotspot-earth.js | ✅喜欢 | 3D 地球创新 |
| 前端 UI | Markdown 渲染 | markdown.js + createMarkdownBody | src/ui/brain-ui/markdown.js | ✅喜欢 | Markdown 支持 |
| 对话展示 | 聊天核心 | initChat + chatHistory + chatMessages + msgInput + chatArea + sendBtn + pasteAttachments + liveEl 流式气泡 + claimRenderedMessage 去重（renderedMessageIds + recentRenderedKeys + RENDER_DEDUPE_TTL_MS 2min）+ autoGrowInput 自适应高度 + setComposerLocked 锁定 + applyActivationWarmupLock 激活预热锁 + ensureAudioContext 音频上下文 + unlockAudioOnFirstGesture 首次手势解锁 | src/ui/brain-ui/chat.js | ✅喜欢 | 对话展示最细腻 |
| 对话展示 | 多渠道标签 | friendlyChannelLabel（WeChat/WeCom/Discord/Feishu） | src/ui/brain-ui/chat.js | ✅喜欢 | 渠道标识清晰 |
| 对话展示 | 粘贴附件 | pendingPastedImages + MAX_PASTED_IMAGES 8 + MAX_PASTED_IMAGE_BYTES 12MB | src/ui/brain-ui/chat.js | ✅喜欢 | 粘贴体验好 |
| 对话展示 | 输入占位符 | PUSH_TO_TALK_PLACEHOLDER "按住空格键开始说话" + idlePlaceholder 聚焦/未聚焦切换 | src/ui/brain-ui/chat.js | ✅喜欢 | 语音输入提示 |
| 对话展示 | 流式渲染 | liveEl 边收 token 边重渲染 + message 事件到达后定稿 | src/ui/brain-ui/chat.js | ✅喜欢 | 流式渲染流畅 |
| 对话展示 | 音效 | audioCtx + audioUnlocked + TTS 音效 | src/ui/brain-ui/tts-fx.js | ✅喜欢 | 音效反馈 |
| 对话展示 | 音频输出路由 | audio-output.js 多设备切换 | src/ui/brain-ui/audio-output.js | ✅喜欢 | 输出设备灵活 |
| 思考过程 | ThoughtStream | TOOL_ZH 57 工具中文映射 + TOOL_ICON 57 工具图标 + isFailureResult 失败检测 + ThoughtStream 类（innerId/color/readCSSVar/thinkingLabel/thinkingDoneLabel/toolDetailLength 160/startedAt/curLine/thinkingEl/lastToolEl/statusEl/statusTimer/hadToolCall/toolFailed）+ tStamp 时间戳 | src/ui/brain-ui/thought-stream.js | ✅喜欢 | 思考过程可视化最完整 |
| 思考过程 | Turn Trace | turn-trace.html + turn-trace.js 回合级轨迹 | turn-trace.html | ✅喜欢 | 回合级追踪 |
| 思考过程 | reasoning_effort | DeepSeek reasoning_effort=high + thinking_enabled | src/llm.js | ✅喜欢 | 深度思考模式 |
| 热点追踪 | 热点主逻辑 | hotspotActive + earth + clockTimer + feedAutoTimer + hotspotRefreshTimer + feedIndex + buildHotspotContext 热点上下文构建（中性系统上下文，不强制 Agent 回复） | src/ui/brain-ui/hotspot.js | ✅喜欢 | 热点上下文智能 |
| 热点追踪 | 4 平台热榜 | PLATFORM_CONFIG：douyin（抖音）/ xiaohongshu（小红书）/ wechat（微信热点）/ weibo（微博） | src/ui/brain-ui/hotspot.js | ✅喜欢 | 国内平台覆盖 |
| 热点追踪 | 实时事件流 | MOCK_FEED 8 类别（自然灾害/科技/财经/体育/社会/政策/旅游）+ time/cat/catColor/title/desc/loc/img | src/ui/brain-ui/hotspot.js | ✅喜欢 | 事件流分类清晰 |
| 热点追踪 | 底部跑马灯 | TICKER_ITEMS 8 条跑马灯文字 | src/ui/brain-ui/hotspot.js | ✅喜欢 | 跑马灯设计 |
| 热点追踪 | 热点元数据 | hotspotMeta：source/fetchedAt/stale/refreshMinutes 30/status | src/ui/brain-ui/hotspot.js | ✅喜欢 | 元数据管理 |
| 热点追踪 | 热榜渲染 | renderList + TREND_ICONS（↑↓—）+ TREND_CLASSES + hs-rank-top1/2/3 排名样式 + hs-trend-up/dn/same 趋势样式 + hs-item-empty 空状态 | src/ui/brain-ui/hotspot.js | ✅喜欢 | 渲染细节丰富 |
| 热点追踪 | 热点后端 | trending.js：CN→微博+知乎 / 其他→HN+Reddit，1h 缓存 | src/trending.js | ✅喜欢 | 国内外分区 |
| 热点追踪 | 3D 地球 | HotspotEarth + Three.js 地球可视化 | src/ui/brain-ui/hotspot-earth.js | ✅喜欢 | 3D 可视化创新 |
| 热点追踪 | 热点面板 | hotspot-panel.js 热点面板状态 | src/ui/brain-ui/hotspot-panel.js | ✅喜欢 | 面板管理 |
| 语音系统 | 云端 ASR | cloud-asr.js 云端语音识别 | src/voice/cloud-asr.js | ✅喜欢 | 云端识别 |
| 语音系统 | macOS 原生 | macos-speech.js + swift 原生语音 | src/voice/macos-speech.js | ✅喜欢 | macOS 原生 |
| 语音系统 | 多 TTS | tts-providers.js 多 TTS 提供商 | src/voice/tts-providers.js | ✅喜欢 | TTS 灵活 |
| 语音系统 | 本地 Whisper | whisper_server.py + whisper/ 本地 Whisper | src/voice/whisper_server.py | ✅喜欢 | 本地识别 |
| 语音系统 | 唤醒词 | kws-model + sherpa-onnx 唤醒词模型 | src/voice/kws-model/ | ✅喜欢 | 唤醒词创新 |
| 语音系统 | 语音管理 | manager.js 统一管理 + voice-panel + voice-core + voice-continuous + voice-ptt + voice-wake | src/ui/brain-ui/voice-panel.js | ✅喜欢 | 语音模式全 |
| 社交 | Discord | discord.js | src/social/discord.js | ✅喜欢 | Discord 集成 |
| 社交 | 飞书 | feishu-ws.js WebSocket 长连接 | src/social/feishu-ws.js | ✅喜欢 | 飞书 WebSocket |
| 社交 | 微信 | wechat-clawbot.js iLink Bot | src/social/wechat-clawbot.js | ✅喜欢 | 微信集成 |
| 社交 | Webhook | webhooks.js + http.js + dispatch.js + targets.js | src/social/ | ✅喜欢 | Webhook 路由 |
| 专题 | 台风预警 | typhoon.js + typhoon-alert-monitor.js + typhoon-panel.js | src/typhoon.js | ✅喜欢 | 台风监控 |
| 专题 | 世界杯 | worldcup.js + worldcup-panel.js | src/worldcup.js | ✅喜欢 | 世界杯直播 |
| 专题 | 天气 | weather.js + geo-weather.js + 7 天缓存 | src/geo-weather.js | ✅喜欢 | 天气服务 |
| 专题 | 地图 | map-service.js 地图服务 | src/map-service.js | ✅喜欢 | 地图集成 |
| 专题 | 人物卡片 | person-cards.js + person-card.js + person-card-panel.js | src/person-cards.js | ✅喜欢 | 人物名片 |
| 专题 | 文档面板 | docs.js + doc-panel.js + doc.js | src/docs.js | ✅喜欢 | 文档管理 |
| 专题 | 社交弹窗 | wechat-popup.js + feishu-popup.js | src/ui/brain-ui/wechat-popup.js | ✅喜欢 | 社交弹窗 |
| 专题 | 面板折叠 | panel-collapse.js | src/ui/brain-ui/panel-collapse.js | ✅喜欢 | 面板折叠 |
| Agent | consciousness-loop | 进程级单例 + watchdog 600s + 优先级抢占（user=100/background=50/tick=10）+ Awakening 前 10 个 tick 固定 10s | src/runtime/consciousness-loop.js | ✅喜欢 | 持续意识循环 |
| Agent | Tick 策略 | LLM-directed 心跳方向决策（silence/内部状态/工具/任务/节奏/联系人）+ custom cadence | src/runtime/tick-policy.js | ✅喜欢 | LLM 自主决策 |
| Agent | 严格评估 | buildStrictEvaluationContext + filterStrictEvaluationTools + resolveStrictEvaluationMode | src/runtime/strict-evaluation.js | ✅喜欢 | 严格评估模式 |
| Agent | 本地 Agent 注册 | agents/registry.js + delegationDiscovery + delegate_to_agent + grant_agent_delegation | src/agents/registry.js | ✅喜欢 | 本地 Agent 注册 |
| Agent | Skills 注册 | skills/registry.js + agent-skills | src/skills/registry.js | ✅喜欢 | Skills 管理 |
| 记忆 | Thread 线索模型 | 多并发线索 + 前台指针（无栈无 pop）+ 温度窗口（warm 6h/cool 48h/cold）+ 承诺机制（openCommitment/closeCommitment）+ 指代-就近回指标记 + MAX_THREADS_IN_MEMORY 12 | src/memory/threads.js | ✅喜欢 | 最接近人类对话 |
| 记忆 | 上下文注入器 | injector.js + injector-retrieval + injector-format + 多路召回 + confidence 调参（low×1.5/medium×1.0/high×0.7）+ prev_recall + activePolicies + UISignals + prefetchCache + sceneManifest + AIVideoPanel | src/memory/injector.js | ✅喜欢 | 注入策略智能 |
| 记忆 | ACI 预判注入 | 3 类预判（A 语义记忆/B 工具链模式/C 定时预热）+ 置信度分级（>0.85 直接注入/0.5-0.85 轻提示/<0.5 不注入）+ 1.5s 超时 | src/memory/injector/ | ✅喜欢 | 预判注入创新 |
| 记忆 | 记忆识别 | recognizer-scheduler + recognizer + 批量去抖 + memories_written 广播 | src/memory/recognizer-scheduler.js | ✅喜欢 | 批量识别 |
| 记忆 | 记忆整理 | consolidation-loop + consolidator + 去重合并 + 滚动摘要 | src/memory/consolidation-loop.js | ✅喜欢 | 整理机制 |
| 记忆 | 自进化 | self-evolution + recordSelfEvolutionFromMemories + 从记忆学习行为更新 | src/memory/self-evolution.js | ✅喜欢 | 自进化能力 |
| 记忆 | 自感知 | self-perception + computeSelfPerception + computeSelfSnapshot | src/memory/self-perception.js | ✅喜欢 | 自感知能力 |
| 记忆 | 焦点压缩 | focus + focus-classifier + focus-compress | src/memory/focus.js | ✅喜欢 | 焦点压缩 |
| 记忆 | 时间解析 | temporal-parser + keywords + concept-extractor | src/memory/temporal-parser.js | ✅喜欢 | 时间词解析 |
| 记忆 | 嵌入回填 | embedding-backfill + embedding-local + @huggingface/transformers | src/memory/embedding-backfill.js | ✅喜欢 | 嵌入回填 |
| 记忆 | 活动策略 | active-policies + tool-router | src/memory/active-policies.js | ✅喜欢 | 策略管理 |
| 工具 | 工具执行器 | 30+ 工具（filesystem/shell/web/media/memory/reminders/rules/scene/software-install/api-capability）+ 审计 + 策略 + 沙箱 + 委托 | src/capabilities/executor.js | ✅喜欢 | 工具最丰富 |
| 工具 | 8 类 Schema | agents/api-capabilities/comms/filesystem/media/memory/reminders/review/scene/shell/system/task/ui/web | src/capabilities/schemas/ | ✅喜欢 | Schema 分类清晰 |
| 工具 | 工具市场 | marketplace + find_tool + install_tool + uninstall_tool + list_tools | src/capabilities/marketplace/ | ✅喜欢 | 工具市场 |
| 工具 | 能力注册 | capability-registry + tool-factory + execManageToolFactory + findCapabilitiesByQuery | src/capabilities/capability-registry.js | ✅喜欢 | 运行时注册 |
| Scene | Scene Protocol v1 | UI = f(scene) 单一真相源 + 幂等 set + 单调递增 rev + ALLOWED_INTENTS（ambient/inform/confront）+ snapshot/manifest/clear + subscribe | src/scene/scene-store.js | ✅喜欢 | UI 规范契约 |
| Scene | Scene Server | WebSocket 传输层 + 握手（hello/welcome）+ 能力协商 + scene/scene.patch/resync/intent/ping/pong | src/scene/scene-server.js | ✅喜欢 | 协议完整 |
| Scene | Scene Shell | 12 kind（awakening/choice/dom/image/layout/metric/progress/selfcheck/text/weather）+ bootstrap + client + dev-server | src/ui/scene-shell/ | ✅喜欢 | UI 类型丰富 |
| LLM | 7 Provider | DeepSeek/MiniMax/OpenAI/Qwen/Moonshot/Zhipu/MiMo + 自定义 | src/llm.js | ✅喜欢 | Provider 丰富 |
| LLM | 流式保护 | STREAM_IDLE_TIMEOUT_MS 45s 空闲超时 + streamOnceWithRetry 重试 + 模型 fallback | src/llm.js | ✅喜欢 | 流式保护完善 |
| LLM | cache token 统计 | cache token 统计 + 流式 sanitizer | src/llm.js | ✅喜欢 | 统计细致 |
| API | 12 路由 | activation/admin/embedding/events/map/media/memory/message/panels/settings/social/static/tts | src/api.js | ✅喜欢 | API 完整 |
| API | WebSocket | scene-server + websocket-security 鉴权 + idle-timeout | src/api/websocket-security.js | ✅喜欢 | 安全的 WS |
| 数据 | SQLite | better-sqlite3 同步 + 12 表 + 4 repository + FTS5 + sqlite-vec | src/db.js | ✅喜欢 | 同步性能优 |
| 数据 | 嵌入 | @huggingface/transformers 本地嵌入 | src/embedding-local.js | ✅喜欢 | 本地嵌入 |
| 任务 | 任务管理 | task-manager.js + 持久化任务 + 步骤跟踪 + 重启恢复 | src/task-manager.js | ✅喜欢 | 任务持久化 |
| 系统信息 | 本地资源扫描 | local-resources-scanner + installed-software-scanner + desktop-scanner + system-info + 本地 Agent 检测（Claude Code/Codex/Hermes/OpenClaw）+ SSH/Git | src/local-resources-scanner.js | ✅喜欢 | 资源感知全面 |
| 配置 | 配置管理 | config.js + Provider/模型/语音/社交/搜索/安全 | src/config.js | ✅喜欢 | 配置集中 |
| 配置 | Key 自动配置 | key-auto-config + software-install-intent | src/key-auto-config.js | ✅喜欢 | 自动配置 |
| 测试 | 60+ 测试套件 | test-*.js 覆盖 threads/injector/focus/recognizer/section-gate/self-evolution/strict-evaluation/tool-router/turn-trace/typhoon/worldcup/voice/websocket-security | src/test-*.js | ✅喜欢 | 测试覆盖全 |
| 入口 | 应用入口 | index.js 2400+ 行 + 60+ 子系统协调 + 启动初始化 | src/index.js | ✅喜欢 | 入口协调全面 |
| 入口 | 控制器 | control.js + isRunning/setScheduler | src/control.js | ✅喜欢 | 主循环控制 |
| 入口 | 队列 | queue.js + popMessage/hasMessages/requeueMessage | src/queue.js | ✅喜欢 | 消息队列 |
| 入口 | 事件 | events.js + emitEvent/setStickyEvent + SSE | src/events.js | ✅喜欢 | 事件系统 |
| 入口 | 时间 | time.js + formatTick/nowTimestamp/describeExistence | src/time.js | ✅喜欢 | 时间感知 |
| 入口 | 配额 | quota.js + getAdaptiveTickInterval/getQuotaStatus | src/quota.js | ✅喜欢 | 配额管理 |
| 入口 | 心跳 | ticker.js + consumeTick/getCustomIntervalMs | src/ticker.js | ✅喜欢 | 心跳消费 |
| 入口 | 身份 | identity.js + PRIMARY_USER_ID/formatPresenceForPrompt | src/identity.js | ✅喜欢 | 身份管理 |
| 入口 | 觉醒 | awakening.js + 觉醒阶段管理 | src/awakening.js | ✅喜欢 | 觉醒阶段 |
| 入口 | 入站消息 | inbound-message.js + pushMessage | src/inbound-message.js | ✅喜欢 | 消息入队 |
| 入口 | TUI | tui.js + startTUI | src/tui.js | ✅喜欢 | 终端 UI |
| 上下文 | gatherer | gatherer + keyword-context | src/context/gatherer.js | ✅喜欢 | 上下文聚合 |
| 上下文 | 规则引擎 | rule-engine + rule-risk + rule-store | src/context/rule-engine.js | ✅喜欢 | 规则引擎 |
| 上下文 | 运行时注入 | runtime-injector + section-gate | src/context/runtime-injector.js | ✅喜欢 | 运行时注入 |
| Prompt | coding-discipline | prompt-blocks/coding-discipline | src/prompt-blocks/coding-discipline.js | ✅喜欢 | 编码纪律 |
| Prompt | prompt 构建 | prompt.js + buildSystemPrompt/buildContextBlock | src/prompt.js | ✅喜欢 | Prompt 构建 |
| 审查 | reviewer | review/reviewer + 工作回顾 | src/review/reviewer.js | ✅喜欢 | 工作回顾 |
| 预取 | prefetch | prefetch/runner + 预取运行器 | src/prefetch/runner.js | ✅喜欢 | 预取机制 |
| 画像 | profile | profile/infer + 用户画像推断 | src/profile/infer.js | ✅喜欢 | 画像推断 |
| 终端 | terminal-stream | terminal-stream + 终端流 | src/terminal-stream.js | ✅喜欢 | 终端集成 |
| UI 桥接 | ui-bridge | ui-bridge + UI 桥接 | src/ui-bridge.js | ✅喜欢 | UI 桥接 |

#### 对话展示方式深度描述

**消息渲染方式**：
- 文本消息：Markdown 渲染（createMarkdownBody）+ 富文本支持 + 代码块高亮
- 代码消息：Markdown 代码块 + 语法高亮 + 可复制
- 图片消息：内嵌展示 + pasteAttachments 粘贴附件 + MAX_PASTED_IMAGES 8 + MAX_PASTED_IMAGE_BYTES 12MB
- 链接消息：卡片预览（通过 Markdown 链接渲染）
- 表格/列表：Markdown 表格 + 列表渲染

**对话流布局**：
- 单列布局：消息自上而下排列
- 用户消息和 AI 消息：统一方向（自上而下），通过气泡颜色/头像区分
- 消息间距：CSS 控制
- 头像位置：左侧头像 + 右侧气泡
- 时间戳位置：tStamp() 格式 HH:MM:SS + 消息底部

**特殊展示元素**：
- 流式气泡：liveEl 边收 token 边重渲染，message 事件到达后定稿
- 消息去重：claimRenderedMessage 双重去重（renderedMessageIds ID 去重 + recentRenderedKeys 内容去重 + RENDER_DEDUPE_TTL_MS 2min TTL）
- 渠道标签：friendlyChannelLabel（WeChat/WeCom/Discord/Feishu）显示消息来源渠道
- 激活预热锁：applyActivationWarmupLock 刚激活时显示"模型预热中… ~Xs"
- 输入锁定：setComposerLocked 系统准备中锁定输入
- 自适应高度：autoGrowInput 输入框高度自适应
- 语音输入提示：PUSH_TO_TALK_PLACEHOLDER "按住空格键开始说话" + 聚焦/未聚焦切换占位符
- 音频反馈：ensureAudioContext + unlockAudioOnFirstGesture + TTS 音效
- 悬停检测：isHoveringChat 检测鼠标是否悬停在聊天区

**实现文件**：
- src/ui/brain-ui/chat.js - 聊天核心
- src/ui/brain-ui/markdown.js - Markdown 渲染
- src/ui/brain-ui/tts-fx.js - TTS 音效
- src/ui/brain-ui/audio-output.js - 音频输出
- src/ui/brain-ui/app.js - 应用入口

#### 思考过程可视化深度描述

**展示形式**：
- 思考链展示：ThoughtStream 类，内嵌在消息气泡中（innerId 挂载到消息内部元素）
- 展示位置：在 AI 回复上方（thinkingEl 思考区 + lastToolEl 工具区 + statusEl 状态区）
- 展示时机：思考过程中实时流式展示（边思考边输出）+ thinkingLabel "思考中" + thinkingDoneLabel 完成标签

**展示内容**：
- 推理步骤：curLine 当前行 + thinkingEl 思考区实时流式输出
- 工具调用：57 个工具的中文映射（TOOL_ZH）+ 57 个工具的图标（TOOL_ICON）+ hadToolCall 标记 + lastToolEl 工具区 + toolDetailLength 160 字符截断
- 中间结果：工具调用结果 + isFailureResult 失败检测（正则匹配"错误/失败/异常/Error/ERROR" + JSON ok:false 检测）
- 自我修正：toolFailed 标记 + 失败状态展示
- 时间戳：tStamp() 格式 HH:MM:SS 每个步骤都有时间

**交互方式**：
- 展开/收起：思考区可折叠（curLine 控制）
- 视觉关联：thinkingEl + lastToolEl + statusEl 三区联动 + statusTimer 状态定时器
- 颜色区分：color 参数 + readCSSVar 读取 CSS 变量（适配主题）
- 工具图标：每个工具调用都有对应图标（TOOL_ICON 57 个 emoji 图标）

**实现文件**：
- src/ui/brain-ui/thought-stream.js - ThoughtStream 类
- turn-trace.html - 回合轨迹页面
- src/llm.js - LLM 流式调用（reasoning_effort=high）

#### 信息热点追踪深度描述

**追踪什么**：
- 4 平台热榜：抖音（douyin）/ 小红书（xiaohongshu）/ 微信热点（wechat）/ 微博（weibo）
- 实时事件流：8 类别（自然灾害/科技/财经/体育/社会/政策/旅游 + 其他）+ time/cat/catColor/title/desc/loc/img
- 底部跑马灯：8 条跑马灯文字（time/text）
- 国内外分区：trending.js CN→微博+知乎 / 其他→HN+Reddit，1h 缓存
- 3D 地球可视化：HotspotEarth + Three.js 地球热点标注

**数据获取方式**：
- 后端 API：/hotspots 接口提供实时热榜数据
- 前端实时源：buildHotspotContext 构建中性系统上下文
- 缓存策略：hotspotMeta.refreshMinutes 30 分钟刷新 + hotspotMeta.stale 缓存标记
- 定时刷新：hotspotRefreshTimer 定时刷新
- 模拟数据兜底：MOCK_FEED 8 条模拟事件 + TICKER_ITEMS 8 条跑马灯

**数据处理**：
- 过滤/排序：按排名（rank）+ 热度（heat）+ 趋势（trend：up/down/same）
- 去重：通过 platform 分组 + hotspotLists 按平台存储
- 关键信息提取：top 函数提取 Top3 + platformText 按平台拼接
- 上下文构建：buildHotspotContext 构建"中性系统上下文"（不强制 Agent 回复，仅在相关时提及）

**呈现方式**：
- 列表渲染：renderList + 排名样式（hs-rank-top1/2/3）+ 趋势样式（hs-trend-up/dn/same）+ 空状态（hs-item-empty）
- 趋势图标：TREND_ICONS（↑↓—）+ TREND_CLASSES
- 实时事件流：feedAutoTimer 自动轮播 + feedIndex 索引
- 底部跑马灯：TICKER_ITEMS 滚动展示
- 3D 地球：HotspotEarth + Three.js 地球可视化 + 热点标注
- 时钟：clockTimer 实时时钟
- 每个热点展示：rank（排名）+ text（标题）+ heat（热度值）+ trend（趋势）+ isNew（新热点标记）

**交互方式**：
- 点击展开：热点列表项可点击
- 发送到对话：buildHotspotContext 构建上下文 + Agent 在相关时主动提及
- 模式切换：hotspot_mode 工具切换热点模式 + moveVoicePanel 语音球搬家
- 刷新：hotspotRefreshTimer 自动刷新 + 手动刷新
- 3D 地球交互：Three.js 地球可旋转缩放

**实现文件**：
- src/ui/brain-ui/hotspot.js - 热点主逻辑
- src/ui/brain-ui/hotspot-earth.js - 3D 地球
- src/ui/brain-ui/hotspot-panel.js - 热点面板
- src/trending.js - 热点后端
- src/hotspots.js - 热点面板状态

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第八部分：功能重叠对比矩阵（72 功能点）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

| # | 功能点 | Pangu Nebula | NomiFun | OpenAkita | BaiLongma | 重叠情况 | 我的偏好来源 | 融合决策 |
|---|--------|-------------|-----------|-----------|-----------|----------|-------------|----------|
| 1 | 桌面框架 | ✅ Tauri 2 薄壳 | ✅ Tauri 2 进程内 | ✅ Tauri 2 Setup Center | ❌ Electron 33 | 三重叠 | NomiFun | 统一 Tauri 2 进程内后端（NomiFun） |
| 2 | 后端语言 | Python | Rust | Python | Node.js | 四重叠 | NomiFun | 统一 Rust（NomiFun axum） |
| 3 | 前端框架 | Preact 10 | React 19 | React 19 | 原生 JS | 三重叠 | NomiFun | 统一 React 19（NomiFun） |
| 4 | 前端 UI 库 | Tailwind | Arco Design + UnoCSS | shadcn/ui + Tailwind | 自研 ACUI | 三重叠 | NomiFun | Arco Design（NomiFun） |
| 5 | 状态管理 | Context+useReducer | React Context+ts-rs | React Context+lazy | sceneStore+Thread State | 四重叠 | NomiFun+BaiLongma | React Context（NomiFun）+ Scene Protocol（BaiLongma） |
| 6 | 数据存储 | SQLite+aiosqlite | SQLite+sqlx+rusqlite | SQLite+aiosqlite | SQLite+better-sqlite3 | 四重叠 | NomiFun | sqlx+rusqlite（NomiFun）+ sqlite-vec（BaiLongma） |
| 7 | 向量检索 | ❌ | ❌ | ChromaDB | sqlite-vec+transformers | 两重叠 | BaiLongma | sqlite-vec+本地嵌入（BaiLongma） |
| 8 | CRDT 多端同步 | ✅ 自研 | ❌ | ❌ | ❌ | Pangu 独有 | Pangu Nebula | automerge-rs 重写（RFC-004） |
| 9 | 对话界面 | ✅ 5 区域布局 | ✅ Arco 三栏 | ✅ shadcn 聊天 | ✅ Brain UI | 四重叠 | BaiLongma | Arco 布局（NomiFun）+ 对话展示（BaiLongma） |
| 10 | 消息渲染 | SSE 流式 | Markdown+CodeBlock+Mermaid | Markdown+Lightbox | Markdown+流式气泡 | 四重叠 | BaiLongma | Markdown（NomiFun）+ 流式气泡（BaiLongma） |
| 11 | 消息去重 | ❌ | ❌ | ❌ | ✅ 双重去重 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 12 | 输入框 | 基础 | SendBox+斜杠+表情+语音+@文件 | 基础 | 自适应+预热锁+语音提示 | 两重叠 | NomiFun+BaiLongma | SendBox（NomiFun）+ 预热锁（BaiLongma） |
| 13 | Agent 引擎 | 蜂群 Orchestrator | Agent engine+plan mode | Ralph 永不放弃 | consciousness-loop | 四重叠 | 全部融合 | DAG（RFC-002）=蜂群+Ralph+consciousness-loop |
| 14 | Agent 管理 | ❌ | ❌ | ✅ AgentManagerView | ✅ agents/registry | 两重叠 | OpenAkita | 选取 OpenAkita AgentManagerView |
| 15 | Agent 仪表盘 | ReactFlow 蜂群 | ❌ | ✅ TopoNode 力导向+粒子 | ❌ | 两重叠 | OpenAkita | 选取 OpenAkita AgentDashboardView |
| 16 | Agent 商店 | ❌ | ❌ | ✅ AgentStoreView | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 17 | 像素办公室 | ❌ | ❌ | ✅ PixelOffice+Phaser | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 18 | 组织编排 | 蜂群 2-8 Worker | ❌ | ✅ CEO/CTO/CFO | 委托 | 两重叠 | OpenAkita | 选取 OpenAkita 组织编排 |
| 19 | 多 Agent 并行 | ✅ 2-8 Worker | ❌ | ✅ 并行+接管+故障转移 | ✅ 委托 | 三重叠 | OpenAkita | 选取 OpenAkita |
| 20 | 任务分解 | ✅ | ✅ plan mode | ✅ | ❌ | 三重叠 | OpenAkita | 选取 OpenAkita |
| 21 | 共识验证 | ✅ | ❌ | ❌ | ❌ | Pangu 独有 | Pangu Nebula | 保留 Pangu |
| 22 | 记忆系统 | ✅ 6 层 L0-L5 | ✅ 1 层 YAML | ✅ 三层+MDRM | ✅ Thread 线索 | 四重叠 | OpenAkita+BaiLongma | 6 层 SoT（RFC-003）=Pangu 6 层+OpenAkita 三层+BaiLongma Thread |
| 23 | 记忆层级 | 6 层 | 1 层 | 三层 | Thread 温度窗口 | 四重叠 | Pangu+OpenAkita+BaiLongma | 6 层 SoT 融合三者 |
| 24 | 记忆可视化 | @antv/g6 图 | ❌ | MemoryGraph3D 3D | D3 记忆图 | 三重叠 | OpenAkita+BaiLongma | MemoryGraph3D（OpenAkita）+ D3（BaiLongma） |
| 25 | 记忆提取 | ❌ | distill 提炼 | MemoryExtractor | recognizer | 三重叠 | OpenAkita | 选取 OpenAkita MemoryExtractor |
| 26 | 记忆整合 | ❌ | ❌ | MemoryConsolidator | consolidation-loop | 两重叠 | OpenAkita | 选取 OpenAkita |
| 27 | 记忆保留策略 | ❌ | ❌ | apply_retention | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 28 | LLM 记忆审查 | ❌ | ❌ | ✅ ReviewProgress | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 29 | 记忆类型分类 | ❌ | ❌ | 8 类型 | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita 8 类型 |
| 30 | Thread 线索模型 | ❌ | ❌ | ❌ | ✅ 多并发+温度窗口+承诺 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma（L2 情景层） |
| 31 | ACI 预判注入 | ❌ | ❌ | ❌ | ✅ 3 类预判+置信度分级 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 32 | 自进化 | ❌ | ❌ | ✅ 每日自检 | ✅ self-evolution | 两重叠 | BaiLongma | 选取 BaiLongma |
| 33 | 自感知 | L5 元认知 | ❌ | ❌ | ✅ self-perception | 两重叠 | Pangu+BaiLongma | 融合为 L5 |
| 34 | 身份系统 | ❌ | ❌ | ✅ SOUL/AGENT/USER/MEMORY+8 personas | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 35 | 思考过程展示 | SSE 流式 | plan mode | ✅ ThinkingChain | ✅ ThoughtStream 57 工具 | 三重叠 | BaiLongma | 选取 BaiLongma ThoughtStream |
| 36 | 回合轨迹 | ❌ | ❌ | ❌ | ✅ turn-trace | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 37 | reasoning_effort | ❌ | ❌ | ❌ | ✅ high | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 38 | 监视面板 | ReactFlow 蜂群 | ❌ | ✅ 11 面板 | Brain UI 多面板 | 三重叠 | OpenAkita | 选取 OpenAkita 11 面板 |
| 39 | Token 统计 | ❌ | ❌ | ✅ 6 周期 5 维度 | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 40 | 进度账本 | ❌ | ❌ | ✅ ProgressLedgerTimeline | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 41 | 故障诊断 | ❌ | ❌ | ✅ TroubleshootPanel | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 42 | 收件箱 | ❌ | ❌ | ✅ InboxView+PendingApprovals | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 43 | 热点追踪 | ❌ | ❌ | ✅ Proactive Engine | ✅ 4 平台+3D 地球 | 两重叠 | BaiLongma | 选取 BaiLongma（3D 地球+4 平台） |
| 44 | 3D 地球 | ❌ | ❌ | ❌ | ✅ Three.js | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 45 | 实时事件流 | ❌ | ❌ | ❌ | ✅ 8 类别+跑马灯 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 46 | Scene Protocol | ❌ | ❌ | ❌ | ✅ UI=f(scene) | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 47 | Scene Shell 12 kind | ❌ | ❌ | ❌ | ✅ awakening/choice/dom/... | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 48 | Tick 心跳 | ❌ | ❌ | ❌ | ✅ LLM-directed | BaiLongma 独有 | BaiLongma | 选取 BaiLongma（L5 运行时） |
| 49 | consciousness-loop | ❌ | ❌ | ❌ | ✅ 持续意识 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma（L5 运行时） |
| 50 | 觉醒阶段 | ❌ | ❌ | ❌ | ✅ 前 10 tick 10s | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 51 | 严格评估 | ❌ | ❌ | ❌ | ✅ strict-evaluation | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 52 | LLM Provider | ❌ | 多 Provider 轮换 | ✅ 30+ LLM | ✅ 7 Provider | 三重叠 | OpenAkita | 选取 OpenAkita 30+ LLM |
| 53 | 流式保护 | SSE 断点续传 | ❌ | ❌ | ✅ idle timeout 45s+watchdog 600s | 两重叠 | Pangu+BaiLongma | 融合两者 |
| 54 | IM 平台 | ❌ | 12 通道 logos | ✅ 6 IM 二维码绑定 | ✅ Discord+飞书+微信 | 三重叠 | OpenAkita | 选取 OpenAkita 6 IM |
| 55 | 安全栈 | ✅ 11 安全栈 | ❌ | ✅ 6 层沙箱 | ❌ | 两重叠 | Pangu+OpenAkita | 融合 11+6=17 层 |
| 56 | Ralph 永不放弃 | ❌ | ❌ | ✅ max_attempts=10 | ❌ | OpenAkita 独有 | OpenAkita | 选取 OpenAkita |
| 57 | 插件系统 | ❌ | ❌ | ✅ 8 类插件+3 层权限 | ✅ 工具市场 | 两重叠 | OpenAkita | 选取 OpenAkita 8 类插件 |
| 58 | 技能系统 | ❌ | Skills 注册 | ✅ SkillManager+Store | ✅ Skills 注册 | 三重叠 | OpenAkita | 选取 OpenAkita |
| 59 | MCP 集成 | ❌ | ✅ mcpRequest | ✅ MCPView | ❌ | 两重叠 | OpenAkita | 选取 OpenAkita |
| 60 | 定时任务 | ❌ | ✅ Cron 完善 | ✅ SchedulerView | ✅ reminders | 三重叠 | NomiFun | 选取 NomiFun Cron |
| 61 | 知识库 | ✅ kb router | ✅ Knowledge 完整 | ❌ | ✅ doc-panel | 三重叠 | NomiFun | 选取 NomiFun Knowledge |
| 62 | 桌宠 | ✅ MascotAssistant | ✅ Companion 完整 | ✅ PetView | ❌ | 三重叠 | NomiFun | 选取 NomiFun Companion |
| 63 | 语音系统 | ❌ | SpeechInputButton | ❌ | ✅ ASR+TTS+Whisper+唤醒词 | 两重叠 | BaiLongma | 选取 BaiLongma |
| 64 | i18n | ❌ | ✅ 类型安全 | ✅ en/zh | ❌ | 两重叠 | NomiFun | 选取 NomiFun |
| 65 | 自动更新 | ✅ GitHub releases | ✅ updater pubkey | ✅ AppUpdateDialog | ✅ electron-updater | 四重叠 | NomiFun | 选取 NomiFun |
| 66 | 测试套件 | ❌ | ✅ cargo test | ✅ pytest | ✅ 60+ 测试 | 三重叠 | NomiFun | 选取 NomiFun cargo test |
| 67 | DAG 画布 | ❌ | ✅ DagCanvas | ❌ | ❌ | NomiFun 独有 | NomiFun | 选取 NomiFun |
| 68 | 代码差异 | ❌ | ✅ Diff2Html | ❌ | ❌ | NomiFun 独有 | NomiFun | 选取 NomiFun |
| 69 | 终端集成 | ❌ | ✅ xterm | ❌ | ✅ terminal-stream | 两重叠 | NomiFun | 选取 NomiFun xterm |
| 70 | 本地资源扫描 | ❌ | ❌ | ❌ | ✅ Agent+SSH+Git 检测 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 71 | 专题面板 | ❌ | ❌ | ❌ | ✅ 台风+世界杯+天气+地图+人物 | BaiLongma 独有 | BaiLongma | 选取 BaiLongma |
| 72 | 配置管理 | ✅ NEBULA_* | ✅ configService | ✅ SetupView | ✅ config.js | 四重叠 | NomiFun | 选取 NomiFun configService |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第九部分：融合决策汇总
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 9.1 主导项目选取分布

基于 72 个功能点的对比，融合策略如下：

- **NomiFun（28 项）**：前端 UI/Arco Design/React 19/状态管理/数据存储/DAG 画布/桌宠/i18n/自动更新/测试套件/知识库/定时任务/配置管理/代码差异/终端/通道 logos/输入框/SendBox/Markdown/多 Provider
- **OpenAkita（22 项）**：Agent 管理/Agent 仪表盘/Agent 商店/像素办公室/组织编排/多 Agent 并行/记忆提取/记忆整合/记忆保留/LLM 审查/记忆类型/身份系统/监视面板/Token 统计/进度账本/故障诊断/收件箱/LLM Provider/IM 平台/Ralph 永不放弃/插件系统/技能系统/MCP
- **BaiLongma（18 项）**：对话展示/消息去重/思考过程/回合轨迹/reasoning_effort/热点追踪/3D 地球/实时事件流/Scene Protocol/Scene Shell/Tick 心跳/consciousness-loop/觉醒阶段/严格评估/ACI 预判注入/Thread 线索模型/自进化/语音系统/本地资源扫描/专题面板
- **Pangu Nebula（4 项保留）**：6 层 L0-L5 架构（SoT 蓝本）/11 安全栈/共识验证/CRDT 意识

### 9.2 融合架构（RFC 对应）

- **RFC-001 crate 边界**：基于 NomiFun 50 crate workspace 划分 SparkFox 14 crate
- **RFC-002 编排协调**：DAG = Pangu 蜂群 + OpenAkita 组织编排 + BaiLongma consciousness-loop
- **RFC-003 记忆 SoT**：6 层 = Pangu 6 层架构 + OpenAkita 三层实现 + BaiLongma Thread 线索模型（L2）
- **RFC-004 CRDT**：automerge-rs（NomiFun Rust 栈最易集成）+ Double Ratchet（新建）
- **RFC-005 并行度**：3-4 并行 + target 隔离

### 9.3 核心结论

1. **主体技术栈**：Tauri 2 + Rust（NomiFun 基础）+ React 19 + Arco Design + SQLite + automerge-rs
2. **记忆系统 SoT**：Pangu Nebula 6 层架构 + OpenAkita 三层实现 + BaiLongma Thread 线索模型（L2 情景层）
3. **编排协调**：OpenAkita 组织编排 + Pangu Nebula 蜂群 + BaiLongma consciousness-loop 融合为 DAG
4. **UI/UX**：NomiFun Arco Design + BaiLongma Scene Protocol v1 + BaiLongma 对话展示 + BaiLongma 思考过程
5. **L5 元认知运行时**：BaiLongma consciousness-loop + Tick 心跳 LLM-directed
6. **AGPL 合规**：OpenAkita 直接吸收，BaiLongma + NomiFun 清洁室重写
7. **下一步**：Phase -1 PoC 启动（4-6 周，4 项 PoC）

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 附录：参考文档
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

- SparkFox-重组优化方案-1.0.md - 主方案 v1.0
- poc-report.md - Phase -1 PoC 验收报告模板
- rfc/RFC-001-crate-boundaries.md - crate 边界重划
- rfc/RFC-002-orchestration-coordination.md - 编排协调
- rfc/RFC-003-memory-source-of-truth.md - 记忆 SoT
- rfc/RFC-004-crdt-selection.md - CRDT 选型
- rfc/RFC-005-parallelism-and-target-isolation.md - 并行度

---

*本报告基于 2026-07-18 的四项目源码深度阅读生成，作为 SparkFox 融合重组的决策依据。*
