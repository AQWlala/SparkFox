//! Sub-Step 12.2.2 — 查询时 SQL JOIN 激活局部超边（TDD-RED → GREEN → REFACTOR）
//!
//! ## SAG 核心创新 — 第 2 步（共 4 步）
//! 12.2.1 已实现 HyperedgeDetector（检测：>2 event 共享 >2 entity → 自动形成超边）。
//! 本 Sub-Step 在其上添加**查询时激活**能力：query 命中超边内任一 entity 时，
//! SQL JOIN 激活整条超边（非预计算），返回所有成员 events。
//!
//! ## 关键设计 — 非预计算 + 局部激活
//! - **非预计算**：超边不预先存储，而是在 query 时动态调用 `detect_hyperedges` 检测，
//!   然后过滤出与 query_entities 有交集的超边。这避免预计算存储开销，且超边随数据
//!   增删自动更新（无需重建索引）。
//! - **局部激活**：仅激活与 query_entities 有交集的超边，不做全局图遍历。这避免
//!   在大图上遍历所有超边，复杂度 O(K)（K = 激活超边数），而非 O(N)（N = 全图超边数）。
//!
//! ## 算法
//! 1. `HyperedgeDetector::activate_local_hyperedges(conn, query_entities)`:
//!    a. 调用 `detect_hyperedges(conn)` 检测所有超边
//!    b. 过滤：`hyperedge.member_entities ∩ query_entities != ∅` → 激活
//!    c. 返回激活的超边列表（局部，非全局）
//! 2. `HyperedgeDetector::collect_activated_events(hyperedges)`:
//!    a. 收集所有激活超边的 member_events
//!    b. 去重 + 排序返回
//! 3. MULTI_ES 集成：用 ES-first 命中的 entity_ids 调用 activate_local_hyperedges，
//!    将激活超边的 member_events 加入候选集，应用 max_join_rows 阀门。
//!
//! ## 5 个测试用例（spec §三 12.2.2 验收指标）
//! 1. `test_query_activates_local_hyperedges`：query 命中超边内任一 entity 时激活整条超边
//! 2. `test_activated_hyperedge_returns_all_member_events`：激活超边返回所有成员 events
//! 3. `test_local_activation_only`：仅激活局部超边（非全局）
//! 4. `test_max_join_rows_valve_applies_to_hyperedge`：max_join_rows 阀门适用于超边 JOIN
//! 5. `test_multi_es_integrates_hyperedge_activation`：MULTI_ES 集成超边激活
//!
//! ## Fixture 设计
//! - 测试 1/2/5：K_{3,3} 完全二分图（3 events × 3 entities = 9 relations → 1 超边）
//! - 测试 3：2 个独立 K_{3,3}（共 6 events × 6 entities = 18 relations → 2 超边）
//! - 测试 4：K_{3,3} + 可配置 max_join_rows=2（3 > 2 触发阀门）
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::hyperedge::HyperedgeDetector;
use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{MultiEsStrategy, SearchStrategy};

// ---------------------------------------------------------------------------
// Fixture 辅助函数：构造完全二分图 K_{n_events, n_entities} 的 SQLite DB
// ---------------------------------------------------------------------------

/// 构造 K_{n_events, n_entities} 完全二分图测试 DB
///
/// - event_id 格式：`{prefix}-evt-{i}`（i=0..n_events-1）
/// - entity_id 格式：`{prefix}-ent-{j}`（j=0..n_entities-1）
/// - 每对 (event, entity) 都生成一条 event_entity_relation，共 n_events × n_entities 条
///
/// ## 实体命名
/// - 第 0 个 entity：name = "张三"（便于 ES-first 测试用 query="张三" 命中）
/// - 第 1 个 entity：name = "北京"
/// - 第 2 个 entity：name = "腾讯"
/// - 后续 entity：name = "{prefix}-entity-{j}"
///
/// ## 实体类型
/// - 第 0 个：default_person（张三）
/// - 第 1 个：default_location（北京）
/// - 第 2 个：default_organization（腾讯）
/// - 后续：default_other
fn setup_k_bipartite_db(n_events: usize, n_entities: usize, prefix: &str) -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 实体命名表（前 3 个用中文名便于 ES-first 测试）
    let named_entities = ["张三", "北京", "腾讯"];

    // 插入 n_entities 个 entity
    for j in 0..n_entities {
        let id = format!("{prefix}-ent-{j}");
        // 显式标注类型统一 if/else 分支返回类型（&str vs String 不兼容）
        let (name, type_id): (String, &str) = if j < 3 {
            (
                named_entities[j].to_string(),
                ["default_person", "default_location", "default_organization"][j],
            )
        } else {
            (format!("{prefix}-entity-{j}"), "default_other")
        };
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, type_id, name, name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 插入 n_events 个 event
    conn.execute_batch("BEGIN;").unwrap();
    for i in 0..n_events {
        let id = format!("{prefix}-evt-{i}");
        let title = format!("{prefix}-事件-{i}");
        let summary = format!("{prefix}-事件 {i} 摘要");
        let content = format!("{prefix}-事件 {i} 内容");
        let created = format!("2026-07-20T{:02}:{:02}:00Z", i / 60, i % 60);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "kb-1", "doc-1", title, summary, content, created, created],
        ).unwrap();
    }
    conn.execute_batch("COMMIT;").unwrap();

    // 插入 n_events × n_entities 条 event_entity_relation
    conn.execute_batch("BEGIN;").unwrap();
    for i in 0..n_events {
        for j in 0..n_entities {
            let rel_id = format!("{prefix}-rel-{i}-{j}");
            let evt_id = format!("{prefix}-evt-{i}");
            let ent_id = format!("{prefix}-ent-{j}");
            conn.execute(
                "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
                rusqlite::params![rel_id, evt_id, ent_id, "2026-07-20T00:00:00Z"],
            ).unwrap();
        }
    }
    conn.execute_batch("COMMIT;").unwrap();

    conn
}

/// 构造 2 个独立 K_{3,3} 测试 DB（用于 test_local_activation_only）
///
/// - Hyperedge A：prefix "A"，3 events × 3 entities（evt-A-0/1/2 × ent-A-0/1/2）
/// - Hyperedge B：prefix "B"，3 events × 3 entities（evt-B-0/1/2 × ent-B-0/1/2）
/// - 两组之间无共享 entity，形成 2 条独立超边
fn setup_two_independent_hyperedges_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // Hyperedge A 的 3 个 entity（张三/北京/腾讯，便于 query 命中）
    let entities_a = [
        ("A-ent-0", "default_person", "张三"),
        ("A-ent-1", "default_location", "北京"),
        ("A-ent-2", "default_organization", "腾讯"),
    ];
    for (id, type_id, name) in &entities_a {
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, type_id, name, name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // Hyperedge B 的 3 个 entity（李四/上海/阿里，与 A 不重叠）
    let entities_b = [
        ("B-ent-0", "default_person", "李四"),
        ("B-ent-1", "default_location", "上海"),
        ("B-ent-2", "default_organization", "阿里"),
    ];
    for (id, type_id, name) in &entities_b {
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, type_id, name, name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 6 个 event（A 组 3 个 + B 组 3 个）
    conn.execute_batch("BEGIN;").unwrap();
    for prefix in ["A", "B"] {
        for i in 0..3 {
            let id = format!("{prefix}-evt-{i}");
            let title = format!("{prefix}-事件-{i}");
            let summary = format!("{prefix}-事件 {i} 摘要");
            let content = format!("{prefix}-事件 {i} 内容");
            let created = format!("2026-07-20T{:02}:00:00Z", i);
            conn.execute(
                "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![id, "kb-1", "doc-1", title, summary, content, created, created],
            ).unwrap();
        }
    }
    conn.execute_batch("COMMIT;").unwrap();

    // 18 条 event_entity_relation（A 组 9 条 + B 组 9 条，K_{3,3} ×2）
    conn.execute_batch("BEGIN;").unwrap();
    for prefix in ["A", "B"] {
        for i in 0..3 {
            for j in 0..3 {
                let rel_id = format!("{prefix}-rel-{i}-{j}");
                let evt_id = format!("{prefix}-evt-{i}");
                let ent_id = format!("{prefix}-ent-{j}");
                conn.execute(
                    "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
                    rusqlite::params![rel_id, evt_id, ent_id, "2026-07-20T00:00:00Z"],
                ).unwrap();
            }
        }
    }
    conn.execute_batch("COMMIT;").unwrap();

    conn
}

// ---------------------------------------------------------------------------
// 测试 1：query 命中超边内任一 entity 时激活整条超边
// ---------------------------------------------------------------------------

/// 验收指标 1：query 命中超边内任一 entity 时激活整条超边
///
/// - Fixture：K_{3,3}（3 events × 3 entities = 9 relations）→ 1 条超边
/// - query_entities = ["K-ent-0"]（超边的 member_entity 之一）
/// - 期望：activate_local_hyperedges 返回 1 条超边
///
/// ## SAG 核心创新体现
/// 传统二元图：query 命中 ent-0 → 仅返回 ent-0 直接关联的 events（hop=1）
/// SAG 超边激活：query 命中 ent-0 → 激活整条超边 → 返回超边所有成员 events
/// （即使部分 event 不直接关联 query entity，也通过超边语义聚合返回）
///
/// ## 非预计算验证
/// 超边在 query 时动态检测（detect_hyperedges）+ 过滤（与 query_entities 有交集），
/// 不依赖预存储的超边表。这保证超边随 event_entity_relation 增删自动更新。
#[test]
fn test_query_activates_local_hyperedges() {
    let conn = setup_k_bipartite_db(3, 3, "K");
    let detector = HyperedgeDetector::new();

    // query 命中 ent-0（超边成员之一）
    let query_entities = vec!["K-ent-0".to_string()];
    let activated = detector
        .activate_local_hyperedges(&conn, &query_entities)
        .expect("activate_local_hyperedges 应成功");

    assert_eq!(
        activated.len(),
        1,
        "query 命中超边成员 → 应激活 1 条超边，实际: {:?}",
        activated
    );

    // 验证超边包含全部 3 个 member_events
    let he = &activated[0];
    assert_eq!(
        he.member_events.len(),
        3,
        "超边应含 3 个 member_events，实际: {:?}",
        he.member_events
    );
}

// ---------------------------------------------------------------------------
// 测试 2：激活超边返回所有成员 events
// ---------------------------------------------------------------------------

/// 验收指标 2：激活超边返回所有成员 events（3 个 events 全部返回）
///
/// - Fixture：K_{3,3}（3 events × 3 entities）→ 1 条超边
/// - query_entities = ["K-ent-0"]
/// - 期望：collect_activated_events 返回 3 个 events（evt-0 / evt-1 / evt-2）
///
/// ## 超边完整性
/// collect_activated_events 必须返回所有激活超边的所有 member_events（去重）。
/// 这是 SAG 超边激活的核心契约：激活超边 = 返回所有成员 events。
#[test]
fn test_activated_hyperedge_returns_all_member_events() {
    let conn = setup_k_bipartite_db(3, 3, "K");
    let detector = HyperedgeDetector::new();

    let query_entities = vec!["K-ent-0".to_string()];
    let activated = detector
        .activate_local_hyperedges(&conn, &query_entities)
        .expect("activate_local_hyperedges 应成功");

    assert_eq!(activated.len(), 1, "应激活 1 条超边");

    // 收集所有激活超边的 member_events（去重）
    let all_events = detector.collect_activated_events(&activated);

    assert_eq!(
        all_events.len(),
        3,
        "应返回 3 个 member_events（去重后），实际: {:?}",
        all_events
    );

    // 验证 3 个 event 全部包含
    for k in 0..3 {
        let expected_evt = format!("K-evt-{k}");
        assert!(
            all_events.contains(&expected_evt),
            "collect_activated_events 应包含 {}，实际: {:?}",
            expected_evt,
            all_events
        );
    }
}

// ---------------------------------------------------------------------------
// 测试 3：仅激活局部超边（非全局）
// ---------------------------------------------------------------------------

/// 验收指标 3：仅激活局部超边（非全局）
///
/// - Fixture：2 条独立超边（Hyperedge A 与 Hyperedge B，无共享 entity）
///   - A: evt-A-0/1/2 × ent-A-0/1/2（K_{3,3}）
///   - B: evt-B-0/1/2 × ent-B-0/1/2（K_{3,3}）
/// - query_entities = ["A-ent-0"]（仅命中超边 A 的成员）
/// - 期望：activate_local_hyperedges 仅返回 1 条超边（A），不返回 B
///
/// ## 局部激活验证
/// 局部激活的核心：仅激活与 query_entities 有交集的超边，不做全局图遍历。
/// 此测试确保 query 命中超边 A 的 entity 时，不误激活超边 B（无交集）。
///
/// ## 复杂度优势
/// 局部激活复杂度 O(K)（K = 激活超边数，通常 K << N 全图超边数），
/// 而非全局遍历 O(N)。在大图上避免不必要的超边检测与返回。
#[test]
fn test_local_activation_only() {
    let conn = setup_two_independent_hyperedges_db();
    let detector = HyperedgeDetector::new();

    // 先验证：全图共 2 条超边（A 和 B）
    let all_hyperedges = detector
        .detect_hyperedges(&conn)
        .expect("detect_hyperedges 应成功");
    assert_eq!(
        all_hyperedges.len(),
        2,
        "全图应含 2 条超边（A + B），实际: {:?}",
        all_hyperedges.len()
    );

    // query 仅命中超边 A 的成员（A-ent-0）
    let query_entities = vec!["A-ent-0".to_string()];
    let activated = detector
        .activate_local_hyperedges(&conn, &query_entities)
        .expect("activate_local_hyperedges 应成功");

    assert_eq!(
        activated.len(),
        1,
        "query 命中 ent-A-0 → 仅激活 1 条超边（A），不应激活 B，实际: {:?}",
        activated
    );

    // 验证激活的是超边 A（member_events 含 A-evt-*）
    let he = &activated[0];
    for evt in &he.member_events {
        assert!(
            evt.starts_with("A-evt-"),
            "激活超边的 member_events 应为 A-evt-*，实际: {:?}",
            he.member_events
        );
    }

    // 验证激活超边的 member_entities 含 A-ent-0（query 命中的 entity）
    assert!(
        he.member_entities.contains(&"A-ent-0".to_string()),
        "激活超边应含 query 命中的 A-ent-0，实际: {:?}",
        he.member_entities
    );
}

// ---------------------------------------------------------------------------
// 测试 4：max_join_rows 阀门适用于超边 JOIN
// ---------------------------------------------------------------------------

/// 验收指标 4：max_join_rows 阀门适用于超边 JOIN
///
/// - Fixture：K_{3,3}（3 events × 3 entities → 1 条超边，3 个 member_events）
/// - 配置：MultiEsStrategy::new(conn).with_max_join_rows(2)（小阈值便于测试）
/// - 期望：超边 JOIN 行数 3 > max_join_rows 2 → 触发阀门
///   - last_valve_warnings 非空（含「超边」或「阀门」字样）
///
/// ## 阀门设计（参考 11.2.4 R-07 三道 LIMIT 阀门）
/// - max_join_rows 阀门适用于超边 JOIN：累计激活超边的 member_events 总数
/// - 超过阈值时：截断（停止处理更多超边）+ 记录 warning
/// - 不修改 SearchResult 结构体，通过 last_valve_warnings 暴露给测试
///
/// ## 测试用小阈值的设计理由
/// 生产环境 max_join_rows = MAX_JOIN_ROWS = 10000（与 multi.rs 一致）。
/// 测试用 with_max_join_rows(2) 小阈值，避免构造 10001 个事件的庞大 fixture
/// （detect_hyperedges 复杂度 O(E × 2^n)，n=单 entity 关联的 event 数，
///   10001 events/entity 会导致 2^10001 子集枚举，不可行）。
/// 阀门逻辑与阈值无关，小阈值等价验证阀门机制。
#[tokio::test]
async fn test_max_join_rows_valve_applies_to_hyperedge() {
    let conn = setup_k_bipartite_db(3, 3, "K");
    // 配置小阈值 max_join_rows=2，超边 3 个 member_events > 2 → 触发阀门
    // 显式开启超边激活（默认关闭以避免大数据集性能退化）
    let strategy = MultiEsStrategy::new(conn)
        .with_hyperedge_activation(true)
        .with_max_join_rows(2);

    // 触发 search（query="张三" 命中 K-ent-0，激活超边）
    let _result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // 验证：阀门触发，last_valve_warnings 非空
    let warnings = strategy.last_valve_warnings();
    assert!(
        !warnings.is_empty(),
        "超边 JOIN 行数 3 > max_join_rows 2 → 应触发阀门（last_valve_warnings 非空）"
    );

    // 验证：warning 含「超边」字样（标识超边 JOIN 阀门，区别于 BFS 阀门）
    let has_hyperedge_warning = warnings.iter().any(|w| w.contains("超边"));
    assert!(
        has_hyperedge_warning,
        "warning 应含「超边」字样，实际: {:?}",
        warnings
    );
}

// ---------------------------------------------------------------------------
// 测试 5：MULTI_ES 集成超边激活
// ---------------------------------------------------------------------------

/// 验收指标 5：MULTI_ES 集成超边激活
///
/// - Fixture：K_{3,3}（3 events × 3 entities → 1 条超边）
///   - ent-0=张三 / ent-1=北京 / ent-2=腾讯
///   - evt-0/1/2 全部关联全部 3 个 entity
/// - 运行 MULTI_ES search("张三") → ES-first 命中 ent-0 → 激活超边
/// - 期望：search 结果含全部 3 个 member_events（evt-0/1/2）
/// - 期望：至少一个 hit 的 via_entities 含全部 3 个 entity（张三/北京/腾讯）
///   （证明超边激活为 hit 注入了完整的超边成员上下文）
///
/// ## 集成验证
/// MULTI_ES 在 BFS 扩展 + 子图预筛选之后，调用 activate_local_hyperedges：
/// 1. 用 ES-first 命中的 entity_ids 作为 query_entities
/// 2. 检测并激活局部超边（与 query_entities 有交集的超边）
/// 3. 将激活超边的 member_events 加入候选集（若不在 BFS 扩展结果中）
/// 4. 为命中超边的 event 增强 via_entities（注入超边 member_entities 上下文）
///
/// ## via_entities 增强设计
/// BFS 在 hop=1 时 via_entities = [seed_entity]（仅 1 个 entity）。
/// 超边激活后，对在超边内的 event，via_entities 增强 = [seed_entity, ...其他 member_entities]，
/// 提供完整的超边语义上下文，便于上层展示「此 event 通过超边关联这些 entities」。
#[tokio::test]
async fn test_multi_es_integrates_hyperedge_activation() {
    let conn = setup_k_bipartite_db(3, 3, "K");
    // 显式开启超边激活（默认关闭以避免大数据集性能退化）
    let strategy = MultiEsStrategy::new(conn).with_hyperedge_activation(true);

    let result = strategy
        .search("张三")
        .await
        .expect("MULTI_ES search 应成功");

    // 验证 1：search 结果含全部 3 个 member_events
    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    for k in 0..3 {
        let expected_evt = format!("K-evt-{k}");
        assert!(
            event_ids.contains(&expected_evt),
            "MULTI_ES search 结果应含超边成员 {}，实际: {:?}",
            expected_evt,
            event_ids
        );
    }

    // 验证 2：至少一个 hit 的 via_entities 含全部 3 个 entity（张三/北京/腾讯）
    // （证明超边激活为 hit 注入了完整的超边成员上下文）
    let mut found_hyperedge_context = false;
    for hit in &result.hits {
        let via_names: Vec<&str> = hit.via_entities.iter().map(|e| e.name.as_str()).collect();
        let has_zhangsan = via_names.contains(&"张三");
        let has_beijing = via_names.contains(&"北京");
        let has_tencent = via_names.contains(&"腾讯");
        if has_zhangsan && has_beijing && has_tencent {
            found_hyperedge_context = true;
            break;
        }
    }
    assert!(
        found_hyperedge_context,
        "至少一个 hit 的 via_entities 应含全部 3 个 entity（张三/北京/腾讯），\
         证明超边激活注入了完整的超边成员上下文，实际 hits: {:?}",
        result
            .hits
            .iter()
            .map(|h| (h.event_id.clone(), h.via_entities.iter().map(|e| e.name.clone()).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    );
}
