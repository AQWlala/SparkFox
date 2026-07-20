//! Sub-Step 11.4.2 / 12.4.1 — EntityEditDrawer IPC 持久化（merge / split / rename）测试
//!
//! ## 测试目标
//! ### spec §三 11.4.2（基础版，6 测试）
//! 验证 `sparkfox_knowledge::entity_ops` 模块的 3 个 free function：
//! 1. [`merge_entities`]：合并实体（转移关系 + 删除 source）
//! 2. [`split_entity`]：拆分实体（新建实体 + round-robin 分配关系）
//! 3. [`rename_entity`]：重命名实体（更新 name + normalized_name）
//!
//! ### spec §三 12.4.1（增强版，4 新测试）
//! 4. [`merge_entities_with_conflict_report`]：合并冲突检测 + 去重 + 报告
//! 5. [`SplitStrategy::ByEntityType`]：按实体类型匹配的拆分策略
//! 6. [`split_entity_with_strategy`]：支持策略选择的拆分（向后兼容原 [`split_entity`]）
//!
//! ## 测试不依赖 Tauri runtime
//! 直接调用 free function（参数为 `&rusqlite::Connection`），
//! 不经过 `#[tauri::command]` 宏，避免 Tauri State 注入。
//!
//! ## 测试 fixture
//! ```text
//! ent-source (张三) ── evt-1
//!                  ── evt-2 ── ent-target (李四)
//!                              ── evt-3
//! ```
//!
//! ## AGPL-3.0-only License

#![forbid(unsafe_code)]

use rusqlite::Connection;

// Sub-Step 12.4.1：新增 merge_entities_with_conflict_report / SplitStrategy / split_entity_with_strategy
use sparkfox_knowledge::entity_ops::{
    merge_entities, merge_entities_with_conflict_report, rename_entity, split_entity,
    split_entity_with_strategy, SplitStrategy,
};
use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};

/// 构造测试 fixture：
///
/// ```text
/// ent-source (张三) ── evt-1
///                  ── evt-2 ── ent-target (李四)
///                              ── evt-3
/// ```
///
/// - 2 个 entity：ent-source（张三）/ ent-target（李四）
/// - 3 个 event：evt-1 / evt-2 / evt-3
/// - 4 条 event_entity_relation：
///   - rel-1: (evt-1, ent-source)
///   - rel-2: (evt-2, ent-source)  ← 与 rel-3 共享 evt-2（merge 时应去重）
///   - rel-3: (evt-2, ent-target)
///   - rel-4: (evt-3, ent-target)
fn setup_merge_fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 2 个 entity
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-source", "default_person", "张三", "张三", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-target", "default_person", "李四", "李四", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 3 个 event
    for (eid, title) in [("evt-1", "事件一"), ("evt-2", "事件二"), ("evt-3", "事件三")] {
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![eid, "kb-1", "doc-1", title, title, title, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).unwrap();
    }

    // 4 条 event_entity_relation（evt-2 同时关联 source 和 target，用于测试 merge 去重）
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-1", "evt-1", "ent-source", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-2", "evt-2", "ent-source", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-3", "evt-2", "ent-target", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-4", "evt-3", "ent-target", "2026-07-20T00:00:00Z"],
    ).unwrap();

    conn
}

/// 构造 split 测试 fixture：
///
/// ```text
/// ent-split (王五) ── evt-1
///                 ── evt-2
///                 ── evt-3
/// ```
///
/// - 1 个 entity：ent-split（王五）
/// - 3 个 event：evt-1 / evt-2 / evt-3
/// - 3 条 event_entity_relation（全部关联到 ent-split）
fn setup_split_fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // 1 个 entity
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-split", "default_person", "王五", "王五", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 3 个 event（按 created_time 递增，便于稳定 round-robin 排序）
    for (i, eid) in ["evt-1", "evt-2", "evt-3"].iter().enumerate() {
        let title = format!("事件{}", i + 1);
        let ts = format!("2026-07-20T0{}:00:00Z", i);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![eid, "kb-1", "doc-1", title, title, title, ts, ts],
        ).unwrap();
    }

    // 3 条 event_entity_relation（全部关联到 ent-split，按 event_id 升序）
    for (i, (rid, eid)) in [
        ("rel-1", "evt-1"),
        ("rel-2", "evt-2"),
        ("rel-3", "evt-3"),
    ]
    .iter()
    .enumerate()
    {
        let ts = format!("2026-07-20T0{}:00:00Z", i);
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rid, eid, "ent-split", ts],
        ).unwrap();
    }

    conn
}

/// 构造 rename 测试 fixture：1 个 entity（ent-rename，name="旧名称"）
fn setup_rename_fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-rename", "default_person", "旧名称", "旧名称", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    conn
}

// ---------------------------------------------------------------------------
// merge_entities 测试
// ---------------------------------------------------------------------------

#[test]
fn test_merge_entities_transfers_relations() {
    // 验收：合并后 source 的关系转移到 target
    //
    // fixture:
    //   ent-source ── evt-1
    //              ── evt-2 ── ent-target
    //                          ── evt-3
    //
    // merge(ent-source, ent-target) 后：
    //   - ent-target 应关联 evt-1（从 source 转移）
    //   - ent-target 应关联 evt-2（原已有，去重后保留 1 条）
    //   - ent-target 应关联 evt-3（原已有）
    //   - ent-target 共 3 条关系（不重复）
    let conn = setup_merge_fixture();
    merge_entities(&conn, "ent-source", "ent-target").expect("merge 应成功");

    // 查询 ent-target 的所有关系
    let mut stmt = conn
        .prepare("SELECT event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC")
        .unwrap();
    let event_ids: Vec<String> = stmt
        .query_map(rusqlite::params!["ent-target"], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    assert!(
        event_ids.contains(&"evt-1".to_string()),
        "evt-1 应从 source 转移到 target，实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-2".to_string()),
        "evt-2 应保留（去重后 1 条），实际: {:?}",
        event_ids
    );
    assert!(
        event_ids.contains(&"evt-3".to_string()),
        "evt-3 应保留（target 原有关系），实际: {:?}",
        event_ids
    );
    // 关键：evt-2 不应出现 2 次（去重后只保留 1 条）
    let evt2_count = event_ids.iter().filter(|id| *id == "evt-2").count();
    assert_eq!(
        evt2_count, 1,
        "evt-2 应去重为 1 条关系，实际 {} 条",
        evt2_count
    );
    // ent-target 总共 3 条关系（evt-1 + evt-2 + evt-3）
    assert_eq!(
        event_ids.len(),
        3,
        "ent-target 应有 3 条关系（去重后），实际: {:?}",
        event_ids
    );
}

#[test]
fn test_merge_entities_deletes_source() {
    // 验收：合并后 source entity 被删除
    //
    // merge(ent-source, ent-target) 后：
    //   - entity 表中 ent-source 不存在
    //   - event_entity_relation 中没有 entity_id = ent-source 的记录
    let conn = setup_merge_fixture();
    merge_entities(&conn, "ent-source", "ent-target").expect("merge 应成功");

    // entity 表中 ent-source 应被删除
    let source_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE id = ?",
            rusqlite::params!["ent-source"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_count, 0,
        "merge 后 source entity 应被删除，但仍存在 {} 行",
        source_count
    );

    // event_entity_relation 中不应有 entity_id = ent-source 的记录
    let source_rel_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-source"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_rel_count, 0,
        "merge 后不应有指向 source 的关系，但仍有 {} 条",
        source_rel_count
    );
}

// ---------------------------------------------------------------------------
// split_entity 测试
// ---------------------------------------------------------------------------

#[test]
fn test_split_entity_creates_new_entities() {
    // 验收：拆分后新建 new_names 对应的实体
    //
    // fixture: ent-split (王五) 关联 evt-1 / evt-2 / evt-3
    // split(ent-split, ["王五丰", "王五风"]) 后：
    //   - 新建 2 个 entity，name 分别为 "王五丰" 和 "王五风"
    //   - 新 entity 的 entity_type_id 与 source 一致（default_person）
    //   - source entity (ent-split) 仍存在（不删除）
    //   - 返回的 new_entity_ids 长度为 2
    let conn = setup_split_fixture();
    let new_ids = split_entity(
        &conn,
        "ent-split",
        &["王五丰".to_string(), "王五风".to_string()],
    )
    .expect("split 应成功");

    assert_eq!(
        new_ids.len(),
        2,
        "应新建 2 个 entity，实际: {:?}",
        new_ids
    );

    // 验证新 entity 的 name + entity_type_id
    for (i, expected_name) in ["王五丰", "王五风"].iter().enumerate() {
        let (name, entity_type_id): (String, String) = conn
            .query_row(
                "SELECT name, entity_type_id FROM entity WHERE id = ?",
                rusqlite::params![new_ids[i]],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|_| panic!("新 entity {} 应存在", new_ids[i]));
        assert_eq!(name, *expected_name, "新 entity[{}] 的 name 应为 {}", i, expected_name);
        assert_eq!(
            entity_type_id, "default_person",
            "新 entity 的 entity_type_id 应继承 source（default_person）"
        );
    }

    // source entity 仍存在
    let source_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE id = ?",
            rusqlite::params!["ent-split"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_count, 1,
        "split 后 source entity 应保留（不删除），实际 {} 行",
        source_count
    );
}

#[test]
fn test_split_entity_distributes_relations_round_robin() {
    // 验收：拆分后关系 round-robin 分配到新实体
    //
    // fixture: ent-split 关联 evt-1 / evt-2 / evt-3（按 event_id 升序）
    // split(ent-split, ["A", "B"]) 后（round-robin，n=2）：
    //   - i=0: rel-1 (evt-1) → new_entity[0] (A)
    //   - i=1: rel-2 (evt-2) → new_entity[1] (B)
    //   - i=2: rel-3 (evt-3) → new_entity[0] (A)  ← 2 % 2 = 0
    //
    // 验证：
    //   - new_entity[0] 关联 evt-1 和 evt-3（2 条）
    //   - new_entity[1] 关联 evt-2（1 条）
    //   - source entity (ent-split) 无关系（全部转出）
    let conn = setup_split_fixture();
    let new_ids = split_entity(
        &conn,
        "ent-split",
        &["实体A".to_string(), "实体B".to_string()],
    )
    .expect("split 应成功");
    assert_eq!(new_ids.len(), 2);

    // new_entity[0] 应关联 evt-1 和 evt-3
    let mut stmt = conn
        .prepare("SELECT event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC")
        .unwrap();
    let a_events: Vec<String> = stmt
        .query_map(rusqlite::params![new_ids[0]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    let b_events: Vec<String> = stmt
        .query_map(rusqlite::params![new_ids[1]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    assert_eq!(
        a_events,
        vec!["evt-1".to_string(), "evt-3".to_string()],
        "new_entity[0] 应关联 evt-1 和 evt-3（round-robin: i=0,2 → 0），实际: {:?}",
        a_events
    );
    assert_eq!(
        b_events,
        vec!["evt-2".to_string()],
        "new_entity[1] 应关联 evt-2（round-robin: i=1 → 1），实际: {:?}",
        b_events
    );

    // source entity (ent-split) 无关系
    let source_rel_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-split"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_rel_count, 0,
        "split 后 source entity 应无关系（全部 round-robin 转出），实际 {} 条",
        source_rel_count
    );
}

// ---------------------------------------------------------------------------
// rename_entity 测试
// ---------------------------------------------------------------------------

#[test]
fn test_rename_entity_updates_name() {
    // 验收：重命名后 name + normalized_name 更新
    //
    // fixture: ent-rename (name="旧名称", normalized_name="旧名称")
    // rename(ent-rename, "新名称") 后：
    //   - name = "新名称"
    //   - normalized_name = "新名称"（trim + lowercase，中文不受 lowercase 影响）
    //
    // 边界：rename(ent-rename, "  New Name  ") 后：
    //   - name = "  New Name  "（保留原样，不 trim）
    //   - normalized_name = "new name"（trim + lowercase）
    let conn = setup_rename_fixture();

    // 测试 1：中文重命名
    rename_entity(&conn, "ent-rename", "新名称").expect("rename 应成功");
    let (name, normalized): (String, String) = conn
        .query_row(
            "SELECT name, normalized_name FROM entity WHERE id = ?",
            rusqlite::params!["ent-rename"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "新名称", "name 应更新为「新名称」");
    assert_eq!(
        normalized, "新名称",
        "normalized_name 应为「新名称」（中文 trim + lowercase 不变）"
    );

    // 测试 2：带空格的英文重命名（验证 trim + lowercase 归一化）
    rename_entity(&conn, "ent-rename", "  New Name  ").expect("rename 应成功");
    let (name, normalized): (String, String) = conn
        .query_row(
            "SELECT name, normalized_name FROM entity WHERE id = ?",
            rusqlite::params!["ent-rename"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "  New Name  ", "name 应保留原始输入（含空格）");
    assert_eq!(
        normalized, "new name",
        "normalized_name 应为「new name」（trim + lowercase）"
    );
}

#[test]
fn test_rename_entity_preserves_id() {
    // 验收：重命名后 entity_id 不变
    //
    // rename(ent-rename, "另一个名称") 后：
    //   - entity 表中 ent-rename 的 id 不变
    //   - entity 表中只有 1 行（不是新建一行）
    let conn = setup_rename_fixture();

    // 重命名前：1 行
    let before_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE id = ?",
            rusqlite::params!["ent-rename"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(before_count, 1, "rename 前应有 1 行");

    rename_entity(&conn, "ent-rename", "另一个名称").expect("rename 应成功");

    // 重命名后：仍 1 行（id 不变，不是新建）
    let after_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE id = ?",
            rusqlite::params!["ent-rename"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        after_count, 1,
        "rename 后 id 应不变，仍为 1 行，实际 {} 行",
        after_count
    );

    // entity 表总行数应仍为 1（未新建）
    let total_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entity", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        total_count, 1,
        "rename 不应新建 entity 行，总行数应为 1，实际 {} 行",
        total_count
    );

    // 验证 name 已更新（确认 rename 真正执行）
    let name: String = conn
        .query_row(
            "SELECT name FROM entity WHERE id = ?",
            rusqlite::params!["ent-rename"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "另一个名称", "name 应已更新");
}

// ---------------------------------------------------------------------------
// Sub-Step 12.4.1 — merge_entities_with_conflict_report 测试（冲突检测 + 去重 + 报告）
// ---------------------------------------------------------------------------

#[test]
fn test_merge_entities_with_conflict_report_returns_conflicts() {
    // 验收（spec §三 12.4.1）：冲突检测返回冲突的 event_id 列表
    //
    // fixture:
    //   ent-source ── evt-1
    //              ── evt-2 ── ent-target  ← 冲突：evt-2 同时关联 source 和 target
    //                          ── evt-3
    //
    // merge_entities_with_conflict_report(ent-source, ent-target) 应返回 ["evt-2"]：
    //   - evt-1 仅 source 关联 → 非冲突
    //   - evt-2 同时被 source 和 target 关联 → 冲突
    //   - evt-3 仅 target 关联 → 非冲突
    let conn = setup_merge_fixture();
    let conflicts =
        merge_entities_with_conflict_report(&conn, "ent-source", "ent-target")
            .expect("merge_with_conflict_report 应成功");

    assert_eq!(
        conflicts,
        vec!["evt-2".to_string()],
        "应返回冲突的 event_id 列表（仅 evt-2 冲突），实际: {:?}",
        conflicts
    );

    // source 应已删除
    let source_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE id = ?",
            rusqlite::params!["ent-source"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(source_count, 0, "merge 后 source entity 应被删除");
}

#[test]
fn test_merge_entities_with_conflict_report_deduplicates_relations() {
    // 验收（spec §三 12.4.1）：冲突去重后 target 只保留 1 条关系（不产生重复对）
    //
    // fixture:
    //   ent-source ── evt-1
    //              ── evt-2 ── ent-target  ← 冲突（去重后 target 仅保留 1 条 evt-2 关系）
    //                          ── evt-3
    //
    // merge_with_conflict_report 后 target 应有 3 条关系（evt-1 + evt-2 + evt-3），
    // 其中 evt-2 仅 1 条（去重后保留 target 的，删除 source 的）。
    let conn = setup_merge_fixture();
    let conflicts =
        merge_entities_with_conflict_report(&conn, "ent-source", "ent-target")
            .expect("merge_with_conflict_report 应成功");
    assert_eq!(conflicts, vec!["evt-2".to_string()]);

    // 查询 target 的所有关系（按 event_id 升序）
    let mut stmt = conn
        .prepare(
            "SELECT event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC",
        )
        .unwrap();
    let event_ids: Vec<String> = stmt
        .query_map(rusqlite::params!["ent-target"], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    // target 应有 3 条关系（evt-1 从 source 转移 + evt-2 去重后保留 + evt-3 原有）
    assert_eq!(
        event_ids.len(),
        3,
        "target 应有 3 条关系（去重后），实际: {:?}",
        event_ids
    );

    // evt-2 应仅出现 1 次（去重后保留 target 的，删除 source 的）
    let evt2_count = event_ids.iter().filter(|id| *id == "evt-2").count();
    assert_eq!(
        evt2_count, 1,
        "evt-2 应去重为 1 条关系（保留 target 的），实际 {} 条",
        evt2_count
    );

    // 不应有任何 entity_id = ent-source 的残留关系
    let source_rel_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-source"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_rel_count, 0,
        "不应有指向 source 的残留关系，实际 {} 条",
        source_rel_count
    );
}

// ---------------------------------------------------------------------------
// Sub-Step 12.4.1 — split_entity_with_strategy 测试（ByEntityType 策略）
// ---------------------------------------------------------------------------

/// 构造 ByEntityType 策略测试 fixture：
///
/// ```text
/// ent-split (王五, default_person) ── evt-1 ── ent-org-a  (default_organization)
///                                 ── evt-2 ── ent-loc-a  (default_location)
///                                 ── evt-3 ── ent-org-b  (default_organization)
/// ```
///
/// - 1 个 source entity：ent-split（王五，default_person）
/// - 3 个其他 entity：ent-org-a / ent-org-b（default_organization）、ent-loc-a（default_location）
/// - 3 个 event：evt-1 / evt-2 / evt-3（每个 event 已有 1 个其他类型的 entity）
/// - 6 条 event_entity_relation：ent-split 关联 3 个 event + 3 个其他 entity 各关联 1 个 event
///
/// ByEntityType 策略预期：
/// - evt-1 的其他 entity 类型签名 = "default_organization" → new_entity[0]
/// - evt-2 的其他 entity 类型签名 = "default_location"     → new_entity[1]
/// - evt-3 的其他 entity 类型签名 = "default_organization" → new_entity[0]（与 evt-1 同组）
fn setup_split_by_entity_type_fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).unwrap();
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();

    // source entity（default_person）
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-split", "default_person", "王五", "王五", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    // 3 个其他 entity（不同类型，用于 ByEntityType 签名匹配）
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-org-a", "default_organization", "组织A", "组织a", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-loc-a", "default_location", "地点A", "地点a", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();
    conn.execute(
        "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["ent-org-b", "default_organization", "组织B", "组织b", "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
    ).unwrap();

    // 3 个 event
    for (i, eid) in ["evt-1", "evt-2", "evt-3"].iter().enumerate() {
        let title = format!("事件{}", i + 1);
        let ts = format!("2026-07-20T0{}:00:00Z", i);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![eid, "kb-1", "doc-1", title, title, title, ts, ts],
        ).unwrap();
    }

    // ent-split 关联 3 个 event（按 event_id 升序，确保遍历顺序稳定）
    for (i, (rid, eid)) in [
        ("rel-1", "evt-1"),
        ("rel-2", "evt-2"),
        ("rel-3", "evt-3"),
    ]
    .iter()
    .enumerate()
    {
        let ts = format!("2026-07-20T0{}:00:00Z", i);
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![rid, eid, "ent-split", ts],
        ).unwrap();
    }

    // 其他 entity 各关联 1 个 event（构成 ByEntityType 签名依据）
    // evt-1 → ent-org-a (default_organization)
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-4", "evt-1", "ent-org-a", "2026-07-20T00:00:00Z"],
    ).unwrap();
    // evt-2 → ent-loc-a (default_location)
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-5", "evt-2", "ent-loc-a", "2026-07-20T00:00:00Z"],
    ).unwrap();
    // evt-3 → ent-org-b (default_organization)
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES (?, ?, ?, ?)",
        rusqlite::params!["rel-6", "evt-3", "ent-org-b", "2026-07-20T00:00:00Z"],
    ).unwrap();

    conn
}

#[test]
fn test_split_entity_by_entity_type_strategy() {
    // 验收（spec §三 12.4.1）：ByEntityType 策略按 event 中已有的实体类型匹配分配
    //
    // fixture:
    //   ent-split (default_person) ── evt-1 ── ent-org-a (default_organization)
    //                              ── evt-2 ── ent-loc-a (default_location)
    //                              ── evt-3 ── ent-org-b (default_organization)
    //
    // split_with_strategy(ent-split, ["新A", "新B"], ByEntityType):
    //   - 新A / 新B 均继承 source 的 entity_type_id = default_person
    //   - 按 event 中"已有的非 source 类型"作为签名分组：
    //     * evt-1 签名 "default_organization" → 首次出现 → new_entity[0] (新A)
    //     * evt-2 签名 "default_location"     → 首次出现 → new_entity[1] (新B)
    //     * evt-3 签名 "default_organization" → 已存在  → new_entity[0] (新A，与 evt-1 同组)
    //   - 结果：新A 关联 evt-1 + evt-3；新B 关联 evt-2
    //
    // 与 RoundRobin 的区别：
    //   - RoundRobin 会分配 新A→evt-1, 新B→evt-2, 新A→evt-3（顺序循环）
    //   - ByEntityType 则按类型签名聚类（evt-1 与 evt-3 同组）
    //   两者在 3 个 event / 2 个新实体场景下结果恰好相同，但分组逻辑不同：
    //   - 修改 fixture 使 evt-3 也为 default_location 时，ByEntityType 会把 evt-2+evt-3 都分给新B
    let conn = setup_split_by_entity_type_fixture();
    let new_ids = split_entity_with_strategy(
        &conn,
        "ent-split",
        &["新A".to_string(), "新B".to_string()],
        SplitStrategy::ByEntityType,
    )
    .expect("split_with_strategy ByEntityType 应成功");
    assert_eq!(new_ids.len(), 2, "应新建 2 个 entity");

    // 查询两个新 entity 的关系
    let mut stmt = conn
        .prepare(
            "SELECT event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC",
        )
        .unwrap();
    let a_events: Vec<String> = stmt
        .query_map(rusqlite::params![new_ids[0]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    let b_events: Vec<String> = stmt
        .query_map(rusqlite::params![new_ids[1]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    // ByEntityType 预期：
    //   - new_entity[0] (新A) 关联 evt-1 + evt-3（都是 default_organization 签名）
    //   - new_entity[1] (新B) 关联 evt-2（default_location 签名）
    assert_eq!(
        a_events,
        vec!["evt-1".to_string(), "evt-3".to_string()],
        "ByEntityType: new_entity[0] 应关联 evt-1 + evt-3（同 default_organization 签名），实际: {:?}",
        a_events
    );
    assert_eq!(
        b_events,
        vec!["evt-2".to_string()],
        "ByEntityType: new_entity[1] 应关联 evt-2（default_location 签名），实际: {:?}",
        b_events
    );

    // source entity 应无关系（全部转出）
    let source_rel_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-split"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_rel_count, 0,
        "split 后 source entity 应无关系（全部转出），实际 {} 条",
        source_rel_count
    );

    // 其他 entity 的关系应保持不变（未被误改）
    let org_a_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-org-a"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(org_a_count, 1, "ent-org-a 应仍关联 evt-1（未被误改）");
    let loc_a_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-loc-a"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(loc_a_count, 1, "ent-loc-a 应仍关联 evt-2（未被误改）");
    let org_b_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-org-b"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(org_b_count, 1, "ent-org-b 应仍关联 evt-3（未被误改）");
}

#[test]
fn test_split_entity_round_robin_backward_compatible() {
    // 验收（spec §三 12.4.1）：原 split_entity 接口仍使用 RoundRobin（向后兼容）
    //
    // 12.4.1 重构后 split_entity 委托给 split_entity_with_strategy(..., RoundRobin)，
    // 必须保持与 11.4.2 原实现等价的 round-robin 分配行为：
    //   - relation[i] → new_entity[i % n]
    //
    // fixture: ent-split 关联 evt-1 / evt-2 / evt-3（按 event_id 升序）
    // split(ent-split, ["A", "B"]) 后（n=2，round-robin）：
    //   - i=0: rel-1 (evt-1) → new_entity[0] (A)
    //   - i=1: rel-2 (evt-2) → new_entity[1] (B)
    //   - i=2: rel-3 (evt-3) → new_entity[0] (A)  ← 2 % 2 = 0
    let conn = setup_split_fixture();
    let new_ids = split_entity(
        &conn,
        "ent-split",
        &["实体A".to_string(), "实体B".to_string()],
    )
    .expect("split_entity（向后兼容）应成功");
    assert_eq!(new_ids.len(), 2, "应新建 2 个 entity");

    // 验证 round-robin 分配
    let mut stmt = conn
        .prepare(
            "SELECT event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC",
        )
        .unwrap();
    let a_events: Vec<String> = stmt
        .query_map(rusqlite::params![new_ids[0]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    let b_events: Vec<String> = stmt
        .query_map(rusqlite::params![new_ids[1]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    // round-robin：new_entity[0] 关联 evt-1 + evt-3；new_entity[1] 关联 evt-2
    assert_eq!(
        a_events,
        vec!["evt-1".to_string(), "evt-3".to_string()],
        "向后兼容: new_entity[0] 应关联 evt-1 + evt-3（round-robin i=0,2 → 0），实际: {:?}",
        a_events
    );
    assert_eq!(
        b_events,
        vec!["evt-2".to_string()],
        "向后兼容: new_entity[1] 应关联 evt-2（round-robin i=1 → 1），实际: {:?}",
        b_events
    );

    // source entity 应无关系（全部转出）
    let source_rel_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?",
            rusqlite::params!["ent-split"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_rel_count, 0,
        "向后兼容: split 后 source entity 应无关系，实际 {} 条",
        source_rel_count
    );

    // 额外验证：split_entity 与 split_entity_with_strategy(..., RoundRobin) 行为一致
    // （在新 fixture 上分别调用，比较 event_id 分布）
    let conn2 = setup_split_fixture();
    let ids2 = split_entity_with_strategy(
        &conn2,
        "ent-split",
        &["实体A".to_string(), "实体B".to_string()],
        SplitStrategy::RoundRobin,
    )
    .expect("split_with_strategy RoundRobin 应成功");

    let mut stmt2 = conn2
        .prepare(
            "SELECT event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC",
        )
        .unwrap();
    let a_events2: Vec<String> = stmt2
        .query_map(rusqlite::params![ids2[0]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    let b_events2: Vec<String> = stmt2
        .query_map(rusqlite::params![ids2[1]], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt2);

    // 两个接口的分布必须完全一致（向后兼容的核心保证）
    assert_eq!(
        a_events, a_events2,
        "split_entity 与 split_entity_with_strategy(RoundRobin) 的 new_entity[0] 分布必须一致"
    );
    assert_eq!(
        b_events, b_events2,
        "split_entity 与 split_entity_with_strategy(RoundRobin) 的 new_entity[1] 分布必须一致"
    );
}
