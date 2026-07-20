# sparkfox-knowledge

> SparkFox 知识库 RAG 引擎 — 分块 / 向量召回 / FTS5 关键词召回 / RRF 融合 / 引用注入

## 概述

`sparkfox-knowledge` 是 SparkFox v1.0.0 知识库子系统的编排层，负责将原始文档转化为可检索、可引用的结构化知识。

本 crate 实现 spec 1.0 Task 3.1-3.8，覆盖 RAG 全流程：

- **分块（Chunking）**：固定大小 + 滑动窗口 + 段落分隔符感知
- **向量召回（Vector Recall）**：cosine 相似度（基于本地 `Embedder` trait）
- **关键词召回（Keyword Recall）**：SQLite FTS5（`unicode61` 分词器，支持中文）
- **混合检索（Hybrid Search）**：RRF 融合（k=60，Cormack 2009）
- **引用协议（Citation Protocol）**：`[citation:kdoc_id:chunk_id:start:end]` 标记注入与解析
- **同步占位（Sync Placeholder）**：v1.0.0 仅 `NoOpSync`，v1.1.0+ 实现 E2EE 同步

## v1.0.0 范围

| 模块 | 文件 | 说明 |
|---|---|---|
| `chunk` | `src/chunk.rs` | 文档分块器（`Chunker` / `Chunk` / `ChunkMetadata`） |
| `rag` | `src/rag.rs` | RAG 引擎（`RagEngine` / `Embedder` / `VectorStore` / `KeywordStore`） |
| `citation` | `src/citation.rs` | 引用协议（`Citation` / `CitationSpan` / `inject_citations`） |
| `sync` | `src/sync.rs` | 同步占位（`KnowledgeSync` trait + `NoOpSync`） |
| `schema` | `src/schema.rs` | SAG 6 表 DDL（Task 3.1 spec 2.0，不在本次修改范围） |
| `processor` | `src/processor.rs` | Prompt 注入防御 re-export（Task 7.2.3，不在本次修改范围） |

## 架构

```
                    ┌─────────────────────────────────────────┐
                    │              RagEngine                   │
                    │  ┌────────────────────────────────────┐ │
   Document ─────►  │  │  Chunker                           │ │
                    │  │   ↓                                │ │
                    │  │  sf_chunks 表（content + offset）   │ │
                    │  │  sf_chunks_fts（FTS5 虚拟表）       │ │
                    │  └────────────────────────────────────┘ │
                    │                                         │
                    │  index_document(doc)                    │
                    │     ├─ chunker.chunk(text)              │
                    │     ├─ INSERT INTO sf_chunks            │
                    │     ├─ INSERT INTO sf_chunks_fts        │
                    │     └─ embedder.embed(chunk) → vectors  │
                    │                                         │
                    │  hybrid_search(query, top_k)            │
                    │     ├─ vector_search()   → VectorHits   │ │
                    │     ├─ keyword_search()  → KeywordHits  │ │
                    │     └─ rrf_fuse(k=60)    → FusedHits    │ │
                    └─────────────────────────────────────────┘
                                       │
                                       ▼
                          [citation:kdoc_id:chunk_id:start:end]
                                       │
                                       ▼
                          inject_citations(text) → [1] [2] [3]
```

## 循环依赖规避

```
┌─────────────────────┐         ┌─────────────────────────┐
│  sparkfox-store     │ ──────► │  sparkfox-knowledge     │
│  (Store API)        │  引用   │  (ALL_SAG_DDL 常量)      │
└─────────────────────┘         └─────────────────────────┘
                                          ▲
                                          │ 反向依赖会循环 ✗
                                          │
                          ┌───────────────┴───────────────┐
                          │  本地 trait 抽象（解耦）       │
                          │  - Embedder                   │
                          │  - VectorStore                │
                          │  - KeywordStore               │
                          └───────────────────────────────┘
                                          ▲
                                          │ 桥接（dev-deps / 调用方注入）
                          ┌───────────────┴───────────────┐
                          │  sparkfox-store::Store        │
                          │  sparkfox-embedding::BgeEmbedder │
                          └───────────────────────────────┘
```

**设计原则**：`sparkfox-knowledge` 不依赖 `sparkfox-store` / `sparkfox-embedding`，
仅定义本地 trait。集成测试通过 `EmbedderAdapter` / `StoreAdapter` 桥接真实实现。

## 使用示例

### 基础检索

```rust
use sparkfox_knowledge::{Chunker, MockEmbedder, RagEngine, InMemoryVectorStore};

// 1. 初始化引擎（使用 MockEmbedder 进行测试）
let mut engine = RagEngine::new(MockEmbedder::default())?;

// 2. 索引文档
engine.index_document("kdoc_001", "机器学习是人工智能的子领域...")?;

// 3. 混合检索（向量 + 关键词 → RRF 融合）
let result = engine.hybrid_search("什么是机器学习", 5)?;

for hit in &result.fused_hits {
    println!("[{}] score={:.4} chunk={}",
        hit.chunk.id, hit.score, hit.chunk.content.chars().take(40).collect::<String>());
}
```

### 引用注入

```rust
use sparkfox_knowledge::{inject_citations, Citation, CitationSource};

// 假设 LLM 返回的文本中包含引用标记
let text = "机器学习是 AI 的核心分支[citation:kdoc_001:chunk_003:0:120]。";

// 注入引用编号
let result = inject_citations(text)?;
println!("{}", result.text);  // "机器学习是 AI 的核心分支[1]。"
println!("{:?}", result.citations);  // [Citation { kdoc_id: "kdoc_001", ... }]
```

## 安全约束

- `#![forbid(unsafe_code)]` — 全 crate 禁用 unsafe
- 引用标记格式严格校验，防止 prompt 注入通过引用字段逃逸
- FTS5 查询构建器对用户输入进行转义（双引号包裹 token，OR 连接）
- `processor` 模块 re-export `sparkfox-security::prompt_defense` 工具函数

## 测试

```bash
# 单元测试（lib）
cargo test -p sparkfox-knowledge --lib

# 集成测试（tests/rag_e2e.rs）
cargo test -p sparkfox-knowledge --test rag_e2e

# 全部测试
cargo test -p sparkfox-knowledge
```

集成测试 `tests/rag_e2e.rs` 包含 6 个 E2E 用例：
- `rag_e2e_mock_full_pipeline` — 完整流水线（Mock Embedder）
- `rag_e2e_mock_chunking_long_document` — 长文档分块 + 索引
- `rag_e2e_mock_delete_and_reindex` — 删除 + 重建索引
- `rag_e2e_mock_rrf_double_hit_ranks_higher` — RRF 双命中提升排序
- `rag_e2e_bge_real_embedding` — 真实 BGE 模型嵌入（模型缺失时跳过）
- `rag_e2e_store_adapter` — sparkfox-store::Store 桥接（sqlite-vec 缺失时跳过）

## 版本规划

| 版本 | 范围 |
|---|---|
| **v1.0.0** | 分块 / 向量召回 / FTS5 / RRF / 引用协议 / NoOpSync（**当前**） |
| v1.1.0 | LLM 实体抽取 pipeline（extractor.rs / parser.rs / saver.rs） |
| v1.2.0+ | SAG MULTI 8-step 检索策略 + 动态超边 |

## 许可证

AGPL-3.0-only，详见 `NOTICE` 与工作区根 `LICENSE`。
