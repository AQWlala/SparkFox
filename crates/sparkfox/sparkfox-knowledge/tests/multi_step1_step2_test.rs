//! Sub-Step 11.1.1 — MULTI 8 步骨架 + Step1/Step2 实施（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.1）
//! 1. MultiStrategy 仍实现 SearchStrategy trait（10.8.2 兼容）
//! 2. Step1 query 向量化（mock embedding，返回 Vec<f32>，384 维）
//! 3. Step2 query 实体抽取（jieba + 正则，返回 Vec<EntityMention>）
//! 4. Step1+Step2 管线返回 MultiState 中间状态
//! 5. 8 步骨架完整（Step3-8 为 stub，返回 Empty）
//!
//! ## 8 步流程定义
//! - Step1: query 向量化（mock embedding）
//! - Step2: query 实体抽取（jieba + 正则）
//! - Step3: 实体向量检索（HnswIndex）— stub
//! - Step4: 事件检索（event_entity_relation）— stub
//! - Step5: 三策略占位（multi/multi1/hopllm）— stub
//! - Step6: 候选合并 + 去重 — stub
//! - Step7: Rerank 重排 — stub
//! - Step8: 返回 SearchResult（thought_process 8 步记录在 MultiState）

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi_step::{
    step1_vectorize, step2_extract_entities, step3_vector_search, step4_event_search,
    step5_strategies_placeholder, step6_merge_dedupe, step7_rerank, step8_build_result, MultiState,
};
use sparkfox_knowledge::search::{MultiStrategy, SearchStrategy};

/// 构造空 SAG DB（仅 DDL + 默认 entity_type，不插入业务数据）
fn setup_empty_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();
    conn
}

#[tokio::test]
async fn test_multi_strategy_implements_search_strategy() {
    // 验收指标 1：MultiStrategy 仍实现 SearchStrategy trait（10.8.2 兼容）
    let conn = setup_empty_db();
    let strategy = MultiStrategy::new(conn);
    let dyn_ref: &dyn SearchStrategy = &strategy;
    assert_eq!(dyn_ref.name(), "multi");
}

#[test]
fn test_step1_query_vectorization() {
    // 验收指标 2：Step1 query 向量化（用 mock embedding，返回 Vec<f32>）
    let state = MultiState::new("张三去了哪里");
    let state = step1_vectorize(state);

    assert!(!state.query_vec.is_empty(), "query_vec 应非空");
    assert_eq!(
        state.query_vec.len(),
        384,
        "mock embedding 应为 384 维（bge-small-zh 维度）"
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step1")),
        "thought_process 应记录 Step1，实际: {:?}",
        state.thought_process
    );
}

#[test]
fn test_step2_entity_extraction_from_query() {
    // 验收指标 3：Step2 从 query 提取实体（jieba + 正则，返回 Vec<EntityMention>）
    let state = MultiState::new("张三去了北京");
    let state = step2_extract_entities(state);

    assert!(!state.entities.is_empty(), "应至少抽取到一个实体");
    let names: Vec<&str> = state.entities.iter().map(|e| e.text.as_str()).collect();
    assert!(
        names.contains(&"张三"),
        "应抽取「张三」（PERSON），实际: {:?}",
        names
    );
    assert!(
        names.contains(&"北京"),
        "应抽取「北京」（LOCATION），实际: {:?}",
        names
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step2")),
        "thought_process 应记录 Step2，实际: {:?}",
        state.thought_process
    );
}

#[test]
fn test_step1_step2_pipeline_returns_intermediate_state() {
    // 验收指标 4：Step1+Step2 返回 MultiState 中间状态
    let state = MultiState::new("张三去了北京");
    let state = step1_vectorize(state);
    let state = step2_extract_entities(state);

    assert!(!state.query_vec.is_empty(), "Step1 应填充 query_vec");
    assert_eq!(state.query_vec.len(), 384, "mock embedding 384 维");
    assert!(!state.entities.is_empty(), "Step2 应填充 entities");
    assert_eq!(
        state.thought_process.len(),
        2,
        "Step1+Step2 应产生 2 条 thought_process，实际: {:?}",
        state.thought_process
    );
}

#[test]
fn test_multi_pipeline_skeleton_has_8_step_stubs() {
    // 验收指标 5：8 步骨架完整（Step3-8 为 stub，返回 Empty）
    let state = MultiState::new("测试 query");
    let state = step1_vectorize(state);
    let state = step2_extract_entities(state);
    let state = step3_vector_search(state);
    let state = step4_event_search(state);
    let state = step5_strategies_placeholder(state);
    let state = step6_merge_dedupe(state);
    let state = step7_rerank(state);

    // Step3-7 stub：entity_ids / candidates / hits 应保持为空
    assert!(
        state.entity_ids.is_empty(),
        "Step3 stub 应保持 entity_ids 为空"
    );
    assert!(
        state.candidates.is_empty(),
        "Step4 stub 应保持 candidates 为空"
    );
    assert!(state.hits.is_empty(), "Step5-7 stub 应保持 hits 为空");

    // thought_process 应有 7 条记录（Step1-7，Step8 不再追加）
    assert_eq!(
        state.thought_process.len(),
        7,
        "Step1-7 应产生 7 条 thought_process，实际: {:?}",
        state.thought_process
    );

    // Step8：构建 SearchResult
    let result = step8_build_result(state);
    assert_eq!(result.strategy_name, "multi");
    assert!(
        result.hits.is_empty(),
        "Step8 应返回空 hits（因为 Step5 stub 未填充）"
    );
}
