//! Sub-Step 10.7.1 — SearchHit 多跳元数据扩展（U-02，spec §三 10.9.1，矩阵 10.7.1）
//!
//! ## 测试目标
//! 1. `SearchHit.hop` 字段类型为 `Option<u8>`（U-02 修复：从 `Option<usize>` 收紧为 `Option<u8>`）
//! 2. `SearchHit.via_entities` 字段类型为 `Vec<EntityRef>`（U-02 修复：从 `Vec<String>` 扩展为
//!    结构化 `EntityRef` 列表，含 `entity_id` / `entity_type` / `name`，支持 MULTI 多跳追溯）
//! 3. `SearchHit.chunk_span` 字段为 `Option<(usize, usize)>`（U-02 新增：未来 MULTI / VECTOR
//!    策略可填充 chunk 内的 (start, end) 位置区间，ATOMIC 检索暂为 `None`）
//! 4. ATOMIC 检索结果携带 `hop=Some(1)` + `via_entities` 非空（验证填充逻辑）
//! 5. `SearchHit` 可被 serde 序列化为 JSON，且 JSON 含 `hop` / `via_entities` / `chunk_span` 字段
//!
//! ## U-02 背景
//! v1.1.0 引入 MULTI 多跳检索策略后，原 `via_entities: Vec<String>`（仅 entity_id）不足以表达
//! "实体是什么类型 / 叫什么名字"，导致调用方需二次查询 `entity` / `entity_type` 表才能渲染路径。
//! U-02 修复扩展为 `Vec<EntityRef>`，把 entity_id / entity_type / name 一次性带出。
//!
//! ## TDD 三阶段
//! - RED：本文件先写 6 个失败测试（`chunk_span` 字段尚未存在 / 类型不匹配）
//! - GREEN：扩展 `SearchHit` + 修改 `AtomicStrategy::find_events` 填充新字段
//! - REFACTOR：提取 `EntityRef` 到 `search/types.rs`，添加文档注释

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{AtomicStrategy, EntityRef, SearchHit, SearchStrategy};

// ---------------------------------------------------------------------------
// Fixture：构造最小可复现的 SAG 数据库
// ---------------------------------------------------------------------------

/// 构造含 2 个 entity + 2 个 event + 3 个 relation 的内存数据库
///
/// - ent-1: 张三（PERSON）
/// - ent-2: 北京（LOCATION）
/// - evt-1: 张三出差（关联 ent-1 + ent-2）
/// - evt-2: 北京天气（关联 ent-2）
fn setup_db_with_data() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

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

// ---------------------------------------------------------------------------
// 测试 1：SearchHit.hop 字段类型为 Option<u8>
// ---------------------------------------------------------------------------

/// 验证 `SearchHit.hop` 字段类型为 `Option<u8>`（U-02 修复：从 `Option<usize>` 收紧）。
///
/// ## 验证点
/// - 字段存在（编译通过）
/// - 类型为 `Option<u8>`（赋值 `Some(1u8)` 通过，`Some(1usize)` 失败）
#[test]
fn test_search_hit_has_hop_field() {
    let hit = SearchHit {
        event_id: "evt-1".to_string(),
        title: "测试事件".to_string(),
        summary: "测试摘要".to_string(),
        chunk_id: None,
        score: 1.0,
        hop: Some(1u8), // U-02: u8 类型（max 255 跳已足够）
        via_entities: vec![],
        chunk_span: None,
    };
    // 类型断言：编译时若 hop 不是 Option<u8>，下面这行会失败
    let _hop: Option<u8> = hit.hop;
    assert_eq!(hit.hop, Some(1u8));
}

// ---------------------------------------------------------------------------
// 测试 2：SearchHit.via_entities 字段类型为 Vec<EntityRef>
// ---------------------------------------------------------------------------

/// 验证 `SearchHit.via_entities` 字段类型为 `Vec<EntityRef>`（U-02 修复）。
///
/// ## 验证点
/// - 字段存在（编译通过）
/// - 类型为 `Vec<EntityRef>`（可放入 `EntityRef` 结构体，不能放入 `String`）
/// - `EntityRef` 含 `entity_id` / `entity_type` / `name` 三个字段
#[test]
fn test_search_hit_has_via_entities_field() {
    let entity_ref = EntityRef {
        entity_id: "ent-1".to_string(),
        entity_type: "PERSON".to_string(),
        name: "张三".to_string(),
    };
    let hit = SearchHit {
        event_id: "evt-1".to_string(),
        title: "测试事件".to_string(),
        summary: "测试摘要".to_string(),
        chunk_id: None,
        score: 1.0,
        hop: Some(1u8),
        via_entities: vec![entity_ref.clone()],
        chunk_span: None,
    };
    // 类型断言：编译时若 via_entities 不是 Vec<EntityRef>，下面这行会失败
    let _via: &Vec<EntityRef> = &hit.via_entities;
    assert_eq!(hit.via_entities.len(), 1);
    assert_eq!(hit.via_entities[0].entity_id, "ent-1");
    assert_eq!(hit.via_entities[0].entity_type, "PERSON");
    assert_eq!(hit.via_entities[0].name, "张三");
    // PartialEq 验证（EntityRef 应派生 PartialEq）
    assert_eq!(&hit.via_entities[0], &entity_ref);
}

// ---------------------------------------------------------------------------
// 测试 3：SearchHit.chunk_span 字段为 Option<(usize, usize)>
// ---------------------------------------------------------------------------

/// 验证 `SearchHit.chunk_span` 字段为 `Option<(usize, usize)>`（U-02 新增）。
///
/// ## 验证点
/// - 字段存在（编译通过）
/// - 类型为 `Option<(usize, usize)>`（可放入 `(0, 100)` 元组）
/// - 默认 `None`（ATOMIC 检索不填充 chunk 位置）
#[test]
fn test_search_hit_has_chunk_span_field() {
    let hit = SearchHit {
        event_id: "evt-1".to_string(),
        title: "测试事件".to_string(),
        summary: "测试摘要".to_string(),
        chunk_id: None,
        score: 1.0,
        hop: Some(1u8),
        via_entities: vec![],
        chunk_span: Some((0, 100)),
    };
    // 类型断言：编译时若 chunk_span 不是 Option<(usize, usize)>，下面这行会失败
    let _span: Option<(usize, usize)> = hit.chunk_span;
    assert_eq!(hit.chunk_span, Some((0, 100)));

    // 默认 None 场景
    let hit_none = SearchHit {
        event_id: "evt-2".to_string(),
        title: "测试事件2".to_string(),
        summary: "测试摘要2".to_string(),
        chunk_id: None,
        score: 1.0,
        hop: None,
        via_entities: vec![],
        chunk_span: None,
    };
    assert_eq!(hit_none.chunk_span, None);
}

// ---------------------------------------------------------------------------
// 测试 4：ATOMIC 检索结果携带 hop=1 + via_entities 非空
// ---------------------------------------------------------------------------

/// 验证 `AtomicStrategy::search` 返回的 `SearchHit` 携带 `hop=Some(1)` + `via_entities` 非空。
///
/// ## 验证点
/// - hits 非空
/// - 每个 hit.hop == Some(1)（ATOMIC 是单跳检索）
/// - 每个 hit.via_entities 非空（含至少 1 个 EntityRef）
/// - EntityRef 含正确的 entity_id / entity_type / name（JOIN entity + entity_type 表的结果）
/// - chunk_span == None（ATOMIC 不填充 chunk 位置）
#[tokio::test]
async fn test_atomic_search_populates_hop_via_entities() {
    let conn = setup_db_with_data();
    let strategy = AtomicStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    assert!(!result.hits.is_empty(), "hits 不应为空");

    let hit = &result.hits[0];
    // ATOMIC 是单跳
    assert_eq!(hit.hop, Some(1u8), "ATOMIC 检索 hop 应为 Some(1)");
    // via_entities 非空
    assert!(
        !hit.via_entities.is_empty(),
        "ATOMIC 检索 via_entities 不应为空"
    );
    // 验证 EntityRef 字段填充正确（evt-1 关联 ent-1 张三 PERSON）
    let has_zhangsan = hit
        .via_entities
        .iter()
        .any(|e| e.entity_id == "ent-1" && e.entity_type == "PERSON" && e.name == "张三");
    assert!(
        has_zhangsan,
        "via_entities 应含 EntityRef {{ ent-1, PERSON, 张三 }}，实际: {:?}",
        hit.via_entities
    );
    // ATOMIC 不填充 chunk_span
    assert_eq!(hit.chunk_span, None, "ATOMIC 检索 chunk_span 应为 None");
}

// ---------------------------------------------------------------------------
// 测试 5：SearchHit 可被 serde 序列化为 JSON（含新字段）
// ---------------------------------------------------------------------------

/// 验证 `SearchHit` 可被 serde 序列化为 JSON，且 JSON 含 `hop` / `via_entities` / `chunk_span` 字段。
///
/// ## 验证点
/// - 序列化成功（`SearchHit` 派生 `serde::Serialize`）
/// - JSON 字符串含 `"hop"` / `"via_entities"` / `"chunk_span"` 字段名
/// - 反序列化回 `SearchHit` 与原对象相等（`PartialEq`）
#[test]
fn test_search_hit_serializes_to_json() {
    let hit = SearchHit {
        event_id: "evt-1".to_string(),
        title: "测试事件".to_string(),
        summary: "测试摘要".to_string(),
        chunk_id: None,
        score: 1.0,
        hop: Some(1u8),
        via_entities: vec![EntityRef {
            entity_id: "ent-1".to_string(),
            entity_type: "PERSON".to_string(),
            name: "张三".to_string(),
        }],
        chunk_span: Some((0, 100)),
    };

    // 序列化
    let json = serde_json::to_string(&hit).expect("SearchHit 应可序列化为 JSON");
    println!("[serializes_to_json] JSON: {}", json);

    // 验证 JSON 含新字段名
    assert!(json.contains("\"hop\""), "JSON 应含 hop 字段: {}", json);
    assert!(
        json.contains("\"via_entities\""),
        "JSON 应含 via_entities 字段: {}",
        json
    );
    assert!(
        json.contains("\"chunk_span\""),
        "JSON 应含 chunk_span 字段: {}",
        json
    );
    // 验证 via_entities 内含 entity_id / entity_type / name
    assert!(json.contains("\"entity_id\""), "JSON 应含 entity_id: {}", json);
    assert!(
        json.contains("\"entity_type\""),
        "JSON 应含 entity_type: {}",
        json
    );
    assert!(json.contains("\"name\""), "JSON 应含 name: {}", json);

    // 反序列化回 SearchHit，验证字段值一致
    let deserialized: SearchHit =
        serde_json::from_str(&json).expect("JSON 应可反序列化为 SearchHit");
    assert_eq!(hit, deserialized, "往返序列化应保持相等");
}

// ---------------------------------------------------------------------------
// 测试 6：EntityRef 类型派生必要 trait（Debug / Clone / Serialize / Deserialize / PartialEq）
// ---------------------------------------------------------------------------

/// 验证 `EntityRef` 派生了 `Debug` / `Clone` / `Serialize` / `Deserialize` / `PartialEq` trait。
///
/// ## 验证点
/// - `Clone`：可 clone
/// - `Debug`：可用 `{:?}` 格式化
/// - `PartialEq`：可比较相等
/// - `Serialize` / `Deserialize`：可往返 JSON
#[test]
fn test_entity_ref_derives_traits() {
    let e1 = EntityRef {
        entity_id: "ent-1".to_string(),
        entity_type: "PERSON".to_string(),
        name: "张三".to_string(),
    };
    // Clone
    let e2 = e1.clone();
    // PartialEq
    assert_eq!(e1, e2);
    // Debug
    let debug_str = format!("{:?}", e1);
    assert!(debug_str.contains("ent-1"), "Debug 输出应含 entity_id: {}", debug_str);
    // Serialize / Deserialize
    let json = serde_json::to_string(&e1).expect("EntityRef 应可序列化");
    let e3: EntityRef = serde_json::from_str(&json).expect("EntityRef 应可反序列化");
    assert_eq!(e1, e3);
}
