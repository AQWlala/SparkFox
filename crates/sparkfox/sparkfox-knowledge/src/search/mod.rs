//! Sub-Step 10.5.1 — SearchStrategy trait + ATOMIC 检索（spec §三 10.8.1 + 10.8.2）
//!
//! ## 设计
//! - [`SearchStrategy`] trait：统一检索接口，支持 VECTOR / ATOMIC / MULTI / MULTI_ES 4 策略
//! - [`AtomicStrategy`]：基于 `event_entity_relation` 表的原子事件检索
//! - [`SearchResult`] / [`SearchHit`]：统一结果类型，含 `hop` / `via_entities` / `chunk_span` 元数据
//!
//! ## ATOMIC 检索流程
//! 1. 从 query 提取实体（复用 [`crate::jieba_ner::JiebaNer`]）
//! 2. SQL JOIN `event_entity_relation` + `entity` + `entity_type`：通过实体找事件 + 填充 [`EntityRef`]
//! 3. 返回 [`SearchHit`] 列表，`hop=1`，`via_entities=[匹配的 EntityRef]`，`chunk_span=None`
//!
//! ## hop 含义
//! - ATOMIC=1：单跳检索（直接通过实体找事件）
//! - MULTI=N：多跳检索（ATOMIC + 1 跳扩展）
//!
//! ## U-02 修复（Sub-Step 10.7.1，spec §三 10.9.1）
//! v1.1.0 引入 MULTI 多跳检索策略后，原 [`SearchHit`] 元数据不足以表达多跳路径：
//! - `hop: Option<usize>` → `Option<u8>`（收紧类型，max 255 跳足够）
//! - `via_entities: Vec<String>` → `Vec<EntityRef>`（含 entity_id / entity_type / name）
//! - 新增 `chunk_span: Option<(usize, usize)>`（未来 MULTI / VECTOR 可填充 chunk 位置）
//! 详见 [`SearchHit`] / [`EntityRef`] 文档。
//!
//! ## dyn compatibility
//! trait 使用 `#[async_trait]` 宏（而非原生 `async fn in trait`），因为 spec §三 10.8.1
//! 要求 `Box<dyn SearchStrategy>` 可作 trait object。原生 `async fn in trait` 在 stable
//! Rust 上不支持 dyn compatibility，`#[async_trait]` 通过 desugar 为
//! `fn search(&self, ...) -> Pin<Box<dyn Future + Send>>` 实现 object safety。
//!
//! ## 与 [`crate::rag::SearchHit`] 的区别
//! - `rag::SearchHit`：基于分块（[`crate::chunk::Chunk`]）的 RAG 检索命中
//! - `search::SearchHit`：基于事件（`knowledge_event` 表）的 SAG 检索命中
//! 两者位于不同模块，调用方按需选择。

pub mod atomic;
pub mod multi;
mod types;

pub use types::EntityRef;

use async_trait::async_trait;
use sparkfox_core::Result;

/// 检索策略 trait
///
/// 实现方负责：从 query 提取实体 → 查询 SAG 表 → 返回 [`SearchHit`] 列表。
///
/// ## 实现约束
/// - 必须 `Send + Sync`（供异步上下文跨线程共享）
/// - 使用 `#[async_trait]` 宏以支持 `Box<dyn SearchStrategy>`（dyn compatibility）
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::{SearchStrategy, AtomicStrategy};
///
/// let strategy: Box<dyn SearchStrategy> = Box::new(AtomicStrategy::new(conn));
/// let result = strategy.search("张三去了哪里").await?;
/// ```
#[async_trait]
pub trait SearchStrategy: Send + Sync {
    /// 执行检索
    ///
    /// ## 参数
    /// - `query`: 自然语言查询字符串
    ///
    /// ## 返回
    /// [`SearchResult`]，含 `hits` 列表 + `latency_ms` + `strategy_name`
    async fn search(&self, query: &str) -> Result<SearchResult>;

    /// 策略名称（如 `"atomic"` / `"multi"` / `"multi_es"`）
    fn name(&self) -> &str;
}

/// 检索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// 命中的 [`SearchHit`] 列表
    pub hits: Vec<SearchHit>,
    /// 检索耗时（毫秒）
    pub latency_ms: u64,
    /// 策略名称
    pub strategy_name: String,
}

/// 单个检索命中
///
/// ## U-02 扩展（Sub-Step 10.7.1，spec §三 10.9.1）
/// v1.1.0 引入 MULTI 多跳检索策略后，对原结构做以下扩展以支持多跳元数据追溯：
/// - `hop`：`Option<usize>` → `Option<u8>`（收紧类型，max 255 跳已足够，节省序列化体积）
/// - `via_entities`：`Vec<String>` → `Vec<EntityRef>`（结构化实体引用，含 entity_id /
///   entity_type / name，避免调用方二次查询 `entity` / `entity_type` 表）
/// - 新增 `chunk_span: Option<(usize, usize)>`：未来 MULTI / VECTOR 策略可填充 chunk 内
///   (start, end) 位置区间；ATOMIC 检索暂为 `None`（不涉及 chunk 位置）
///
/// ## 序列化
/// 派生 `Serialize` / `Deserialize`，便于 API 返回 / 跨设备 CRDT 同步。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SearchHit {
    /// 事件 ID（对应 `knowledge_event.id`）
    pub event_id: String,
    /// 事件标题
    pub title: String,
    /// 事件摘要
    pub summary: String,
    /// 关联 chunk ID（可选；ATOMIC 检索可能为 `None`）
    pub chunk_id: Option<String>,
    /// 相关性得分（ATOMIC 默认 1.0；MULTI 按跳数衰减）
    pub score: f64,
    /// 跳跃层级（ATOMIC=1 / MULTI=N），U-02 收紧为 `u8`（max 255 跳已足够）
    pub hop: Option<u8>,
    /// 经过的实体路径（ATOMIC 为匹配的单个实体；MULTI 为多跳路径）
    ///
    /// U-02 扩展：从 `Vec<String>`（仅 entity_id）改为 `Vec<EntityRef>`（含 entity_id /
    /// entity_type / name），避免调用方二次查询 `entity` / `entity_type` 表。
    pub via_entities: Vec<EntityRef>,
    /// chunk 内位置区间 `(start, end)`（U-02 新增）
    ///
    /// 未来 MULTI / VECTOR 策略可填充 chunk 内的字符位置区间，用于高亮 / 截取；
    /// ATOMIC 检索不涉及 chunk 位置，固定为 `None`。
    pub chunk_span: Option<(usize, usize)>,
}

pub use atomic::AtomicStrategy;
pub use multi::MultiStrategy;

