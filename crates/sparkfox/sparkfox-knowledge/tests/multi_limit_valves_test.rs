//! Sub-Step 11.2.4 — R-07 三道 LIMIT 阀门（防 graph explosion，TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.2.4 / RISK-SAG-07 / R-07）
//! 1. `test_max_hop_3_truncates_expansion`：max_hop=3 触发截断（4 跳深链 A→B→C→D→E）
//! 2. `test_max_intermediate_entities_100_truncates`：中间实体 > 100 触发截断
//! 3. `test_max_join_rows_10000_truncates`：JOIN 行数 > 10000 触发截断
//! 4. `test_truncated_result_includes_warning`：截断结果含 warning（thought_process / log::warn!）
//! 5. `test_three_valves_independent`：三道阀门独立触发（互不影响）
//!
//! ## 三道 LIMIT 阀门设计（RISK-SAG-07 / R-07）
//! - **阀门 1**：`MAX_HOP=3` — BFS 扩展深度上限（已有 max_hop 字段）
//! - **阀门 2**：`MAX_INTERMEDIATE_ENTITIES=100` — 中间实体数量上限
//! - **阀门 3**：`MAX_JOIN_ROWS=10000` — JOIN 行数上限（event_entity_relation 查询返回行数）
//!
//! 任一阀门触发时：截断扩展 + 记录 warning + 返回已收集的 hits（部分结果）
//!
//! ## Warning 字段方案
//! 不修改 `SearchResult` 结构体（避免回归）。将 warning 记录到：
//! - `MultiState.thought_process`（内部，8 步流程记录）
//! - `log::warn!`（日志输出）
//! - `MultiStrategy::last_valve_warnings()`（测试访问入口，返回上次 search 的阀门警告列表）
//!
//! ## 测试 fixture
//! - **阀门 1**（4 跳深链）：5 entity / 4 event / 8 relation
//!   ```text
//!   张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四 ── evt-4 ── 王五
//!   ```
//! - **阀门 2**（101 中间实体）：1 hub entity + 101 entity + 101 event + 202 relation
//!   ```text
//!   hub(A) ── evt-i ── Bi  (i=0..100)
//!   ```
//! - **阀门 3**（10001 JOIN 行）：1 hub entity + 10001 event + 10001 relation
//!   ```text
//!   hub(A) ── evt-i  (i=0..10000)
//!   ```
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::{MAX_HOP, MAX_INTERMEDIATE_ENTITIES, MAX_JOIN_ROWS};
use sparkfox_knowledge::search::{MultiStrategy, SearchStrategy};

// ---------------------------------------------------------------------------
// Fixture 1：4 跳深链（阀门 1 — max_hop=3 截断）
// ---------------------------------------------------------------------------

/// 构造 4 跳深链测试图（验证阀门 1：max_hop=3 截断）：
///
/// ```text
/// 张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四 ── evt-4 ── 王五
/// ```
///
/// - 5 个 entity（张三 / 北京 / 腾讯 / 李四 / 王五）
/// - 4 个 event（evt-1 张三出差 / evt-2 北京天气 / evt-3 腾讯财报 / evt-4 李四入职）
/// - 8 条 event_entity_relation（每个 event 关联 2 个 entity）
///
/// 查询「张三」期望 BFS 扩展结果（max_hop=3）：
/// - hop=1：evt-1（张三直接关联）
/// - hop=2：evt-2（经北京扩展）
/// - hop=3：evt-3（经腾讯扩展）
/// - evt-4（hop=4）**不应**出现（受 max_hop=3 限制，阀门 1 截断）
fn setup_4hop_chain_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 5 个 entity（A=张三 / B=北京 / C=腾讯 / D=李四 / E=王五）
    let entities = [
        ("ent-A", "default_person", "张三"),
        ("ent-B", "default_location", "北京"),
        ("ent-C", "default_organization", "腾讯"),
        ("ent-D", "default_person", "李四"),
        ("ent-E", "default_person", "王五"),
    ];
    for (id, type_id, name) in &entities {
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, type_id, name, name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 4 个 event（按 created_time 递增，便于稳定排序）
    let events = [
        ("evt-1", "张三出差", "张三去北京出差", "张三昨天去北京出差", "2026-07-20T00:00:00Z"),
        ("evt-2", "北京天气", "北京今天晴朗", "北京今天天气晴朗", "2026-07-20T01:00:00Z"),
        ("evt-3", "腾讯财报", "腾讯发布财报", "腾讯今天发布财报", "2026-07-20T02:00:00Z"),
        ("evt-4", "李四入职", "李四入职新公司", "李四今天入职新公司", "2026-07-20T03:00:00Z"),
    ];
    for (id, title, summary, content, created) in &events {
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "kb-1", "doc-1", title, summary, content, created, created],
        ).unwrap();
    }

    // 8 条 event_entity_relation（链式：A-e1-B-e2-C-e3-D-e4-E）
    let relations = [
        ("rel-1", "evt-1", "ent-A"),
        ("rel-2", "evt-1", "ent-B"),
        ("rel-3", "evt-2", "ent-B"),
        ("rel-4", "evt-2", "ent-C"),
        ("rel-5", "evt-3", "ent-C"),
        ("rel-6", "evt-3", "ent-D"),
        ("rel-7", "evt-4", "ent-D"),
        ("rel-8", "evt-4", "ent-E"),
    ];
    for (id, event_id, entity_id) in &relations {
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![id, event_id, entity_id, "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    conn
}

// ---------------------------------------------------------------------------
// Fixture 2：101 中间实体（阀门 2 — max_intermediate_entities=100 截断）
// ---------------------------------------------------------------------------

/// 构造 101 中间实体测试图（验证阀门 2：max_intermediate_entities=100 截断）：
///
/// ```text
/// hub(A) ── evt-i ── Bi  (i=0..100，共 101 个中间实体)
/// ```
///
/// - 1 个 hub entity（A=张三，seed 实体）
/// - 101 个中间 entity（B0..B100）
/// - 101 个 event（evt-0..evt-100，每个 event 连接 A + Bi）
/// - 202 条 event_entity_relation（每个 event 关联 2 个 entity）
///
/// 查询「张三」期望 BFS 扩展行为：
/// - hop=0：A 入队
/// - hop=1：A 处理 → 发现 101 个 evt-i → 101 个 Bi 入队
/// - 逐个处理 Bi：visited_entities 增长，当 len >= 100 时阀门 2 触发，截断 BFS
/// - 阀门 2 触发后剩余 Bi 不再处理（截断扩展）
fn setup_101_intermediate_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 1 个 hub entity（A=张三）
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-A", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 101 个中间 entity（B0..B100）
    for i in 0..=100 {
        let id = format!("ent-B{}", i);
        let name = format!("中间实体{}", i);
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "default_person", name, name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 101 个 event（evt-0..evt-100）
    for i in 0..=100u32 {
        let id = format!("evt-{}", i);
        let title = format!("事件{}", i);
        let summary = format!("事件{}摘要", i);
        let content = format!("事件{}内容", i);
        let minute = (i % 60) as u32;
        let hour = (i / 60) % 24;
        let created = format!("2026-07-20T{:02}:{:02}:00Z", hour, minute);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "kb-1", "doc-1", title, summary, content, created, created],
        ).unwrap();
    }

    // 202 条 event_entity_relation（每个 evt-i 关联 A + Bi）
    for i in 0..=100u32 {
        let evt_id = format!("evt-{}", i);
        let b_id = format!("ent-B{}", i);
        let rel_a = format!("rel-{}-A", i);
        let rel_b = format!("rel-{}-B", i);
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rel_a, evt_id, "ent-A", "2026-07-20T00:00:00Z"],
        ).unwrap();
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rel_b, evt_id, b_id, "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    conn
}

// ---------------------------------------------------------------------------
// Fixture 3：10001 JOIN 行（阀门 3 — max_join_rows=10000 截断）
// ---------------------------------------------------------------------------

/// 构造 10001 JOIN 行测试图（验证阀门 3：max_join_rows=10000 截断）：
///
/// ```text
/// hub(A) ── evt-i  (i=0..10000，共 10001 个 event，10001 条 relation)
/// ```
///
/// - 1 个 hub entity（A=张三，seed 实体）
/// - 10001 个 event（evt-0..evt-10000）
/// - 10001 条 event_entity_relation（每条关联 evt-i + A）
///
/// 查询「张三」期望 BFS 扩展行为：
/// - hop=0：A 入队
/// - hop=1：A 处理 → find_events_by_entity(A) 返回 10001 行
/// - 阀门 3 触发：total_join_rows = 10001 > MAX_JOIN_ROWS(10000)
/// - 截断扩展：处理当前 10001 个 event（加入 results），但不再入队 next entities
fn setup_10001_join_rows_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 1 个 hub entity（A=张三）
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-A", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 10001 个 event（evt-0..evt-10000），使用事务批量插入以提升性能
    conn.execute_batch("BEGIN;").unwrap();
    for i in 0..=10000u32 {
        let id = format!("evt-{}", i);
        let title = format!("事件{}", i);
        let summary = format!("事件{}摘要", i);
        let content = format!("事件{}内容", i);
        // created_time 按分钟递增（i 最大 10000 → 166 小时 40 分钟，超过 1 天但仅用于排序）
        let minute = (i % 60) as u32;
        let hour = (i / 60) % 24;
        let day = (i / 1440) + 20;
        let created = format!("2026-07-{:02}T{:02}:{:02}:00Z", day, hour, minute);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "kb-1", "doc-1", title, summary, content, created, created],
        ).unwrap();
    }

    // 10001 条 event_entity_relation（每条关联 evt-i + A）
    for i in 0..=10000u32 {
        let evt_id = format!("evt-{}", i);
        let rel_id = format!("rel-{}", i);
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rel_id, evt_id, "ent-A", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }
    conn.execute_batch("COMMIT;").unwrap();

    conn
}

// ---------------------------------------------------------------------------
// 测试 1：max_hop=3 触发截断（阀门 1）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_max_hop_3_truncates_expansion() {
    // 验收指标 1：max_hop=3 触发截断
    //
    // 在 4 跳深链上查询「张三」（默认 max_hop=3）：
    // - hop=1：evt-1（张三直接关联）
    // - hop=2：evt-2（经北京扩展）
    // - hop=3：evt-3（经腾讯扩展）
    // - evt-4（hop=4）**不应**出现（受 max_hop=3 限制，阀门 1 截断）
    //
    // 断言：
    // 1. hits 含 evt-1 / evt-2 / evt-3
    // 2. hits 不含 evt-4
    // 3. 所有 hit 的 hop ≤ 3（= MAX_HOP）
    let conn = setup_4hop_chain_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "hop=1 应返回 evt-1，实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "hop=2 应返回 evt-2，实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-3".to_string()),
        "hop=3 应返回 evt-3，实际: {:?}",
        event_ids
    );
    assert!(
        !event_ids.contains(&"evt-4".to_string()),
        "evt-4（hop=4）不应出现（阀门 1：max_hop={} 截断），实际: {:?}",
        MAX_HOP,
        event_ids
    );

    // 所有 hit 的 hop 应 ≤ MAX_HOP
    for hit in &result.hits {
        let hop = hit.hop.unwrap_or(0);
        assert!(
            hop <= MAX_HOP,
            "hop 应 ≤ MAX_HOP({})（阀门 1 截断），实际 evt-{} hop={:?}",
            MAX_HOP,
            hit.event_id,
            hit.hop
        );
    }
}

// ---------------------------------------------------------------------------
// 测试 2：中间实体 > 100 触发截断（阀门 2）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_max_intermediate_entities_100_truncates() {
    // 验收指标 2：中间实体 > 100 触发截断
    //
    // 在 101 中间实体图上查询「张三」（默认 max_hop=3）：
    // - hop=0：A 入队
    // - hop=1：A 处理 → 发现 101 个 evt-i → 101 个 Bi 入队
    // - 逐个处理 Bi：visited_entities 增长
    // - 当 visited_entities.len() >= MAX_INTERMEDIATE_ENTITIES(100) 时，阀门 2 触发，截断 BFS
    //
    // 断言：
    // 1. last_valve_warnings 非空（阀门 2 触发）
    // 2. warning 含「阀门」字样
    // 3. hits 非空（部分结果返回）
    let conn = setup_101_intermediate_db();
    let strategy = MultiStrategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // hits 应非空（部分结果返回 — hop=1 的 101 个 event 在阀门触发前已收集）
    assert!(
        !result.hits.is_empty(),
        "阀门 2 截断后应返回部分结果（hop=1 events），实际 hits 为空"
    );

    // 阀门 2 应触发 warning
    let warnings = strategy.last_valve_warnings();
    assert!(
        !warnings.is_empty(),
        "阀门 2 应触发 warning（中间实体 > {}），实际 warnings 为空",
        MAX_INTERMEDIATE_ENTITIES
    );

    // warning 应含「阀门」字样
    let has_valve_warning = warnings.iter().any(|w| w.contains("阀门"));
    assert!(
        has_valve_warning,
        "warning 应含「阀门」字样，实际: {:?}",
        warnings
    );

    // warning 应提及中间实体或阀门 2
    let has_intermediate_warning = warnings
        .iter()
        .any(|w| w.contains("中间实体") || w.contains("阀门 2"));
    assert!(
        has_intermediate_warning,
        "warning 应提及「中间实体」或「阀门 2」，实际: {:?}",
        warnings
    );
}

// ---------------------------------------------------------------------------
// 测试 3：JOIN 行数 > 10000 触发截断（阀门 3）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_max_join_rows_10000_truncates() {
    // 验收指标 3：JOIN 行数 > 10000 触发截断
    //
    // 在 10001 JOIN 行图上查询「张三」（默认 max_hop=3）：
    // - hop=0：A 入队
    // - hop=1：A 处理 → find_events_by_entity(A) 返回 10001 行
    // - 阀门 3 触发：total_join_rows = 10001 > MAX_JOIN_ROWS(10000)
    // - 截断扩展：处理当前 10001 个 event（加入 results），但不再入队 next entities
    //
    // 断言：
    // 1. last_valve_warnings 非空（阀门 3 触发）
    // 2. warning 含「阀门」字样
    // 3. hits 非空（部分结果返回）
    let conn = setup_10001_join_rows_db();
    // 使用较大的 top_k 以容纳所有 10001 个 hop=1 event（验证部分结果返回）
    let strategy = MultiStrategy::new_with_top_k_and_max_hop(conn, 20000, MAX_HOP);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // hits 应非空（部分结果返回 — 阀门 3 触发前已收集的 10001 个 hop=1 event）
    assert!(
        !result.hits.is_empty(),
        "阀门 3 截断后应返回部分结果（hop=1 events），实际 hits 为空"
    );

    // 阀门 3 应触发 warning
    let warnings = strategy.last_valve_warnings();
    assert!(
        !warnings.is_empty(),
        "阀门 3 应触发 warning（JOIN 行数 > {}），实际 warnings 为空",
        MAX_JOIN_ROWS
    );

    // warning 应含「阀门」字样
    let has_valve_warning = warnings.iter().any(|w| w.contains("阀门"));
    assert!(
        has_valve_warning,
        "warning 应含「阀门」字样，实际: {:?}",
        warnings
    );

    // warning 应提及 JOIN 或阀门 3
    let has_join_warning = warnings
        .iter()
        .any(|w| w.contains("JOIN") || w.contains("阀门 3"));
    assert!(
        has_join_warning,
        "warning 应提及「JOIN」或「阀门 3」，实际: {:?}",
        warnings
    );
}

// ---------------------------------------------------------------------------
// 测试 4：截断结果含 warning
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_truncated_result_includes_warning() {
    // 验收指标 4：截断结果含 warning
    //
    // 使用阀门 2 fixture（101 中间实体），验证截断时 warning 被正确记录：
    // - warning 出现在 last_valve_warnings 中
    // - warning 含「阀门」字样
    // - warning 含「RISK-SAG-07」或「R-07」标识
    //
    // 注：thought_process 内部记录（不通过 SearchResult 暴露），通过
    // `MultiStrategy::last_valve_warnings()` 方法访问（避免修改 SearchResult 结构体）。
    let conn = setup_101_intermediate_db();
    let strategy = MultiStrategy::new(conn);
    let _result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    let warnings = strategy.last_valve_warnings();
    assert!(
        !warnings.is_empty(),
        "截断时应记录 warning，实际 warnings 为空"
    );

    // 至少一个 warning 含「阀门」字样
    let has_valve_keyword = warnings.iter().any(|w| w.contains("阀门"));
    assert!(
        has_valve_keyword,
        "warning 应含「阀门」字样，实际: {:?}",
        warnings
    );

    // 至少一个 warning 含 RISK-SAG-07 或 R-07 标识
    let has_risk_tag = warnings
        .iter()
        .any(|w| w.contains("RISK-SAG-07") || w.contains("R-07"));
    assert!(
        has_risk_tag,
        "warning 应含「RISK-SAG-07」或「R-07」标识，实际: {:?}",
        warnings
    );
}

// ---------------------------------------------------------------------------
// 测试 5：三道阀门独立触发（互不影响）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_three_valves_independent() {
    // 验收指标 5：三道阀门独立触发
    //
    // 分别构造三个 fixture，验证每个阀门单独触发时其他阀门不触发：
    //
    // 1. **阀门 1**（4 跳深链）：max_hop=3 截断 evt-4
    //    - 阀门 2 不触发：visited_entities 仅 4 个（A/B/C/D）< 100
    //    - 阀门 3 不触发：total_join_rows 仅 ~8 行 < 10000
    //
    // 2. **阀门 2**（101 中间实体）：visited_entities >= 100 截断
    //    - 阀门 1 不触发：所有 hop ≤ 1 < 3（Bi 的 event 已访问，不再扩展）
    //    - 阀门 3 不触发：total_join_rows = 101(A) + 101(Bi 各 1) = 202 < 10000
    //
    // 3. **阀门 3**（10001 JOIN 行）：total_join_rows > 10000 截断
    //    - 阀门 1 不触发：hop=1 < 3
    //    - 阀门 2 不触发：visited_entities 仅 1 个（A）< 100

    // --- 阀门 1 独立触发 ---
    let conn1 = setup_4hop_chain_db();
    let strategy1 = MultiStrategy::new(conn1);
    let result1 = strategy1
        .search("张三")
        .await
        .expect("search 应成功");

    let event_ids1: Vec<String> = result1.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        !event_ids1.contains(&"evt-4".to_string()),
        "阀门 1：evt-4（hop=4）不应出现（max_hop=3 截断），实际: {:?}",
        event_ids1
    );
    // 阀门 1 不产生 warning（max_hop 是正常配置，非异常截断）
    // 阀门 2/3 不触发（数据量小）
    let warnings1 = strategy1.last_valve_warnings();
    let valve2_triggered_1 = warnings1.iter().any(|w| w.contains("阀门 2"));
    let valve3_triggered_1 = warnings1.iter().any(|w| w.contains("阀门 3"));
    assert!(
        !valve2_triggered_1,
        "阀门 1 触发时阀门 2 不应触发，实际 warnings: {:?}",
        warnings1
    );
    assert!(
        !valve3_triggered_1,
        "阀门 1 触发时阀门 3 不应触发，实际 warnings: {:?}",
        warnings1
    );

    // --- 阀门 2 独立触发 ---
    let conn2 = setup_101_intermediate_db();
    let strategy2 = MultiStrategy::new(conn2);
    let result2 = strategy2
        .search("张三")
        .await
        .expect("search 应成功");

    assert!(
        !result2.hits.is_empty(),
        "阀门 2 截断后应返回部分结果，实际 hits 为空"
    );
    let warnings2 = strategy2.last_valve_warnings();
    let valve2_triggered_2 = warnings2.iter().any(|w| w.contains("阀门 2"));
    assert!(
        valve2_triggered_2,
        "阀门 2 应触发（101 中间实体 > 100），实际 warnings: {:?}",
        warnings2
    );
    // 阀门 3 不触发（total_join_rows < 10000）
    let valve3_triggered_2 = warnings2.iter().any(|w| w.contains("阀门 3"));
    assert!(
        !valve3_triggered_2,
        "阀门 2 触发时阀门 3 不应触发（JOIN 行数 < 10000），实际 warnings: {:?}",
        warnings2
    );

    // --- 阀门 3 独立触发 ---
    let conn3 = setup_10001_join_rows_db();
    let strategy3 = MultiStrategy::new_with_top_k_and_max_hop(conn3, 20000, MAX_HOP);
    let result3 = strategy3
        .search("张三")
        .await
        .expect("search 应成功");

    assert!(
        !result3.hits.is_empty(),
        "阀门 3 截断后应返回部分结果，实际 hits 为空"
    );
    let warnings3 = strategy3.last_valve_warnings();
    let valve3_triggered_3 = warnings3.iter().any(|w| w.contains("阀门 3"));
    assert!(
        valve3_triggered_3,
        "阀门 3 应触发（10001 JOIN 行 > 10000），实际 warnings: {:?}",
        warnings3
    );
    // 阀门 2 不触发（visited_entities 仅 1 个）
    let valve2_triggered_3 = warnings3.iter().any(|w| w.contains("阀门 2"));
    assert!(
        !valve2_triggered_3,
        "阀门 3 触发时阀门 2 不应触发（中间实体 < 100），实际 warnings: {:?}",
        warnings3
    );
}
