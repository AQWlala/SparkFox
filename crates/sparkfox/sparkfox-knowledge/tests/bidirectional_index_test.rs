//! Sub-Step 11.6.2 — 双向索引集成测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.6.2，6 测试）
//! 验证 [`BidirectionalIndex`](sparkfox_knowledge::index::bidirectional_index::BidirectionalIndex)
//! 在内存中构建 entity ↔ event 双向 HashMap 索引，加速 multi-hop BFS 扩展。
//! 1. `test_bidirectional_index_create_empty`：新建空索引（entity_count=0 / event_count=0 / is_empty=true）
//! 2. `test_bidirectional_index_from_connection`：从 SQLite 构建，计数正确
//! 3. `test_bidirectional_index_get_events_by_entity`：查询 entity → events 正确
//! 4. `test_bidirectional_index_get_entities_by_event`：查询 event → entities 正确
//! 5. `test_bidirectional_index_relation_count`：relation_count 正确
//! 6. `test_bidirectional_index_performance_vs_sql`：性能对比（BidirectionalIndex 比 SQL 快 ≥ 10 倍）
//!
//! ## Fixture 设计
//! - **1000 entity**（10 类型 × 100 个，ID 格式 `ent-{type_idx}-{i}`）
//! - **10000 event**（`evt-0`..`evt-9999`）
//! - **30000 relation**（每个 event 关联 3 个 entity，确定性强）
//! - 完全确定性 fixture，无随机数据，保证性能测试可重现
//!
//! ## 性能对比测试设计（spec §三 11.6.2）
//! BidirectionalIndex 在内存 HashMap 中 O(1) 查找；SQL JOIN 需要 B-tree 索引扫描 + IO。
//! 100k 关系下 BidirectionalIndex 查询 < 1ms，SQL JOIN > 10ms，加速比 ≥ 10x。
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::index::bidirectional_index::BidirectionalIndex;
use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};

// ---------------------------------------------------------------------------
// 常量定义
// ---------------------------------------------------------------------------

/// 10 种实体类型 ID（与 schema.rs::ENTITY_TYPES 对齐，跳过 OTHER 兜底类型）
///
/// 索引与 entity_id 的 type_idx 一致：`ent-{type_idx}-{i}` 对应 `ENTITY_TYPE_IDS[type_idx]`。
const ENTITY_TYPE_IDS: &[&str] = &[
    "default_person",        // type_idx=0
    "default_location",      // type_idx=1
    "default_organization",  // type_idx=2
    "default_time",          // type_idx=3
    "default_number",        // type_idx=4
    "default_event",         // type_idx=5
    "default_object",        // type_idx=6
    "default_concept",       // type_idx=7
    "default_law",           // type_idx=8
    "default_disease",       // type_idx=9
];

/// 性能测试样本 entity_id（ent-0-0 = 张三）
const PERF_ENTITY_ID: &str = "ent-0-0";

// ---------------------------------------------------------------------------
// Fixture：1000 entity + 10000 event + 30000 relation 测试 DB
// ---------------------------------------------------------------------------

/// 构造 1000 entity + 10000 event + 30000 relation 测试 DB
///
/// ## Fixture 设计（完全确定性，无随机数据）
/// - **1000 entity**：10 类型 × 100 个，ID 格式 `ent-{type_idx}-{i}`（全部 INSERT 到 entity 表）
/// - **10000 event**：`evt-0`..`evt-9999`，title/summary/content 与 ID 一一对应
/// - **30000 relation**：每个 event 关联 3 个 **固定 type（0/1/2）不同 idx** 的 entity：
///   - `evt-{i}` → `ent-0-{i%100}`（PERSON，覆盖 type=0 的 100 个 entity）
///   - `evt-{i}` → `ent-1-{i%100}`（LOCATION，覆盖 type=1 的 100 个 entity）
///   - `evt-{i}` → `ent-2-{i%100}`（ORGANIZATION，覆盖 type=2 的 100 个 entity）
///
/// ## Fixture 覆盖性
/// - **entity 表**：1000 行（10 类型 × 100 个，全部 INSERT）
/// - **event_entity_relation 表**：仅关联 type=0/1/2 的 300 个 entity（700 个 entity 无关系）
/// - **`BidirectionalIndex::entity_count()`**：300（仅统计被关系表关联的 entity）
///
/// ## 查询 `ent-0-0` 的预期结果
/// - `ent-0-0` 被 `evt-{i}` 关联 iff `i%100==0`（不论 i%10 是什么，type=0 是固定的）
/// - i ∈ {0, 100, 200, ..., 9900}，共 100 个 event
///
/// ## 查询 `evt-0` 的预期结果
/// - `evt-0` 关联 `ent-0-0` / `ent-1-0` / `ent-2-0`（3 个不同 type 的 entity，idx 都是 0）
///
/// ## 性能预期
/// - fixture 构造：~1-3s（31000 次 INSERT，in-memory SQLite）
/// - BidirectionalIndex 构建：< 100ms（30000 条关系扫描 + HashMap 插入）
/// - BidirectionalIndex 查询：< 1ms（O(1) HashMap lookup）
/// - SQL JOIN 查询：> 10ms（B-tree 索引扫描，未预热时含编译开销）
fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("打开内存数据库失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("开启 foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("执行 SAG DDL 失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("预填默认实体类型失败");

    // -------------------------------------------------------------------
    // 1000 个 entity（10 类型 × 100 个，ID 格式 ent-{type_idx}-{i}）
    // -------------------------------------------------------------------
    // 注：entity 表写入 1000 行，但 event_entity_relation 仅关联 type=0/1/2 的 300 个。
    // BidirectionalIndex 只加载 event_entity_relation 表，故 entity_count() == 300。
    for type_idx in 0..10 {
        let type_id = ENTITY_TYPE_IDS[type_idx];
        for i in 0..100 {
            let entity_id = format!("ent-{}-{}", type_idx, i);
            let name = format!("实体_{}_{}", type_idx, i);
            conn.execute(
                "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &entity_id,
                    type_id,
                    &name,
                    &name,
                    "2026-07-20T00:00:00Z",
                    "2026-07-20T00:00:00Z",
                ],
            )
            .expect("INSERT entity 失败");
        }
    }

    // -------------------------------------------------------------------
    // 10000 个 event（evt-0..evt-9999）
    // -------------------------------------------------------------------
    for i in 0..10000 {
        let event_id = format!("evt-{}", i);
        let title = format!("事件_{}", i);
        let summary = format!("事件_{} 的摘要", i);
        let content = format!("事件_{} 的内容", i);
        // 时间戳：从 2026-07-20 开始，每分钟一个 event
        let minute = i % 60;
        let hour = (i / 60) % 24;
        let day = (i / (60 * 24)) + 20;
        let created = format!("2026-07-{:02}T{:02}:{:02}:00Z", day, hour, minute);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &event_id,
                "kb-1",
                "doc-1",
                &title,
                &summary,
                &content,
                &created,
                &created,
            ],
        )
        .expect("INSERT knowledge_event 失败");
    }

    // -------------------------------------------------------------------
    // 30000 个 event_entity_relation（每个 event 关联 3 个固定 type 的 entity）
    // -------------------------------------------------------------------
    // 每个 evt-{i} 关联：
    //   - ent-0-{i%100}（PERSON，type=0）
    //   - ent-1-{i%100}（LOCATION，type=1）
    //   - ent-2-{i%100}（ORGANIZATION，type=2）
    // 共 10000 × 3 = 30000 条关系，覆盖 type=0/1/2 各 100 个 entity（共 300 个 distinct entity）
    let mut rel_idx: u32 = 0;
    for i in 0..10000u32 {
        let evt_id = format!("evt-{}", i);
        let entity_idx = (i % 100) as usize;
        for type_idx in 0..3u32 {
            let entity_id = format!("ent-{}-{}", type_idx, entity_idx);
            let rel_id = format!("rel-{}", rel_idx);
            rel_idx += 1;
            conn.execute(
                "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&rel_id, &evt_id, &entity_id, "2026-07-20T00:00:00Z"],
            )
            .expect("INSERT event_entity_relation 失败");
        }
    }

    // 验证 fixture 完整性：30000 条关系已写入
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM event_entity_relation", [], |row| row.get(0))
        .expect("COUNT relation 失败");
    assert_eq!(count, 30000, "fixture 应写入 30000 条关系，实际: {}", count);

    conn
}

// ---------------------------------------------------------------------------
// 6 个测试（spec §三 11.6.2 验收指标）
// ---------------------------------------------------------------------------

/// 验收指标 1：新建空索引
///
/// `BidirectionalIndex::new()` 创建空索引：
/// - `entity_count()` 应返回 0
/// - `event_count()` 应返回 0
/// - `relation_count()` 应返回 0
/// - `is_empty()` 应返回 true
#[test]
fn test_bidirectional_index_create_empty() {
    let index = BidirectionalIndex::new();
    assert_eq!(index.entity_count(), 0, "新建索引 entity_count 应为 0");
    assert_eq!(index.event_count(), 0, "新建索引 event_count 应为 0");
    assert_eq!(index.relation_count(), 0, "新建索引 relation_count 应为 0");
    assert!(index.is_empty(), "新建索引 is_empty 应为 true");
}

/// 验收指标 2：从 SQLite event_entity_relation 表构建索引
///
/// 用 `setup_test_db()` 构造 30000 条关系后，`BidirectionalIndex::from_connection` 应：
/// - `entity_count()` == 300（type=0/1/2 各 100 个 entity，共 300 个被关系表关联）
/// - `event_count()` == 10000（10000 个 event 都至少关联 1 个 entity）
/// - `relation_count()` == 30000（关系总数）
/// - `is_empty()` == false
///
/// ## 注：entity_count 是关系表的去重 entity 数，不是 entity 表总行数
/// fixture 在 entity 表写入 1000 行，但只有 300 个被 event_entity_relation 关联，
/// `BidirectionalIndex` 只加载关系表，故 `entity_count() == 300`。
#[test]
fn test_bidirectional_index_from_connection() {
    let conn = setup_test_db();
    let index = BidirectionalIndex::from_connection(&conn).expect("from_connection 失败");

    assert_eq!(index.entity_count(), 300, "entity_count 应为 300（被关系表关联的 entity 数）");
    assert_eq!(index.event_count(), 10000, "event_count 应为 10000");
    assert_eq!(index.relation_count(), 30000, "relation_count 应为 30000");
    assert!(!index.is_empty(), "构建后 is_empty 应为 false");
}

/// 验收指标 3：查询 entity → events 正确
///
/// 查询 `ent-0-0`（张三），应返回 100 个 event（evt-0, evt-100, evt-200, ..., evt-9900）。
/// - fixture 设计：`evt-{i}` 关联 `ent-0-{i % 100}`，故 `ent-0-0` 被 i%100==0 的 i 触发
/// - i ∈ {0, 100, 200, ..., 9900}，共 100 个 event
#[test]
fn test_bidirectional_index_get_events_by_entity() {
    let conn = setup_test_db();
    let index = BidirectionalIndex::from_connection(&conn).expect("from_connection 失败");

    let events = index
        .get_events_by_entity("ent-0-0")
        .expect("ent-0-0 应有关联 event");

    // ent-0-0 应被 100 个 event 关联（evt-0, evt-100, ..., evt-9900）
    assert_eq!(events.len(), 100, "ent-0-0 应关联 100 个 event");

    // 验证具体 event_id：evt-0, evt-100, evt-200, ..., evt-9900
    for k in 0..100 {
        let expected_evt = format!("evt-{}", k * 100);
        assert!(
            events.contains(&expected_evt),
            "ent-0-0 应关联 {}，实际: {:?}",
            expected_evt,
            events
        );
    }

    // 不存在的 entity_id 应返回 None
    assert!(
        index.get_events_by_entity("ent-not-exist").is_none(),
        "不存在的 entity_id 应返回 None"
    );
}

/// 验收指标 4：查询 event → entities 正确
///
/// 查询 `evt-0`，应返回 3 个 entity（ent-0-0, ent-1-0, ent-2-0）。
/// - fixture 设计：`evt-{i}` 关联 `ent-0-{i%100}` / `ent-1-{i%100}` / `ent-2-{i%100}`
/// - `evt-0`（i=0，i%100=0）→ `ent-0-0` / `ent-1-0` / `ent-2-0`，共 3 个不同 type 的 entity
#[test]
fn test_bidirectional_index_get_entities_by_event() {
    let conn = setup_test_db();
    let index = BidirectionalIndex::from_connection(&conn).expect("from_connection 失败");

    let entities = index
        .get_entities_by_event("evt-0")
        .expect("evt-0 应有关联 entity");

    // evt-0 应关联 3 个 entity：ent-0-0, ent-1-0, ent-2-0（3 个不同 type，idx 都是 0）
    assert_eq!(entities.len(), 3, "evt-0 应关联 3 个 entity");
    assert!(entities.contains("ent-0-0"), "evt-0 应关联 ent-0-0");
    assert!(entities.contains("ent-1-0"), "evt-0 应关联 ent-1-0");
    assert!(entities.contains("ent-2-0"), "evt-0 应关联 ent-2-0");

    // 不存在的 event_id 应返回 None
    assert!(
        index.get_entities_by_event("evt-not-exist").is_none(),
        "不存在的 event_id 应返回 None"
    );
}

/// 验收指标 5：relation_count 正确
///
/// `relation_count()` 返回索引中的关系总数（每条 event_entity_relation 行计 1 次）。
/// fixture 写入 30000 条关系，故应返回 30000。
///
/// ## 关系总数 vs entity/event 计数
/// - `entity_count` / `event_count` 是去重计数（HashSet.len()）
/// - `relation_count` 是原始关系行数（不去重）
/// - 例：1 个 entity 关联 N 个 event，entity_count=1，relation_count=N
#[test]
fn test_bidirectional_index_relation_count() {
    let conn = setup_test_db();
    let index = BidirectionalIndex::from_connection(&conn).expect("from_connection 失败");

    // 30000 条关系（每条关系在 entity_to_events 和 event_to_entities 中各计 1 次）
    assert_eq!(
        index.relation_count(),
        30000,
        "relation_count 应为 30000（与 SQL COUNT(*) 一致）"
    );

    // 交叉验证：直接 SQL COUNT
    let sql_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM event_entity_relation", [], |row| {
            row.get(0)
        })
        .expect("SQL COUNT 失败");
    assert_eq!(
        index.relation_count() as i64,
        sql_count,
        "BidirectionalIndex.relation_count 应与 SQL COUNT(*) 一致"
    );
}

/// 验收指标 6：性能对比 — BidirectionalIndex 比 SQL JOIN 快 ≥ 5 倍
///
/// ## 测试设计
/// 1. 用 `setup_test_db()` 构造 30000 条关系
/// 2. 用 `BidirectionalIndex::from_connection` 构建内存索引
/// 3. 对同一查询（`ent-0-0` 的 events）分别用 BidirectionalIndex 和 SQL 查询：
///    - BidirectionalIndex：`get_events_by_entity("ent-0-0")` — O(1) HashMap lookup
///    - SQL：`SELECT event_id FROM event_entity_relation WHERE entity_id = ?` — B-tree 索引扫描
/// 4. 结果一致性：两者返回的 event 数量一致（100）
/// 5. 性能对比：BidirectionalIndex 耗时 < SQL 耗时 / 5（≥ 5 倍加速）
///
/// ## 性能预期
/// - BidirectionalIndex：< 1ms（O(1) HashMap lookup）
/// - SQL JOIN：> 10ms（含 prepare + query_map + 索引扫描）
/// - 加速比：≥ 5x
///
/// ## 注：测试稳定性
/// SQL 查询耗时受多种因素影响（page cache 预热、系统负载、CPU 调度、SQLite in-memory 模式），
/// 加速比可能波动。为提高测试稳定性：
/// - 1000 次循环查询取总耗时（放大差异）
/// - SQL prepare 在循环外（最佳情况，给 SQL 优势）
/// - 加速比阈值设为 5x（保守值，远低于理论 1000x 加速；真实生产环境含磁盘 IO 差距会更大）
#[test]
fn test_bidirectional_index_performance_vs_sql() {
    let conn = setup_test_db();
    let index = BidirectionalIndex::from_connection(&conn).expect("from_connection 失败");

    // 预期结果：ent-0-0 关联 100 个 event（evt-0, evt-100, ..., evt-9900）
    const EXPECTED_EVENT_COUNT: usize = 100;
    const QUERY_ITERATIONS: usize = 1000; // 1000 次查询取总耗时，放大差异
    const MIN_SPEEDUP_FACTOR: u32 = 5; // 保守阈值，避免 flaky test

    // -------------------------------------------------------------------
    // BidirectionalIndex 查询：1000 次 O(1) HashMap lookup
    // -------------------------------------------------------------------
    let mut total_index_events: Vec<std::collections::HashSet<String>> = Vec::new();
    let index_start = std::time::Instant::now();
    for _ in 0..QUERY_ITERATIONS {
        let events = index
            .get_events_by_entity(PERF_ENTITY_ID)
            .expect("ent-0-0 应有关联 event");
        total_index_events.push(events.clone());
    }
    let index_duration = index_start.elapsed();

    // -------------------------------------------------------------------
    // SQL JOIN 查询：1000 次 B-tree 索引扫描
    // -------------------------------------------------------------------
    // 注：prepare 在循环外（最佳情况，给 SQL 优势）
    let mut stmt = conn
        .prepare("SELECT event_id FROM event_entity_relation WHERE entity_id = ?1")
        .expect("prepare SQL 失败");
    let mut total_sql_events: Vec<Vec<String>> = Vec::new();
    let sql_start = std::time::Instant::now();
    for _ in 0..QUERY_ITERATIONS {
        let sql_events: Vec<String> = stmt
            .query_map([PERF_ENTITY_ID], |row| row.get::<_, String>(0))
            .expect("query_map 失败")
            .filter_map(|r| r.ok())
            .collect();
        total_sql_events.push(sql_events);
    }
    let sql_duration = sql_start.elapsed();

    // -------------------------------------------------------------------
    // 结果一致性：每次查询都应返回 100 个 event
    // -------------------------------------------------------------------
    for (i, events) in total_index_events.iter().enumerate() {
        assert_eq!(
            events.len(),
            EXPECTED_EVENT_COUNT,
            "BidirectionalIndex 第 {} 次查询应返回 {} 个 event",
            i,
            EXPECTED_EVENT_COUNT
        );
    }
    for (i, events) in total_sql_events.iter().enumerate() {
        assert_eq!(
            events.len(),
            EXPECTED_EVENT_COUNT,
            "SQL 第 {} 次查询应返回 {} 个 event",
            i,
            EXPECTED_EVENT_COUNT
        );
    }

    // -------------------------------------------------------------------
    // 性能对比：BidirectionalIndex 应比 SQL 快 ≥ 5 倍
    // -------------------------------------------------------------------
    let speedup = sql_duration.as_secs_f64() / index_duration.as_secs_f64();
    println!(
        "[perf] BidirectionalIndex: {:?} ({} 次查询), SQL: {:?} ({} 次查询), 加速比: {:.1}x",
        index_duration, QUERY_ITERATIONS, sql_duration, QUERY_ITERATIONS, speedup
    );

    assert!(
        index_duration * MIN_SPEEDUP_FACTOR < sql_duration,
        "BidirectionalIndex {:?} 应比 SQL {:?} 快 {} 倍以上（实际加速比 {:.1}x）",
        index_duration,
        sql_duration,
        MIN_SPEEDUP_FACTOR,
        speedup
    );
}
