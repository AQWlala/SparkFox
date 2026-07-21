//! Sub-Step 10.2.1 — EventExtractor（事件抽取编排器）
//!
//! ## 职责
//! 消费 [`Chunk`] 流（`Vec<Chunk>`），对每个 chunk 委托给 [`EventProcessor`]
//! 进行事件抽取，将全部 [`EventCandidate`] 汇聚为单一 `Vec` 返回。
//!
//! ## 边界
//! - 本 crate 只定义 trait [`EventProcessor`]，具体 LLM-backed 实现由
//!   Sub-Step 10.2.2 在 `processor.rs` 中完成。
//! - [`EventExtractor`] 不感知 LLM / JSON repair / jieba 降级，仅做编排。
//! - 任一 chunk 处理失败 → 立即短路返回 `Err`，不再处理后续 chunk。
//!
//! ## 设计参考
//! - `docs/SparkFox-v1.1.0-规划.md` Sub-Step 10.2.1
//! - SAG 论文 Chunk → Event pipeline

#![forbid(unsafe_code)]

// 通过 crate 名绝对路径引用 Chunk：
// - 当本文件被 lib.rs 注册为 `pub mod extractor;` 时，`sparkfox_knowledge::chunk`
//   是 crate 自引用，Rust 2018+ 允许此写法。
// - 当本文件被测试通过 `#[path = "../src/extractor.rs"] mod extractor;` 引入时，
//   `sparkfox_knowledge` 由 dev-dependency `sparkfox-knowledge = { path = "." }` 提供。
// 两种上下文均能解析，避免修改 lib.rs（Sub-Step 10.2.1 约束）。
use crate::chunk::Chunk;

pub use sparkfox_core::Result;

/// 实体提及（NER 结果，简化类型）
///
/// 对应 `schema.rs::DDL_ENTITY` 中的 `entity` 行，但 v1.1.0 抽取阶段仅保留
/// 必要字段：实体类型 / 文本 / 在 chunk 内的字符偏移。
/// 归一化与持久化由后续步骤完成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMention {
    /// 实体类型，对齐 [`crate::schema::ENTITY_TYPES`] 中的 `type` 字段
    /// （PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT / OBJECT / CONCEPT / LAW / DISEASE / OTHER）
    pub entity_type: String,
    /// 实体文本（原文字面量，未归一化）
    pub text: String,
    /// 在 chunk.content 中的字符起始偏移（含）
    pub start: usize,
    /// 在 chunk.content 中的字符结束偏移（不含）
    pub end: usize,
}

/// 事件候选（单 chunk 抽取结果）
///
/// 对应 `schema.rs::DDL_KNOWLEDGE_EVENT` 中的 `knowledge_event` 行，
/// v1.1.0 抽取阶段产出的中间结构；持久化前的归一化 / rank / level / 时间戳
/// 由 `EventSaver`（Sub-Step 10.2.4）补全。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCandidate {
    /// 事件标题（一句话摘要，≤ 50 字符）
    pub title: String,
    /// 事件摘要（≤ 200 字符）
    pub summary: String,
    /// 事件完整内容（chunk 关联片段或 LLM 改写）
    pub content: String,
    /// 事件分类（可选，如 "会议" / "出差" / "就医"）
    pub category: Option<String>,
    /// 关键词列表（用于 FTS5 索引增强）
    pub keywords: Vec<String>,
    /// 涉及的实体提及列表
    pub entities: Vec<EntityMention>,
}

/// 事件处理器 trait
///
/// 实现方负责：将单个 [`Chunk`] 通过 LLM（或降级路径 jieba+规则）抽取为
/// 0..N 条 [`EventCandidate`]。
///
/// ## 实现约束
/// - 必须 `Send + Sync`（供异步上下文跨线程共享）
/// - 实现方需自行处理 prompt 注入防御（S-03，复用 v1.0.0 [`crate::processor`] 工具）
/// - LLM 失败时由实现方自行重试 + JSON repair + jieba 降级（Sub-Step 10.2.2）
/// - 抽取阶段不可恢复错误向上透传给 [`EventExtractor`]
// 抑制 `async fn in public trait` lint：本 trait 仅在 crate 内部 + 测试中实现，
// 实现方（LlmEventProcessor / MockProcessor）已通过 `'static` / `Send + Sync`
// 约束保证 Future 可跨线程传递；引入 async_trait 会改写签名且增加 Box 开销，
// 与 v1.1.0 设计意图（零成本泛型 + 静态分发）冲突，故选择就地抑制。
#[allow(async_fn_in_trait)]
pub trait EventProcessor: Send + Sync {
    /// 处理单个 chunk，返回 0..N 条事件候选
    async fn process(&self, chunk: &Chunk) -> Result<Vec<EventCandidate>>;
}

/// 事件抽取编排器
///
/// 泛型 `P` 接受任何实现 [`EventProcessor`] 的类型，便于测试注入 Mock。
///
/// ## 用法
/// ```ignore
/// let processor = MyLlmProcessor::new(llm_client);
/// let extractor = EventExtractor::new(processor);
/// let events = extractor.extract(chunks).await?;
/// ```
pub struct EventExtractor<P: EventProcessor> {
    processor: P,
}

impl<P: EventProcessor> EventExtractor<P> {
    /// 创建 EventExtractor，绑定具体 EventProcessor 实现
    pub fn new(processor: P) -> Self {
        Self { processor }
    }

    /// 消费 chunk 流，输出 EventCandidate 流
    ///
    /// ## 行为
    /// 1. 顺序遍历 `chunks`（保持输入顺序，便于事件时序还原）
    /// 2. 对每个 chunk 调用且仅调用 1 次 `processor.process(&chunk)`
    /// 3. 收集所有 `EventCandidate` 到单一 `Vec` 返回
    /// 4. 任一 chunk 处理失败 → 立即短路返回 `Err`，不再处理后续 chunk
    /// 5. 空 `chunks` → 返回空 `Vec`（不调用 processor）
    ///
    /// ## 并发说明
    /// v1.1.0 实现为顺序处理（保证事件时序 + LLM 限流友好）。
    /// v1.2.0+ 若需并发，需在外层加 Semaphore 限流，且保留 chunk.index 排序。
    pub async fn extract(&self, chunks: Vec<Chunk>) -> Result<Vec<EventCandidate>> {
        let mut events = Vec::new();
        for chunk in &chunks {
            let chunk_events = self.processor.process(chunk).await?;
            events.extend(chunk_events);
        }
        Ok(events)
    }
}

// ---------------------------------------------------------------------------
// REFACTOR 阶段：prompt 构造逻辑占位（Sub-Step 10.2.2 由 EventProcessor 复用）
// ---------------------------------------------------------------------------

/// 构造事件抽取 prompt 的占位函数
///
/// ## 职责
/// 将 [`Chunk`] 转换为发送给 LLM 的 prompt 字符串，包括：
/// 1. system 段：角色 + 任务 + 输出格式（JSON Schema）
/// 2. user 段：经 prompt 注入防御处理后的 chunk 文本
/// 3. few-shot 段：中文事件抽取示例（10.3.2 提供）
///
/// ## v1.1.0 占位说明
/// Sub-Step 10.2.1 仅占位，实际 prompt 模板与注入防御逻辑在 Sub-Step 10.2.2
/// （`processor.rs::EventProcessor`）和 Sub-Step 10.3.1（`prompt/` 模块）实现。
/// 此函数保留为接口锚点，便于 10.2.2 直接复用。
///
/// ## 参数
/// - `chunk`：待抽取事件的文档分块
///
/// ## 返回
/// 构造完成的 prompt 字符串（未转义；调用方负责 `escape_document_content`）
///
/// ## 错误
/// v1.1.0 占位实现始终返回 `Err(Internal)`，提示调用方此函数待 10.2.2 实现。
pub fn build_extraction_prompt(_chunk: &Chunk) -> Result<String> {
    Err(sparkfox_core::Error::internal(
        "build_extraction_prompt 待 Sub-Step 10.2.2 实现（当前为 10.2.1 REFACTOR 占位）",
    ))
}
