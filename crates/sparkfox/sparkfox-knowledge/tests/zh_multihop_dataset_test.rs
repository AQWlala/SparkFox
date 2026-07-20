//! Sub-Step 12.3.1 — 中文多跳 Benchmark 数据集完整性测试（TDD-RED → GREEN）
//!
//! ## 测试目标（spec §三 12.3.1，6 测试）
//! 1. `test_entities_json_has_200_entries`：entities.json 含 200 个实体
//! 2. `test_events_json_has_500_entries`：events.json 含 500 个事件
//! 3. `test_relations_json_has_1500_entries`：relations.json 含 1500 个关系
//! 4. `test_queries_json_has_50_entries`：queries.json 含 50 个查询
//! 5. `test_queries_hop_distribution`：查询跳数分布正确（15 单跳 + 20 双跳 + 15 三跳）
//! 6. `test_all_query_entities_exist_in_entities`：所有查询的 query_entities 在 entities.json 中存在
//!
//! ## 数据集
//! - 文件位置：`tests/fixtures/zh_multihop/{entities,events,relations,queries}.json`
//! - 数据规模：200 实体 + 500 事件 + 1500 关系 + 50 查询
//! - 实体类型：11 类（PERSON/LOCATION/ORGANIZATION/TIME/EVENT/CONCEPT/ARTIFACT/SOFTWARE/HARDWARE/DOCUMENT/OTHER）
//! - 查询跳数：15 单跳 + 20 双跳 + 15 三跳
//! - 主题：中国科技场景（互联网公司 / 技术概念 / 产品发布 / 创始人动向）
//!
//! ## 用途
//! - 12.3.1 本文件：数据集完整性验证（6 测试）
//! - 12.3.2 后续：4 策略对比测试（MULTI1 / MULTI2 / MULTI3 / MULTI_LLM）
//! - 12.3.3 后续：Recall@10 > 0.85 调优
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use serde::Deserialize;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// 类型定义：对应 tests/fixtures/zh_multihop/*.json 的结构
// ---------------------------------------------------------------------------

/// 实体（entities.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Entity {
    id: String,
    name: String,
    #[allow(dead_code)]
    normalized_name: String,
    #[allow(dead_code)]
    entity_type_id: u32,
    #[allow(dead_code)]
    entity_type: String,
    #[allow(dead_code)]
    description: String,
}

/// 事件（events.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Event {
    id: String,
    #[allow(dead_code)]
    content: String,
    #[allow(dead_code)]
    entities: Vec<String>,
    #[allow(dead_code)]
    hop: u32,
}

/// 关系（relations.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Relation {
    event_id: String,
    entity_id: String,
    #[allow(dead_code)]
    relation_type: String,
}

/// 查询 + ground truth（queries.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Query {
    query: String,
    expected_event_ids: Vec<String>,
    expected_hop: u32,
    query_entities: Vec<String>,
}

// ---------------------------------------------------------------------------
// 数据集加载：使用 include_str! 在编译时嵌入 JSON
// ---------------------------------------------------------------------------

/// 加载 200 实体数据集
fn load_entities() -> Vec<Entity> {
    let json = include_str!("fixtures/zh_multihop/entities.json");
    serde_json::from_str(json).expect("entities.json 解析失败")
}

/// 加载 500 事件数据集
fn load_events() -> Vec<Event> {
    let json = include_str!("fixtures/zh_multihop/events.json");
    serde_json::from_str(json).expect("events.json 解析失败")
}

/// 加载 1500 关系数据集
fn load_relations() -> Vec<Relation> {
    let json = include_str!("fixtures/zh_multihop/relations.json");
    serde_json::from_str(json).expect("relations.json 解析失败")
}

/// 加载 50 查询数据集
fn load_queries() -> Vec<Query> {
    let json = include_str!("fixtures/zh_multihop/queries.json");
    serde_json::from_str(json).expect("queries.json 解析失败")
}

// ---------------------------------------------------------------------------
// 测试 1：entities.json 含 200 个实体
// ---------------------------------------------------------------------------

/// 验证 entities.json 含 200 个实体，且 id 唯一、格式为 `ent-XXX`。
#[test]
fn test_entities_json_has_200_entries() {
    let entities = load_entities();
    assert_eq!(entities.len(), 200, "entities.json 应含 200 个实体，实际 {}", entities.len());

    // 验证 id 唯一性
    let ids: HashSet<&str> = entities.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids.len(), 200, "entities.json 的 id 应全部唯一，实际唯一 id 数 {}", ids.len());

    // 验证 id 格式 ent-001..ent-200
    for (i, ent) in entities.iter().enumerate() {
        let expected = format!("ent-{:03}", i + 1);
        assert_eq!(ent.id, expected, "第 {} 个实体 id 应为 {}，实际 {}", i + 1, expected, ent.id);
    }
}

// ---------------------------------------------------------------------------
// 测试 2：events.json 含 500 个事件
// ---------------------------------------------------------------------------

/// 验证 events.json 含 500 个事件，且 id 唯一、格式为 `evt-XXX`。
#[test]
fn test_events_json_has_500_entries() {
    let events = load_events();
    assert_eq!(events.len(), 500, "events.json 应含 500 个事件，实际 {}", events.len());

    // 验证 id 唯一性
    let ids: HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids.len(), 500, "events.json 的 id 应全部唯一，实际唯一 id 数 {}", ids.len());

    // 验证 id 格式 evt-001..evt-500
    for (i, evt) in events.iter().enumerate() {
        let expected = format!("evt-{:03}", i + 1);
        assert_eq!(evt.id, expected, "第 {} 个事件 id 应为 {}，实际 {}", i + 1, expected, evt.id);
    }
}

// ---------------------------------------------------------------------------
// 测试 3：relations.json 含 1500 个关系
// ---------------------------------------------------------------------------

/// 验证 relations.json 含 1500 个关系，且 event_id / entity_id 引用合法。
#[test]
fn test_relations_json_has_1500_entries() {
    let relations = load_relations();
    assert_eq!(relations.len(), 1500, "relations.json 应含 1500 个关系，实际 {}", relations.len());

    // 验证引用完整性：所有 event_id 在 events.json 中存在
    let events = load_events();
    let event_ids: HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();

    // 验证引用完整性：所有 entity_id 在 entities.json 中存在
    let entities = load_entities();
    let entity_ids: HashSet<&str> = entities.iter().map(|e| e.id.as_str()).collect();

    for (i, r) in relations.iter().enumerate() {
        assert!(event_ids.contains(r.event_id.as_str()),
            "relations.json 第 {} 条关系的 event_id '{}' 不在 events.json 中", i + 1, r.event_id);
        assert!(entity_ids.contains(r.entity_id.as_str()),
            "relations.json 第 {} 条关系的 entity_id '{}' 不在 entities.json 中", i + 1, r.entity_id);
    }
}

// ---------------------------------------------------------------------------
// 测试 4：queries.json 含 50 个查询
// ---------------------------------------------------------------------------

/// 验证 queries.json 含 50 个查询，且 expected_event_ids 引用合法。
#[test]
fn test_queries_json_has_50_entries() {
    let queries = load_queries();
    assert_eq!(queries.len(), 50, "queries.json 应含 50 个查询，实际 {}", queries.len());

    // 验证 expected_event_ids 引用合法性
    let events = load_events();
    let event_ids: HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();

    for (i, q) in queries.iter().enumerate() {
        // 每个查询至少有 1 个 expected_event_id（ground truth 非空）
        assert!(!q.expected_event_ids.is_empty(),
            "queries.json 第 {} 个查询 '{}' 的 expected_event_ids 不应为空", i + 1, q.query);
        // 所有 expected_event_ids 都应在 events.json 中存在
        for eid in &q.expected_event_ids {
            assert!(event_ids.contains(eid.as_str()),
                "queries.json 第 {} 个查询 '{}' 的 expected_event_id '{}' 不在 events.json 中",
                i + 1, q.query, eid);
        }
        // 每个查询至少有 1 个 query_entities
        assert!(!q.query_entities.is_empty(),
            "queries.json 第 {} 个查询 '{}' 的 query_entities 不应为空", i + 1, q.query);
    }
}

// ---------------------------------------------------------------------------
// 测试 5：查询跳数分布正确（15 单跳 + 20 双跳 + 15 三跳）
// ---------------------------------------------------------------------------

/// 验证查询跳数分布精确为 15/20/15。
#[test]
fn test_queries_hop_distribution() {
    let queries = load_queries();

    let hop1_count = queries.iter().filter(|q| q.expected_hop == 1).count();
    let hop2_count = queries.iter().filter(|q| q.expected_hop == 2).count();
    let hop3_count = queries.iter().filter(|q| q.expected_hop == 3).count();

    assert_eq!(hop1_count, 15, "hop=1 单跳查询应为 15 个，实际 {}", hop1_count);
    assert_eq!(hop2_count, 20, "hop=2 双跳查询应为 20 个，实际 {}", hop2_count);
    assert_eq!(hop3_count, 15, "hop=3 三跳查询应为 15 个，实际 {}", hop3_count);

    // 跳数值合法性：必须为 1/2/3 之一
    for (i, q) in queries.iter().enumerate() {
        assert!(matches!(q.expected_hop, 1 | 2 | 3),
            "queries.json 第 {} 个查询的 expected_hop 应为 1/2/3，实际 {}", i + 1, q.expected_hop);
    }
}

// ---------------------------------------------------------------------------
// 测试 6：所有查询的 query_entities 在 entities.json 中存在
// ---------------------------------------------------------------------------

/// 验证所有查询的 `query_entities` 中的实体名都能在 `entities.json` 中找到。
///
/// 这是 12.3.2 / 12.3.3 评估 Recall@10 的前提：查询实体必须能在数据集中定位，
/// 否则多跳 BFS 无法启动，Recall 评估无意义。
#[test]
fn test_all_query_entities_exist_in_entities() {
    let entities = load_entities();
    let queries = load_queries();

    // 构建实体名 → 实体 ID 的映射（按 name 索引）
    let name_to_id: std::collections::HashMap<&str, &str> = entities
        .iter()
        .map(|e| (e.name.as_str(), e.id.as_str()))
        .collect();

    for (i, q) in queries.iter().enumerate() {
        for ent_name in &q.query_entities {
            assert!(name_to_id.contains_key(ent_name.as_str()),
                "queries.json 第 {} 个查询 '{}' 的 query_entity '{}' 不在 entities.json 中",
                i + 1, q.query, ent_name);
        }
    }
}
