//! Sub-Step 10.5.1b — AtomicStrategy 实现（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 10.8.2）
//! 1. AtomicStrategy 实现 SearchStrategy trait
//! 2. name() 返回 "atomic"
//! 3. 从 query 提取实体（jieba + 正则）
//! 4. SQL JOIN event_entity_relation 返回 event + chunk
//! 5. 返回 SearchHit 含 event_id / chunk_id
//! 6. 无匹配返回空 Vec
//! 7. top_k 参数生效

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{AtomicStrategy, SearchStrategy};

fn setup_db_with_data() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 插入测试数据：2 个 entity + 2 个 event + 3 个 relation
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-1", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-2", "default_location", "北京", "北京", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-1", "kb-1", "doc-1", "张三出差", "张三去北京出差", "张三昨天去北京出差", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["evt-2", "kb-1", "doc-1", "北京天气", "北京今天晴", "北京今天天气晴朗", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-1", "evt-1", "ent-1", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-2", "evt-1", "ent-2", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-3", "evt-2", "ent-2", "2026-07-20T00:00:00Z"],
    ).unwrap();

    conn
}

#[tokio::test]
async fn test_atomic_strategy_implements_trait() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    // 验证实现了 SearchStrategy trait
    let _: &dyn SearchStrategy = &strategy;
}

#[tokio::test]
async fn test_atomic_strategy_name_returns_atomic() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    assert_eq!(strategy.name(), "atomic");
}

#[tokio::test]
async fn test_atomic_search_extracts_entities_from_query() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    // query 含「张三」应能提取并匹配 ent-1
    let result = strategy.search("张三去了哪里").await.expect("search 应成功");
    assert!(
        result.hits.iter().any(|h| h.event_id == "evt-1"),
        "应返回 evt-1（张三出差）"
    );
}

#[tokio::test]
async fn test_atomic_search_joins_event_entity_relation() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    // query 含「北京」应通过 event_entity_relation JOIN 返回 evt-1 和 evt-2
    let result = strategy.search("北京").await.expect("search 应成功");
    let event_ids: Vec<_> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "应返回 evt-1（张三去北京出差）"
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "应返回 evt-2（北京天气）"
    );
}

#[tokio::test]
async fn test_atomic_search_returns_hits_with_metadata() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    let result = strategy.search("张三").await.expect("search 应成功");
    assert!(!result.hits.is_empty());
    let hit = &result.hits[0];
    assert!(!hit.event_id.is_empty());
    assert!(!hit.title.is_empty());
    assert!(!hit.summary.is_empty());
}

#[tokio::test]
async fn test_atomic_search_handles_no_match() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    let result = strategy
        .search("完全不存在的查询XYZ123")
        .await
        .expect("search 应成功");
    assert!(result.hits.is_empty(), "无匹配应返回空 Vec");
}

#[tokio::test]
async fn test_atomic_search_limits_to_top_k() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new_with_top_k(conn, 1);
    let result = strategy.search("北京").await.expect("search 应成功");
    assert!(
        result.hits.len() <= 1,
        "top_k=1 应限制返回 1 条，实际 {}",
        result.hits.len()
    );
}
