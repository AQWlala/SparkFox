//! Sub-Step 11.4.2 — EntityEditDrawer IPC 持久化（merge / split / rename）测试
//!
//! ## 测试目标（spec §三 11.4.2）
//! 验证 `sparkfox_knowledge::entity_ops` 模块的 3 个 free function：
//! 1. [`merge_entities`]：合并实体（转移关系 + 删除 source）
//! 2. [`split_entity`]：拆分实体（新建实体 + round-robin 分配关系）
//! 3. [`rename_entity`]：重命名实体（更新 name + normalized_name）
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

use sparkfox_knowledge::entity_ops::{merge_entities, rename_entity, split_entity};
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
