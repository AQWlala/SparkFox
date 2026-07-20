//! SparkFox Knowledge — 知识库 RAG 引擎
//!
//! ## 边界声明（spec 1.0 Task 3.1）
//! - **sparkfox-knowledge**：RAG 编排层（分块 / 嵌入 / 向量召回 / FTS5 关键词召回 / RRF 融合 / 引用注入）
//! - **sparkfox-store**：底层持久化（SQLite + sqlite-vec 向量检索）
//! - **sparkfox-embedding**：嵌入与重排（bge-small-zh / bge-large-zh，candle-transformers 推理）
//! - **sparkfox-security**：prompt 注入防御（[`processor`] 模块 re-export）
//!
//! ## 循环依赖规避
//! `sparkfox-store` 已依赖本 crate（用于 `ALL_SAG_DDL` 迁移），因此本 crate **不**反向依赖
//! `sparkfox-store` / `sparkfox-embedding`。RAG 引擎通过本地定义的 [`rag::Embedder`] /
//! [`rag::VectorStore`] trait 解耦具体后端，由调用方（或集成测试）注入实现。
//!
//! ## v1.0.0 范围（Task 3.1-3.8）
//! - [`chunk`]：文档分块（固定大小 + 滑动窗口 + 分隔符感知）
//! - [`rag`]：RagEngine（向量召回 + FTS5 关键词召回 + RRF 融合）
//! - [`citation`]：引用协议（CitationSpan / Citation / inject_citations）
//! - [`sync`]：E2EE 同步占位（v1.0.0 仅 NoOpSync，v1.1.0+ 实现完整同步）
//! - [`schema`]：SAG 6 表 DDL（Task 3.1 spec 2.0 已完成，不在本次修改范围）
//! - [`processor`]：prompt 注入防御 re-export（Task 7.2.3 已完成，不在本次修改范围）
//!
//! ## v1.1.0 范围
//! - **Sub-Step 10.7.1**：[`config`] extract.yaml 实体类型配置加载（`ExtractConfig` / `EntityTypeConfig` /
//!   `load_extract_config` / `load_extract_config_from`）；[`schema`] 新增 `ENTITY_TYPES`（11 种默认实体类型常量）+
//!   `INSERT_DEFAULT_ENTITY_TYPES`（INSERT OR IGNORE 幂等预填 SQL）
//! - **Sub-Step 10.2.1**：[`extractor`] EventExtractor + EventCandidate + EventProcessor trait（SAG 提取管线入口）
//! - **Sub-Step 10.2.2**：[`processor`] LlmEventProcessor（LLM-backed EventProcessor 实现，含 R-06 降级 + S-03 防御）
//! - **Sub-Step 10.2.3**：[`parser`] ResultParser（JSON 解析 + jieba 降级，4 级降级链路 R-06：
//!   JSON 直解 → JSON repair（10.1.5）→ 正则提取 → jieba NER（10.6.1））
//! - **Sub-Step 10.2.4**：[`saver`] EventSaver + 事务（写入 knowledge_event / entity /
//!   event_entity_relation 三表，BEGIN/COMMIT/ROLLBACK 原子性 + entity 归一化去重）
//! - **Sub-Step 10.3.1**：[`prompt`] 7 段式 prompt 模板（NerPrompt / ExtractPrompt，D2.15 决策）
//! - **Sub-Step 10.6.1**：[`jieba_ner`] jieba-rs 集成 + 规则匹配（R-06 LLM 失败降级路径）
//! - **Sub-Step 10.5.1**：[`search`] SearchStrategy trait + AtomicStrategy（基于
//!   event_entity_relation 的 ATOMIC 检索，spec §三 10.8.1 + 10.8.2）
//! - **Sub-Step 10.4.1**：[`entity_normalize`] NfkcNormalizer + levenshtein_normalized（NFKC 归一化 +
//!   编辑距离，RISK-SAG-08 阈值 0.2；可经 `EventSaver::with_normalizer` 注入替换 DefaultEntityNormalizer）
//! - **Sub-Step 10.4.2**：[`alias_table`] AliasTable + alias.yaml 种子数据 + 审核日志
//!   （历史名 / 尊称 / 简称 3 类 60 条 canonical，未命中回退到 NFKC + 编辑距离）
//!
//! ## v1.1.0+ 范围
//! 完整 SAG 5-table schema / 4 检索策略 / MULTI 8-step pipeline / E2EE 同步。
//!
//! ## v1.1.0 Sub-Step 11.4.2（EntityEditDrawer IPC 持久化）
//! - [`entity_ops`]：3 个 free function（[`entity_ops::merge_entities`] /
//!   [`entity_ops::split_entity`] / [`entity_ops::rename_entity`]），
//!   直接操作 entity 表 + event_entity_relation 表，不依赖 Tauri runtime。
//!   由 `sparkfox-ipc::commands` 的 3 个 `#[tauri::command]` 调用，
//!   前端 `EntityEditDrawer.tsx` 通过 `invoke('entity_merge' | 'entity_split' | 'entity_rename')` 触发。

#![forbid(unsafe_code)]

pub mod alias_table;
pub mod chunk;
pub mod citation;
pub mod config;
// Sub-Step 11.4.2: 实体编辑操作（merge / split / rename）free function 模块
pub mod entity_ops;
pub mod entity_normalize;
pub mod extractor;
// Sub-Step 12.2.1: 超边检测算法（SAG 核心创新 — >2 event 共享 >2 entity 自动形成超边）
pub mod hyperedge;
// Sub-Step 11.6.1: HnswIndex（hnsw_rs 真实 HNSW 实现，替代/补充 sqlite-vec）
pub mod index;
pub mod jieba_ner;
pub mod parser;
pub mod processor;
pub mod prompt;
pub mod rag;
pub mod saver;
pub mod schema;
pub mod search;
pub mod sync;

pub use chunk::{Chunk, ChunkMetadata, Chunker};
pub use citation::{inject_citations, Citation, CitationSource, CitationSpan};
pub use config::{load_extract_config, load_extract_config_from, ExtractConfig, EntityTypeConfig};
pub use extractor::{EventCandidate, EventProcessor, EventExtractor, EntityMention};
pub use jieba_ner::JiebaNer;
// Sub-Step 10.2.2: LlmEventProcessor（LLM-backed EventProcessor 实现，含 R-06 降级 + S-03 防御）
pub use processor::LlmEventProcessor;
// Sub-Step 10.2.3: ResultParser（JSON 解析 + jieba 降级，4 级降级链路 R-06）
pub use parser::ResultParser;
// Sub-Step 10.2.4: EventSaver（写入 knowledge_event / entity / event_entity_relation 三表 + 事务）
pub use saver::{DefaultEntityNormalizer, EntityNormalizer, EventSaver, SaveStats};
pub use prompt::{ExtractPrompt, NerPrompt, PromptContext, PromptTemplate};
pub use rag::{
    Embedder, HybridSearchResult, InMemoryVectorStore, KeywordStore, MockEmbedder, RagEngine,
    SearchHit, SearchSource, VectorStore,
};
// Sub-Step 10.5.1: SearchStrategy trait + AtomicStrategy（ATOMIC 检索）
// 注意：仅 re-export 不与 root 现有类型冲突的项。
// - `search::SearchStrategy`（trait）与 root 的 `SearchStrategy`（enum）冲突，不 re-export
// - `search::SearchHit`（事件级）与 `rag::SearchHit`（分块级）冲突，不 re-export
// 调用方通过 `sparkfox_knowledge::search::{...}` 路径访问 trait 与 SearchHit。
pub use search::{AtomicStrategy, SearchResult};
pub use sync::{KnowledgeSync, NoOpSync};
// Sub-Step 10.4.1 / 10.4.2: NfkcNormalizer + levenshtein_normalized + AliasTable
pub use entity_normalize::{levenshtein_normalized, NfkcNormalizer};
pub use alias_table::{AliasAuditEntry, AliasTable};
// Sub-Step 12.2.1: 超边检测器（SAG 核心创新 — >2 event 共享 >2 entity 自动形成超边）
pub use hyperedge::{Hyperedge, HyperedgeDetector};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志（幂等，多次调用安全）
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-knowledge v{} initialized", VERSION);
}

// ---------------------------------------------------------------------------
// 知识库 / 文档 / 检索领域类型（Task 3.1 spec 1.0）
// ---------------------------------------------------------------------------

/// 知识库句柄
///
/// 一个知识库包含若干 [`Document`]，文档经分块后生成 [`Chunk`]，
/// 分块嵌入后由 [`RagEngine`] 提供向量 / 关键词 / 混合检索。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeBase {
    /// 知识库 ID（UUID v7 字符串）
    pub id: String,
    /// 知识库名称
    pub name: String,
    /// 知识库描述
    pub description: String,
    /// 创建时间（RFC 3339 字符串）
    pub created_at: String,
}

/// 文档句柄
///
/// 文档内容经 [`Chunker::chunk`] 分块后写入存储，由 [`RagEngine::index_document`] 编排。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    /// 文档 ID（全局唯一）
    pub id: String,
    /// 所属知识库 ID
    pub kb_id: String,
    /// 文档标题
    pub title: String,
    /// 文档正文
    pub content: String,
    /// 文档来源
    pub source: DocumentSource,
    /// 创建时间（RFC 3339 字符串）
    pub created_at: String,
}

/// 文档来源
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value")]
pub enum DocumentSource {
    /// 文件系统路径
    File(String),
    /// URL（HTTP/HTTPS）
    Url(String),
    /// 纯文本（用户直接输入）
    Text,
}

/// 检索请求参数
#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// 查询文本
    pub query: String,
    /// 知识库 ID（用于过滤；None 表示跨库检索）
    pub kb_id: Option<String>,
    /// 返回 top-k
    pub top_k: usize,
    /// 检索策略
    pub strategy: SearchStrategy,
}

/// 检索策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    /// 仅向量召回
    Vector,
    /// 仅关键词召回（FTS5）
    Keyword,
    /// 混合检索（向量 + 关键词 → RRF 融合）
    Hybrid,
}
