//! Sub-Step 11.4.2 — 实体编辑操作（merge / split / rename）free function 模块
//!
//! ## 职责
//! 提供 3 个直接操作 entity 表 + event_entity_relation 表的 free function：
//! - [`merge_entities`]：合并实体（转移关系 + 删除 source）
//! - [`split_entity`]：拆分实体（新建实体 + round-robin 分配关系）
//! - [`rename_entity`]：重命名实体（更新 name + normalized_name）
//!
//! ## 设计原则
//! - **free function**：不依赖 Tauri runtime，参数为 `&rusqlite::Connection`，
//!   可独立单测（参考 `tests/entity_commands_test.rs`，6 测试不依赖 Tauri State）
//! - **事务原子性**：`merge_entities` / `split_entity` 在单个 SQLite 事务中执行，
//!   任一步失败 ROLLBACK；`rename_entity` 是单条 UPDATE，无需显式事务
//! - **不删除 source entity（split）**：保留历史，避免数据丢失
//! - **去重（merge）**：source 与 target 共享同一 event 的关系，merge 前先删除
//!   source 的重复关系，避免 UPDATE 后产生重复 (event_id, entity_id) 对
//!
//! ## 调用关系
//! ```text
//! 前端 EntityEditDrawer.tsx → invoke('entity_merge' | 'entity_split' | 'entity_rename')
//!   ↓
//! sparkfox-ipc::commands::entity_merge / entity_split / entity_rename（#[tauri::command]）
//!   ↓
//! sparkfox_knowledge::entity_ops::merge_entities / split_entity / rename_entity（本模块）
//!   ↓
//! rusqlite::Connection（SQLite）
//! ```
//!
//! ## AGPL-3.0-only License

#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use sparkfox_core::{Error, Result};

// ---------------------------------------------------------------------------
// SQL 常量
// ---------------------------------------------------------------------------

/// 查询 entity 的 entity_type_id（split 时用于继承到新 entity）
const SELECT_ENTITY_TYPE_SQL: &str = "SELECT entity_type_id FROM entity WHERE id = ?";

/// INSERT 新 entity 行（split 时新建实体）
const INSERT_ENTITY_SQL: &str = r#"
INSERT INTO entity (
    id, source_config_id, entity_type_id, name, normalized_name,
    int_value, float_value, datetime_value, bool_value, enum_value,
    value_unit, description, extra_data, created_time, updated_time
) VALUES (?, NULL, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, ?)
"#;

/// 查询 source entity 的所有 event_entity_relation（按 event_id 排序，确保 round-robin 稳定）
const SELECT_SOURCE_RELATIONS_SQL: &str =
    "SELECT id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC";

/// UPDATE event_entity_relation.entity_id（split 时 round-robin 分配）
const UPDATE_RELATION_ENTITY_SQL: &str =
    "UPDATE event_entity_relation SET entity_id = ? WHERE id = ?";

/// 删除 source 与 target 重复的关系（merge 时，source 和 target 都关联同一 event）
///
/// 参数顺序：`[source_id, target_id]`
/// - 第 1 个 `?` = source_id：要删除的 entity_id
/// - 第 2 个 `?` = target_id：子查询的 entity_id（找出 target 已关联的 event）
///
/// 删除的是 SOURCE 的重复关系（保留 TARGET 的），避免 UPDATE 后产生重复对。
const DELETE_DUPLICATE_RELATIONS_SQL: &str = r#"
DELETE FROM event_entity_relation
WHERE entity_id = ? AND event_id IN (
    SELECT event_id FROM event_entity_relation WHERE entity_id = ?
)
"#;

/// 转移 source 的剩余关系到 target（merge 时）
const TRANSFER_RELATIONS_SQL: &str =
    "UPDATE event_entity_relation SET entity_id = ? WHERE entity_id = ?";

/// 删除 entity（merge 时删除 source）
const DELETE_ENTITY_SQL: &str = "DELETE FROM entity WHERE id = ?";

/// 重命名 entity（更新 name + normalized_name + updated_time）
const RENAME_ENTITY_SQL: &str =
    "UPDATE entity SET name = ?, normalized_name = ?, updated_time = ? WHERE id = ?";

/// v1.1.0 测试用固定时间戳（生产环境切换为真实时间戳，与 saver.rs 保持一致）
const FIXED_TIMESTAMP: &str = "2026-07-20T00:00:00Z";

// ---------------------------------------------------------------------------
// merge_entities — 合并实体
// ---------------------------------------------------------------------------

/// 合并实体：将 source_entity 的所有关系转移到 target_entity，然后删除 source_entity
///
/// ## 流程（单事务原子性）
/// 1. 校验 `source_id != target_id`（自合并是 no-op 且会误删 source）
/// 2. 删除 source 与 target 重复的关系（保留 target 的，避免 UPDATE 后重复）
/// 3. UPDATE event_entity_relation SET entity_id = target_id WHERE entity_id = source_id
/// 4. DELETE FROM entity WHERE id = source_id
///
/// ## 参数
/// - `conn`：SQLite 连接（调用方持有，便于复用 + 单测注入 in-memory DB）
/// - `source_id`：被合并的实体 ID（合并后删除）
/// - `target_id`：合并目标的实体 ID（保留）
///
/// ## 返回
/// 成功返回 `Ok(())`；失败返回 `Err(Error)`（事务自动 ROLLBACK）
///
/// ## 示例
/// ```ignore
/// use sparkfox_knowledge::entity_ops::merge_entities;
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// // ... 建表 + 插入数据 ...
/// merge_entities(&conn, "ent-source", "ent-target")?;
/// ```
pub fn merge_entities(conn: &Connection, source_id: &str, target_id: &str) -> Result<()> {
    // 自合并保护：source_id == target_id 时直接返回错误（避免误删 source）
    if source_id == target_id {
        return Err(Error::invalid_argument(
            "source_id 与 target_id 不能相同".to_string(),
            "merge_entities",
        ));
    }

    conn.execute_batch("BEGIN")?;
    let result = merge_entities_inner(conn, source_id, target_id);
    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT")?;
            Ok(())
        }
        Err(e) => {
            // 回滚事务（忽略回滚自身的错误）
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// merge_entities 内部实现（在事务内执行）
fn merge_entities_inner(conn: &Connection, source_id: &str, target_id: &str) -> Result<()> {
    // 1. 删除 source 与 target 重复的关系（保留 target 的，避免 UPDATE 后产生重复对）
    conn.execute(
        DELETE_DUPLICATE_RELATIONS_SQL,
        rusqlite::params![source_id, target_id],
    )?;

    // 2. 转移 source 的剩余关系到 target
    conn.execute(
        TRANSFER_RELATIONS_SQL,
        rusqlite::params![target_id, source_id],
    )?;

    // 3. 删除 source entity
    conn.execute(DELETE_ENTITY_SQL, rusqlite::params![source_id])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// split_entity — 拆分实体
// ---------------------------------------------------------------------------

/// 拆分实体：新建 new_names 对应的实体，round-robin 分配 source entity 的关系
///
/// ## 流程（单事务原子性）
/// 1. 校验 `new_names` 非空
/// 2. 查询 source entity 的 entity_type_id（用于继承到新 entity）
/// 3. 为每个 new_name 新建 entity（保留 source 的 entity_type_id）
/// 4. 查询 source 的所有 event_entity_relation（按 event_id 升序，确保 round-robin 稳定）
/// 5. round-robin 分配：relation[i] → new_entity_ids[i % new_names.len()]
/// 6. **不删除 source entity**（保留历史）
///
/// ## 参数
/// - `conn`：SQLite 连接
/// - `source_id`：被拆分的实体 ID（保留不删除）
/// - `new_names`：新实体名称列表（不可为空）
///
/// ## 返回
/// 成功返回 `Ok(Vec<String>)`（新建的 entity_id 列表，顺序与 new_names 对应）；
/// 失败返回 `Err(Error)`（事务自动 ROLLBACK）
///
/// ## round-robin 示例
/// ```text
/// source 关联 [evt-1, evt-2, evt-3]，new_names = ["A", "B"]
///   → i=0: evt-1 → new_entity[0] (A)
///   → i=1: evt-2 → new_entity[1] (B)
///   → i=2: evt-3 → new_entity[0] (A)  ← 2 % 2 = 0
/// ```
pub fn split_entity(conn: &Connection, source_id: &str, new_names: &[String]) -> Result<Vec<String>> {
    if new_names.is_empty() {
        return Err(Error::invalid_argument(
            "new_names 不能为空".to_string(),
            "split_entity",
        ));
    }

    conn.execute_batch("BEGIN")?;
    let result = split_entity_inner(conn, source_id, new_names);
    match result {
        Ok(ids) => {
            conn.execute_batch("COMMIT")?;
            Ok(ids)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// split_entity 内部实现（在事务内执行）
fn split_entity_inner(
    conn: &Connection,
    source_id: &str,
    new_names: &[String],
) -> Result<Vec<String>> {
    // 1. 查询 source entity 的 entity_type_id（用于继承到新 entity）
    let entity_type_id: String = conn
        .query_row(SELECT_ENTITY_TYPE_SQL, rusqlite::params![source_id], |row| {
            row.get(0)
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::not_found("entity", source_id),
            other => Error::Db(other),
        })?;

    // 2. 为每个 new_name 新建 entity（生成稳定且唯一的 entity_id）
    let ts_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut new_entity_ids: Vec<String> = Vec::with_capacity(new_names.len());
    for (i, name) in new_names.iter().enumerate() {
        // ID 格式与 saver.rs 保持一致：entity-{ts}-{counter}，便于调试与排查
        let new_id = format!("entity-split-{ts_nanos}-{i}");
        // normalized_name 与 DefaultEntityNormalizer 一致：trim + lowercase
        let normalized = name.trim().to_lowercase();
        conn.execute(
            INSERT_ENTITY_SQL,
            rusqlite::params![&new_id, &entity_type_id, name, &normalized, FIXED_TIMESTAMP, FIXED_TIMESTAMP],
        )?;
        new_entity_ids.push(new_id);
    }

    // 3. 查询 source 的所有关系（按 event_id 升序，保证 round-robin 分配稳定可复现）
    //    使用 block scope 提前释放 prepared statement 的 borrow，避免与后续 UPDATE 冲突
    let relation_ids: Vec<String> = {
        let mut stmt = conn.prepare(SELECT_SOURCE_RELATIONS_SQL)?;
        let rows = stmt.query_map(rusqlite::params![source_id], |row| row.get::<_, String>(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // 4. round-robin 分配：relation[i] → new_entity_ids[i % n]
    let n = new_names.len();
    for (i, rel_id) in relation_ids.iter().enumerate() {
        let target_entity_id = &new_entity_ids[i % n];
        conn.execute(
            UPDATE_RELATION_ENTITY_SQL,
            rusqlite::params![target_entity_id, rel_id],
        )?;
    }

    // 5. 不删除 source entity（保留历史，spec §三 11.4.2 明确要求）
    Ok(new_entity_ids)
}

// ---------------------------------------------------------------------------
// rename_entity — 重命名实体
// ---------------------------------------------------------------------------

/// 重命名实体：修改 entity.name + normalized_name
///
/// ## 流程
/// 1. 计算 `normalized_name = new_name.trim().to_lowercase()`（与 DefaultEntityNormalizer 一致）
/// 2. UPDATE entity SET name = ?, normalized_name = ?, updated_time = ? WHERE id = ?
///
/// ## 参数
/// - `conn`：SQLite 连接
/// - `entity_id`：要重命名的实体 ID
/// - `new_name`：新名称（原始输入，name 列保留原样；normalized_name 列做归一化）
///
/// ## 返回
/// 成功返回 `Ok(())`；失败返回 `Err(Error)`
///
/// ## 注意
/// - entity_id 不变（仅更新 name / normalized_name / updated_time 三列）
/// - updated_time 更新为 `FIXED_TIMESTAMP`（v1.1.0 测试用固定时间戳，
///   生产环境后续切换为真实时间戳）
/// - 单条 UPDATE 无需显式事务（SQLite 自动原子）
pub fn rename_entity(conn: &Connection, entity_id: &str, new_name: &str) -> Result<()> {
    // normalized_name 与 DefaultEntityNormalizer 保持一致：trim + lowercase
    let normalized = new_name.trim().to_lowercase();
    conn.execute(
        RENAME_ENTITY_SQL,
        rusqlite::params![new_name, &normalized, FIXED_TIMESTAMP, entity_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 单元测试 — 验证 SQL 语句本身的正确性（不依赖 Tauri runtime）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};

    /// 构造最小测试 DB：1 个 entity（ent-1，name="张三"）
    fn setup_minimal_db() -> Connection {
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
        conn
    }

    /// merge_entities 自合并应返回 InvalidArgument 错误
    #[test]
    fn test_merge_entities_rejects_self_merge() {
        let conn = setup_minimal_db();
        let result = merge_entities(&conn, "ent-1", "ent-1");
        assert!(result.is_err(), "自合并应返回错误");
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::InvalidArgument { .. }),
            "应为 InvalidArgument 错误，实际: {:?}",
            err
        );
        // 自合并不应删除 entity
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entity WHERE id = ?",
                rusqlite::params!["ent-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "自合并拒绝后 entity 应仍存在");
    }

    /// split_entity 空名称列表应返回 InvalidArgument 错误
    #[test]
    fn test_split_entity_rejects_empty_names() {
        let conn = setup_minimal_db();
        let result = split_entity(&conn, "ent-1", &[]);
        assert!(result.is_err(), "空 new_names 应返回错误");
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::InvalidArgument { .. }),
            "应为 InvalidArgument 错误，实际: {:?}",
            err
        );
    }

    /// split_entity 不存在的 source_id 应返回 NotFound 错误
    #[test]
    fn test_split_entity_rejects_nonexistent_source() {
        let conn = setup_minimal_db();
        let result = split_entity(&conn, "ent-nonexistent", &["新实体".to_string()]);
        assert!(result.is_err(), "不存在的 source_id 应返回错误");
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::NotFound { .. }),
            "应为 NotFound 错误，实际: {:?}",
            err
        );
    }
}
