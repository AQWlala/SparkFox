# SparkFox 重组优化方案 1.0

> **报告日期**: 2026-07-18
> **版本**: v1.0（基于 v0.0 七专家评审优化迭代）
> **报告目的**: 设计 Pangu Nebula + NomiFun + OpenAkita + BaiLongma 四项目的深度融合重组方案，作为 SparkFox 项目蓝图
> **品牌**: SparkFox（斯帕克狐）—— 全新独立品牌，致谢 NomiFun / BaiLongma / Pangu Nebula
> **数据来源**:
> - Pangu Nebula 本地源码审查（D:\Pangu Nebula，v1.1.0，Tauri 2 + Python sidecar 架构）
> - NomiFun GitHub 仓库（nomifun/nomifun-tauri，v0.2.28，50+ Rust crate workspace）
> - OpenAkita GitHub 仓库（openakita/openakita，v1.27.13，AGPL-3.0）
> - BaiLongma GitHub 仓库（xiaoyuanda666-ship-it/BaiLongma，v2.1.515，Electron + Node.js）
> - 用户 memory 中 2026-07-18 已确认的 12 步融合决策
> - **v1.0 新增**：7 专家评审意见（架构 / 记忆系统 / 安全 / 性能 / 工程化 / 产品 / 风险评估）
> **融合原则**: 以 NomiFun 为骨架、Pangu Nebula 记忆体系为护城河、BaiLongma 与 OpenAkita 补强交互与执行层；架构借鉴不复制代码；Rust + React 全栈重写
> **目标用户群**: 桌面端为主，需支持 Web 端与移动端，给其他人用
> **产品定位**: 具备元认知的本地优先超级 AI 工作站

---

## v1.0 修订摘要（相对 v0.0）

| # | 修订项 | v0.0 | v1.0 |
|---|--------|------|------|
| 1 | 品牌 | Pangu Nebula v3.0 | **SparkFox（斯帕克狐）** |
| 2 | 实施路线 | Phase 0-6（33-46 周） | **Phase -1 ~ 6（75-130 周）** |
| 3 | 并行度 | 14 并行 agent | **3-4 并行 agent** |
| 4 | CRDT 选型 | 自研 LWW+OR-Set | **automerge-rs** |
| 5 | E2EE | X25519+HKDF+AES-256-GCM | **+ Double Ratchet（前向保密）** |
| 6 | L5 元认知 | 直接进入 Phase 1 | **Phase -1 PoC 验证价值后决定** |
| 7 | 遗忘机制 | 缺失 | **新增 5 种遗忘策略** |
| 8 | AGPL 合规 | 仅声明借鉴 | **建立清洁室流程** |
| 9 | 工程化 | 仅 CI/CD | **+ SBOM/feature flag/sentry/测试金字塔/代码签名自动化** |
| 10 | Kill Switch | 无 | **每 Phase 失败终止条件 + 回滚预案** |
| 11 | MVP 验收 | 无 | **Phase 1.5 PMF 验证（10 用户访谈）** |
| 12 | RFC 前置 | 无 | **5 个 RFC（crate 边界/编排协调/记忆 SoT/CRDT/并行度）** |

---

## 第一部分：各项目深度拆解

> 本部分内容与 v0.0 一致，保留原拆解。详细内容见各项目小节。

### 1.1 Pangu Nebula（本地项目，v1.1.0）

**项目定位一句话总结**：Pangu Nebula 是一个具备元认知的多 Agent 认知运行时——不是聊天机器人，而是有状态认知系统 `output = f(input, memory, metacognition, evolution)`，解决 LLM 无记忆、单 Agent 推理不可靠、Agent 不学习三大难题。

**核心护城河**（5 条）：
1. 6 层记忆图谱（L0-L5）+ L5 元认知（业界少数独立元认知层设计之一，v1.0 修正"业界唯一"过强声明）
2. 海绵/黑洞双引擎工程分离
3. 11 项安全栈已落地
4. CRDT 跨设备同步独有（v1.0 改用 automerge-rs）
5. 蜂群共识 + 三维预算

**技术栈**：Tauri 2 + Python sidecar + Preact + SQLite（将全栈 Rust 重写）

**完整拆解**：见 v0.0 第一部分 1.1（21 项功能清单 + 5 优势 + 5 局限 + 可复用资产清单）

### 1.2 NomiFun（nomifun-tauri，v0.2.28）

**项目定位一句话总结**：NomiFun 是一个毫无保留、完全开源、本地优先的「超级 AI 工作站」——一套 React 前端 + 一套 Rust 后端，在用户本机内提供会成长的桌面伙伴、无人值守自动化平台、统一知识库、原生 Computer/Browser Use，以及任何智能体都能驱动的开放能力总线。

**核心优势**（7 条）：
1. 真正的本地优先 + 完全开源（Apache-2.0）
2. 进程内自研 Computer Use + Browser Use（无 Playwright）
3. 开放能力总线设计优雅（~20 域 150+ 工具单一注册表）
4. 无人值守自动化闭环完整（Requirements + AutoWork + IDMM）
5. 多伙伴家庭 + 共享记忆中枢（记忆中枢将被 Pangu 6 层替代）
6. 工程严谨（50+ crate 清晰分层，ts-rs 契约）
7. 26+ Provider + 19 ACP 外部 agent

**技术栈**：Rust 2024 + Tauri 2 + React 19 + Arco Design + UnoCSS + SQLite + sqlx

**完整拆解**：见 v0.0 第一部分 1.2（22 项功能清单 + 7 优势 + 6 局限 + 可复用 crate 清单）

### 1.3 OpenAkita（v1.27.13，AGPL-3.0）

**项目定位一句话总结**：OpenAkita 是一个开源的、自进化的多 Agent AI 助手框架——「不止于聊天，是一支能干活的 AI 团队」。

**核心优势**（7 条）：
1. 零 CLI 上手（全 GUI）
2. 多 Agent + 组织编排深度
3. 透明可解释的 Markdown 记忆
4. 自我进化日更流水线（03:00 整合 + 04:00 自检）
5. 6 层原生 OS 沙箱（bwrap/seatbelt/MIC）
6. 30+ LLM 智能故障转移
7. 6 IM 平台深度集成 + 扫码绑定

**协议风险**：AGPL-3.0（v1.0 建立清洁室流程规避传染性）

**完整拆解**：见 v0.0 第一部分 1.3（24 项功能清单 + 7 优势 + 5 局限 + 可复用资产清单）

### 1.4 BaiLongma（v2.1.515）

**项目定位一句话总结**：BaiLongma 是一个持续运行的桌面 AI Agent 项目，由 Electron 桌面壳驱动，让 AI 从「会回答问题的聊天框」进化成「常驻本地、能记住、能行动、能展示、能自我维护的桌面智能体框架」。

**核心优势**（8 条）：
1. 持续运行 Agent 范式（Tick 心跳）
2. ACI 预判注入创新（v1.0 改为异步，不阻塞主循环）
3. Scene Protocol 声明式 UI 架构
4. 本地优先长期记忆系统
5. 按需工具注入 + find_tool 自发现
6. 多模型 + 多渠道接入
7. 本地隐私优先
8. 工具市场 + 动态 API slots

**完整拆解**：见 v0.0 第一部分 1.4（30 项功能清单 + 8 优势 + 7 局限 + 可复用资产清单）

---

## 第二部分：功能交叉对比矩阵

> 本部分内容与 v0.0 一致，保留原 5 个维度对比矩阵（记忆与认知 / Agent 编排与执行 / UI/UX / 能力总线与集成 / 安全与工程）。

### 2.1 融合决策汇总

| 决策类型 | 数量 | 说明 |
|---------|------|------|
| **合并增强** | 22 项 | 取各项目之长融合（记忆/编排/UI/能力总线/安全） |
| **择优选取** | 18 项 | 选最强方案（NomiFun DAG/UI/Computer Use、OpenAkita 沙箱/插件、Pangu 安全/CRDT） |
| **新增补充** | 9 项 | 引入新维度（向量检索、ACI 预判、线索模型、Scene Protocol、热点追踪） |
| **保留增强** | 11 项 | Pangu 独有护城河全部保留 Rust 重写（6 层记忆/双引擎/CRDT/E2EE/安全栈等） |
| **删减淘汰** | 5 项 | Electron/原生 JS/Preact/AGPL-3.0/AI 公司组织编排 |

**完整对比矩阵**：见 v0.0 第二部分 2.1-2.6（5 个维度对比表）

---

## 第三部分：融合重组蓝图

### 3.1 整体架构选型

#### 3.1.1 前端框架：React 19 + Arco Design + UnoCSS（来自 NomiFun）

直接复用 NomiFun ui/ 全套，Pangu Preact 24 组件废弃，BaiLongma 原生 JS 重写为 React。

#### 3.1.2 后端框架：Rust 2024 + Tauri 2（来自 NomiFun，全栈 Rust 重写）

直接复用 NomiFun 50+ crate workspace，Pangu Python 32K 行 + BaiLongma Node.js 86KB 单文件全部 Rust 重写。

#### 3.1.3 记忆体系：Pangu 6 层为 SoT + OpenAkita 三层身份为投影 + BaiLongma 线索/焦点为投影 + sqlite-vec 向量

**v1.0 关键修正**（RFC-003）：
- **6 层记忆为单一真相源（SoT）**，身份层文件（SOUL.md/AGENT.md/USER.md/MEMORY.md）为投影
- **L5 元认知重新定位为"横向平面"**（跨 L0-L4 的元数据），非"第 6 层"纵向叠加
- **新增遗忘机制**（5 种策略：TTL / 容量上限 / 用户主动 / 合规擦除 / 黑洞压缩）
- **automerge-rs 替代自研 CRDT**（RFC-004）

#### 3.1.4 数据存储：SQLite + sqlite-vec + Markdown 身份文件 + JSONL 事件流

与 v0.0 一致。

### 3.2 模块拓扑

#### 3.2.1 整体架构图（v1.0 更新品牌）

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SparkFox 1.0（融合版）                              │
│           "具备元认知的本地优先超级 AI 工作站"                          │
├─────────────────────────────────────────────────────────────────────┤
│  Tauri 2 桌面壳（NomiFun）                                            │
│  ├─ 7 Tauri Plugins + keepawake                                       │
│  ├─ 多伙伴家庭窗口（NomiFun，透明置顶可拖动）                            │
│  └─ WebUI 扫码配对（NomiFun + Pangu E2EE 安全引导）                     │
├─────────────────────────────────────────────────────────────────────┤
│  React 19 + Arco Design + UnoCSS 前端（NomiFun）                      │
│  ├─ Scene Protocol 声明式 UI（BaiLongma）← 统一 UI 协议                │
│  ├─ 11 桌面面板（OpenAkita 布局借鉴）                                   │
│  ├─ DAG 画布（NomiFun react-flow）                                     │
│  ├─ 实时思考流 + 自主行动流（BaiLongma，异步）                           │
│  ├─ 记忆节点图 D3 力导向 + MDRM 3D（BaiLongma + OpenAkita）             │
│  ├─ 信息热点追踪面板（BaiLongma，v0.4+）                                │
│  ├─ 知识库审阅收件箱（NomiFun unified-diff）                            │
│  └─ ts-rs 类型契约（NomiFun，Rust→TypeScript 自动生成）                  │
├─────────────────────────────────────────────────────────────────────┤
│  Rust 2024 后端（NomiFun 50+ crate workspace + SparkFox 重写）          │
│  ┌─ 记忆层（SparkFox 重写为核心护城河，6 层为 SoT）                      │
│  │  ├─ 6 层记忆图谱（L0-L4 纵向存储 + L5 横向元认知平面）                 │
│  │  ├─ 海绵引擎 + 黑洞引擎（双引擎压缩可追溯）                            │
│  │  ├─ 身份层文件（OpenAkita 投影：SOUL.md/AGENT.md/USER.md/MEMORY.md）  │
│  │  ├─ 线索模型 + 焦点栈（BaiLongma 投影）                                │
│  │  ├─ sqlite-vec 向量检索 + bge-large-zh-v1.5 本地 embedding            │
│  │  ├─ MDRM 5 维度关系图谱（L3 关系索引）                                 │
│  │  ├─ CRDT 跨设备同步（automerge-rs，v1.0 修正）                         │
│  │  ├─ E2EE 加密（X25519+HKDF+AES-256-GCM+Double Ratchet，v1.0 增强）    │
│  │  ├─ 03:00 记忆整合 + 04:00 自检修复日更流水线（OpenAkita）             │
│  │  ├─ LoopEngine 4 阶段反思闭环（SparkFox）                             │
│  │  └─ 遗忘机制（v1.0 新增：TTL/容量/用户主动/合规/黑洞）                  │
│  │                                                                      │
│  ┌─ Agent 编排层（DAG 为主，其他为策略插件，RFC-002）                    │
│  │  ├─ DAG 聚合根（NomiFun：react-flow + 节点级 preflight）← 主编排      │
│  │  ├─ 蜂群共识（Pangu：Advisor+Orchestrator+Worker+Verifier）← 策略插件 │
│  │  ├─ Ralph 永不放弃引擎（OpenAkita）← 失败重试策略插件                  │
│  │  ├─ AutoWork + IDMM 三层保活（NomiFun）                                │
│  │  ├─ Tick 心跳机制（BaiLongma，默认关闭，低成本模型，异步）              │
│  │  ├─ ACI 预判注入（BaiLongma，异步非阻塞，v1.0 修正）                   │
│  │  ├─ ReAct 推理引擎 + Checkpoint（OpenAkita）                           │
│  │  ├─ Plan Mode 任务分解 + 回滚（OpenAkita）                             │
│  │  ├─ 三维预算控制（SparkFox：Token/时间/费用）                          │
│  │  ├─ 渐进式工具披露 3 层（OpenAkita）+ 按需注入（BaiLongma）            │
│  │  ├─ find_tool 自发现（BaiLongma）                                     │
│  │  └─ 动态 API Slots（BaiLongma）                                       │
│  │                                                                      │
│  ┌─ 能力总线层（NomiFun 为基）                                           │
│  │  ├─ 开放能力总线（NomiFun：~20 域 150+ 工具单一注册表）                 │
│  │  ├─ ACP 外部 Agent 协议（NomiFun：19 个外部 agent 直连）               │
│  │  ├─ 32 平台 Gateway 工具（NomiFun：nomi_* 前缀）                      │
│  │  ├─ MCP Server OAuth + 适配器（NomiFun）                              │
│  │  ├─ 26+ LLM Provider + 4 线缆协议（NomiFun + OpenAkita 智能故障转移） │
│  │  ├─ 11 IM 渠道（NomiFun：4 完整 + 7 占位）+ OpenAkita 扫码绑定         │
│  │  ├─ 插件系统（OpenAkita：8 类型/3 级权限/10 钩子，v0.5+）              │
│  │  ├─ Skills 系统（NomiFun 赠予 + OpenAkita 一键安装 + SparkFox 蒸馏）   │
│  │  ├─ Computer Use 进程内 Rust（NomiFun：无障碍树+SoM+OCR）             │
│  │  ├─ Browser Use 进程内 CDP（NomiFun：chromiumoxide + ARIA + 防火墙）  │
│  │  ├─ 知识库安全写回（NomiFun：staged/direct + unified-diff + URL 快照）│
│  │  ├─ Cron 定时任务（NomiFun：按时区归一化）                             │
│  │  └─ 终端模式 PTY（NomiFun：portable-pty）                             │
│  │                                                                      │
│  ┌─ 安全层（SparkFox 11 项 + OpenAkita 6 层沙箱 + Double Ratchet）      │
│  │  ├─ 11 项安全栈（SparkFox：DPAPI/AES-GCM/ACL/Injection/SSRF/Sandbox/ │
│  │  │  Audit/KeyRotation/OAuth/DID/Redactor）                            │
│  │  ├─ 6 层原生 OS 沙箱（OpenAkita：bwrap/seatbelt/MIC）                 │
│  │  ├─ CU 安全四件套（SparkFox：sandbox+audit+emergency_stop+rollback）  │
│  │  ├─ 排他服务器锁（NomiFun：fs2 Unix flock/Win LockFileEx）             │
│  │  ├─ 凭据保险库（NomiFun：eTLD+1 域绑定 + AES-GCM）                     │
│  │  ├─ POLICIES.yaml 策略引擎（OpenAkita：危险操作确认 + 死循环检测）     │
│  │  └─ Double Ratchet 前向保密（v1.0 新增）                              │
│  │                                                                      │
│  ┌─ 数据层（NomiFun 为基 + sqlite-vec）                                 │
│  │  ├─ SQLite（sqlx 0.8 + rusqlite 0.32 bundled）                       │
│  │  ├─ ~20 对 repository trait + Sqlite 实现（NomiFun）                  │
│  │  ├─ sqlite-vec 向量检索（BaiLongma）                                  │
│  │  ├─ bge-large-zh-v1.5 本地 embedding（BaiLongma，Rust 推理 PoC-3）    │
│  │  ├─ Markdown 身份层文件（OpenAkita 投影）                              │
│  │  ├─ JSONL 事件流（NomiFun：companion/shared/events/YYYYMMDD.jsonl）   │
│  │  ├─ 知识库文件树（NomiFun：.sparkfox/knowledge/ + junction/symlink）  │
│  │  ├─ AES-256-GCM 静态加密（NomiFun）                                   │
│  │  ├─ sqlx 迁移内嵌（NomiFun：schema 只向前演进）                       │
│  │  └─ 遗忘日志（v1.0 新增：memory_forget_log 表）                       │
│  │                                                                      │
│  └─ 多端访问层                                                          │
│     ├─ Tauri 2 桌面（Win/Mac/Linux + 自动更新）                          │
│     ├─ Web 主机（NomiFun：sparkfox-web 无头部署）                        │
│     ├─ Mobile（OpenAkita：Capacitor 包装 Android/iOS，v1.0+）            │
│     └─ Docker（NomiFun：Dockerfile + docker-compose）                    │
└─────────────────────────────────────────────────────────────────────┘
```

#### 3.2.2 模块来源映射表

与 v0.0 一致（保留原表，仅品牌替换 Pangu Nebula → SparkFox）。详见 v0.0 第三部分 3.2.2。

### 3.3 关键取舍决策表（v1.0 更新）

| 决策项 | 选择了什么 | 放弃了什么 | v1.0 变更 |
|--------|-----------|-----------|----------|
| **前端框架** | React 19 + Arco Design + UnoCSS（NomiFun） | Preact / 原生 JS | 不变 |
| **后端语言** | Rust 2024（NomiFun 50+ crate） | Python / Node.js | 不变 |
| **桌面壳** | Tauri 2（NomiFun） | PyWebView / Electron | 不变 |
| **数据库** | SQLite + sqlite-vec + Markdown | LanceDB | 不变 |
| **记忆体系** | 6 层为 SoT + 身份/线索为投影 + sqlite-vec | NomiFun 共享记忆中枢 | **v1.0: 6 层为 SoT，身份/线索为投影（RFC-003）** |
| **元认知层** | L5 横向元认知平面（PoC-1 验证后决定） | 无 | **v1.0: L5 改为横向平面，PoC 验证价值** |
| **跨设备同步** | automerge-rs（RFC-004） | 自研 CRDT | **v1.0: 改用 automerge-rs** |
| **E2EE** | X25519+HKDF+AES-256-GCM+Double Ratchet | 无 | **v1.0: 新增 Double Ratchet 前向保密** |
| **安全栈** | SparkFox 11 项 + OpenAkita 6 层沙箱 | NomiFun 3 项 | 不变 |
| **DAG 编排** | NomiFun react-flow（主编排） | Pangu SwarmProgress | **v1.0: DAG 为主，其他为策略插件（RFC-002）** |
| **Computer Use** | NomiFun 进程内 Rust | Pangu Playwright | 不变 |
| **ACI 预判** | BaiLongma（异步非阻塞） | 无 | **v1.0: 异步化，修正 1.5s 同步阻塞** |
| **Tick 心跳** | BaiLongma（默认关闭，低成本模型） | 无 | **v1.0: 默认关闭，月成本 <$3** |
| **品牌策略** | **SparkFox 全新品牌** + 致谢 NomiFun/BaiLongma/Pangu Nebula | Pangu Nebula 主品牌 | **v1.0: 全新 SparkFox 品牌** |
| **许可协议** | Apache-2.0 | OpenAkita AGPL-3.0 | 不变 |
| **AGPL 合规** | **清洁室流程** | 仅声明借鉴 | **v1.0: 建立清洁室流程** |
| **并行度** | **3-4 并行 agent** | 14 并行 | **v1.0: 降级到 3-4（RFC-005）** |
| **遗忘机制** | **5 种策略** | 无 | **v1.0: 新增遗忘机制** |

---

## 第四部分：实施路线图（v1.0 重写）

> **总体策略**: 增量迁移（用户 memory 决策），分阶段提交，每阶段单次 Git commit
> **并行策略**: Phase -1/0 串行，Phase 1-5 使用 3-4 并行 agent（RFC-005）
> **测试策略**: 每 Phase 必须包含 2-3 配套测试 + 测试金字塔 70% 覆盖率
> **v1.0 新增**: Kill Switch + 回滚预案 + PMF 验证 + RFC 前置

### 阶段 -1：PoC 验证（必须最先，串行）

**目标**：用最小成本验证 4 个最高风险假设，不通过则方案推翻重来

**内容**（4-6 周，串行）:
1. **PoC-1: L5 元认知价值验证**（Kill Switch: 失败→重评定位+MVP）
   - A 组（无 L5）vs B 组（有 L5）50 轮对话对比
   - 验收：任务完成率 B ≥ A+10% 或 token 成本 B ≤ A×85%
2. **PoC-2: automerge-rs CRDT 可行性**
   - 2 设备 1000 条同步 + 离线 1 小时 + 3 设备并发
   - 验收：0 冲突丢失、同步 <2s、CPU <5%
3. **PoC-3: bge-large-zh Rust 推理性**
   - candle/ort 推理 10 万向量
   - 验收：单条 <50ms、10 万检索 <500ms、内存 <300MB
4. **PoC-4: NomiFun + sqlite-vec 性能基线**
   - 三平台实测启动/RAG/包体/内存
   - 验收：启动 <3s、RAG 10 万 <800ms、包体 <200MB

**产出**: `docs/poc-report.md`（含 go/no-go 决策，7 专家签字）

**预计难度**: 中等
**时间估算**: 4-6 周
**验收标准**:
- [ ] 4 项 PoC 全部 GO 或条件性 GO
- [ ] PoC 报告经 7 专家评审签字
- [ ] 任何 NO-GO 项的 Kill Switch 已执行

**Kill Switch**:
- PoC-1 失败 → 砍 L5，重评"具备元认知"定位 + MVP 范围
- PoC-2 失败 → CRDT 推迟到 v0.5+，先做单机版
- PoC-3 失败 → 退回 Python sidecar（仅 embedding 模块）
- PoC-4 失败 → 重设性能目标 + 砍 30% 功能

### 阶段 0：基座 + 工程化（必须完成，串行）

**目标**: 搭建 SparkFox 品牌基座 + 工程化基础设施 + 5 个 RFC 前置决策

**内容**（3-4 周，串行）:
1. **5 个 RFC 前置决策**（每个 1-2 天）
   - RFC-001: crate 边界重划
   - RFC-002: 编排引擎协调机制
   - RFC-003: 记忆体系单一真相源
   - RFC-004: CRDT 选型（automerge-rs）
   - RFC-005: 并行度与 target 隔离
2. **品牌重命名**: NomiFun → SparkFox（24+ 文件，80+ 处引用）
   - Tauri 配置: tauri.conf.json / tauri.dev.conf.json
   - Cargo.toml: 36 处 crate name（nomifun-* → sparkfox-*）
   - package.json: nomifun-tauri → sparkfox-tauri
   - Rust 源码: main.rs / channel.rs 等
   - 前端源码: 40+ 处版权声明
   - 脚本: seed-dev-from-prod.mjs 等
   - 环境变量: NOMI_CHANNEL → SPARKFOX_CHANNEL
   - localStorage: __nomifun_theme → __sparkfox_theme
3. **删除 NomiFun 共享记忆中枢**（将被 SparkFox 6 层记忆替代）
4. **工程化基础设施补全**（v1.0 新增，工程化专家要求）:
   - monorepo 工具: cargo workspace + turbo/nx（前端）
   - 依赖更新: dependabot / renovate
   - 错误监控: sentry-rust + sentry-react
   - feature flag: unleash / launchdarkly（L5/CRDT 等高风险灰度）
   - 供应链安全: cargo-audit + cargo-deny + SBOM（syft）
   - 测试金字塔: 单元 70% / 集成 20% / E2E 10%，覆盖率门槛 70%
   - 代码签名自动化: GitHub Actions + Apple notarytool + Windows signtool
5. **AGPL 清洁室流程建立**（v1.0 新增，安全专家要求）:
   - 隔离阅读区: OpenAkita 源码阅读与 SparkFox 实现物理隔离
   - 写规范: 阅读者写设计规范文档，不含代码
   - 第三方实现: 实现者只看规范，不看 OpenAkita 源码
   - 法务审计: 最终代码经法务审计确认无 AGPL 传染
6. **Apache-2.0 许可证 + README 致谢** NomiFun / BaiLongma / Pangu Nebula
7. **配置 CI/CD**: GitHub Actions + swatinem/rust-cache 三平台构建
8. **配置 ts-rs 类型契约 + insta 快照测试**

**预计难度**: 中等
**时间估算**: 3-4 周
**验收标准**:
- [ ] 5 个 RFC 经 3 专家评审定稿
- [ ] Tauri 2 桌面应用以 SparkFox 品牌启动（Win/Mac/Linux）
- [ ] 50+ crate workspace 编译通过
- [ ] React 19 前端构建通过
- [ ] sqlx 数据层迁移成功
- [ ] CI/CD 三平台构建通过
- [ ] 测试覆盖率 70%
- [ ] AGPL 清洁室流程文档化
- [ ] sentry / feature flag / SBOM 集成

### 阶段 1：SparkFox 核心护城河 Rust 重写（3-4 并行，4 批次）

**目标**: 将 SparkFox 的 14 项核心护城河 Rust 重写（MVP 不削减，含 L5/CRDT/E2EE）

**批次划分**（RFC-005 target 隔离）:

**批次 1（3 并行，5-7 周）**:
- Agent-1: `sparkfox-memory` crate（6 层记忆图谱 L0-L5 + sqlite-vec 向量索引）
- Agent-2: `sparkfox-sponge` crate（海绵吸收引擎）
- Agent-3: `sparkfox-blackhole` crate（黑洞压缩引擎，压缩可追溯）

**批次 2（3 并行，5-7 周）**:
- Agent-1: `sparkfox-crdt` crate（automerge-rs CRDT 同步）
- Agent-2: `sparkfox-crypto` crate（E2EE: X25519+HKDF+AES-256-GCM+Double Ratchet）
- Agent-3: `sparkfox-budget` crate（三维预算控制 Token/时间/费用）

**批次 3（4 并行，5-7 周）**:
- Agent-1: `sparkfox-security` crate 集合（11 项安全栈）
- Agent-2: `sparkfox-cu-safety` crate（CU 安全四件套）
- Agent-3: `sparkfox-loop` crate（4 阶段反思闭环）
- Agent-4: `sparkfox-evolution` crate（4 阶段进化引擎）

**批次 4（3 并行，5-9 周）**:
- Agent-1: `sparkfox-swarm` crate（蜂群编排: Advisor+Orchestrator+Worker+Verifier）
- Agent-2: `sparkfox-distiller` crate（Skill 蒸馏器）+ `sparkfox-dag` crate（DAG 任务编排）
- Agent-3: `sparkfox-mcp` crate（MCP Client/Server，参考 rmcp 1.5）

**v1.0 新增遗忘机制**（合入 sparkfox-memory）:
- TTL 遗忘: L0 工作记忆 24h 后自动归档
- 容量遗忘: L3 语义记忆超 10 万条时低重要性压缩
- 用户主动: 手动删除任意记忆
- 合规擦除: GDPR 被遗忘权
- 黑洞压缩: 30 天未访问压缩为高维摘要

**预计难度**: 困难
**时间估算**: 20-30 周（4 批次 × 5-7 周 + 集成测试 2-4 周）
**验收标准**:
- [ ] 14 个 crate 编译通过 + 单元测试 80% 覆盖
- [ ] 6 层记忆 CRUD + 向量检索 + 双向链接
- [ ] 海绵/黑洞双引擎端到端测试
- [ ] CRDT（automerge-rs）跨设备同步测试
- [ ] E2EE（含 Double Ratchet）加密解密测试
- [ ] 11 项安全栈集成测试
- [ ] 蜂群共识多数投票测试
- [ ] DAG 任务编排与 NomiFun react-flow 前端对接
- [ ] 遗忘机制 5 种策略测试
- [ ] L5 元认知价值验证（若 PoC-1 通过）

### 阶段 1.5：v0.1 验收 + PMF 验证（必须，串行）

**目标**: v0.1 内部发布 + 10 用户访谈 + PMF 评分决定是否进入 Phase 2

**内容**（2-3 周，串行）:
1. v0.1 内部发布（含 Phase 0 + Phase 1 全部功能）
2. 10 用户访谈（5 个开发者 + 3 个知识工作者 + 2 个普通用户）
3. PMF 评分（5 分制）:
   - 4-5 分: 强烈推荐 → 进入 Phase 2
   - 3-3.5 分: 条件性推荐 → 优化后进入 Phase 2
   - <3 分: 不推荐 → 回到 Phase 1 优化

**预计难度**: 中等
**时间估算**: 2-3 周
**验收标准**:
- [ ] v0.1 三平台构建通过
- [ ] 10 用户访谈完成
- [ ] PMF 评分 ≥3.5/5

**Kill Switch**: PMF <3 → 回到 Phase 1 优化，不进 Phase 2

### 阶段 2：BaiLongma 交互层融合（3-4 并行，3 批次）

**目标**: 将 BaiLongma 的 7 项核心功能融入，提升交互体验

**批次划分**:
- 批次 1（3 并行，3-4 周）: scene-protocol + aci-engine（异步）+ tick-engine（默认关闭）
- 批次 2（3 并行，3-4 周）: threads + find-tool + dynamic-api
- 批次 3（3 并行，2-3 周）: 前端 React 重写（Brain UI / 思考流 / 记忆图 / 人物卡片）

**v1.0 修正**（性能专家要求）:
- ACI 预判: 异步非阻塞，1.5s 超时则放弃（原方案同步阻塞 = UX 灾难）
- Tick 心跳: 默认关闭，使用低成本模型（GPT-4o-mini），月成本 <$3
- 信息热点追踪: 推迟到 v0.4+

**预计难度**: 困难
**时间估算**: 8-10 周
**验收标准**:
- [ ] Scene Protocol core 实现握手/快照/补丁/意图
- [ ] ACI 三类预判场景测试（异步，置信度 > 0.85 直接注入）
- [ ] Tick 心跳 LLM 主导 + 看门狗保护 + 默认关闭
- [ ] 线索模型解决话题漂移测试
- [ ] find_tool 自发现测试
- [ ] Brain UI 多面板 React 重写完成

### 阶段 3：OpenAkita 执行层融合（3-4 并行，4 批次，AGPL 清洁室）

**目标**: 借鉴 OpenAkita 7 项核心功能，补强执行引擎（必须走清洁室流程）

**批次划分**:
- 批次 1（3 并行，3-4 周）: ralph-engine + os-sandbox（Win/Mac 先）+ persona-8（4 人格先）
- 批次 2（3 并行，3-4 周）: daily-pipeline + prompt-compiler + mdrm
- 批次 3（3 并行，3-4 周）: 前端 React 重写（Agent Dashboard / 11 面板 / MDRM 3D）
- 批次 4（2 并行，1-2 周）: persona-8 补全到 8 人格 + os-sandbox Linux

**v1.0 修正**:
- AGPL 清洁室流程必须执行（Phase 0 已建立）
- 6 层 OS 沙箱: Win/Mac 先，Linux 推迟
- 8 人格: 4 人格先，8 人格后
- 插件系统: 推迟到 v0.5+

**预计难度**: 困难
**时间估算**: 10-14 周
**验收标准**:
- [ ] Ralph 永不放弃引擎测试（失败 5 次仍重试，重试上限可配）
- [ ] 6 层 OS 沙箱在 Win/Mac 测试通过（Linux 后补）
- [ ] MDRM 5 维度多跳遍历测试
- [ ] 4 人格切换 + 主动交互测试（8 人格后补）
- [ ] 03:00 记忆整合 + 04:00 自检修复定时任务
- [ ] AGPL 清洁室审计通过

### 阶段 4：NomiFun 能力总线深度整合（3-4 并行，2 批次）

**目标**: 深度整合 NomiFun 6 项独有功能，作为开放能力总线基座

**批次划分**:
- 批次 1（4 并行，3-4 周）: ACP 19 个外部 agent + IDMM 三层保活 + WebUI 扫码配对 + 150+ 工具注册表
- 批次 2（3 并行，3-4 周）: IM 渠道（先 Telegram/飞书/钉钉 3 个）+ 多伙伴家庭 + 桌面伙伴窗口 + skill 赠予

**v1.0 修正**:
- IM 渠道: 先 3 个（Telegram/飞书/钉钉），其他推迟
- 11 IM 渠道完整可用推迟到 v0.5+

**预计难度**: 中等
**时间估算**: 6-8 周
**验收标准**:
- [ ] ACP 19 个外部 agent 全部接入测试
- [ ] IDMM 三层保活无人值守 8 小时测试
- [ ] WebUI 扫码配对 + 局域网远程操控
- [ ] 3 个 IM 渠道完整可用（Telegram/飞书/钉钉）
- [ ] 多伙伴家庭 + 桌面伙伴窗口 + skill 赠予
- [ ] 150+ 工具 + 32 平台 Gateway 全部注册

### 阶段 5：多端访问与打包分发（2-3 并行）

**目标**: 实现 Web 端 + Docker 部署（Mobile 推迟到 v1.0+）

**内容**（4-6 周，2-3 并行）:
1. **Web 主机**: NomiFun nomifun-web 改为 sparkfox-web 无头部署（不含 Computer/Browser Use）
2. **Docker 部署**: Dockerfile + docker-compose
3. **Mobile（推迟到 v1.0+）**: OpenAkita Capacitor 验证的 Android APK + iOS TestFlight

**预计难度**: 中等
**时间估算**: 4-6 周
**验收标准**:
- [ ] Web 主机无头部署通过
- [ ] Docker 部署通过
- [ ] 三平台自动更新测试

### 阶段 6：集成验收与压测（必须最后，串行）

**目标**: 端到端集成测试 + 记忆系统压测 + 性能验证 + 安全审计

**内容**（4-6 周，串行）:
1. **端到端测试**: 5 个场景（搜索/命令/代码/RAG/Browser Use）
2. **记忆系统压测**:
   - 100 轮对话后 L3 语义提取准确率
   - L5 元认知反思价值（对比有/无 L5，若 PoC-1 通过）
   - 黑洞体压缩后信息损失率
   - 海绵体吸收的噪声过滤率
   - 遗忘机制有效性测试
   - 产出: `docs/memory-benchmark.md`
3. **安全审计**: 11 项安全栈 + 6 层沙箱 + CU 四件套 + Double Ratchet + AGPL 清洁室合规
4. **性能验证**（v1.0 校准后目标）:
   - 工具调用循环不超 10 轮
   - RAG 检索 10 万向量 < 800ms（v1.0 校准）
   - 100 万向量 < 5s（压力测试）
   - 启动时间 < 3s（v1.0 校准）
   - 安装包体积 < 200MB（v1.0 校准）
   - 内存空闲 < 300MB，满载 < 800MB
   - 50 轮对话内存增长 < 100MB
5. **回归测试 100% 通过**

**预计难度**: 中等
**时间估算**: 4-6 周
**验收标准**:
- [ ] 5 个端到端场景全部通过
- [ ] 记忆系统压测报告产出
- [ ] 安全审计无高危漏洞
- [ ] 性能指标全部达标（v1.0 校准后）
- [ ] 回归测试 100% 通过
- [ ] AGPL 清洁室最终审计通过

### 总体时间线（v1.0 校准）

| 阶段 | 内容 | 难度 | 时间 | 并行度 |
|------|------|------|------|--------|
| Phase -1 | PoC 验证 | 中等 | 4-6 周 | 串行 |
| Phase 0 | 基座+工程化+RFC | 中等 | 3-4 周 | 串行 |
| Phase 1 | SparkFox 核心护城河 | 困难 | 20-30 周 | 3-4 并行 |
| Phase 1.5 | v0.1 验收+PMF | 中等 | 2-3 周 | 串行 |
| Phase 2 | BaiLongma 交互层 | 困难 | 8-10 周 | 3-4 并行 |
| Phase 3 | OpenAkita 执行层 | 困难 | 10-14 周 | 3-4 并行 |
| Phase 4 | NomiFun 深整合 | 中等 | 6-8 周 | 3-4 并行 |
| Phase 5 | 多端访问 | 中等 | 4-6 周 | 2-3 并行 |
| Phase 6 | 集成验收+压测 | 中等 | 4-6 周 | 串行 |
| **总计** | | | **61-87 周（约 14-20 个月，乐观）** | |
| **含缓冲** | | | **75-130 周（约 18-30 个月，含风险缓冲）** | |

**v1.0 校准理由**:
- 原 33-46 周被 7 专家一致认为严重乐观
- 风险评估专家建议 61-107 周
- 3-4 并行（用户决策）拉长 Phase 1 到 20-30 周
- v1.0 含 30% 风险缓冲，总工期 75-130 周

---

## 第五部分：风险与注意事项（v1.0 增强）

### 5.1 Kill Switch 与回滚预案（v1.0 新增）

| Phase | Kill Switch 触发条件 | 动作 |
|-------|---------------------|------|
| Phase -1 | PoC-1 L5 价值验证失败 | 砍 L5，重评定位+MVP |
| Phase -1 | PoC-2 automerge-rs 失败 | CRDT 推迟到 v0.5+，先单机版 |
| Phase -1 | PoC-3 bge-large-zh Rust 失败 | 退回 Python sidecar（仅 embedding） |
| Phase -1 | PoC-4 性能基线不达标 | 重设性能目标 + 砍 30% 功能 |
| Phase 1 | 任一 crate 集成测试失败 | 该 crate 回滚到上一 commit + 单独修复 |
| Phase 1.5 | PMF <3/5 | 回到 Phase 1 优化，不进 Phase 2 |
| Phase 3 | AGPL 清洁室审计失败 | 暂停 OpenAkita 借鉴，重走清洁室流程 |
| 任一 Phase | 超时 1.5 倍 | 暂停 + 7 专家评审 + 范围调整 |
| 任一 Phase | 安全审计发现高危 | 立即暂停 + 修复 + 重新审计 |

**回滚预案**:
- 每个 Phase 提交前创建回滚 tag（如 `rollback-pre-phase1-20260718`）
- Pangu Nebula v1.1.0 冻结为最终回滚点（`backup/before-fusion-v3-20260718` 分支）
- 数据迁移脚本必须可逆

### 5.2 技术栈不兼容的风险点（v1.0 更新）

| 风险点 | 严重度 | v1.0 缓解措施 |
|--------|:------:|--------------|
| **Python → Rust 重写工作量大** | 🔴 高 | 3-4 并行 agent + 增量迁移 + 优先重写核心护城河 |
| **React 19 vs Preact 不兼容** | 🔴 高 | 直接复用 NomiFun ui/ 全套 |
| **Scene Protocol v1 仍为草案** | 🟡 中 | 先实现核心握手/快照/补丁/意图 |
| **OpenAkita AGPL-3.0 传染性** | 🔴 高 | **v1.0: AGPL 清洁室流程（隔离阅读→规范→第三方实现→审计）** |
| **sqlite-vec Tauri 兼容性** | 🟡 中 | PoC-4 提前测试 |
| **bge-large-zh 模型体积** | 🟡 中 | 模型按需下载 + **v1.0: PoC-3 Rust 推理性验证** |
| **automerge-rs 性能** | 🟡 中 | **v1.0: PoC-2 验证 10 万条同步** |
| **Double Ratchet 实现复杂度** | 🟡 中 | **v1.0: 可降级为无前向保密，v0.3+ 补** |
| **Tick 心跳成本** | 🟡 中 | **v1.0: 默认关闭 + 低成本模型（<$3/月）** |
| **ACI 1.5s 同步阻塞** | 🔴 高 | **v1.0: 异步非阻塞，超时放弃** |
| **14→3-4 并行度降级** | 🟡 中 | **v1.0: RFC-005 target 隔离 + 工期校准到 75-130 周** |

### 5.3 融合过程中最容易踩坑的地方

与 v0.0 一致（保留 Pangu Nebula 项目踩坑历史 16 项 + 融合过程新风险 10 项），详见 v0.0 第五部分 5.2。

**v1.0 新增踩坑风险**:

| 踩坑风险 | 影响 | v1.0 预防措施 |
|---------|------|--------------|
| **automerge-rs 与 6 层记忆映射复杂** | 同步失败 | RFC-004 提前定义映射规则 |
| **Double Ratchet 实现错误** | 前向保密失效 | 参考 libsignal，安全专家审计 |
| **遗忘机制误删用户记忆** | 数据丢失 | 软删除 + 30 天撤销 + 审计日志 |
| **AGPL 清洁室流程执行不严** | 法务风险 | 隔离阅读区物理隔离 + 第三方实现审计 |
| **3-4 并行导致批次等待** | 工期延长 | RFC-005 target 隔离 + 批次优化 |
| **L5 PoC 失败后定位动摇** | 品牌危机 | Kill Switch 明确：砍 L5 + 重评定位 |

### 5.4 Pangu Nebula 项目需要提前准备什么

与 v0.0 一致（保留代码资产准备 + 文档资产准备 + 环境与工具准备），详见 v0.0 第五部分 5.3。

**v1.0 新增准备项**:

| 准备项 | 优先级 | v1.0 说明 |
|--------|:------:|----------|
| **5 个 RFC 模板** | 🔴 高 | 已创建于 `docs/rfc/` |
| **PoC 报告模板** | 🔴 高 | 已创建于 `docs/poc-report.md` |
| **automerge-rs 依赖** | 🔴 高 | Phase -1 PoC-2 验证 |
| **candle/ort Rust ML** | 🔴 高 | Phase -1 PoC-3 验证 |
| **Double Ratchet 参考实现** | 🔴 高 | libsignal / Signal Protocol |
| **AGPL 清洁室流程文档** | 🔴 高 | Phase 0 建立 |
| **sentry / feature flag / SBOM** | 🔴 高 | Phase 0 工程化补全 |
| **PMF 验证脚本** | 🟡 中 | Phase 1.5 执行 |

### 5.5 关键决策回顾（v1.0 更新）

> 用户已在 2026-07-18 通过 12 步决策-making 确认 v0.0 方案
> v1.0 基于七专家评审 + 用户 8 项决策确认优化

| # | 决策项 | v0.0 选择 | v1.0 优化 | 用户确认 |
|---|--------|----------|----------|:--------:|
| 1 | 基座项目 | NomiFun | 不变 | ✅ |
| 2 | Pangu Python 重写 | Rust 重写 | 不变 | ✅ |
| 3 | 借鉴方式 | 只借鉴架构 | 不变 | ✅ |
| 4 | 许可协议 | Apache-2.0 | 不变 | ✅ |
| 5 | 品牌策略 | Pangu Nebula 主品牌 | **SparkFox 全新品牌** | ✅ |
| 6 | NomiFun 6 项独有 | 全部纳入 | 不变 | ✅ |
| 7 | Pangu 13 项独有 | 全部保留 Rust 重写 | 不变（含 L5/CRDT/E2EE） | ✅ |
| 8 | BaiLongma 7 项核心 | 纳入 | 不变 | ✅ |
| 9 | OpenAkita 7 项核心 | 借鉴 | 不变 + **AGPL 清洁室** | ✅ |
| 10 | 重叠功能 | Pangu 优先 | 不变 | ✅ |
| 11 | 迁移策略 | 增量迁移 | 不变 | ✅ |
| 12 | 整体架构 | Rust+React 重写 | 不变 | ✅ |
| **v1.0-13** | **PoC 阶段** | 无 | **Phase -1 PoC 验证 4 项高风险** | ✅ |
| **v1.0-14** | **CRDT 选型** | 自研 | **automerge-rs** | ✅ |
| **v1.0-15** | **MVP 范围** | 14 crate | **不削减（14 crate 全做）** | ✅ |
| **v1.0-16** | **并行度** | 14 并行 | **3-4 并行** | ✅ |
| **v1.0-17** | **总工期** | 33-46 周 | **75-130 周（18-30 个月）** | ✅ |
| **v1.0-18** | **产品定位** | 保留原定位 | 不变 | ✅ |
| **v1.0-19** | **AGPL 合规** | 仅声明借鉴 | **清洁室流程** | ✅ |
| **v1.0-20** | **E2EE** | X25519+HKDF+AES-GCM | **+ Double Ratchet** | 自动（安全专家要求） |
| **v1.0-21** | **遗忘机制** | 缺失 | **5 种策略** | 自动（记忆专家要求） |

### 5.6 宣传语建议（v1.0 调整）

基于用户 memory 偏好"强调数据主权"+"声明式优势描述"+"避免直接竞品对比"，v1.0 修正"业界唯一"过强声明：

| 宣传语 | 适用场景 |
|--------|---------|
| **"别把第二大脑租给别人——你的思考，不该成为别人的养料"** | 数据主权宣传 |
| **"会记住、会反思、会进化的本地 AI 工作站"** | 产品定位 |
| **"6 层记忆 + 元认知 + 蜂群共识 + 双引擎压缩——业界少数独立元认知层设计之一"** | 技术优势（v1.0 修正） |
| **"4 项目融合，1 套 Rust 重写，0 数据外泄"** | 融合卖点 |
| **"你的 AI 不只是回答问题，而是记住、反思、进化、永不放弃"** | 能力描述 |
| **"SparkFox——斯帕克狐，你的本地记忆管家"** | 品牌口号 |

---

## 第六部分：v1.0 新增工程化基础设施（工程化专家要求）

### 6.1 测试金字塔

```
        ┌─────────┐
        │ E2E 10% │  ← 5 个端到端场景（搜索/命令/代码/RAG/Browser Use）
        ├─────────┤
        │集成 20% │  ← crate 间接口契约测试 + 跨 crate 集成
        ├─────────┤
        │单元 70% │  ← 每 crate 80% 覆盖率门槛
        └─────────┘
```

### 6.2 供应链安全

- **cargo-audit**: 每日扫描 Rust 依赖漏洞
- **cargo-deny**: 禁用 GPL/AGPL 依赖（避开传染性）
- **SBOM 生成**: syft 自动生成 Software Bill of Materials
- **dependabot/renovate**: 依赖更新自动 PR

### 6.3 错误监控

- **sentry-rust**: 后端错误上报
- **sentry-react**: 前端错误上报
- **本地优先**: 错误上报可关闭，保护用户隐私
- **feature flag**: L5/CRDT/Double Ratchet 等高风险功能灰度发布

### 6.4 代码签名自动化

- **GitHub Actions**: 三平台自动构建 + 签名
- **Apple notarytool**: macOS 公证自动化
- **Windows signtool**: Windows EV 证书签名
- **关键**: 签名证书密钥安全存储（HSM 或 GitHub Encrypted Secrets）

### 6.5 AGPL 清洁室流程

```
Step 1: 隔离阅读区
  └─ 阅读者 A: 阅读 OpenAkita 源码，写设计规范文档（不含代码）

Step 2: 规范评审
  └─ 法务 + 架构师: 评审规范文档，确认无 OpenAkita 专有代码

Step 3: 第三方实现
  └─ 实现者 B: 只看规范文档，不看 OpenAkita 源码，Rust 实现

Step 4: 法务审计
  └─ 法务: 对比实现代码与 OpenAkita 源码，确认无相似性

Step 5: 文档归档
  └─ 保留全过程文档，作为合规证据
```

---

## 附录：v1.0 新增文档

| 文档 | 路径 | 说明 |
|------|------|------|
| **PoC 验收报告模板** | `docs/poc-report.md` | Phase -1 PoC 验收归档 |
| **RFC-001 crate 边界** | `docs/rfc/RFC-001-crate-boundaries.md` | memory 独占数据层方案 |
| **RFC-002 编排协调** | `docs/rfc/RFC-002-orchestration-coordination.md` | DAG 为主，其他为策略插件 |
| **RFC-003 记忆 SoT** | `docs/rfc/RFC-003-memory-source-of-truth.md` | 6 层为 SoT，身份/线索为投影 |
| **RFC-004 CRDT 选型** | `docs/rfc/RFC-004-crdt-selection.md` | automerge-rs + Double Ratchet |
| **RFC-005 并行度** | `docs/rfc/RFC-005-parallelism-and-target-isolation.md` | 3-4 并行 + target 隔离 |

**v0.0 既有文档保留**（详见 v0.0 附录）:
- Pangu Nebula README / fusion-report.md / comparative-analysis.md / nomifun-对比报告.md
- v2.2.0-architecture-plan.md / design.md / spec.md / tasks.md
- BUGFIX.md / BUG与调整需求记录.md / CHANGELOG.md / SECURITY.md

---

**报告完成。**

> **v1.0 核心结论**: 本方案基于 v0.0 七专家评审意见 + 用户 8 项决策确认优化迭代。融合后项目以 **SparkFox** 为全新品牌（致谢 NomiFun/BaiLongma/Pangu Nebula），NomiFun 50+ crate workspace 为骨架，SparkFox 6 层记忆 + 双引擎 + automerge-rs CRDT + Double Ratchet E2EE + 11 项安全栈为核心护城河（Rust 重写），BaiLongma Scene Protocol + ACI（异步）+ Tick（默认关闭）+ 线索/焦点补强交互层，OpenAkita Ralph + 6 层沙箱 + MDRM + 8 人格 + 日更流水线补强执行层（AGPL 清洁室流程合规）。预计 **75-130 周（约 18-30 个月）** 完成，分 9 个阶段（Phase -1 ~ 6），3-4 并行 agent，含 Kill Switch + PMF 验证 + 5 个 RFC 前置决策。融合后项目定位为"具备元认知的本地优先超级 AI 工作站"，Apache-2.0 协议，支持桌面/Web/Mobile 三端，是 4 个项目各自优势的最大化融合。
>
> **下一步**: Phase -1 PoC 验证（4-6 周），4 项 PoC 全部 GO 后进入 Phase 0。
