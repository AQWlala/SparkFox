//! Sub-Step 10.2.1 — EventExtractor 测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标
//! 1. EventExtractor 接受 `Vec<Chunk>` 输入，可消费整条 chunk 流
//! 2. EventExtractor 输出 `Vec<EventCandidate>`（每 chunk 0..N 条事件）
//! 3. 每个 chunk 调用且仅调用 1 次 `EventProcessor::process`
//! 4. 空 `Vec<Chunk>` 返回空 `Vec<EventCandidate>`
//! 5. EventProcessor 报错时立即短路返回 `Err`，并不再处理后续 chunk
//!
//! ## TDD-RED 说明
//! 本测试在 GREEN 实现前应全部失败（编译失败或断言失败）。
//! 第四波合并后已通过 `lib.rs` 正式导出，无需 `#[path]` 绕过。
//!
//! ## 设计要点
//! - EventProcessor 定义为 trait（10.2.2 会实现具体 LLM-backed processor）
//! - 测试用 MockProcessor（实现 EventProcessor trait）注入
//! - 通过 `Arc<AtomicUsize>` 统计调用次数

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use sparkfox_knowledge::chunk::{Chunk, ChunkMetadata};
use sparkfox_knowledge::extractor::{build_extraction_prompt, EventCandidate, EventExtractor, EventProcessor, EntityMention};

// ---------------------------------------------------------------------------
// 测试辅助：MockProcessor
// ---------------------------------------------------------------------------

/// 测试用 Mock EventProcessor
///
/// - `calls`：统计 `process` 被调用的次数
/// - `fail_on`：若 Some(idx)，则在第 idx 次（0-based）调用时返回 Err
struct MockProcessor {
    calls: Arc<AtomicUsize>,
    fail_on: Option<usize>,
}

impl MockProcessor {
    fn new() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            fail_on: None,
        }
    }

    fn with_failure(idx: usize) -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            fail_on: Some(idx),
        }
    }
}

impl EventProcessor for MockProcessor {
    async fn process(&self, chunk: &Chunk) -> sparkfox_core::Result<Vec<EventCandidate>> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(fail_idx) = self.fail_on {
            if n == fail_idx {
                return Err(sparkfox_core::Error::llm(format!(
                    "mock failure on chunk id={}",
                    chunk.id
                )));
            }
        }
        // 每个 chunk 返回 1 条 EventCandidate 作为占位
        Ok(vec![EventCandidate {
            title: format!("event-for-{}", chunk.id),
            summary: chunk.content.chars().take(20).collect(),
            content: chunk.content.clone(),
            category: Some("mock".to_string()),
            keywords: vec!["测试".to_string()],
            entities: vec![EntityMention {
                entity_type: "PERSON".to_string(),
                text: "张三".to_string(),
                start: 0,
                end: 2,
            }],
        }])
    }
}

// ---------------------------------------------------------------------------
// 测试辅助：构造 Chunk
// ---------------------------------------------------------------------------

fn make_chunk(doc_id: &str, idx: usize, content: &str) -> Chunk {
    let char_count = content.chars().count();
    Chunk {
        id: format!("{doc_id}#{idx}"),
        content: content.to_string(),
        start_offset: 0,
        end_offset: char_count,
        metadata: ChunkMetadata {
            doc_id: doc_id.to_string(),
            index: idx,
            char_count,
        },
    }
}

// ---------------------------------------------------------------------------
// 测试 1：EventExtractor 接受 Chunk 流并完成消费
// ---------------------------------------------------------------------------

/// 验证 EventExtractor 接受 `Vec<Chunk>` 输入且能完成遍历（不 panic / 不卡死）。
#[tokio::test]
async fn test_event_extractor_accepts_chunk_stream() {
    let processor = MockProcessor::new();
    let extractor = EventExtractor::new(processor);

    let chunks = vec![
        make_chunk("doc-a", 0, "第一段内容：张三到北京出差。"),
        make_chunk("doc-a", 1, "第二段内容：李四前往上海开会。"),
        make_chunk("doc-a", 2, "第三段内容：王五回到广州。"),
    ];

    // 仅验证 extract 不 panic 即视为可消费
    let result = extractor.extract(chunks).await;
    assert!(
        result.is_ok(),
        "EventExtractor 应能消费正常 chunk 流，实际错误: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// 测试 2：EventExtractor 输出 EventCandidate 流
// ---------------------------------------------------------------------------

/// 验证 EventExtractor 输出 `Vec<EventCandidate>`，且数量符合预期。
#[tokio::test]
async fn test_event_extractor_yields_event_candidates() {
    let processor = MockProcessor::new();
    let extractor = EventExtractor::new(processor);

    let chunks = vec![
        make_chunk("doc-b", 0, "张三吃了一个苹果。"),
        make_chunk("doc-b", 1, "李四去北京旅游。"),
    ];

    let events = extractor
        .extract(chunks)
        .await
        .expect("正常 chunk 流应返回 Ok(Vec<EventCandidate>)");

    // Mock 每个 chunk 产出 1 条事件 → 总计 2 条
    assert_eq!(
        events.len(),
        2,
        "应返回 2 条 EventCandidate，实际 {}",
        events.len()
    );

    // 验证 EventCandidate 字段被正确填充
    assert_eq!(events[0].title, "event-for-doc-b#0");
    assert_eq!(events[0].category.as_deref(), Some("mock"));
    assert_eq!(events[0].entities.len(), 1);
    assert_eq!(events[0].entities[0].entity_type, "PERSON");
    assert_eq!(events[0].entities[0].text, "张三");

    assert_eq!(events[1].title, "event-for-doc-b#1");
}

// ---------------------------------------------------------------------------
// 测试 3：每个 chunk 调用 1 次 EventProcessor
// ---------------------------------------------------------------------------

/// 验证 EventExtractor 对每个 chunk 恰好调用 1 次 `EventProcessor::process`。
#[tokio::test]
async fn test_event_extractor_calls_processor_for_each_chunk() {
    let processor = MockProcessor::new();
    let call_counter = processor.calls.clone();
    let extractor = EventExtractor::new(processor);

    let chunks = vec![
        make_chunk("doc-c", 0, "chunk 0"),
        make_chunk("doc-c", 1, "chunk 1"),
        make_chunk("doc-c", 2, "chunk 2"),
        make_chunk("doc-c", 3, "chunk 3"),
    ];

    extractor
        .extract(chunks)
        .await
        .expect("4 个 chunk 应全部处理成功");

    // 4 个 chunk → 4 次调用，恰好 1:1
    assert_eq!(
        call_count(&call_counter),
        4,
        "4 个 chunk 应触发 4 次 process 调用，实际 {}",
        call_count(&call_counter)
    );
}

// ---------------------------------------------------------------------------
// 测试 4：空 Chunk 流返回空 EventCandidate 流
// ---------------------------------------------------------------------------

/// 验证 `Vec<Chunk>` 为空时 `extract` 返回空 `Vec<EventCandidate>` 且不调用 processor。
#[tokio::test]
async fn test_event_extractor_handles_empty_chunk_stream() {
    let processor = MockProcessor::new();
    let call_counter = processor.calls.clone();
    let extractor = EventExtractor::new(processor);

    let chunks: Vec<Chunk> = Vec::new();
    let events = extractor
        .extract(chunks)
        .await
        .expect("空 chunk 流应返回 Ok");

    assert!(
        events.is_empty(),
        "空 chunk 流应返回空 Vec<EventCandidate>，实际长度 {}",
        events.len()
    );
    assert_eq!(
        call_count(&call_counter),
        0,
        "空 chunk 流不应触发任何 process 调用"
    );
}

// ---------------------------------------------------------------------------
// 测试 5：processor 报错时正确传播 Err
// ---------------------------------------------------------------------------

/// 验证 EventProcessor 返回 Err 时，EventExtractor 立即短路返回 Err，
/// 且不再处理后续 chunk。
#[tokio::test]
async fn test_event_extractor_propagates_processor_errors() {
    // 在第 1 次（idx=1，即第二个 chunk）调用时失败
    let processor = MockProcessor::with_failure(1);
    let call_counter = processor.calls.clone();
    let extractor = EventExtractor::new(processor);

    let chunks = vec![
        make_chunk("doc-e", 0, "chunk 0"),
        make_chunk("doc-e", 1, "chunk 1 - 将失败"),
        make_chunk("doc-e", 2, "chunk 2 - 不应被处理"),
    ];

    let result = extractor.extract(chunks).await;

    assert!(
        result.is_err(),
        "processor 报错时 extract 应返回 Err，实际 Ok"
    );

    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("mock failure"),
        "错误信息应包含 'mock failure'，实际: {err_msg}"
    );

    // 第 0 个 chunk 处理成功 → 调用 1 次
    // 第 1 个 chunk 触发失败 → 调用 1 次（失败也算被调用）
    // 第 2 个 chunk 应未被处理
    let calls = call_count(&call_counter);
    assert_eq!(
        calls, 2,
        "失败时应短路，processor 应被调用 2 次（第 0、1 个 chunk），实际 {calls}"
    );
}

// ---------------------------------------------------------------------------
// 辅助：通过 Arc<AtomicUsize> 读取计数
// ---------------------------------------------------------------------------

fn call_count(counter: &Arc<AtomicUsize>) -> usize {
    counter.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// 测试 6（REFACTOR）：build_extraction_prompt 占位函数返回 Err
// ---------------------------------------------------------------------------

/// 验证 REFACTOR 阶段新增的 `build_extraction_prompt` 占位函数返回 `Err(Internal)`，
/// 提示此函数待 Sub-Step 10.2.2 实现。
///
/// 此测试同时消除 dead_code warning（占位函数已被引用）。
#[tokio::test]
async fn test_build_extraction_prompt_placeholder_returns_err() {
    let chunk = make_chunk("doc-p", 0, "占位测试 chunk");
    let result = build_extraction_prompt(&chunk);
    assert!(
        result.is_err(),
        "占位函数应返回 Err，实际 Ok"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("10.2.2"),
        "错误信息应提及 '10.2.2'，实际: {err_msg}"
    );
}
