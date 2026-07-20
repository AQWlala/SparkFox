<a name="top"></a>

<div align="center">

<h1>SparkFox</h1>

<h3>本地优先的 AI Agent 桌面工作站 · 数据主权至上</h3>

<p>
  <b>别把第二大脑租给别人——你的思考，不该成为别人的养料。</b>
</p>

<p>
  <img alt="License: AGPL-3.0-only" src="https://img.shields.io/badge/License-AGPL_3.0_only-FF6F91?style=for-the-badge">
  <img alt="Platform" src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-7583B2?style=for-the-badge">
  <img alt="Status" src="https://img.shields.io/badge/Status-pre--1.0-FBBF24?style=for-the-badge">
  <img alt="Version" src="https://img.shields.io/badge/Version-v0.2.28%20(v1.1.0%20进行中)-24C8DB?style=for-the-badge">
</p>

<p>
  <img alt="Built with Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white">
  <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-edition_2024-CE412B?style=flat-square&logo=rust&logoColor=white">
  <img alt="Preact" src="https://img.shields.io/badge/Preact-10-673AB8?style=flat-square&logo=preact&logoColor=white">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white">
  <img alt="Arco Design" src="https://img.shields.io/badge/Arco_Design-2-0FC6C2?style=flat-square">
</p>

<p>
  <b>简体中文</b>&nbsp;·&nbsp;<a href="README.zh-CN.md">English</a>
</p>

<p>
  <a href="#-核心特性">🎯 核心特性</a>&nbsp;·&nbsp;
  <a href="#-架构">🏗️ 架构</a>&nbsp;·&nbsp;
  <a href="#-快速开始">🚀 快速开始</a>&nbsp;·&nbsp;
  <a href="#-开发">🛠️ 开发</a>&nbsp;·&nbsp;
  <a href="#-致谢">💛 致谢</a>&nbsp;·&nbsp;
  <a href="#-许可证">⚖️ 许可证</a>
</p>

</div>

---

## 🎯 项目定位

**SparkFox** 是一款本地优先（local-first）的 AI Agent 桌面工作站，融合多源开源项目精华，专注于**记忆系统优化**与**Agent 编排创新**。

- **数据主权至上**：所有数据驻留本机，无云账号、无遥测、无订阅。唯一的外发流量是你显式配置的 LLM 调用。
- **Apple system style 桌面设计**：遵循 macOS 设计语言，原生 Tauri 2 壳，无 Electron、无 Node 宿主。
- **AGPL-3.0-only**：强 copyleft 许可证，确保衍生作品同样开源，守护数据主权承诺。

---

## ✨ 核心特性

### 🧠 6 层记忆架构（Pangu Nebula L0-L5）

基于 Pangu Nebula 的 6 层架构作为记忆系统基石：

- **L0 Raw**：原始数据层（events / chunks）
- **L1 Indexed**：索引层（HnswIndex + sqlite-vec 双引擎）
- **L2 Associative**：关联层（event_entity_relation 图）
- **L3 Episodic/Semantic**：情景记忆 + 语义记忆（GraphNode）
- **L4 Metacognitive**：元认知层（思考过程可视化）
- **L5 Procedural**：程序性记忆（技能沉淀）

### 🔍 SAG（Semantic Agentic Graph）多跳检索

v1.1.0 核心能力，三策略并行检索：

- **multi**：BFS 多跳扩展（max_hop=3）
- **multi1**：单跳剪枝（性能优先）
- **hopllm**：LLM 引导多跳扩展（语义优先，失败降级到 multi1）
- **R-07 三道 LIMIT 阀门**：MAX_HOP=3 / MAX_INTERMEDIATE_ENTITIES=100 / MAX_JOIN_ROWS=10000，防止 graph explosion
- **MULTI 8 步流程**：query 向量化 → 实体抽取 → 实体检索 → 事件检索 → 三策略合并 → chunk 关联 → Rerank → 返回 SearchResult

### 🐝 蜂群编排（OpenAkita + Pangu Nebula）

双主控 + 蜂群 worker + persona 自进化设计：

- **主星·编排者**：任务分解 + DAG 调度
- **化身·灵魂分身**：persona 自进化
- **星尘群**：蜂群 worker 并行执行
- **星魂**：长期记忆沉淀

### 🎨 Scene Protocol + 思考过程可视化（BaiLongma）

- **Scene Protocol**：场景化对话展示
- **ReasoningChainPanel**：7 步推理链可视化（hop 颜色映射 + via_entities 高亮）
- **CitationDetailDrawer**：三级溯源（Entity → Event → Chunk）
- **KnowledgeGraphView**：知识图谱可视化（@xyflow/react v12 + 11 类着色 + EntityEditDrawer 编辑）

### 🔒 端到端加密（E2EE）

- **X25519** 密钥协商 + **HKDF** 密钥派生 + **AES-256-GCM** 对称加密
- **Double Ratchet** 算法前向保密（直接实现，非 ratchetx2）

### 🔄 CRDT 多设备同步

- **automerge-rs** 实现（非自研方案）
- `AutoCommit::save()` + `load()` + `merge()` 全快照 CRDT 合并

---

## 🏗️ 架构

一个 Preact 前端 + 一个 Rust 后端，**两种宿主模式**，同一后端进程内运行。

| | `sparkfox-desktop` | `sparkfox-web` |
|---|---|---|
| **壳** | Tauri 2 桌面应用 | 独立 axum 服务器 |
| **后端** | 进程内嵌入，私有环回端口 | 同一后端，进程内 |
| **认证** | 本地信任令牌注入 webview | 默认要求登录 |
| **服务** | 原生桌面 UI + 托盘 + 伴侣窗口 | API + `/ws` + 内嵌 SPA |

<details>
<summary><b>仓库结构</b></summary>

```text
apps/
  desktop/      Tauri 2 壳 + 桌面独有命令
  web/          独立 Web 宿主（API + SPA）
crates/
  sparkfox/     14 个核心 crate：
                sparkfox-knowledge（SAG 检索）
                sparkfox-memory（6 层记忆）
                sparkfox-orchestrator（蜂群编排）
                sparkfox-graph（图遍历）
                sparkfox-e2ee（端到端加密）
                sparkfox-crdt（多设备同步）
                sparkfox-embedding（向量化）
                sparkfox-llm（LLM 抽象）
                sparkfox-chat / sparkfox-agent / sparkfox-hotspot
                sparkfox-monitor / sparkfox-parser / sparkfox-core
  backend/      29 个 nomifun-* 后端 crate（保留命名兼容）
  agent/        15 个 nomi-* / sparkfox-ag-* Agent crate
ui/             Preact + Vite SPA（桌面与 Web 共享）
docs/           技术文档、用户指南、架构说明
```

</details>

---

## 🚀 快速开始

**前置要求**

- [Rust](https://rustup.rs) — stable 工具链，edition 2024
- [Bun](https://bun.sh) ≥ 1.3.13
- 推荐 PATH 包含：`node` / `npm` / `npx`、`git`、`ripgrep`

**桌面应用（从源码构建）**

```bash
git clone git@github.com:AQWlala/SparkFox.git
cd SparkFox
bun install

bun run dev      # 热重载开发
bun run build    # 构建当前 OS 的桌面安装包
```

**Web 服务器（自托管）**

```bash
bun run build:ui && bun run serve:web
# 在 http://127.0.0.1:8787 提供 API + SPA（默认要求登录）
```

详见 [`docs/getting-started/installation.md`](docs/getting-started/installation.md)。

---

## 🛠️ 开发

```bash
bun install        # 安装依赖（一次性）
bun run dev        # 桌面应用开发（热重载）
bun run dev:web    # Web 宿主 + Vite 开发
bun run build:ui   # 构建 SPA
bun run check      # 前端 typecheck + i18n + 主题 + 脚本登记门禁
bun run test       # Rust 测试（含 doctest）
bun run test:fast  # nextest 快速跑 Rust 测试（日常）
```

### 📦 桌面打包

每个 OS 有独立命令，**包只能在匹配的 OS 上构建**：

| OS | 命令 | 产物 |
|---|---|---|
| macOS | `bun run build:mac` | `.dmg`（universal / arm / intel） |
| Windows | `bun run build:win` | `.exe`（NSIS，x64 / arm64） |
| Linux | `bun run build:linux` | `.deb` / `.AppImage` / `.rpm` |

签名与公证：`--signed` 标志，详见 [`apps/desktop/signing/README.md`](apps/desktop/signing/README.md)。

---

## 📖 文档

- [`docs/SparkFox-v1.1.0-规划.md`](docs/SparkFox-v1.1.0-规划.md) — v1.1.0 实施规划与进度矩阵
- [`docs/SparkFox-最终融合蓝图-1.0.md`](docs/SparkFox-最终融合蓝图-1.0.md) — 四项目融合蓝图
- [`docs/SparkFox-重组优化方案-1.0.md`](docs/SparkFox-重组优化方案-1.0.md) — 重组优化方案
- [`docs/SAG-深度评估与重构方案-1.0.md`](docs/SAG-深度评估与重构方案-1.0.md) — SAG 重构方案
- [`docs/architecture/`](docs/architecture/) — 技术架构
- [`docs/getting-started/`](docs/getting-started/) — 安装与首次运行
- [`docs/guides/`](docs/guides/) — 用户与运维指南
- [`docs/rfc/`](docs/rfc/) — RFC 设计文档

---

## 🗺️ 当前版本与路线图

**当前版本**：v0.2.28（v1.1.0 进行中）

**v1.1.0 进度**（Task 11.x SAG 多跳检索）：

- ✅ W4 里程碑：32/32 sub-step 完成（5 前端组件集成到现有页面）
- ✅ Task 11.x：12/18 sub-step 完成（2/3 进度）
  - 11.1.x MULTI 8 步流程骨架 + Step1-Step8 真实实现 + E2E 集成（Recall@5=0.80）
  - 11.2.x multi / multi1 / hopllm 三策略 + R-07 三道 LIMIT 阀门
  - 11.3.x KnowledgeGraphView 入口 + 11 类着色 + EntityEditDrawer
  - 11.4.x 数据契约 + @xyflow/react v12 渲染

**后续方向**：

- v1.1.0 收尾：11.4.2 EntityEditDrawer IPC / 11.5.x 多跳路径渲染 / 11.6.x hnswlib-rs 集成
- v1.2.0+：完整 MULTI 策略 + 动态超图
- v2.0.0：降级为维护版

---

## 💛 致谢

SparkFox 站在巨人的肩膀上，深度借鉴以下开源项目（按字母序）：

| 项目 | 许可证 | 借鉴内容 |
|---|---|---|
| **BaiLongma** | MIT | 对话展示方法、思考过程可视化、信息热点跟踪；Scene Protocol 经清洁室重写 |
| **NomiFun** | Apache-2.0 | Arco Design 界面基础、功能模块设计；crate 命名经 `nomi-*` / `nomifun-*` → `sparkfox-*` 重命名 |
| **OpenAkita** | MIT | Agent 菜单设计、监控面板设计、组织编排模型 |

**合规说明**：

- AGPL-3.0-only 与 MIT / Apache-2.0 兼容，衍生作品同样开源
- BaiLongma MIT 组件经清洁室重写（schema 借鉴与字段重命名）以维持 AGPL 合规
- API 契约字段（`agent_type === 'nomi'`、`nomi_delegate`、`NOMI_SKILL_DIR` 等）保留以避免功能中断
- 详见 [`NOTICE`](NOTICE) 与 [`docs/SparkFox-重组优化方案-1.0.md`](docs/SparkFox-重组优化方案-1.0.md)

---

## 🤝 贡献

- 阅读 [`CONTRIBUTING.md`](CONTRIBUTING.md) 了解开发环境与检查阶梯
- 遵守 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- 发现漏洞请按 [`SECURITY.md`](SECURITY.md) 报告
- 浏览 [open issues](https://github.com/AQWlala/SparkFox/issues) 寻找切入点

---

## ⚖️ 许可证

[AGPL-3.0-only](LICENSE) © 2025–2026 SparkFox Contributors.

本仓库未附带 LICENSE 文件时，以 AGPL-3.0-only 为准（见 [`package.json`](package.json) 声明）。

第三方归属详见 [`NOTICE`](NOTICE)。

<div align="center">
<br/>
<sub>本地优先 · 数据主权 · AGPL 守护</sub>
<br/><br/>
<a href="#top">⬆ 返回顶部</a>
</div>
