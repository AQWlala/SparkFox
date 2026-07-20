//! Sub-Step 12.1.2 — MULTI_ES 子图预筛选 + JOIN 优化测试（TDD-RED → GREEN → REFACTOR）
//!
//! spec §三 12.1.2 要求在 12.1.1 已实现的 [`MultiEsStrategy`] 基础上添加**子图预筛选**优化：
//! 1. 在 ES-first 实体检索后、事件检索前，先抽取所有命中的 entity_ids
//! 2. 用 `WHERE entity_id IN (?, ?, ...)` 参数绑定过滤 events（而非全表 JOIN）
//! 3. 添加 `count_join_rows` 辅助函数统计 JOIN 行数（用于测试断言）
//! 4. 保持 Recall@5 不变（预筛选只过滤 events，不影响 entity 检索）
//!
//! ## 测试目标（4 测试，spec §三 12.1.2）
//! 1. `test_subgraph_prefilter_reduces_join_rows`：子图预筛选减少 JOIN 行数
//!    - 对比有/无预筛选的 JOIN 行数（用小图 fixture）
//!    - 断言：with_prefilter < without_prefilter
//! 2. `test_multi_es_join_rows_below_threshold`：MULTI_ES JOIN 行数 < MULTI
//!    - 用 zh_multihop 数据集（200 实体 + 500 事件）
//!    - 断言：MULTI_ES 的 JOIN 行数严格小于 MULTI（用无预筛选的 MULTI_ES 等效 MULTI 行为）
//! 3. `test_multi_es_uses_subgraph_ids_filter`：用子图 entity_ids 过滤 events
//!    - 断言：子图预筛选 SQL 含 `IN (` 参数绑定
//!    - 断言：实际查询返回的 events 含 DISTINCT 去重（同一 event 不重复）
//! 4. `test_multi_es_preserves_recall_at_5`：预筛选不损失 Recall@5
//!    - 用 zh_multihop 数据集（50 查询）
//!    - 断言：MULTI_ES 有/无预筛选的 Recall@5 差值 < 0.05
//!
//! ## 测试 fixture
//! - **小图**：与 `multi_es_strategy_test.rs::setup_multi_hop_db` 一致（4 entity + 3 event）
//! - **zh_multihop 数据集**：`tests/fixtures/zh_multihop/{entities,events,relations,queries}.json`
//!   - 200 实体 + 500 事件 + 1500 关系 + 50 查询（固定种子 20260721）
//!
//! ## 关键设计
//! - MULTI_ES 默认开启子图预筛选（`with_subgraph_prefilter(true)`）
//! - `with_subgraph_prefilter(false)` 可关闭预筛选（用于 Recall@5 对比测试）
//! - `last_join_rows()` 返回上次 `search` 调用产生的 JOIN 行数（测试断言入口）
//! - MULTI 的 JOIN 行数 = MULTI_ES 关闭预筛选时的 JOIN 行数（BFS 算法相同）
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;
use serde::Deserialize;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{MultiEsStrategy, SearchStrategy};

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
    /// zh_multihop 数据集中为 0-10 的整数（0=PERSON, 1=LOCATION, ...）
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
    /// 事件内容（用作 title/summary/content 三合一）
    content: String,
    /// 关联实体 ID 列表（从 events.json 派生关系）
    entities: Vec<String>,
    #[allow(dead_code)]
    hop: u32,
}

/// 查询 + ground truth（queries.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Query {
    query: String,
    expected_event_ids: Vec<String>,
    #[allow(dead_code)]
    expected_hop: u32,
    query_entities: Vec<String>,
}

// ---------------------------------------------------------------------------
// 数据集加载：使用 include_str! 在编译时嵌入 JSON
// ---------------------------------------------------------------------------

fn load_entities() -> Vec<Entity> {
    let json = include_str!("fixtures/zh_multihop/entities.json");
    serde_json::from_str(json).expect("entities.json 解析失败")
}

fn load_events() -> Vec<Event> {
    let json = include_str!("fixtures/zh_multihop/events.json");
    serde_json::from_str(json).expect("events.json 解析失败")
}

fn load_queries() -> Vec<Query> {
    let json = include_str!("fixtures/zh_multihop/queries.json");
    serde_json::from_str(json).expect("queries.json 解析失败")
}

/// zh_multihop entity_type_id (0-10) 映射到 schema.rs 中的 default entity_type.id
///
/// zh_multihop 数据集的 entity_type_id 与 schema.rs::ENTITY_TYPES 的对应关系：
/// - 0=PERSON → default_person
/// - 1=LOCATION → default_location
/// - 2=ORGANIZATION → default_organization
/// - 3=TIME → default_time
/// - 4=EVENT → default_event
/// - 5=CONCEPT → default_concept
/// - 6=ARTIFACT → default_object（物品，schema 无 ARTIFACT 类型，映射到 default_object）
/// - 7=SOFTWARE → default_other（schema 无 SOFTWARE 类型，映射到 default_other 兜底）
/// - 8=HARDWARE → default_other
/// - 9=DOCUMENT → default_other
/// - 10=OTHER → default_other
fn entity_type_id_to_str(id: u32) -> &'static str {
    match id {
        0 => "default_person",
        1 => "default_location",
        2 => "default_organization",
        3 => "default_time",
        4 => "default_event",
        5 => "default_concept",
        6 => "default_object",
        7 => "default_other",
        8 => "default_other",
        9 => "default_other",
        10 => "default_other",
        _ => "default_other",
    }
}

// ---------------------------------------------------------------------------
// Fixture 1：小图（与 multi_es_strategy_test.rs::setup_multi_hop_db 一致）
// ---------------------------------------------------------------------------

/// 构造 BFS 3 跳测试图（与 multi_es_strategy_test.rs 一致）
///
/// ```text
/// 张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四
/// ```
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

// ---------------------------------------------------------------------------
// Fixture 2：zh_multihop 数据集（200 实体 + 500 事件 + 1500 关系）
// ---------------------------------------------------------------------------

/// 构造 zh_multihop 数据集 DB（200 entity + 500 event + ~1500 relation）
///
/// ## 数据来源
/// - `tests/fixtures/zh_multihop/entities.json`：200 实体（11 类）
/// - `tests/fixtures/zh_multihop/events.json`：500 事件（含 entities 引用）
/// - relations 从 events.json 的 `entities` 字段派生（(event_id, entity_id) 对）
///
/// ## entity_type 映射
/// zh_multihop 数据集的 `entity_type_id` 为 0-10 的整数，
/// 通过 [`entity_type_id_to_str`] 映射到 schema.rs 的 default entity_type.id（如 "default_person"）。
fn setup_zh_multihop_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 200 个 entity
    let entities = load_entities();
    for ent in &entities {
        let type_id = entity_type_id_to_str(ent.entity_type_id);
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![ent.id, type_id, ent.name, ent.name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 500 个 event
    let events = load_events();
    for (i, evt) in events.iter().enumerate() {
        // title 取前 20 字符（截断处理）
        let title: String = evt.content.chars().take(20).collect();
        // created_time 按分钟递增，确保稳定排序
        let total_minutes = i;
        let minute = total_minutes % 60;
        let hour = (total_minutes / 60) % 24;
        let day = (total_minutes / (60 * 24)) + 20;
        let created = format!("2026-07-{:02}T{:02}:{:02}:00Z", day, hour, minute);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![evt.id, "kb-1", "doc-1", &title, &evt.content, &evt.content, &created, &created],
        ).unwrap();
    }

    // ~1500 条 event_entity_relation（从 events.json 的 entities 字段派生）
    let mut rel_idx: u32 = 0;
    for evt in &events {
        for eid in &evt.entities {
            let rel_id = format!("rel-{:04}", rel_idx);
            conn.execute(
                "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&rel_id, &evt.id, eid, "2026-07-20T00:00:00Z"],
            ).unwrap();
            rel_idx += 1;
        }
    }

    conn
}

// ---------------------------------------------------------------------------
// 辅助函数：Recall@k 计算
// ---------------------------------------------------------------------------

/// 计算 Recall@k：top_k 命中中含多少 ground truth 事件 / ground truth 总数
///
/// Recall@k = |top_k ∩ expected| / |expected|
///
/// 当 expected 为空时返回 0.0（避免除零）。
fn recall_at_k(top_k_hits: &[String], expected: &[String]) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let top_set: std::collections::HashSet<&str> =
        top_k_hits.iter().map(|s| s.as_str()).collect();
    let expected_set: std::collections::HashSet<&str> =
        expected.iter().map(|s| s.as_str()).collect();
    let intersection = top_set.intersection(&expected_set).count();
    intersection as f64 / expected_set.len() as f64
}

// ---------------------------------------------------------------------------
// 测试 1：子图预筛选减少 JOIN 行数（小图 fixture）
// ---------------------------------------------------------------------------

/// 验收指标 1：子图预筛选减少 JOIN 行数
///
/// 在小图上对比有/无预筛选的 JOIN 行数：
/// - **无预筛选**：BFS 逐个 entity 调用 `find_events_by_entity`（不去重）
///   - 张三 (ent-A) → 1 event (evt-1)
///   - 北京 (ent-B) → 2 events (evt-1, evt-2) — evt-1 重复
///   - 腾讯 (ent-C) → 2 events (evt-2, evt-3) — evt-2 重复
///   - 总计：5 JOIN 行（含重复）
/// - **有预筛选**：用 `WHERE entity_id IN (...)` 一次查询 + DISTINCT 去重
///   - 子图 entities = {ent-A, ent-B, ent-C, ent-D}
///   - SELECT DISTINCT event_id WHERE entity_id IN (...) → 3 unique events
///   - 总计：3 JOIN 行（无重复）
///
/// 断言：`with_prefilter < without_prefilter` 且 `with_prefilter > 0`
#[tokio::test]
async fn test_subgraph_prefilter_reduces_join_rows() {
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);

    // seed = 张三（ent-A）— BFS 3 跳扩展会访问 ent-A / ent-B / ent-C
    let seed = vec!["ent-A".to_string()];

    let without_prefilter = strategy
        .count_join_rows_without_prefilter(&seed)
        .expect("count_join_rows_without_prefilter 应成功");

    let with_prefilter = strategy
        .count_join_rows_with_prefilter(&seed)
        .expect("count_join_rows_with_prefilter 应成功");

    assert!(
        with_prefilter < without_prefilter,
        "子图预筛选应减少 JOIN 行数: with_prefilter={} 应 < without_prefilter={}",
        with_prefilter,
        without_prefilter
    );
    assert!(
        with_prefilter > 0,
        "预筛选后应仍有 JOIN 行数 > 0，实际: {}",
        with_prefilter
    );
}

// ---------------------------------------------------------------------------
// 测试 2：MULTI_ES JOIN 行数 < MULTI（zh_multihop 数据集）
// ---------------------------------------------------------------------------

/// 验收指标 2：MULTI_ES JOIN 行数严格小于 MULTI
///
/// 用 zh_multihop 数据集（200 实体 + 500 事件），用第一个查询
/// （query_entities=["张三"]）做种子，对比 MULTI_ES（有预筛选）与 MULTI（无预筛选）的 JOIN 行数。
///
/// 由于 MULTI 与 MULTI_ES（关闭预筛选）的 BFS 算法相同，二者 JOIN 行数一致。
/// 因此 MULTI_ES 的 JOIN 行数（有预筛选）应严格小于 MULTI（无预筛选）。
///
/// 断言：`multi_es_join_rows < multi_join_rows`
#[tokio::test]
async fn test_multi_es_join_rows_below_threshold() {
    let conn = setup_zh_multihop_db();
    let strategy = MultiEsStrategy::new(conn);

    // 用第一个查询的 query_entities 作为种子（zh_multihop 第一个查询的 query_entities=["张三"]）
    let queries = load_queries();
    let q = &queries[0];
    let entity_name = &q.query_entities[0];

    // 通过 ES-first 找到 entity_ids（与 MULTI_ES 的 search() Step1 一致）
    let entity_ids = strategy
        .find_entity_ids_by_query(entity_name)
        .expect("find_entity_ids_by_query 应成功");
    assert!(
        !entity_ids.is_empty(),
        "zh_multihop 数据集中应能找到 entity「{}」，实际: {:?}",
        entity_name,
        entity_ids
    );

    // MULTI 等效 JOIN 行数（无预筛选，逐个 entity 查询）
    let multi_join_rows = strategy
        .count_join_rows_without_prefilter(&entity_ids)
        .expect("count_join_rows_without_prefilter 应成功");

    // MULTI_ES JOIN 行数（有预筛选，IN 子句 + DISTINCT 去重）
    let multi_es_join_rows = strategy
        .count_join_rows_with_prefilter(&entity_ids)
        .expect("count_join_rows_with_prefilter 应成功");

    assert!(
        multi_es_join_rows < multi_join_rows,
        "MULTI_ES JOIN 行数 ({}) 应严格小于 MULTI ({})",
        multi_es_join_rows,
        multi_join_rows
    );
}

// ---------------------------------------------------------------------------
// 测试 3：MULTI_ES 用子图 entity_ids 过滤 events（含 IN 参数绑定）
// ---------------------------------------------------------------------------

/// 验收指标 3：MULTI_ES 用子图 entity_ids 过滤 events
///
/// 验证子图预筛选 SQL 含 `IN (` 参数绑定（防注入）：
/// - SQL 模板含 `IN ({placeholders})` 占位符
/// - 占位符替换后含 `IN (?, ?)` 形式
/// - 实际查询返回的 events 含 DISTINCT 去重（同一 event 不重复）
///
/// 断言：
/// - `subgraph_filter_sql_template()` 含 `IN (`
/// - `subgraph_filter_sql_template()` 含 `?` 占位符
/// - `find_events_by_subgraph_entities([ent-A, ent-B])` 返回 evt-1 仅 1 次（去重）
#[tokio::test]
async fn test_multi_es_uses_subgraph_ids_filter() {
    let conn = setup_multi_hop_db();
    let strategy = MultiEsStrategy::new(conn);

    // 验证 SQL 模板含 IN ( 参数绑定
    let sql = strategy.subgraph_filter_sql_template();
    assert!(
        sql.contains("IN ("),
        "子图预筛选 SQL 应含 'IN (' 参数绑定，实际: {}",
        sql
    );
    assert!(
        sql.contains("?"),
        "子图预筛选 SQL 应含 ? 占位符（参数绑定防注入），实际: {}",
        sql
    );
    assert!(
        sql.contains("entity_id IN"),
        "子图预筛选 SQL 应在 entity_id 字段上 IN 过滤，实际: {}",
        sql
    );

    // 实际调用 find_events_by_subgraph_entities 验证返回结果
    // ent-A (张三) 关联 evt-1
    // ent-B (北京) 关联 evt-1, evt-2
    // 子图 [ent-A, ent-B] 的 events 应为 [evt-1, evt-2]（DISTINCT 去重，evt-1 仅 1 次）
    let seed = vec!["ent-A".to_string(), "ent-B".to_string()];
    let events = strategy
        .find_events_by_subgraph_entities(&seed)
        .expect("find_events_by_subgraph_entities 应成功");

    assert!(
        !events.is_empty(),
        "子图预筛选应返回 events，实际: {:?}",
        events
    );
    // evt-1 同时关联 ent-A 和 ent-B，DISTINCT 去重后应只出现 1 次
    let evt1_count = events.iter().filter(|e| *e == "evt-1").count();
    assert_eq!(
        evt1_count, 1,
        "evt-1 应只出现 1 次（DISTINCT 去重），实际: {}（events={:?}）",
        evt1_count, events
    );
}

// ---------------------------------------------------------------------------
// 测试 4：预筛选不损失 Recall@5（zh_multihop 数据集）
// ---------------------------------------------------------------------------

/// 验收指标 4：预筛选不损失 Recall@5
///
/// 在 zh_multihop 数据集（50 查询）上对比 MULTI_ES 有/无预筛选的 Recall@5：
/// - **有预筛选**：`MultiEsStrategy::new(conn)` 默认开启子图预筛选
/// - **无预筛选**：`MultiEsStrategy::new(conn).with_subgraph_prefilter(false)` 关闭预筛选
///
/// 子图预筛选只过滤 events，不影响 entity 检索（ES-first Step1 不变），
/// 因此 Recall@5 应保持不变（差值 < 0.05）。
///
/// 断言：`|recall_with_prefilter - recall_without_prefilter| < 0.05`
#[tokio::test]
async fn test_multi_es_preserves_recall_at_5() {
    let queries = load_queries();

    // 有预筛选（默认）
    let conn_with = setup_zh_multihop_db();
    let strategy_with = MultiEsStrategy::new(conn_with);
    let mut recall_sum_with = 0.0;

    // 无预筛选
    let conn_without = setup_zh_multihop_db();
    let strategy_without = MultiEsStrategy::new(conn_without).with_subgraph_prefilter(false);
    let mut recall_sum_without = 0.0;

    for q in &queries {
        // 有预筛选
        let result_with = strategy_with.search(&q.query).await.expect("search 应成功");
        let top5_with: Vec<String> = result_with
            .hits
            .iter()
            .take(5)
            .map(|h| h.event_id.clone())
            .collect();
        recall_sum_with += recall_at_k(&top5_with, &q.expected_event_ids);

        // 无预筛选
        let result_without = strategy_without
            .search(&q.query)
            .await
            .expect("search 应成功");
        let top5_without: Vec<String> = result_without
            .hits
            .iter()
            .take(5)
            .map(|h| h.event_id.clone())
            .collect();
        recall_sum_without += recall_at_k(&top5_without, &q.expected_event_ids);
    }

    let recall_with = recall_sum_with / queries.len() as f64;
    let recall_without = recall_sum_without / queries.len() as f64;
    let diff = (recall_with - recall_without).abs();

    assert!(
        diff < 0.05,
        "预筛选不应损失 Recall@5：差值 {} 应 < 0.05（with={}, without={}）",
        diff,
        recall_with,
        recall_without
    );
}
