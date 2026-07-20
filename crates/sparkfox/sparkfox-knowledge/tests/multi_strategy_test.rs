//! Sub-Step 10.8.2 — MultiStrategy 实现（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.2.1 简化版）
//! 1. MultiStrategy 实现 SearchStrategy trait
//! 2. name() 返回 "multi"
//! 3. BFS 多跳扩展（max_hop=3）：
//!    - hop1: seed entity → event
//!    - hop2: event → 其他 entity → 其他 event
//!    - hop3: 重复扩展
//! 4. via_entities 收集路径上所有 entity
//! 5. hop 字段递增（1 → 2 → 3）
//! 6. 通过共享 entity 找到连通的 events
//!
//! ## 测试 fixture（多跳图）
//! ```text
//! entity_A (张三) ── event_1 (张三出差) ── entity_B (北京)
//!                                          │
//! entity_B (北京) ── event_2 (北京天气) ── entity_C (腾讯)
//!                                          │
//! entity_C (腾讯) ── event_3 (腾讯财报) ── entity_D (李四)
//! ```
//!
//! 查询 "张三" 期望 BFS 扩展结果：
//! - hop1: event_1（直接关联张三）
//! - hop2: event_2（经北京到达）
//! - hop3: event_3（经腾讯到达）
//! - via_entities 应包含路径上的 entity（北京 / 腾讯）

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{MultiStrategy, SearchStrategy};

/// 构造 BFS 3 跳测试图：
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
async fn test_multi_strategy_implements_trait_and_name() {
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new(conn);
    // 验证可作为 trait object
    let dyn_ref: &dyn SearchStrategy = &strategy;
    assert_eq!(dyn_ref.name(), "multi");
}

#[tokio::test]
async fn test_multi_strategy_expands_bfs_3_hops() {
    // 验收指标 1：BFS 扩展 3 跳
    //
    // 查询「张三」→ hop1: evt-1 → hop2: evt-2 → hop3: evt-3
    // 三个 event 都应被检索到（BFS 扩展 3 跳到达 evt-3）
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "hop1 应返回 evt-1（张三直接关联），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "hop2 应返回 evt-2（经北京扩展），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-3".to_string()),
        "hop3 应返回 evt-3（经腾讯扩展），实际: {:?}",
        event_ids
    );
}

#[tokio::test]
async fn test_multi_strategy_returns_hop_path() {
    // 验收指标 2：返回 hop 路径（hop1 → hop2 → hop3 递增）
    //
    // 查询「张三」应返回 3 个 hop 层级：1, 2, 3
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // 按 event_id 查找对应的 hop
    let hop_of = |eid: &str| {
        result
            .hits
            .iter()
            .find(|h| h.event_id == eid)
            .and_then(|h| h.hop)
    };

    assert_eq!(hop_of("evt-1"), Some(1), "evt-1 应为 hop=1");
    assert_eq!(hop_of("evt-2"), Some(2), "evt-2 应为 hop=2");
    assert_eq!(hop_of("evt-3"), Some(3), "evt-3 应为 hop=3");

    // 验证 hop 递增路径存在（1 → 2 → 3）
    let mut hops: Vec<u8> = result
        .hits
        .iter()
        .filter_map(|h| h.hop)
        .collect();
    hops.sort();
    assert!(
        hops.contains(&1) && hops.contains(&2) && hops.contains(&3),
        "应包含 hop=1/2/3 三个层级，实际 hops: {:?}",
        hops
    );
}

#[tokio::test]
async fn test_multi_strategy_collects_intermediate_entities() {
    // 验收指标 3：收集中间实体（via_entities 含路径上所有 entity）
    //
    // 查询「张三」：
    // - evt-1 (hop1): via_entities 含 [张三]
    // - evt-2 (hop2): via_entities 含 [张三, 北京]（路径：张三 → evt-1 → 北京 → evt-2）
    // - evt-3 (hop3): via_entities 含 [张三, 北京, 腾讯]（路径：张三 → evt-1 → 北京 → evt-2 → 腾讯 → evt-3）
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let via_of = |eid: &str| {
        result
            .hits
            .iter()
            .find(|h| h.event_id == eid)
            .map(|h| h.via_entities.clone())
            .unwrap_or_default()
    };

    // hop1: evt-1 至少含「张三」
    let evt1_vias = via_of("evt-1");
    assert!(
        evt1_vias.iter().any(|e| e.name == "张三"),
        "evt-1 的 via_entities 应含「张三」，实际: {:?}",
        evt1_vias
    );

    // hop2: evt-2 应含路径上的中间 entity（至少含「北京」）
    let evt2_vias = via_of("evt-2");
    assert!(
        evt2_vias.iter().any(|e| e.name == "北京"),
        "evt-2 的 via_entities 应含中间实体「北京」，实际: {:?}",
        evt2_vias
    );

    // hop3: evt-3 应含「腾讯」
    let evt3_vias = via_of("evt-3");
    assert!(
        evt3_vias.iter().any(|e| e.name == "腾讯"),
        "evt-3 的 via_entities 应含中间实体「腾讯」，实际: {:?}",
        evt3_vias
    );
}

#[tokio::test]
async fn test_multi_strategy_finds_connected_events() {
    // 验收指标 4：找到连通 events（不同 entity 关联的 event 通过共享 entity 连通）
    //
    // 查询「李四」（从链式图的另一端开始）应能找到：
    // - hop1: evt-3（李四直接关联）
    // - hop2: evt-2（经腾讯扩展）
    // - hop3: evt-1（经北京扩展）
    //
    // 验证 BFS 双向连通性（从任一 entity 出发都能扩展到整条链）
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("李四")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-3".to_string()),
        "hop1 应返回 evt-3（李四直接关联），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "hop2 应返回 evt-2（经腾讯扩展），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "hop3 应返回 evt-1（经北京扩展），实际: {:?}",
        event_ids
    );
}

#[tokio::test]
async fn test_multi_strategy_handles_no_match() {
    // 无匹配时返回空 Vec（不应 panic）
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("完全不存在的查询XYZ123")
        .await
        .expect("search 应成功");
    assert!(result.hits.is_empty(), "无匹配应返回空 Vec");
    assert_eq!(result.strategy_name, "multi");
}

#[tokio::test]
async fn test_multi_strategy_respects_max_hop() {
    // max_hop=1 时，MULTI 等价于 ATOMIC（仅返回 hop=1 的直接关联 event）
    let conn = setup_multi_hop_db();
    let strategy = MultiStrategy::new_with_max_hop(conn, 1);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // max_hop=1：仅返回 evt-1（hop1），不应扩展到 evt-2 / evt-3
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
