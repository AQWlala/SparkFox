//! Sub-Step 12.1.1 — MULTI_ES 策略 ES-first 实现（TDD-RED → GREEN → REFACTOR）
//!
//! spec §三 12.1.1 要求实现 MULTI_ES 策略，与 MULTI / MULTI1 / HOPLLM 并列第 4 种策略。
//! ES-first 表示先用实体检索（Entity Search first）缩小候选集，再做多跳扩展。
//!
//! ## 测试目标
//! 1. MultiEsStrategy 实现 SearchStrategy trait
//! 2. search() 返回 SearchResult
//! 3. 精确匹配实体名（query="张三" → 直接命中 ent-A）
//! 4. LIKE 匹配部分实体名（query="张" → LIKE '%张%' 命中 ent-A 张三）
//! 5. 无匹配时降级到 MultiStrategy 行为（query 含实体名但非子串 → jieba NER 抽取后 BFS）
//! 6. max_hop 限制生效（max_hop=1 时仅返回 hop=1 的直接关联 event）
//!
//! ## 测试 fixture（多跳图，与 multi_strategy_test.rs 一致）
//! ```text
//! entity_A (张三) ── event_1 (张三出差) ── entity_B (北京)
//!                                          │
//! entity_B (北京) ── event_2 (北京天气) ── entity_C (腾讯)
//!                                          │
//! entity_C (腾讯) ── event_3 (腾讯财报) ── entity_D (李四)
//! ```
//!
//! ## ES-first 算法（spec §三 12.1.1）
//! - Step1: query 直接作为 entity_name 查询 entity 表（跳过 Step2 NER 抽取）
//!   - `SELECT id FROM entity WHERE name LIKE '%query%' OR normalized_name = query`
//!   - 若无匹配实体，降级到 MultiStrategy 行为（jieba NER + BFS）
//! - Step2-Step8: 用匹配到的 entities 作为种子，复用 MultiStrategy 的 BFS 扩展逻辑
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{MultiEsStrategy, SearchStrategy};

/// 构造 BFS 3 跳测试图（与 multi_strategy_test.rs::setup_multi_hop_db 一致）
///
/// ```text
/// 张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四
/// ```
///
/// - 4 个 entity（张三 / 北京 / 腾讯 / 李四）
/// - 3 个 event（evt-1 张三出差 / evt-2 北京天气 / evt-3 腾讯财报）
/// - 6 条 event_entity_relation（每个 event 关联 2 个 entity）
fn setup_multi_hop_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 4 个 entity
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

#[tokio::test]
async fn test_multi_es_strategy_implements_search_strategy() {
    // 验收指标 1：MultiEsStrategy 实现 SearchStrategy trait
    //
    // 构造 MultiEsStrategy 实例后应能作为 `&dyn SearchStrategy` 使用，
    // 且 name() 返回 "multi_es"（spec §三 12.1.1 第 4 种策略）
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);
    let dyn_ref: &dyn SearchStrategy = &strategy;
    assert_eq!(dyn_ref.name(), "multi_es");
}

#[tokio::test]
async fn test_multi_es_strategy_search_returns_result() {
    // 验收指标 2：search 返回 SearchResult
    //
    // - search 不应 panic
    // - 返回 Ok(SearchResult)
    // - strategy_name == "multi_es"
    // - latency_ms >= 0
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功返回 SearchResult");
    assert_eq!(result.strategy_name, "multi_es");
    // latency_ms 由 Instant::now() 测量，至少为 0
    assert!(
        result.latency_ms < u64::MAX,
        "latency_ms 应为合理值，实际: {}",
        result.latency_ms
    );
}

#[tokio::test]
async fn test_multi_es_strategy_finds_entity_by_exact_name() {
    // 验收指标 3：精确匹配实体名（ES-first 直接命中）
    //
    // query="张三" → entity 表 name="张三" 精确匹配（name LIKE '%张三%'）
    // → 找到 ent-A 作为 seed → BFS 扩展找到 evt-1（hop=1）
    //
    // ES-first 优势：跳过 jieba NER 抽取，直接用 query 作为 entity name 检索
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "精确匹配「张三」应返回 evt-1（张三直接关联），实际: {:?}",
        event_ids
    );
    // 验证 via_entities 含「张三」（seed entity 应出现在路径上）
    let evt1 = result
        .hits
        .iter()
        .find(|h| h.event_id == "evt-1")
        .expect("evt-1 应在结果中");
    assert!(
        evt1.via_entities.iter().any(|e| e.name == "张三"),
        "evt-1 的 via_entities 应含「张三」作为 seed entity，实际: {:?}",
        evt1.via_entities
    );
}

#[tokio::test]
async fn test_multi_es_strategy_finds_entity_by_partial_name() {
    // 验收指标 4：LIKE 匹配部分实体名
    //
    // query="张" → entity.name LIKE '%张%' 匹配 "张三" → 找到 ent-A
    // → BFS 扩展找到 evt-1
    //
    // ES-first 的 LIKE 匹配支持缩写/简称检索（如输入「张」匹配「张三」）
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);
    let result = strategy
        .search("张")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "LIKE 匹配「张」应找到 ent-A（张三）并返回 evt-1，实际: {:?}",
        event_ids
    );
}

#[tokio::test]
async fn test_multi_es_strategy_falls_back_to_multi_when_no_entity_match() {
    // 验收指标 5：无匹配时降级到 MultiStrategy 行为
    //
    // query="张三的行程" 在 entity 表中无 name LIKE '%张三的行程%' 匹配
    // （因为 entity name 最长只有 "张三" / "北京" / "腾讯" / "李四"，都不包含 "张三的行程" 子串）
    // → ES-first 失败 → 降级到 MultiStrategy 行为（jieba NER 抽取 + BFS 扩展）
    // → jieba 从 "张三的行程" 抽取出 "张三" → 找到 ent-A → 返回 evt-1
    //
    // 验证降级路径仍能找到结果（fallback 不返回空）
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);
    let result = strategy
        .search("张三的行程")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "降级到 MultiStrategy 后 jieba 应抽出「张三」并找到 evt-1，实际: {:?}",
        event_ids
    );
    // 验证 strategy_name 仍为 "multi_es"（降级不改变策略名）
    assert_eq!(
        result.strategy_name, "multi_es",
        "降级不改变 strategy_name，仍应为 multi_es"
    );
}

#[tokio::test]
async fn test_multi_es_strategy_max_hop_limit() {
    // 验收指标 6：max_hop 限制生效
    //
    // max_hop=1 时，MULTI_ES 等价于 ATOMIC 检索（仅返回 hop=1 的直接关联 event）
    // - query="张三" → 找到 ent-A → hop=1 返回 evt-1
    // - 不应扩展到 evt-2 / evt-3（hop=2/3 被截断）
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new_with_max_hop(conn, 1);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "max_hop=1 应返回 evt-1，实际: {:?}",
        event_ids
    );
    assert!(
        !event_ids.contains(&"evt-2".to_string()),
        "max_hop=1 不应扩展到 evt-2，实际: {:?}",
        event_ids
    );
    assert!(
        !event_ids.contains(&"evt-3".to_string()),
        "max_hop=1 不应扩展到 evt-3，实际: {:?}",
        event_ids
    );
    // 所有 hit 的 hop 应为 1
    for hit in &result.hits {
        assert_eq!(
            hit.hop,
            Some(1),
            "max_hop=1 时所有 hop 应为 1，实际 {:?}",
            hit.hop
        );
    }
}
