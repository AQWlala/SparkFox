//! Sub-Step 11.2.3 — HOPLLM 检索策略（LLM 引导多跳，TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.2.3）
//! 1. HopllmStrategy 实现 SearchStrategy trait，name() 返回 "hopllm"
//! 2. 每个 hop 调用 LLM 从候选实体中选择下一个实体（用 [`MockLlm`] 验证）
//! 3. 最多 `max_hop=3` 跳（深链 fixture 验证 hop ≤ 3）
//! 4. LLM 失败时降级到 multi1（用 [`FailLlm`] 验证 strategy_name 仍为 "hopllm" 但 hits 全为 hop=1）
//!
//! ## 设计要点
//! hopllm = LLM 引导多跳扩展。在每个 hop，调用 LLM 从候选实体中选择最相关的下一个实体，
//! 而非 BFS 全扩展。LLM 失败时降级到 multi1（`max_hop=1`）保证基本可用性。
//!
//! ## 测试 fixture
//! - **3 跳小图**（`setup_hopllm_3hop_db`）：4 entity / 3 event / 6 relation，验证 LLM 选路
//! - **4 跳深链**（`setup_hopllm_4hop_db`）：5 entity / 4 event / 8 relation，验证 `max_hop=3`
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::{FailLlm, HopllmStrategy, MockLlm};
use sparkfox_knowledge::search::SearchStrategy;

// ---------------------------------------------------------------------------
// Fixture 1：3 跳小图（验证 LLM 选路 + 语义扩展）
// ---------------------------------------------------------------------------

/// 构造 3 跳测试图（复用 10.8.2 链式拓扑）：
///
/// ```text
/// 张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四
/// ```
///
/// - 4 个 entity（张三 / 北京 / 腾讯 / 李四）
/// - 3 个 event（evt-1 张三出差 / evt-2 北京天气 / evt-3 腾讯财报）
/// - 6 条 event_entity_relation（每个 event 关联 2 个 entity）
///
/// 查询「张三」期望 HOPLLM 扩展结果（[`MockLlm`] 选第一个候选）：
/// - hop=1：evt-1（张三直接关联）
/// - hop=2：evt-2（LLM 选择「北京」后扩展）
/// - hop=3：evt-3（LLM 选择「腾讯」后扩展）
fn setup_hopllm_3hop_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 4 个 entity（A=张三 / B=北京 / C=腾讯 / D=李四）
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-A", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-B", "default_location", "北京", "北京", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-C", "default_organization", "腾讯", "腾讯", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-D", "default_person", "李四", "李四", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 3 个 event（按 created_time 递增，便于稳定排序）
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-1", "kb-1", "doc-1", "张三出差", "张三去北京出差", "张三昨天去北京出差", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-2", "kb-1", "doc-1", "北京天气", "北京今天晴朗", "北京今天天气晴朗", "2026-07-20T01:00:00Z", "2026-07-20T01:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-3", "kb-1", "doc-1", "腾讯财报", "腾讯发布财报", "腾讯今天发布财报", "2026-07-20T02:00:00Z", "2026-07-20T02:00:00Z"],
    ).unwrap();

    // 6 条 event_entity_relation（链式：A-e1-B-e2-C-e3-D）
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-1", "evt-1", "ent-A", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-2", "evt-1", "ent-B", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-3", "evt-2", "ent-B", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-4", "evt-2", "ent-C", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-5", "evt-3", "ent-C", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-6", "evt-3", "ent-D", "2026-07-20T00:00:00Z"],
    ).unwrap();

    conn
}

// ---------------------------------------------------------------------------
// Fixture 2：4 跳深链（验证 max_hop=3 上限）
// ---------------------------------------------------------------------------

/// 构造 4 跳深链测试图：
///
/// ```text
/// 张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四 ── evt-4 ── 王五
/// ```
///
/// - 5 个 entity（张三 / 北京 / 腾讯 / 李四 / 王五）
/// - 4 个 event（evt-1 张三出差 / evt-2 北京天气 / evt-3 腾讯财报 / evt-4 李四入职）
/// - 8 条 event_entity_relation（每个 event 关联 2 个 entity）
///
/// 查询「张三」期望 HOPLLM 扩展结果（[`MockLlm`] 选第一个候选，`max_hop=3`）：
/// - hop=1：evt-1（张三直接关联）
/// - hop=2：evt-2（LLM 选择「北京」后扩展）
/// - hop=3：evt-3（LLM 选择「腾讯」后扩展）
/// - evt-4（hop=4）**不应**出现在结果中（受 `max_hop=3` 限制）
fn setup_hopllm_4hop_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 5 个 entity（A=张三 / B=北京 / C=腾讯 / D=李四 / E=王五）
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-A", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-B", "default_location", "北京", "北京", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-C", "default_organization", "腾讯", "腾讯", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-D", "default_person", "李四", "李四", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-E", "default_person", "王五", "王五", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 4 个 event（按 created_time 递增，便于稳定排序）
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-1", "kb-1", "doc-1", "张三出差", "张三去北京出差", "张三昨天去北京出差", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-2", "kb-1", "doc-1", "北京天气", "北京今天晴朗", "北京今天天气晴朗", "2026-07-20T01:00:00Z", "2026-07-20T01:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-3", "kb-1", "doc-1", "腾讯财报", "腾讯发布财报", "腾讯今天发布财报", "2026-07-20T02:00:00Z", "2026-07-20T02:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-4", "kb-1", "doc-1", "李四入职", "李四入职新公司", "李四今天入职新公司", "2026-07-20T03:00:00Z", "2026-07-20T03:00:00Z"],
    ).unwrap();

    // 8 条 event_entity_relation（链式：A-e1-B-e2-C-e3-D-e4-E）
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-1", "evt-1", "ent-A", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-2", "evt-1", "ent-B", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-3", "evt-2", "ent-B", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-4", "evt-2", "ent-C", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-5", "evt-3", "ent-C", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-6", "evt-3", "ent-D", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-7", "evt-4", "ent-D", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-8", "evt-4", "ent-E", "2026-07-20T00:00:00Z"],
    ).unwrap();

    conn
}

// ---------------------------------------------------------------------------
// 测试 1：hopllm 调用 LLM 选择下一跳
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hopllm_strategy_calls_llm_for_next_hop() {
    // 验收指标 1：hopllm 调用 LLM 选择下一跳
    //
    // 使用 [`MockLlm`]（始终返回第一个候选），查询「张三」：
    // - hop=1：evt-1（张三直接关联，seed entity）
    // - hop=2：evt-2（LLM 选择「北京」后扩展，证明 LLM 被调用）
    // - hop=3：evt-3（LLM 选择「腾讯」后扩展）
    //
    // 若 LLM 未被调用（即纯 BFS 也会扩展到 evt-2/evt-3），通过 [`FailLlm`] 对比测试
    // （见测试 4）验证 LLM 路径生效。本测试主要验证 MockLlm 路径下结果非空且包含多跳。
    let conn = setup_hopllm_3hop_db();
    let strategy = HopllmStrategy::new(conn, Box::new(MockLlm));
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // 验证策略名
    assert_eq!(result.strategy_name, "hopllm");
    assert_eq!(strategy.name(), "hopllm");

    // 应至少返回 1 个 hit（evt-1）
    assert!(
        !result.hits.is_empty(),
        "hopllm 应返回至少 1 个 hit（evt-1），实际: {:?}",
        result.hits
    );

    // MockLlm 选第一个候选 → 应扩展到 hop=2/3 的 events
    // （若 LLM 未被调用，evt-2/evt-3 不会出现在结果中）
    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "应含 evt-1（hop=1，seed entity 直接关联），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "应含 evt-2（hop=2，LLM 选择「北京」后扩展），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-3".to_string()),
        "应含 evt-3（hop=3，LLM 选择「腾讯」后扩展），实际: {:?}",
        event_ids
    );
}

// ---------------------------------------------------------------------------
// 测试 2：最多 max_hop=3 跳（深链 fixture）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hopllm_strategy_respects_max_hop_3() {
    // 验收指标 2：最多 max_hop=3 跳
    //
    // 在 4 跳深链上查询「张三」（[`MockLlm`] 选第一个候选）：
    // - hop=1：evt-1（张三直接关联）
    // - hop=2：evt-2（LLM 选择「北京」后扩展）
    // - hop=3：evt-3（LLM 选择「腾讯」后扩展）
    // - evt-4（hop=4）**不应**在结果中（受 `max_hop=3` 限制，LLM 不会再选「李四」）
    let conn = setup_hopllm_4hop_db();
    let strategy = HopllmStrategy::new(conn, Box::new(MockLlm));
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "应含 evt-1（hop=1），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "应含 evt-2（hop=2），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-3".to_string()),
        "应含 evt-3（hop=3），实际: {:?}",
        event_ids
    );
    assert!(
        !event_ids.contains(&"evt-4".to_string()),
        "evt-4（hop=4）不应出现在结果中（受 max_hop=3 限制），实际: {:?}",
        event_ids
    );

    // 所有 hit 的 hop 应 ≤ 3
    for hit in &result.hits {
        let hop = hit.hop.unwrap_or(0);
        assert!(
            hop <= 3,
            "hop 应 ≤ 3（max_hop=3 限制），实际 evt-{} hop={:?}",
            hit.event_id,
            hit.hop
        );
    }
}

// ---------------------------------------------------------------------------
// 测试 3：语义扩展（LLM 选择的路径上的 events）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hopllm_strategy_semantic_expansion() {
    // 验收指标 3：语义扩展（LLM 选择的路径上的 events）
    //
    // 使用 [`MockLlm`]（选第一个候选），查询「张三」：
    // - evt-2 (hop=2) via_entities 应含 LLM 选择的「北京」
    // - evt-3 (hop=3) via_entities 应含 LLM 选择的「腾讯」
    //
    // 验证 via_entities 记录了 LLM 选路的历史路径（语义扩展轨迹）。
    let conn = setup_hopllm_3hop_db();
    let strategy = HopllmStrategy::new(conn, Box::new(MockLlm));
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // evt-2 (hop=2) 路径：张三 → evt-1 → 北京 → evt-2
    // via_entities 应含「北京」（LLM 在 hop=1 后选择的下一跳实体）
    let evt2 = result
        .hits
        .iter()
        .find(|h| h.event_id == "evt-2")
        .expect("应含 evt-2（hop=2，LLM 选路后扩展）");
    assert!(
        evt2.via_entities.iter().any(|e| e.name == "北京"),
        "evt-2 的 via_entities 应含 LLM 选择的「北京」，实际: {:?}",
        evt2.via_entities
    );

    // evt-3 (hop=3) 路径：张三 → evt-1 → 北京 → evt-2 → 腾讯 → evt-3
    // via_entities 应含「腾讯」（LLM 在 hop=2 后选择的下一跳实体）
    let evt3 = result
        .hits
        .iter()
        .find(|h| h.event_id == "evt-3")
        .expect("应含 evt-3（hop=3，LLM 选路后扩展）");
    assert!(
        evt3.via_entities.iter().any(|e| e.name == "腾讯"),
        "evt-3 的 via_entities 应含 LLM 选择的「腾讯」，实际: {:?}",
        evt3.via_entities
    );
}

// ---------------------------------------------------------------------------
// 测试 4：LLM 失败时降级到 multi1
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hopllm_strategy_fallback_to_multi1_on_llm_failure() {
    // 验收指标 4：LLM 失败时降级到 multi1
    //
    // 使用 [`FailLlm`]（始终返回 Err），查询「张三」：
    // - LLM 全程失败 → 触发降级到 multi1 行为
    // - strategy_name 仍为 "hopllm"（保持策略标识不变）
    // - hits 全部为 hop=1（multi1 单跳扩展，无多跳扩展）
    // - evt-1 应在结果中（seed entity 直接关联）
    // - evt-2/evt-3 不应在结果中（multi1 不扩展）
    let conn = setup_hopllm_3hop_db();
    let strategy = HopllmStrategy::new(conn, Box::new(FailLlm));
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // strategy_name 仍为 "hopllm"（保持策略标识不变，即使降级到 multi1 行为）
    assert_eq!(result.strategy_name, "hopllm");
    assert_eq!(strategy.name(), "hopllm");

    // 降级到 multi1：所有 hit 的 hop 应为 1（无多跳扩展）
    assert!(
        !result.hits.is_empty(),
        "降级后仍应返回 hop=1 的 hits（evt-1），实际: {:?}",
        result.hits
    );
    for hit in &result.hits {
        assert_eq!(
            hit.hop,
            Some(1),
            "LLM 失败降级后所有 hit 应为 hop=1（multi1 行为），实际 evt-{} hop={:?}",
            hit.event_id,
            hit.hop
        );
    }

    // evt-1 应在结果中（multi1 单跳扩展命中）
    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "降级后应含 evt-1（hop=1，seed entity 直接关联），实际: {:?}",
        event_ids
    );
    // evt-2/evt-3 不应在结果中（multi1 不扩展到 hop=2/3）
    assert!(
        !event_ids.contains(&"evt-2".to_string()),
        "降级后不应含 evt-2（multi1 不扩展到 hop=2），实际: {:?}",
        event_ids
    );
    assert!(
        !event_ids.contains(&"evt-3".to_string()),
        "降级后不应含 evt-3（multi1 不扩展到 hop=3），实际: {:?}",
        event_ids
    );
}
