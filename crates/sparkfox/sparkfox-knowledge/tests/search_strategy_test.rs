//! Sub-Step 10.5.1a — SearchStrategy trait + 类型定义（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 10.8.1）
//! 1. SearchStrategy trait 可作 trait object（Box<dyn SearchStrategy>）
//! 2. trait 含 async fn search(&self, query: &str) -> Result<SearchResult>
//! 3. trait 含 fn name(&self) -> &str
//! 4. SearchResult 含 hits + latency_ms + strategy_name

#![forbid(unsafe_code)]

use async_trait::async_trait;
use sparkfox_knowledge::search::{SearchHit, SearchResult, SearchStrategy};

/// 测试用 Mock 策略
struct MockStrategy;

#[async_trait]
impl SearchStrategy for MockStrategy {
    async fn search(&self, _query: &str) -> sparkfox_core::Result<SearchResult> {
        Ok(SearchResult {
            hits: vec![SearchHit {
                event_id: "evt-1".to_string(),
                title: "测试事件".to_string(),
                summary: "测试摘要".to_string(),
                chunk_id: None,
                score: 1.0,
                hop: None,
                via_entities: vec![],
                chunk_span: None,
            }],
            latency_ms: 5,
            strategy_name: "mock".to_string(),
        })
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[test]
fn test_search_strategy_trait_can_be_object() {
    // 验证可作为 trait object
    let strategy: Box<dyn SearchStrategy> = Box::new(MockStrategy);
    assert_eq!(strategy.name(), "mock");
}

#[test]
fn test_search_strategy_has_name_method() {
    let strategy = MockStrategy;
    assert_eq!(strategy.name(), "mock");
}

#[tokio::test]
async fn test_search_strategy_has_search_method() {
    let strategy = MockStrategy;
    let result = strategy.search("test").await.expect("search 应成功");
    assert!(!result.hits.is_empty());
}

#[tokio::test]
async fn test_search_result_contains_hits_and_metadata() {
    let strategy = MockStrategy;
    let result = strategy.search("test").await.expect("search 应成功");
    assert!(!result.hits.is_empty(), "hits 不应为空");
    assert!(result.latency_ms >= 0, "latency_ms 应 ≥ 0");
    assert!(!result.strategy_name.is_empty(), "strategy_name 不应为空");
    // SearchHit 字段验证
    let hit = &result.hits[0];
    assert!(!hit.event_id.is_empty());
    assert!(!hit.title.is_empty());
}
