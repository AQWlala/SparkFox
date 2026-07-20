# SparkFox 四项目深度分析与融合决策 v1.0

> **文档版本**：v1.0
> **创建日期**：2026-07-18
> **文档性质**：四项目（Pangu Nebula + NomiFun + OpenAkita + BaiLongma）深度分析 + 融合决策依据
> **上游文档**：[SparkFox-重组优化方案-1.0.md](./SparkFox-重组优化方案-1.0.md)
> **下游 RFC**：RFC-001 crate 边界 / RFC-002 编排协调 / RFC-003 记忆 SoT / RFC-004 CRDT / RFC-005 并行度

---

## 目录

- [第一部分：四项目技术栈汇总对比](#第一部分四项目技术栈汇总对比)
- [第二部分：关键差异深度分析](#第二部分关键差异深度分析)
- [第三部分：Pangu Nebula 深度功能拆解](#第三部分pangu-nebula-深度功能拆解)
- [第四部分：NomiFun 深度功能拆解](#第四部分nomifun-深度功能拆解)
- [第五部分：OpenAkita 深度功能拆解](#第五部分openakita-深度功能拆解)
- [第六部分：BaiLongma 深度功能拆解](#第六部分bailongma-深度功能拆解)
- [第七部分：功能重叠对比矩阵（72 功能点）](#第七部分功能重叠对比矩阵72-功能点)
- [第八部分：融合决策汇总](#第八部分融合决策汇总)
- [第九部分：风险评估与下一步](#第九部分风险评估与下一步)

---

## 第一部分：四项目技术栈汇总对比

### 1.1 四项目一句话定位

| 项目 | 一句话定位 |
|------|-----------|
| **Pangu Nebula** | 元认知多 Agent Runtime（6 层记忆 L0-L5 + 蜂群 + 双引擎 + 11 安全栈 + CRDT） |
| **NomiFun** | 无限制全开源本地优先超级 AI 工作站（50 crate workspace 全栈 Rust） |
| **OpenAkita** | 开源全能自进化多 Agent AI 助手（Ralph 永不放弃 + 6 层沙箱 + 组织编排） |
| **BaiLongma** | 持续运行的桌面 AI Agent 数字意识框架（Tick 心跳 + ACI 预判注入 + Scene Protocol + Thread 线索模型） |

### 1.2 技术栈对比表格

| 维度 | Pangu Nebula | NomiFun | OpenAkita | BaiLongma |
|------|----------|---------|-----------|-----------|
| **桌面框架** | Tauri 2（薄壳 + Python sidecar） | Tauri 2（进程内后端，axum on 127.0.0.1） | Tauri 2.x Setup Center + Capacitor Mobile + Web | Electron 33（electron-builder 25 + NSIS/dmg + 单实例 + 焦点横幅） |
| **前端框架** | Preact 10 | React 19.1 | React 19 + TypeScript | 原生 HTML/CSS/JS（无框架）+ D3 7.9 + Three.js |
| **前端 UI 库** | Tailwind 3 + ReactFlow 11 + @antv/g6 5 | Arco Design + UnoCSS 66 + @xyflow/react + xterm + Monaco + CodeMirror + mermaid + KaTeX | shadcn/ui + Tailwind + lucide-react | 自研 ACUI 卡片 + Scene Protocol 驱动 + 3 主题 + 记忆图物理控制 |
| **后端/逻辑层** | Python 3.11 + FastAPI + PyWebView sidecar | Rust 2024（edition）+ axum + tokio + 50 crate workspace | Python 3.11+ + FastAPI + Typer + asyncio + Pydantic v2 | Node.js（ESM）+ 本地 HTTP 服务（端口 3721）+ SSE + WebSocket + better-sqlite3 同步