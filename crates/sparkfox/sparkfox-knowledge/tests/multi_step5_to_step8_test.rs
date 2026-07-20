//! Sub-Step 11.1.3 — MULTI Step5/Step6/Step7/Step8 真实实施（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.1.3，6 测试）
//! 1. Step5 占位用 multi1（async 调用，hits 非空）
//! 2. Step6 events → chunks 关联（chunk_id 填充）
//! 3. Step7 rerank 生成 thought_process（含 hits 数量和 top-3 摘要）
//! 4. thought_process 含 Step1..Step7（7 条记录）
//! 5. Step8 聚合返回 SearchResult（strategy_name / latency_ms / hits）
//! 6. SearchHit 含 hop / via_entities（multi1 返回的 hits 应有 hop=Some(1) + via_entities 非空）
//!
//! ## 设计要点
//! - Step5 用 async 版本（`step5_with_multi1_async`），调用 `Multi1Strategy::search`
//!   填充 `state.hits`（max_hop=1 单跳剪枝）
//! - Step6 查询 `knowledge_event.chunk_id` 填充未设置的 `SearchHit.chunk_id`
//! - Step7 按 score 降序稳定排序 + 取 top_k + 生成 top-3 摘要到 thought_process
//! - Step8 校验 hop / via_entities 已填充，包装为 SearchResult
//!
//! ## 测试 fixture
//! - 3 个 entity（张三 / 北京 / 腾讯）
//! - 2 个 event（含 chunk_id 字段：evt-1 → chunk-1 / evt-2 → chunk-2）
//! - 4 条 event_entity_relation
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::Multi1Strategy;
use sparkfox_knowledge::search::multi_step::{
    step1_vectorize, step2_extract_entities, step3_vector_search, step4_event_search,
    step5_with_multi1_async, step6_associate_chunks, step7_rerank_with_thought,
    step8_build_result_with_hop, MultiState,
};
use sparkfox_knowledge::search::{EntityRef, SearchHit};

// ---------------------------------------------------------------------------
// 测试 fixture：3 entity / 2 event（含 chunk_id）/ 4 event_entity_relation
// ---------------------------------------------------------------------------

/// 构造含 chunk_id 的 SAG DB（3 entity / 2 event / 4 relation）
///
/// ```text
/// 张三 ── evt-1 (chunk-1) ── 北京 ── evt-2 (chunk-2) ── 腾讯
/// ```
///
/// - 3 个 entity：ent-A 张三（PERSON）/ ent-B 北京（LOCATION）/ ent-C 腾讯（ORGANIZATION）
/// - 2 个 event：evt-1 张三出差（chunk_id="chunk-1"）/ evt-2 腾讯在北京（chunk_id="chunk-2"）
/// - 4 条 event_entity_relation：evt-1↔(ent-A, ent-B) / evt-2↔(ent-B, ent-C)
///
/// 查询「张三」的 multi1（max_hop=1）期望：
/// - 仅返回 evt-1（hop=1，张三直接关联）
/// - evt-1.chunk_id="chunk-1"（由 BFS find_event_detail 自动填充）
/// - evt-1.via_entities=[EntityRef{张三}]
fn setup_sag_db_with_chunks() -> Connection {
    let conn = Connection::open_in_memory().expect("open_in_memory 失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("PRAGMA foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("DDL 执行失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("INSERT_DEFAULT_ENTITY_TYPES 失败");

    // 3 个 entity + 2 个 event（含 chunk_id）+ 4 条 event_entity_relation
    conn.execute_batch(
        "INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-A', '张三', '张三', 'default_person', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-B', '北京', '北京', 'default_location', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-C', '腾讯', '腾讯', 'default_organization', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO knowledge_event (id, kb_id, doc_id, chunk_id, title, summary, content, created_time, updated_time) \
         VALUES ('evt-1', 'kb-1', 'doc-1', 'chunk-1', '张三出差北京', '张三前往北京', '张三昨天前往北京出差', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO knowledge_event (id, kb_id, doc_id, chunk_id, title, summary, content, created_time, updated_time) \
         VALUES ('evt-2', 'kb-1', 'doc-2', 'chunk-2', '腾讯在北京', '腾讯北京分公司', '腾讯在北京设有分公司', '2026-07-20T01:00:00Z', '2026-07-20T01:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-1', 'evt-1', 'ent-A', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-2', 'evt-1', 'ent-B', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-3', 'evt-2', 'ent-B', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-4', 'evt-2', 'ent-C', '2026-07-20T00:00:00Z');",
    )
    .expect("insert 业务数据失败");

    conn
}

/// 构造一个空的 SearchHit（仅 event_id / title / summary，chunk_id=None）
fn make_hit_without_chunk(event_id: &str, title: &str, summary: &str, score: f64) -> SearchHit {
    SearchHit {
        event_id: event_id.to_string(),
        title: title.to_string(),
        summary: summary.to_string(),
        chunk_id: None,
        score,
        hop: Some(1),
        via_entities: vec![EntityRef {
            entity_id: "ent-X".to_string(),
            entity_type: "PERSON".to_string(),
            name: "测试".to_string(),
        }],
        chunk_span: None,
    }
}

// ---------------------------------------------------------------------------
// 测试 1：Step5 占位用 multi1（async 调用，hits 非空）
// ---------------------------------------------------------------------------

/// 验收指标 1：Step5 真实实现用 multi1 单跳剪枝
///
/// 构造含 chunk_id 的 SAG DB，创建 Multi1Strategy 实例，调用
/// `step5_with_multi1_async(state, &multi1)`，应：
/// - `state.hits` 非空（至少 1 个 hit，evt-1）
/// - `thought_process` 含 "Step5"
#[tokio::test]
async fn test_step5_uses_multi1_strategy_as_placeholder() {
    let conn = setup_sag_db_with_chunks();
    let multi1 = Multi1Strategy::new(conn);

    let state = MultiState::new("张三");
    let state = step5_with_multi1_async(state, &multi1).await;

    // hits 应非空（multi1 max_hop=1 检索到 evt-1）
    assert!(
        !state.hits.is_empty(),
        "Step5 后 hits 应非空（multi1 检索到 evt-1），实际: {:?}",
        state.hits
    );

    // thought_process 应含 Step5
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step5")),
        "thought_process 应含 \"Step5\"，实际: {:?}",
        state.thought_process
    );

    // evt-1 应在 hits 中（张三直接关联）
    let event_ids: Vec<&str> = state.hits.iter().map(|h| h.event_id.as_str()).collect();
    assert!(
        event_ids.contains(&"evt-1"),
        "hits 应含 evt-1（张三 → rel-1 → evt-1），实际: {:?}",
        event_ids
    );
}

// ---------------------------------------------------------------------------
// 测试 2：Step6 events → chunks 关联
// ---------------------------------------------------------------------------

/// 验收指标 2：Step6 将 event_id 关联的 chunk_id 填充到 SearchHit.chunk_id
///
/// 构造 2 个 chunk_id=None 的 SearchHit（evt-1 / evt-2），SAG DB 中 evt-1.chunk_id="chunk-1"
/// / evt-2.chunk_id="chunk-2"。调用 `step6_associate_chunks(state, &conn)` 后：
/// - `hits[0].chunk_id == Some("chunk-1")`
/// - `hits[1].chunk_id == Some("chunk-2")`
/// - `thought_process` 含 "Step6" 和 "2"（关联了 2 个）
#[test]
fn test_step6_chunk_association() {
    let conn = setup_sag_db_with_chunks();

    // 构造 2 个 chunk_id=None 的 hits（模拟 Step5 后但 chunk_id 未填充的场景）
    let mut state = MultiState::new("测试 query");
    state.hits = vec![
        make_hit_without_chunk("evt-1", "张三出差北京", "张三前往北京", 1.0),
        make_hit_without_chunk("evt-2", "腾讯在北京", "腾讯北京分公司", 0.5),
    ];

    let state = step6_associate_chunks(state, &conn);

    // 验证 chunk_id 已填充
    assert_eq!(
        state.hits[0].chunk_id,
        Some("chunk-1".to_string()),
        "evt-1.chunk_id 应填充为 chunk-1"
    );
    assert_eq!(
        state.hits[1].chunk_id,
        Some("chunk-2".to_string()),
        "evt-2.chunk_id 应填充为 chunk-2"
    );

    // thought_process 应含 Step6 + 关联数量 2
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step6")),
        "thought_process 应含 \"Step6\"，实际: {:?}",
        state.thought_process
    );
    assert!(
        state
            .thought_process
            .iter()
            .any(|s| s.contains("Step6") && s.contains('2')),
        "thought_process Step6 记录应含关联数量 2，实际: {:?}",
        state.thought_process
    );
}

// ---------------------------------------------------------------------------
// 测试 3：Step7 rerank 生成 thought_process（含 hits 数量和 top-3 摘要）
// ---------------------------------------------------------------------------

/// 验收指标 3：Step7 按.score 降序排序 + 取 top_k + 生成 top-3 摘要
///
/// 构造 3 个 hits（score=0.5 / 0.9 / 0.3），调用 `step7_rerank_with_thought(state, 2)`：
/// - 排序后：[0.9, 0.5, 0.3] → 截断 top_k=2 → [0.9, 0.5]
/// - `thought_process` 含 "Step7" + "top-3" + hits 数量
#[test]
fn test_step7_rerank_with_thought_process() {
    let mut state = MultiState::new("测试 query");
    // 故意乱序构造，验证降序排序
    state.hits = vec![
        make_hit_without_chunk("evt-A", "title-A", "summary-A", 0.5),
        make_hit_without_chunk("evt-B", "title-B", "summary-B", 0.9),
        make_hit_without_chunk("evt-C", "title-C", "summary-C", 0.3),
    ];

    let state = step7_rerank_with_thought(state, 2);

    // 验证排序：[0.9, 0.5]（top_k=2 截断）
    assert_eq!(state.hits.len(), 2, "top_k=2 应保留 2 个 hits");
    assert!(
        state.hits[0].score >= state.hits[1].score,
        "hits 应按 score 降序排序，实际: [{}, {}]",
        state.hits[0].score,
        state.hits[1].score
    );
    assert_eq!(state.hits[0].event_id, "evt-B", "最高 score 应为 evt-B (0.9)");
    assert_eq!(state.hits[1].event_id, "evt-A", "次高 score 应为 evt-A (0.5)");

    // thought_process 应含 Step7
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step7")),
        "thought_process 应含 \"Step7\"，实际: {:?}",
        state.thought_process
    );

    // thought_process 应含 top-3 摘要（即便 hits 已截断到 2 个，top-3 摘要仍记录前 2 个）
    let step7_entry = state
        .thought_process
        .iter()
        .find(|s| s.contains("Step7"))
        .expect("应存在 Step7 记录");
    assert!(
        step7_entry.contains("top-3"),
        "Step7 记录应含 \"top-3\"，实际: {}",
        step7_entry
    );
    assert!(
        step7_entry.contains("evt-B"),
        "Step7 top-3 摘要应含 evt-B，实际: {}",
        step7_entry
    );
}

// ---------------------------------------------------------------------------
// 测试 4：thought_process 含 Step1..Step7（7 条记录）
// ---------------------------------------------------------------------------

/// 验收指标 4：完整 Step1..Step7 pipeline 后 thought_process 含 7 条记录
///
/// 执行：Step1（向量化）→ Step2（实体抽取）→ Step3（stub）→ Step4（stub）
/// → Step5（multi1）→ Step6（chunk 关联）→ Step7（rerank）
/// 断言 thought_process 含 Step1/Step2/Step3/Step4/Step5/Step6/Step7 共 7 条记录。
#[tokio::test]
async fn test_step7_thought_process_contains_7_steps() {
    let conn = setup_sag_db_with_chunks();
    let multi1 = Multi1Strategy::new(conn);

    // Step1：query 向量化
    let state = MultiState::new("张三");
    let state = step1_vectorize(state);

    // Step2：query 实体抽取
    let state = step2_extract_entities(state);

    // Step3：stub（留空 entity_ids）
    let state = step3_vector_search(state);

    // Step4：stub（留空 candidates）
    let state = step4_event_search(state);

    // Step5：multi1 单跳剪枝（异步调用，需要 conn 引用 — 但 multi1 持有 conn 所有权）
    // 注：此处 conn 已移入 multi1，Step6 需要单独的 conn 引用
    // 测试中重新打开一个 conn 用于 Step6（fixture 确定性，DB 内容一致）
    let conn_for_step6 = setup_sag_db_with_chunks();
    let state = step5_with_multi1_async(state, &multi1).await;

    // Step6：events → chunks 关联
    let state = step6_associate_chunks(state, &conn_for_step6);

    // Step7：rerank + thought_process
    let state = step7_rerank_with_thought(state, 10);

    // 验证 thought_process 含 Step1..Step7（7 条记录）
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step1")),
        "thought_process 应含 Step1，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step2")),
        "thought_process 应含 Step2，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step3")),
        "thought_process 应含 Step3，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step4")),
        "thought_process 应含 Step4，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step5")),
        "thought_process 应含 Step5，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step6")),
        "thought_process 应含 Step6，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step7")),
        "thought_process 应含 Step7，实际: {:?}",
        state.thought_process
    );

    // 计数：thought_process 至少 7 条（Step1..Step7 各一条）
    let step_count = state
        .thought_process
        .iter()
        .filter(|s| {
            s.contains("Step1")
                || s.contains("Step2")
                || s.contains("Step3")
                || s.contains("Step4")
                || s.contains("Step5")
                || s.contains("Step6")
                || s.contains("Step7")
        })
        .count();
    assert!(
        step_count >= 7,
        "thought_process 应至少 7 条 Step 记录，实际: {} 条 ({:?})",
        step_count,
        state.thought_process
    );
}

// ---------------------------------------------------------------------------
// 测试 5：Step8 聚合返回 SearchResult
// ---------------------------------------------------------------------------

/// 验收指标 5：Step8 聚合返回 SearchResult（含 strategy_name / latency_ms / hits）
///
/// 构造含 2 个 hits 的 state，调用 `step8_build_result_with_hop(state, "multi1", 42)`：
/// - `result.strategy_name == "multi1"`
/// - `result.latency_ms == 42`
/// - `result.hits.len() == 2`
#[test]
fn test_step8_result_aggregation() {
    let mut state = MultiState::new("测试 query");
    state.hits = vec![
        make_hit_without_chunk("evt-1", "title-1", "summary-1", 0.9),
        make_hit_without_chunk("evt-2", "title-2", "summary-2", 0.5),
    ];
    state
        .thought_process
        .push("Step1..Step7 模拟记录".to_string());

    let result = step8_build_result_with_hop(state, "multi1", 42);

    assert_eq!(result.strategy_name, "multi1", "strategy_name 应为 multi1");
    assert_eq!(result.latency_ms, 42, "latency_ms 应为 42");
    assert_eq!(
        result.hits.len(),
        2,
        "hits 应保留 2 个（不截断），实际: {}",
        result.hits.len()
    );
    assert_eq!(result.hits[0].event_id, "evt-1");
    assert_eq!(result.hits[1].event_id, "evt-2");
}

// ---------------------------------------------------------------------------
// 测试 6：SearchHit 含 hop / via_entities（multi1 返回的 hits 应有 hop=Some(1) + via_entities 非空）
// ---------------------------------------------------------------------------

/// 验收指标 6：Step8 后 SearchHit 含 hop / via_entities（来自 multi1 BFS）
///
/// 完整跑 Step5（multi1）→ Step8，断言每个 hit：
/// - `hop == Some(1)`（multi1 max_hop=1）
/// - `via_entities` 非空（BFS 路径上的 EntityRef 列表）
#[tokio::test]
async fn test_step8_populates_hop_via_entities() {
    let conn = setup_sag_db_with_chunks();
    let multi1 = Multi1Strategy::new(conn);

    // Step5：multi1 单跳剪枝（hits 含 hop + via_entities）
    let state = MultiState::new("张三");
    let state = step5_with_multi1_async(state, &multi1).await;

    // 断言 Step5 后 hits 已含 hop / via_entities
    assert!(
        !state.hits.is_empty(),
        "Step5 后 hits 应非空（multi1 检索到 evt-1）"
    );
    for hit in &state.hits {
        assert_eq!(
            hit.hop,
            Some(1),
            "multi1 返回的 hit.hop 应为 Some(1)，实际 evt-{} hop={:?}",
            hit.event_id,
            hit.hop
        );
        assert!(
            !hit.via_entities.is_empty(),
            "multi1 返回的 hit.via_entities 应非空，实际 evt-{} via_entities={:?}",
            hit.event_id,
            hit.via_entities
        );
    }

    // Step8：聚合返回 SearchResult
    let result = step8_build_result_with_hop(state, "multi1", 100);

    assert_eq!(result.strategy_name, "multi1");
    assert_eq!(result.latency_ms, 100);
    assert!(!result.hits.is_empty(), "Step8 后 hits 应非空");

    // 验证 Step8 保留 hop / via_entities 字段
    for hit in &result.hits {
        assert_eq!(
            hit.hop,
            Some(1),
            "Step8 后 hit.hop 应保留为 Some(1)，实际: {:?}",
            hit.hop
        );
        assert!(
            !hit.via_entities.is_empty(),
            "Step8 后 hit.via_entities 应非空，实际: {:?}",
            hit.via_entities
        );
        // 验证 via_entities 中 EntityRef 字段已填充
        for ent_ref in &hit.via_entities {
            assert!(
                !ent_ref.entity_id.is_empty(),
                "EntityRef.entity_id 应非空，实际: {:?}",
                ent_ref
            );
            assert!(
                !ent_ref.name.is_empty(),
                "EntityRef.name 应非空，实际: {:?}",
                ent_ref
            );
        }
    }
}
