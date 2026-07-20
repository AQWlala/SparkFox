//! Sub-Step 11.2.2 — MULTI1 检索策略（单跳剪枝，TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.2.2）
//! 1. Multi1Strategy 实现 SearchStrategy trait，name() 返回 "multi1"
//! 2. multi1 检索结果所有 hits 的 hop 均为 1（不扩展到 2/3 跳）
//! 3. multi1 比 multi 快（性能对比，1k events fixture，multi1 耗时 < multi 耗时 × 0.8）
//! 4. multi1 仅返回 hop=1 的 hits（与 multi 区分，multi 会返回 hop=1/2/3）
//!
//! ## 设计要点
//! multi1 = max_hop=1 的 MULTI 策略，仅扩展 1 跳（等价于 ATOMIC 检索但保留 MULTI
//! 的 8 步骨架和 thought_process）。通过委托内部 [`MultiStrategy`]（max_hop=1）实现。
//!
//! ## 测试 fixture
//! - **3 跳小图**（`setup_multi_hop_db`）：4 entity / 3 event / 6 relation，验证 hop 字段
//! - **1k events 大图**（`setup_1k_events_db`）：100 entity / 1000 event / 2000 relation，
//!   中心枢纽拓扑（hub entity 连接所有 events），用于性能对比测试
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::time::Instant;

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::Multi1Strategy;
use sparkfox_knowledge::search::{MultiStrategy, SearchStrategy};

// ---------------------------------------------------------------------------
// Fixture 1：3 跳小图（复用 10.8.2 的链式拓扑）
// ---------------------------------------------------------------------------

/// 构造 BFS 3 跳测试图：
///
/// ```text
/// 张三 ── evt-1 ── 北京 ── evt-2 ── 腾讯 ── evt-3 ── 李四
/// ```
///
/// - 4 个 entity（张三 / 北京 / 腾讯 / 李四）
/// - 3 个 event（evt-1 张三出差 / evt-2 北京天气 / evt-3 腾讯财报）
/// - 6 条 event_entity_relation（每个 event 关联 2 个 entity）
///
/// 查询「张三」期望 BFS 扩展结果：
/// - multi1（max_hop=1）：仅返回 evt-1（hop=1）
/// - multi（max_hop=3）：返回 evt-1（hop=1）/ evt-2（hop=2）/ evt-3（hop=3）
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
// Fixture 2：1k events 大图（中心枢纽拓扑，用于性能对比）
// ---------------------------------------------------------------------------

/// 构造 1k events 性能测试图（中心枢纽拓扑）：
///
/// ```text
/// 张三 ── evt-0 ── hub ── evt-1, evt-2, ..., evt-999
///                    │
///                    └── 每个 evt-i 还关联 ent-((i%98)+2) 形成多跳路径
/// ```
///
/// - 100 个 entity（ent-0=张三 / ent-1=hub / ent-2..ent-99=实体N）
/// - 1000 个 event（evt-0..evt-999）
/// - 2000 条 event_entity_relation：
///   - evt-0 → 张三 + hub（2 条）
///   - evt-i (i=1..999) → hub + ent-((i%98)+2)（2 条 × 999 = 1998 条）
///
/// ## 查询「张三」的 BFS 扩展行为
/// - **multi1（max_hop=1）**：仅返回 evt-0（hop=1），约 1 hit
/// - **multi（max_hop=3）**：
///   - hop=1：evt-0（从张三）
///   - hop=2：evt-1..evt-999（从 hub，999 hits）
///   - hop=3：部分 evt-i（从 ent-2..ent-99 中的非已访问 events）
///   - 总计 ~1000 hits
///
/// ## 性能对比预期
/// multi1 仅做 ~5 次 SQL 查询；multi 做 ~2100 次 SQL 查询（100 entities × 1 + 1000
/// events × 2）。multi1 应比 multi 快至少 20%（留 20% 余量避免 flaky）。
fn setup_1k_events_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 100 个 entity：ent-0=张三（查询入口）/ ent-1=hub（中心枢纽）/ ent-2..ent-99=实体N
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-0", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-1", "default_person", "hub", "hub", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    for i in 2..100 {
        let id = format!("ent-{}", i);
        let name = format!("实体{}", i);
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "default_person", name, name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 1000 个 event（evt-0..evt-999），按分钟递增 created_time 便于稳定排序
    for i in 0..1000 {
        let id = format!("evt-{}", i);
        let title = format!("事件{}", i);
        let summary = format!("事件{}摘要", i);
        let content = format!("事件{}内容", i);
        let minute = i % 60;
        let hour = (i / 60) % 24;
        let created = format!("2026-07-20T{:02}:{:02}:00Z", hour, minute);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, "kb-1", "doc-1", title, summary, content, created, created],
        ).unwrap();
    }

    // event_entity_relation：
    // - evt-0 → 张三 (ent-0) + hub (ent-1)
    // - evt-i (i=1..999) → hub (ent-1) + ent-((i%98)+2)
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-0-0", "evt-0", "ent-0", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-0-1", "evt-0", "ent-1", "2026-07-20T00:00:00Z"],
    ).unwrap();
    for i in 1..1000 {
        let other_ent_idx = (i % 98) + 2; // 2..99
        let rel1_id = format!("rel-{}-1", i);
        let rel2_id = format!("rel-{}-2", i);
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rel1_id, format!("evt-{}", i), "ent-1", "2026-07-20T00:00:00Z"],
        ).unwrap();
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rel2_id, format!("evt-{}", i), format!("ent-{}", other_ent_idx), "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    conn
}

// ---------------------------------------------------------------------------
// 测试 1：multi1 检索结果所有 hits 的 hop 均为 1
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multi1_strategy_expands_only_1_hop() {
    // 验收指标 1：multi1（max_hop=1）检索结果所有 hits 的 hop 均为 1
    //
    // 查询「张三」：
    // - multi1 应仅返回 evt-1（hop=1，张三直接关联）
    // - 不应扩展到 evt-2（hop=2）/ evt-3（hop=3）
    let conn = setup_multi_hop_db();
    let strategy = Multi1Strategy::new(conn);
    let result = strategy
        .search("张三")
        .await
        .expect("search 应成功");

    // 验证策略名
    assert_eq!(result.strategy_name, "multi1");
    assert_eq!(strategy.name(), "multi1");

    // 应至少返回 1 个 hit（evt-1）
    assert!(
        !result.hits.is_empty(),
        "multi1 应返回至少 1 个 hit（evt-1），实际: {:?}",
        result.hits
    );

    // 所有 hit 的 hop 必须为 1（multi1 不扩展到 2/3 跳）
    for hit in &result.hits {
        assert_eq!(
            hit.hop,
            Some(1),
            "multi1 所有 hit 的 hop 应为 1，实际 evt-{} hop={:?}",
            hit.event_id,
            hit.hop
        );
    }

    // evt-1 必须在结果中（张三直接关联）
    let event_ids: Vec<String> = result.hits.iter().map(|h| h.event_id.clone()).collect();
    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "multi1 应返回 evt-1（张三直接关联），实际: {:?}",
        event_ids
    );

    // evt-2 / evt-3 不应在结果中（超出 max_hop=1）
    assert!(
        !event_ids.contains(&"evt-2".to_string()),
        "multi1 不应扩展到 evt-2（hop=2），实际: {:?}",
        event_ids
    );
    assert!(
        !event_ids.contains(&"evt-3".to_string()),
        "multi1 不应扩展到 evt-3（hop=3），实际: {:?}",
        event_ids
    );
}

// ---------------------------------------------------------------------------
// 测试 2：multi1 比 multi 快（性能对比，1k events fixture）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multi1_strategy_faster_than_multi() {
    // 验收指标 2：multi1 比 multi 快（性能对比，1k events fixture）
    //
    // 中心枢纽拓扑下查询「张三」：
    // - multi1（max_hop=1）：仅 evt-0（hop=1），~5 次 SQL 查询
    // - multi（max_hop=3）：~1000 hits，~2100 次 SQL 查询
    //
    // 预期 multi1 耗时 < multi 耗时 × 0.8（留 20% 余量避免 flaky）。
    // 同时验证 multi 返回的 hits 数明显多于 multi1（排除空查询假阳性）。
    let conn1 = setup_1k_events_db();
    let conn2 = setup_1k_events_db();

    let multi1 = Multi1Strategy::new(conn1);
    let multi = MultiStrategy::new(conn2);

    // 计时 multi1
    let t1 = Instant::now();
    let multi1_result = multi1
        .search("张三")
        .await
        .expect("multi1 search 应成功");
    let multi1_time = t1.elapsed();

    // 计时 multi
    let t2 = Instant::now();
    let multi_result = multi
        .search("张三")
        .await
        .expect("multi search 应成功");
    let multi_time = t2.elapsed();

    // 验证 multi1 返回非空（排除空查询假阳性）
    assert!(
        !multi1_result.hits.is_empty(),
        "multi1 应返回至少 1 个 hit（evt-0），实际: {:?}",
        multi1_result.hits
    );

    // 验证 multi 返回的 hits 数明显多于 multi1（证明 multi1 进行了剪枝）
    assert!(
        multi_result.hits.len() > multi1_result.hits.len(),
        "multi ({}) 应返回更多 hits 比 multi1 ({})，否则性能对比无意义",
        multi_result.hits.len(),
        multi1_result.hits.len()
    );

    // 性能断言：multi1 应比 multi 快至少 20%
    // 使用 4/5 整数运算避免浮点比较（multi1_time < multi_time * 4 / 5）
    let threshold = multi_time * 4 / 5;
    assert!(
        multi1_time < threshold,
        "multi1 ({:?}) 应比 multi ({:?}) 快至少 20%（阈值 {:?}），\
         multi1 hits={}, multi hits={}",
        multi1_time,
        multi_time,
        threshold,
        multi1_result.hits.len(),
        multi_result.hits.len()
    );
}

// ---------------------------------------------------------------------------
// 测试 3：multi1 仅返回 hop=1 的 hits（与 multi 区分）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multi1_strategy_returns_hop1_only() {
    // 验收指标 3：multi1 仅返回 hop=1 的 hits，multi 返回 hop=1/2/3
    //
    // 在 3 跳小图上查询「张三」：
    // - multi1：仅 hop=1（evt-1）
    // - multi：hop=1/2/3（evt-1 / evt-2 / evt-3）
    //
    // 通过对比 multi1 与 multi 的 hop 集合，验证 multi1 进行了单跳剪枝。
    let conn1 = setup_multi_hop_db();
    let conn2 = setup_multi_hop_db();

    let multi1 = Multi1Strategy::new(conn1);
    let multi = MultiStrategy::new(conn2);

    let multi1_result = multi1
        .search("张三")
        .await
        .expect("multi1 search 应成功");
    let multi_result = multi
        .search("张三")
        .await
        .expect("multi search 应成功");

    // multi1：hop 集合应仅含 1
    let multi1_hops: HashSet<u8> = multi1_result
        .hits
        .iter()
        .filter_map(|h| h.hop)
        .collect();
    assert!(
        !multi1_hops.is_empty(),
        "multi1 应至少返回 1 个 hit，实际: {:?}",
        multi1_result.hits
    );
    assert_eq!(
        multi1_hops,
        HashSet::from([1u8]),
        "multi1 的 hop 集合应仅含 1，实际: {:?}",
        multi1_hops
    );

    // multi：hop 集合应含 1/2/3
    let multi_hops: HashSet<u8> = multi_result
        .hits
        .iter()
        .filter_map(|h| h.hop)
        .collect();
    assert!(
        multi_hops.contains(&1) && multi_hops.contains(&2) && multi_hops.contains(&3),
        "multi 的 hop 集合应含 1/2/3，实际: {:?}",
        multi_hops
    );

    // multi 的 hits 数应多于 multi1（multi 扩展到了 2/3 跳）
    assert!(
        multi_result.hits.len() > multi1_result.hits.len(),
        "multi ({}) 应返回更多 hits 比 multi1 ({})，\
         因为 multi 扩展到了 hop=2/3",
        multi_result.hits.len(),
        multi1_result.hits.len()
    );

    // multi1 不应包含 hop=2 / hop=3 的 hits
    assert!(
        !multi1_hops.contains(&2) && !multi1_hops.contains(&3),
        "multi1 不应包含 hop=2/3 的 hits，实际 hop 集合: {:?}",
        multi1_hops
    );
}
