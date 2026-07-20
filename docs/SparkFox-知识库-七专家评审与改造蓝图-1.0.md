# SparkFox 知识库功能 — 七专家评审与改造蓝图 1.0

**评审日期**: 2026-07-19
**评审对象**: SparkFox 知识库（NomiFun Knowledge 已融合基线 + RAG 引擎补全方案）
**评审方法**: 7 专家独立评审 → 共识收敛 → 融合建议 → 改造蓝图
**评审依据**:
- [SparkFox-四项目深度分析与融合拆解报告.md](file:///D:/xin%20kaifa/SparkFox/docs/SparkFox-四项目深度分析与融合拆解报告.md) 第 982、1003 行（已决策"选取 NomiFun Knowledge"）
- [SparkFox-最终融合蓝图-1.0.md](file:///D:/xin%20kaifa/SparkFox/docs/SparkFox-最终融合蓝图-1.0.md) 第 46 行（弃 Python sidecar，全部 Rust）
- [docs/architecture/external-knowledge-mcp.zh.md](file:///D:/xin%20kaifa/SparkFox/docs/architecture/external-knowledge-mcp.zh.md)（MCP Broker 架构）
- 6 个外部知识库项目（RAGFlow / Dify / FastGPT / AnythingLLM / MaxKB / Yuxi）功能对比

---

## 一、评审背景与基线

### 1.1 现状

SparkFox 已完整融合 NomiFun Knowledge（Apache-2.0）：
- 后端 18 个 .rs 文件位于 [crates/backend/nomifun-knowledge/](file:///D:/xin%20kaifa/SparkFox/crates/backend/nomifun-knowledge/)
- 前端 27 个 .tsx/.ts 文件位于 [ui/src/renderer/pages/knowledge/](file:///D:/xin%20kaifa/SparkFox/ui/src/renderer/pages/knowledge/)
- 架构文档 [external-knowledge-mcp.zh.md](file:///D:/xin%20kaifa/SparkFox/docs/architecture/external-knowledge-mcp.zh.md) 已落地

### 1.2 NomiFun Knowledge 已有能力（基线）

| 能力 | 状态 |
|---|---|
| 文件系统真源 + Markdown 挂载 | ✅ NTFS junction / Unix symlink |
| 文档句柄 `kdoc_{base64}` | ✅ |
| URL 抓取（HttpFetcher / PageFetcher） | ✅ |
| 飞书文档连接器 | ✅ connector_feishu.rs + feishu_md.rs |
| zip 导入导出 | ✅ |
| AI 自动补全（autogen.rs） | ✅ |
| 写回机制（staged / direct + conservative / aggressive） | ✅ |
| 4 种 binding target（workpath / conversation / terminal / companion） | ✅ |
| MCP 暴露（KnowledgeMcpServer） | ✅ |
| 外部 broker（OS 身份边界） | ✅ |
| 完整 UI（KnowledgeListPage / DetailPage / CreateStudio / InboxReviewPanel / TagFilterBar / ConnectorDrawer） | ✅ |

### 1.3 NomiFun Knowledge 功能缺口（对比 6 个开源项目）

| 缺口 | 严重度 | 用户感知 |
|---|---|---|
| **向量检索** | 🔴 P0 | >100 文件知识库几乎不可用 |
| **文档分块 chunking** | 🔴 P0 | 长文档无法被有效检索 |
| **多格式解析（PDF/Word/Excel/PPT）** | 🟠 P1 | 仅支持 md |
| **混合检索（向量 + 关键词 RRF）** | 🟠 P1 | 单一检索召回率低 |
| **多模态（图/表/OCR）** | 🟡 P2 | 图片表格无法解析 |
| **知识图谱** | 🟡 P3 | 无实体关系推理 |

### 1.4 6 个外部项目对照

| 项目 | 协议 | 桌面端适配 | 强项 | 弱项 |
|---|---|---|---|---|
| RAGFlow | Apache-2.0 | ❌ | DeepDoc 解析最强 | Docker + ES + Minio + Redis + MySQL + Infinity |
| Dify | Apache-2.0 | ❌ | 工作流编排最完整 | 知识库非一等公民 |
| FastGPT | 商用付费 | ❌ | Node 全栈 + 工作流 | MongoDB 强依赖 |
| AnythingLLM | **MIT** | **✅ Electron 桌面** | 桌面端定位 + 多 VectorDB | RAG 简单 |
| MaxKB | **GPLv3**（不兼容） | ❌ | MCP + 多模态 | 协议不兼容 AGPL |
| Yuxi | MIT | ❌ | **Milvus 知识图谱** | 5 个重服务 |

---

## 二、七专家评审

### 专家 1：架构师视角（系统分层与边界）

**评审意见**：

✅ **认可**：
1. NomiFun Knowledge 已落地，框架设计在桌面端定位下优于 6 个外部项目（文件系统真源契合"数据主权"理念，NTFS junction 无需管理员权限）
2. 知识库应作为独立 crate（`sparkfox-knowledge`），与 6 层记忆 `sparkfox-memory` 解耦，避免职责混淆
3. 知识库是"事实知识"，6 层记忆是"经验/情景/策略"，二者不应混同

⚠️ **风险**：
1. NomiFun Knowledge 当前是"LLM 工具调用式检索"（让 LLM 自己 grep markdown），无向量召回，对大知识库（>100 文件）几乎不可用 — 必须补 RAG 引擎
2. 补 RAG 引擎不应引入外部向量库服务（Milvus/Chroma/Qdrant），应复用 `sparkfox-store` 的 sqlite-vec 扩展加载
3. 文档解析（PDF/Word/Excel）必须用 Rust crate，不得引入 Python sidecar

🔴 **强烈反对**：
1. 整体替换为 RAGFlow/Dify/MaxKB — 违反"全部 Rust 重写"蓝图第 46 行决策
2. 引入 Python sidecar 跑 embedding/解析 — 违反蓝图弃用 Python 决策

**建议架构**：

```
sparkfox-knowledge (新建 crate，封装 NomiFun knowledge 业务)
  ├─ 对外 API：KnowledgeBase / Document / Search / Ingest
  ├─ 内部依赖：
  │   ├─ nomifun-knowledge (复用 18 .rs 文件)
  │   ├─ sparkfox-store (sqlite-vec 向量检索)
  │   ├─ sparkfox-embedding (新建，bge 嵌入)
  │   └─ sparkfox-parser (新建，PDF/Word/Excel 解析)
  └─ 前端：复用 ui/src/renderer/pages/knowledge/ 27 .tsx
```

**评分**：方案总分 9/10（保留 NomiFun + Rust 增量是最佳路径）

---

### 专家 2：Rust 工程师视角（crate 设计与依赖）

**评审意见**：

✅ **认可**：
1. 嵌入式向量库选 `sqlite-vec` 是正确选择（BaiLongma 已验证，sparkfox-store 已设计加载机制）
2. 文档分块用纯 Rust 实现，无外部依赖

⚠️ **技术选型关注**：
1. **嵌入模型**：bge-large-zh-v1.5（768 维）— Rust 推理选 `candle-transformers`（Hugging Face 官方 Rust 栈）或 `ort`（ONNX Runtime）
   - 推荐 `candle-transformers` + `candle-core`：纯 Rust，无外部运行时
   - 模型大小约 1.2GB，需评估桌面端启动延迟
   - PoC-3 已计划验证此路径（v0.2.1）
2. **PDF 解析**：`lopdf` (PDF 文本提取) + `pdf-extract` (备用)
   - 表格识别复杂，桌面端可暂用纯文本提取，表格识别留 v0.5+
3. **Word 解析**：`docx-rs` (Rust 原生 docx 读写)
4. **Excel 解析**：`calamine` (Rust 原生 xlsx/xls 读取，性能极好)
5. **OCR**：`tesseract-rs` (绑定 Tesseract OCR) — 桌面端用户若需 OCR 自行安装 Tesseract

⚠️ **依赖风险**：
1. `candle-transformers` 当前在快速演进，API 可能在 0.10 → 0.11 间破坏性变更 — 需 pin 版本
2. `ratchetx2 0.3`（E2EE）虽与本评审无关，但同属 PoC-2 验证范围，需注意它依赖 `rand_core 0.6`，与 `ring 0.17` 的 rand 版本可能冲突 — 建议在 v0.2 spec 中已明确
3. `automerge 0.10` 的 sync API 在 0.11 已重构 — 建议锁定 0.10 直到 PoC-2 完成

🔴 **反对**：
1. 用 `chromadb` Rust 客户端 — 引入外部服务
2. 用 `qdrant-client` — 同上

**crate 设计建议**：

```toml
# crates/sparkfox/sparkfox-knowledge/Cargo.toml
[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
sparkfox-store = { path = "../sparkfox-store" }
sparkfox-embedding = { path = "../sparkfox-embedding" }  # 新建
sparkfox-parser = { path = "../sparkfox-parser" }        # 新建
nomifun-knowledge = { path = "../../backend/nomifun-knowledge" }
serde = { workspace = true }
tokio = { workspace = true }
```

```toml
# crates/sparkfox/sparkfox-embedding/Cargo.toml
[dependencies]
candle-core = "0.7"
candle-transformers = "0.7"
candle-nn = "0.7"
tokenizers = "0.20"
hf-hub = "0.3"  # 模型下载
sparkfox-core = { path = "../sparkfox-core" }
```

```toml
# crates/sparkfox/sparkfox-parser/Cargo.toml
[dependencies]
lopdf = "0.34"
docx-rs = "0.4"
calamine = "0.26"
quick-xml = "0.36"  # SVG/HTML 兜底
sparkfox-core = { path = "../sparkfox-core" }
```

**评分**：8.5/10（技术选型清晰，但 candle-transformers 与 PoC-3 强耦合需先验证）

---

### 专家 3：前端工程师视角（UI 复用与一致性）

**评审意见**：

✅ **认可**：
1. NomiFun 27 个 .tsx UI 组件已就位，复用率 100%
2. SparkFox 已套上 Apple 主题 CSS 变量令牌（`--sf-radius-*` / `--sf-shadow-*` / `--sf-color-*`），与 SparkFoxSider 视觉统一

⚠️ **UI 缺口**：
1. **SparkFoxSider 未挂载"知识库"入口** — 这是用户报告"知识库功能未实现"的直接原因！修复方案：
   - 在 [ui/src/renderer/components/layout/SparkFoxSider.tsx](file:///D:/xin%20kaifa/SparkFox/ui/src/renderer/components/layout/SparkFoxSider.tsx) 添加 `/knowledge` 路由项
   - 或确认 NomiFun 原 `/knowledge` 路由仍可访问（在 ProtectedLayout 内，需鉴权）
2. **RAG 状态可视化缺失**：
   - 需要"向量化进度"进度条（处理 1000 文件时用户需看到进度）
   - 需要"重建索引"按钮
   - 需要"向量检索结果"展示区（区分"全文匹配"vs"语义匹配"）
3. **多格式上传组件缺失**：
   - 当前 CreateStudio 仅支持 md/zip/URL
   - 需扩展文件拖拽（PDF/Word/Excel/PPT/图片）
4. **检索结果引用溯源可视化**：
   - NomiFun 已有文档句柄 `kdoc_{base64}`
   - 需在 ChatView 中渲染引用 chip（点击跳转到文档位置）

🔴 **强烈反对**：
1. 用 Vue 重写知识库 UI — FastGPT/MaxKB/Yuxi 都是 Vue，但 SparkFox 是 React 19.1 + Zustand
2. 引入 shadcn/ui 替换 NomiFun 自有组件 — NomiFun UI 已成熟，不要替换

**前端任务清单**：

```
Task F1: 在 SparkFoxSider 添加"知识库"菜单（路由 /knowledge）
Task F2: ChatView 添加引用 chip 渲染（CitationChip 组件）
Task F3: CreateStudio 扩展文件拖拽（PDF/Word/Excel/PPT）
Task F4: KnowledgeDetailPage 添加"向量化进度"和"重建索引"按钮
Task F5: KnowledgeListPage 添加"检索模式"切换（全文 / 语义 / 混合）
```

**评分**：9/10（UI 复用率高，缺口清晰可补）

---

### 专家 4：AI 算法工程师视角（RAG 引擎与嵌入）

**评审意见**：

✅ **认可**：
1. bge-large-zh-v1.5 是中文检索 SOTA，1024 tokens 上下文，768 维
2. sqlite-vec 余弦相似度检索在 10 万向量下可达到 < 800ms（PoC-4 验收门槛）

⚠️ **算法选型**：

1. **分块策略**（chunking）：
   - 推荐：固定长度 + 滑动窗口（256 tokens + 50 重叠）
   - 中文场景需用 tokenizer 分词（不能用空格分词）
   - 长文档（>10000 字）需递归分块（章节 → 段落 → 256 token 块）
   - 借鉴 RAGFlow 的 DeepDoc 思路：表格作为独立 chunk 保留结构

2. **混合检索（RRF）**：
   - 公式：`score(d) = Σ 1/(k + rank_i(d))` 其中 k=60（ Cormack 2009）
   - 不用加权融合（0.5*vector + 0.5*keyword），因为向量与关键词分数尺度不一致
   - 借鉴 FastGPT 但不用其代码（FastGPT 商用授权）

3. **重排（Rerank）**：
   - 召回 top-50 → bge-reranker-v2-m3 重排 → 取 top-10
   - bge-reranker 模型约 560MB，桌面端可接受
   - 重排延迟预期 < 200ms（10 万向量场景）

4. **嵌入缓存**：
   - 同一文档同一段落 hash 不变，嵌入结果可缓存
   - 缓存表：`embedding_cache(content_hash, model, embedding BLOB, ts)`

🔴 **反对**：
1. 用 OpenAI text-embedding-3-large — 违反"数据主权"理念，桌面端必须本地嵌入
2. 用 BGE-M3（多语言）— 768 维但中文场景 bge-large-zh 更准；M3 留作 v0.5 多语言选项

**RAG 引擎流程图**：

```
用户提问
  ↓
[Query Embedding] (bge-large-zh, 缓存查询嵌入)
  ↓
[向量召回] sqlite-vec top-50
  ↓
[关键词召回] FTS5 top-50
  ↓
[RRF 融合] 取前 50
  ↓
[Rerank] bge-reranker-v2-m3 → top-10
  ↓
[Context 拼装] 引用 chip + 段落原文
  ↓
[LLM 生成] 复用 sparkfox-llm Provider
```

**评分**：8/10（算法清晰，但 rerank 增加桌面端模型负担，建议 v0.2 不含 rerank，v0.3 加入）

---

### 专家 5：安全工程师视角（数据主权与端到端加密）

**评审意见**：

✅ **认可**：
1. 文件系统真源设计契合"数据主权"理念（你的偏好：别把第二大脑租给别人）
2. NomiFun 的 MCP Broker 已有 OS 身份边界（Unix SO_PEERCRED / Windows named pipe + SID）

⚠️ **安全关注点**：

1. **嵌入模型本地化**：
   - 必须 100% 本地推理，禁止调用云端嵌入 API
   - candle-transformers 是 Hugging Face 官方 Rust 栈，可信度高
   - 模型下载走 hf-hub，需校验 SHA256（防供应链攻击）

2. **PDF/Word 解析安全**：
   - PDF 是攻击面最大的格式（CVE 历史最多）
   - `lopdf` 是纯 Rust 实现，比 Poppler 更安全
   - 但仍需限制解析文件大小（建议 < 100MB）和解析超时（建议 30s）

3. **知识库与 E2EE 集成**：
   - 知识库文档是否需要跨设备同步？
   - 若需要：必须经 sparkfox-e2ee 加密后再传输
   - 若不需要：仅本地存储，明确标注"不同步"

4. **MCP 暴露的访问控制**：
   - 当前 KnowledgeMcpServer 仅暴露 `knowledge_search` 单工具
   - capability 短期 access + renewal proof 安全模型已设计
   - 但需补 audit log（记录谁在何时调用了 search）

🔴 **强烈反对**：
1. 调用云端 OCR API（如阿里云 OCR / Google Vision）— 违反数据主权
2. 知识库元数据存云端（如 Pinecone / Weaviate Cloud）

**安全任务清单**：

```
Task S1: 嵌入模型下载校验 SHA256
Task S2: PDF/Word 解析限制文件大小 + 超时
Task S3: 知识库 E2EE 同步策略决策（同步 vs 不同步）
Task S4: MCP Broker audit log
```

**评分**：9/10（数据主权理念贯彻，安全任务清晰）

---

### 专家 6：AGPL 合规专家视角（协议与清洁室）

**评审意见**：

✅ **协议兼容性矩阵**：

| 借鉴源 | 协议 | 借鉴方式 | 合规性 |
|---|---|---|---|
| NomiFun Knowledge | Apache-2.0 | 已落地（保留版权声明） | ✅ |
| AnythingLLM RAG 设计 | MIT | 思路借鉴 + Rust 重写 | ✅ |
| RAGFlow DeepDoc 思路 | Apache-2.0 | 思路借鉴 + Rust 重写 | ✅ |
| FastGPT RRF 公式 | FastGPT 商用 | **仅算法公式，不抄代码**（算法不受版权保护） | ✅ |
| Yuxi 图谱 prompt | MIT | 思路借鉴 + Rust 重写 | ✅ |
| BaiLongma sqlite-vec | MIT | 直接借鉴 | ✅ |
| OpenAkita MDRM | AGPL-3.0 | **必须清洁室重写** | ⚠️ |
| MaxKB | **GPLv3** | **完全不借鉴** | ✅（不借鉴即合规） |

⚠️ **合规风险**：
1. FastGPT 协议在 2024 年从 Apache-2.0 改为商用付费 — 不能拷贝其代码，仅可用其公开算法思路
2. RAGFlow DeepDoc 是 Apache-2.0，但若直接翻译其 Python 代码到 Rust，需保留原版权声明
3. AnythingLLM MIT 协议最宽松，但前端用 React 19.1 + Zustand 与 AnythingLLM 的 React + Express 不同，无代码重用空间

🔴 **必须遵守**：
1. 所有借鉴代码必须保留原版权声明（NOTICE 文件）
2. NomiFun Knowledge 的 NOTICE 必须保留（Apache-2.0 要求）
3. AGPL 项目（SparkFox 自身）的修改必须开源
4. 商标保护：不能用"RAGFlow"/"Dify"/"FastGPT"等名称做营销

**NOTICE 文件更新任务**：

```
Task L1: 在 crates/sparkfox/sparkfox-knowledge/NOTICE 列出所有借鉴源
Task L2: 在 crates/sparkfox/sparkfox-embedding/NOTICE 标注 bge 模型 LICENSE
Task L3: 在 crates/sparkfox/sparkfox-parser/NOTICE 标注 lopdf/docx-rs/calamine LICENSE
```

**评分**：9.5/10（合规矩阵清晰，所有借鉴路径已明确）

---

### 专家 7：产品经理视角（用户体验与发布节奏）

**评审意见**：

✅ **认可**：
1. 用户偏好"versioned releases (v0.1 → v0.2 → v0.3 → v0.5 → v1.0)"，本方案完美契合
2. 用户偏好"单一版本提交而非增量推送"，每个 v0.x 一次 Git commit
3. 用户偏好"渐进式增量"，不一次性堆所有功能

⚠️ **产品风险**：
1. **v0.2 范围过大风险**：
   - v0.2 同时含占位功能补全（P0 6 项）+ 知识库 RAG 引擎（v0.2 范围）+ SparkFoxSider 修复
   - 建议拆分：v0.2.0 = 占位功能补全（已在前一份 spec）；v0.2.1 = 知识库 RAG 引擎
2. **bge-large-zh 模型 1.2GB 体积问题**：
   - 桌面端用户首次启动需下载 1.2GB，体验不佳
   - 建议提供"轻量版"（bge-small-zh，120MB）作为默认，用户可选升级
3. **PoC-3 (bge Rust 推理性) 阻塞 v0.2.1**：
   - 若 PoC-3 NO-GO，需退回 Python sidecar（仅 embedding 模块）
   - 这是知识库 RAG 引擎的最大风险

🔴 **强烈反对**：
1. v0.2 一次性上线所有 6 个 RAG 功能（向量+分块+多格式+混合+多模态+图谱）— 违反渐进式
2. 跳过 PoC-3 直接上 RAG — 违反 Phase -1 PoC 验收流程

**版本节奏建议**：

| 版本 | 范围 | 依赖 | 估时 |
|---|---|---|---|
| **v0.2.0** | 占位功能补全（spec A）+ SparkFoxSider 知识库入口修复 | Task 1-7 完成 | 1 周 |
| **v0.2.1** | PoC-3 bge Rust 推理性验证 | PoC-2/4 已 GO | 1 周 |
| **v0.3.0** | 知识库 RAG 引擎（向量+分块+嵌入） | PoC-3 已 GO | 2 周 |
| **v0.3.1** | 多格式解析（PDF/Word/Excel） | v0.3.0 已发布 | 1 周 |
| **v0.3.2** | 混合检索（RRF）+ Rerank | v0.3.0 已发布 | 1 周 |
| **v0.5.0** | 多模态（图片/表格 OCR） | v0.3.x 稳定 | 2 周 |
| **v1.0.0** | 知识图谱（petgraph + SQLite） | v0.5 稳定 | 3 周 |

**评分**：9/10（节奏清晰，依赖明确）

---

## 三、七专家共识与分歧

### 3.1 七专家共识（7/7 一致）

1. **保留 NomiFun Knowledge 框架**，不替换为外部项目（架构师/Rust/前端/算法/安全/合规/PM 全票）
2. **不引入外部向量库服务**（Milvus/Chroma/Qdrant），用 sqlite-vec 嵌入式（架构师/Rust/算法/安全 4 票核心 + 3 票附议）
3. **不引入 Python sidecar**（架构师/合规 2 票核心 + 5 票附议）
4. **不借鉴 MaxKB**（GPLv3 不兼容 AGPL）（合规 1 票核心 + 6 票附议）
5. **创建独立 `sparkfox-knowledge` crate**（架构师 1 票核心 + 6 票附议）
6. **遵循版本化渐进式发布**（PM 1 票核心 + 6 票附议）
7. **必须先过 PoC-3（bge Rust 推理性）才能启动 RAG 引擎**（Rust/算法/PM 3 票核心 + 4 票附议）

### 3.2 七专家分歧（需用户决策）

| 分歧点 | 阵营 A | 阵营 B | 建议决策 |
|---|---|---|---|
| **是否在 v0.3.0 引入 Rerank** | 算法工程师：引入（提升精度） | PM：v0.3.2 再加（避免范围蔓延） | **采用 PM 建议**：v0.3.0 不含 Rerank，v0.3.2 加入 |
| **嵌入模型默认大小** | 算法：bge-large-zh（精度优先） | PM：bge-small-zh 默认（体积优先） | **采用 PM 建议**：默认 small，用户可切 large |
| **知识库是否跨设备同步** | 架构师：可同步（复用 sparkfox-crdt + e2ee） | 安全：默认不同步（数据主权优先） | **采用安全建议**：默认不同步，用户显式开启 |
| **PDF 表格识别深度** | 算法：v0.3.1 就要 DeepDoc 级表格识别 | PM：v0.5 再做（避免范围蔓延） | **采用 PM 建议**：v0.3.1 仅纯文本，v0.5 表格识别 |

---

## 四、融合建议

### 4.1 总体融合策略

**保留 NomiFun Knowledge 框架 + Rust 化增量补全 RAG 引擎**

- 不替换、不重写 NomiFun 已有代码（18 .rs + 27 .tsx）
- 在 NomiFun Knowledge 之上叠加 3 个新 crate：
  - `sparkfox-embedding`：bge 嵌入（candle-transformers）
  - `sparkfox-parser`：PDF/Word/Excel 解析（lopdf/docx-rs/calamine）
  - `sparkfox-knowledge`：业务封装（调用 nomifun-knowledge + embedding + parser + store）
- 前端复用 27 .tsx，仅补 5 个 UI 任务（SparkFoxSider 入口 + 引用 chip + 文件拖拽 + 向量化进度 + 检索模式切换）

### 4.2 关键技术决策

| 决策项 | 选定方案 | 理由 |
|---|---|---|
| 向量库 | sqlite-vec 嵌入式 | 桌面端无外部服务 |
| 嵌入模型 | bge-small-zh 默认 / bge-large-zh 可选 | 体积与精度平衡 |
| 嵌入推理 | candle-transformers 0.7 | Hugging Face 官方 Rust 栈 |
| 分块策略 | 固定长度 256 tokens + 50 重叠 | 中文场景简单有效 |
| 混合检索 | RRF (k=60) | Cormack 2009 经典公式 |
| 重排 | bge-reranker-v2-m3 (v0.3.2 加入) | 渐进式 |
| PDF 解析 | lopdf 0.34 | 纯 Rust，安全 |
| Word 解析 | docx-rs 0.4 | Rust 原生 |
| Excel 解析 | calamine 0.26 | 性能极好 |
| OCR | tesseract-rs（用户可选安装） | 桌面端不强依赖 |
| 知识图谱 | petgraph + SQLite (v1.0 加入) | 不引入 Neo4j |
| 跨设备同步 | 默认关闭，用户显式开启 | 数据主权优先 |

---

## 五、改造蓝图

### 5.1 版本路线图

```
v0.2.0 (占位功能补全 spec A，1 周)
  ├─ Task 1-7: sparkfox-core/memory/crdt/e2ee/store 落地
  ├─ SparkFoxSider 知识库入口修复（前端 F1）
  └─ PoC-1/2/4 实测数据填入 poc-report.md

v0.2.1 (PoC-3 bge Rust 推理性，1 周)
  ├─ candle-transformers 加载 bge-small-zh-v1.5
  ├─ 单条/批量/10万向量检索基准测试
  └─ 决策：GO → 进入 v0.3.0；NO-GO → 退回 Python sidecar（仅 embedding）

v0.3.0 (RAG 引擎核心，2 周)
  ├─ 新建 sparkfox-embedding crate
  ├─ 新建 sparkfox-knowledge crate（封装 NomiFun + embedding + store）
  ├─ 文档分块（256 tokens + 50 重叠）
  ├─ 向量召回（sqlite-vec top-50）
  ├─ 关键词召回（FTS5 top-50）
  ├─ RRF 融合
  ├─ 前端 F2 (引用 chip) + F4 (向量化进度) + F5 (检索模式切换)
  └─ 嵌入缓存表 embedding_cache

v0.3.1 (多格式解析，1 周)
  ├─ 新建 sparkfox-parser crate
  ├─ PDF 解析（lopdf，纯文本提取）
  ├─ Word 解析（docx-rs）
  ├─ Excel 解析（calamine）
  ├─ 前端 F3 (文件拖拽扩展)
  └─ 解析安全限制（文件 < 100MB / 超时 30s）

v0.3.2 (混合检索增强，1 周)
  ├─ bge-reranker-v2-m3 集成
  ├─ 召回 top-50 → 重排 top-10
  └─ 检索结果可视化（区分向量/关键词/重排）

v0.5.0 (多模态，2 周)
  ├─ 图片 OCR（tesseract-rs，用户可选）
  ├─ PDF 表格识别（DeepDoc 思路 Rust 重写）
  └─ 图片嵌入（CLIP Rust 实现）

v1.0.0 (知识图谱，3 周)
  ├─ 新建 sparkfox-graph crate（petgraph + SQLite）
  ├─ 实体抽取（LLM function calling）
  ├─ 关系抽取（LLM function calling）
  ├─ 图谱可视化（react-flow）
  └─ 多跳遍历（参考 OpenAkita MDRM 5 维，清洁室重写）
```

### 5.2 关键文件清单

**新建 crate**：
```
crates/sparkfox/sparkfox-embedding/
  ├─ Cargo.toml
  ├─ NOTICE                          # bge 模型 LICENSE
  ├─ src/lib.rs                      # Embedder trait + BgeEmbedder
  ├─ src/cache.rs                    # 嵌入缓存
  └─ tests/bge_inference.rs          # PoC-3 性能测试

crates/sparkfox/sparkfox-parser/
  ├─ Cargo.toml
  ├─ NOTICE                          # lopdf/docx-rs/calamine LICENSE
  ├─ src/lib.rs                      # Parser trait
  ├─ src/pdf.rs                      # lopdf 包装
  ├─ src/docx.rs                     # docx-rs 包装
  ├─ src/xlsx.rs                     # calamine 包装
  └─ tests/parse_samples.rs

crates/sparkfox/sparkfox-knowledge/
  ├─ Cargo.toml
  ├─ NOTICE                          # NomiFun Apache-2.0 + 借鉴源
  ├─ src/lib.rs                      # KnowledgeBase / Document / Search
  ├─ src/chunk.rs                    # 文档分块
  ├─ src/rag.rs                      # RAG 引擎（向量+关键词+RRF）
  ├─ src/rerank.rs                   # 重排（v0.3.2）
  └─ tests/rag_e2e.rs
```

**修改的前端文件**：
```
ui/src/renderer/components/layout/SparkFoxSider.tsx       # F1 添加知识库入口
ui/src/renderer/views/ChatView/components/CitationChip.tsx # F2 新建引用 chip
ui/src/renderer/pages/knowledge/CreateStudio/SourceConfig.tsx # F3 文件拖拽扩展
ui/src/renderer/pages/knowledge/KnowledgeDetailPage/index.tsx # F4 向量化进度
ui/src/renderer/pages/knowledge/KnowledgeListPage/index.tsx   # F5 检索模式切换
```

### 5.3 PoC-3 验收门槛（v0.2.1）

| 指标 | 验收门槛 | Kill Switch |
|---|---|---|
| 单条 embedding 延迟 | < 50ms | NO-GO → Python sidecar |
| 1000 条批量 embedding | < 30s | NO-GO → Python sidecar |
| 10 万向量检索延迟 | < 500ms | NO-GO → 限制向量规模 |
| 内存占用（10 万向量） | < 300MB | NO-GO → 限制向量规模 |
| 与 Python 结果一致性 | cosine > 0.99 | NO-GO → 评估精度损失 |

### 5.4 验收 Checklist（v0.3.0 发布前）

- [ ] PoC-3 已 GO
- [ ] sparkfox-embedding 单测全过
- [ ] sparkfox-knowledge 单测全过
- [ ] RAG 端到端测试（10 文档 → 提问 → 返回带引用答案）
- [ ] 前端 F1/F2/F4/F5 完成
- [ ] NOTICE 文件完整
- [ ] `cargo test --workspace` 全过
- [ ] `bun run typecheck` + `bun run build` 全过
- [ ] 单一 Git commit

---

## 六、风险评估

### 6.1 高风险项

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| PoC-3 bge Rust 推理性不达标 | 中 | 高（阻塞 v0.3） | Kill Switch：退回 Python sidecar 仅 embedding |
| candle-transformers API 破坏性变更 | 中 | 中 | pin 版本到 0.7，锁文件 |
| bge-large-zh 1.2GB 体积影响用户首次体验 | 高 | 中 | 默认 bge-small-zh 120MB |
| PDF 表格识别质量不达预期 | 中 | 低 | v0.3.1 仅纯文本，表格留 v0.5 |
| NomiFun knowledge crate 与 sparkfox-knowledge 边界模糊 | 中 | 中 | 明确：nomifun-knowledge 是底层 IO，sparkfox-knowledge 是业务层 |

### 6.2 中风险项

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| sqlite-vec 在 100 万向量下性能不达标 | 低 | 中 | PoC-4 验收 10 万向量，100 万作压力测试 |
| 嵌入缓存命中率低 | 中 | 低 | 仅缓存查询嵌入，文档嵌入每次重建 |
| Rerank 模型加载延迟 | 中 | 低 | 懒加载，首次检索时才加载 |

---

## 七、决策与下一步

### 7.1 七专家评审结论

✅ **共识通过**：保留 NomiFun Knowledge + Rust 化增量补全 RAG 引擎
⚠️ **条件性通过**：v0.3.0 启动前必须 PoC-3 GO
🔴 **明确反对**：整体替换为外部项目 / 引入 Python sidecar / 引入外部向量库服务

### 7.2 下一步行动

1. **立即（v0.2.0）**：
   - 执行 spec A（占位功能补全）
   - 修复 SparkFoxSider 知识库入口（前端 F1）
   - 填入 PoC-1/2/4 实测数据

2. **v0.2.1**：
   - PoC-3 bge Rust 推理性验证
   - 创建 sparkfox-embedding crate 骨架

3. **v0.3.0**：
   - RAG 引擎核心实现（本蓝图 5.1 节）
   - 需先写 v0.3.0 详细 spec（基于本蓝图细化）

4. **v0.3.1+**：
   - 多格式解析 / 混合检索 / Rerank
   - 多模态 / 知识图谱

### 7.3 需要用户决策的开放问题

| 问题 | 选项 A | 选项 B | 建议 |
|---|---|---|---|
| 知识库是否跨设备同步 | 同步（复用 CRDT + E2EE） | 不同步（数据主权优先） | **B**（默认不同步，用户显式开启） |
| 嵌入模型默认 | bge-small-zh (120MB) | bge-large-zh (1.2GB) | **A**（默认 small，可切 large） |
| Rerank 引入时机 | v0.3.0（精度优先） | v0.3.2（避免范围蔓延） | **B**（v0.3.2 加入） |
| PDF 表格识别 | v0.3.1（DeepDoc 级） | v0.5（仅纯文本起步） | **B**（v0.3.1 仅纯文本） |

---

**评审报告完成。**

> 本报告作为 SparkFox 知识库功能改造的归档文档，需用户审核通过后启动 v0.2.0 执行。
