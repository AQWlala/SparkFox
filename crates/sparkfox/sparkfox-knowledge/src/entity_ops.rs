//! Sub-Step 11.4.2 / 12.4.1 — 实体编辑操作（merge / split / rename）free function 模块
//!
//! ## 职责
//! 提供直接操作 entity 表 + event_entity_relation 表的 free function：
//! - [`merge_entities`] / [`merge_entities_with_conflict_report`]：合并实体（转移关系 + 删除 source）
//! - [`split_entity`] / [`split_entity_with_strategy`]：拆分实体（新建实体 + 分配关系）
//! - [`rename_entity`]：重命名实体（更新 name + normalized_name）
//! - [`SplitStrategy`]：拆分策略枚举（RoundRobin / ByEntityType）
//!
//! ## 设计原则
//! - **free function**：不依赖 Tauri runtime，参数为 `&rusqlite::Connection`，
//!   可独立单测（参考 `tests/entity_commands_test.rs`，10 测试不依赖 Tauri State）
//! - **事务原子性**：`merge_entities*` / `split_entity*` 在单个 SQLite 事务中执行，
//!   任一步失败 ROLLBACK；`rename_entity` 是单条 UPDATE，无需显式事务
//! - **不删除 source entity（split）**：保留历史，避免数据丢失
//! - **去重（merge）**：source 与 target 共享同一 event 的关系，merge 前先删除
//!   source 的重复关系，避免 UPDATE 后产生重复 (event_id, entity_id) 对
//! - **向后兼容（12.4.1）**：原 11.4.2 接口（[`merge_entities`] / [`split_entity`]）签名保持不变，
//!   内部委托给 12.4.1 新增的增强版函数；Tauri command 层无需改动
//!
//! ## 调用关系
//! ```text
//! 前端 EntityEditDrawer.tsx → invoke('entity_merge' | 'entity_split' | 'entity_rename')
//!   ↓
//! sparkfox-ipc::commands::entity_merge / entity_split / entity_rename（#[tauri::command]）
//!   ↓
//! sparkfox_knowledge::entity_ops::merge_entities / split_entity / rename_entity（本模块）
//!   ↓（12.4.1 内部委托）
//! merge_entities_with_conflict_report / split_entity_with_strategy（增强版）
//!   ↓
//! rusqlite::Connection（SQLite）
//! ```
//!
//! ## AGPL-3.0-only License

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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

/// 查询 source entity 的所有 event_entity_relation（按 event_id 排序，确保分配稳定可复现）
///
/// 12.4.1 调整：返回 `(id, event_id)` 两列（11.4.2 仅返回 `id`），
/// 因为 [`SplitStrategy::ByEntityType`] 需要 `event_id` 查询签名。
const SELECT_SOURCE_RELATIONS_SQL: &str =
    "SELECT id, event_id FROM event_entity_relation WHERE entity_id = ? ORDER BY event_id ASC";

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

/// Sub-Step 12.4.1：查询 source 和 target 都关联的 event_id 列表（冲突检测）
///
/// 参数顺序：`[source_id, target_id]`
/// - 第 1 个 `?` = source_id：外层查询的 entity_id
/// - 第 2 个 `?` = target_id：子查询的 entity_id（找出 target 已关联的 event）
///
/// 返回冲突的 event_id 列表（source 和 target 都关联的 event），
/// 由 [`merge_entities_with_conflict_report`] 用于冲突报告。
const SELECT_CONFLICT_EVENT_IDS_SQL: &str = r#"
SELECT event_id FROM event_entity_relation
WHERE entity_id = ?
  AND event_id IN (
      SELECT event_id FROM event_entity_relation WHERE entity_id = ?
  )
"#;

/// Sub-Step 12.4.1：查询某个 event 中除 source 外的其他 entity 的 entity_type_id（去重 + 排除 source 自身类型）
///
/// 参数顺序：`[event_id, source_id, source_type_id]`
/// - 第 1 个 `?` = event_id：要查询的 event
/// - 第 2 个 `?` = source_id：被拆分的 source entity（排除自身）
/// - 第 3 个 `?` = source_type_id：source 的 entity_type_id（排除同类型，避免签名退化为常量）
///
/// 用于 [`SplitStrategy::ByEntityType`]：将 event 中已有的非 source 类型作为签名，
/// 同签名的 event 分配给同一新 entity，实现按类型聚类。
const SELECT_EVENT_OTHER_ENTITY_TYPES_SQL: &str = r#"
SELECT DISTINCT e.entity_type_id
FROM event_entity_relation r
JOIN entity e ON r.entity_id = e.id
WHERE r.event_id = ?
  AND r.entity_id != ?
  AND e.entity_type_id != ?
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
// Sub-Step 12.4.2 — 重命名全局影响预览 / 事务执行 SQL 常量
// ---------------------------------------------------------------------------

/// 查询 entity 当前 name（重命名影响预览 / 执行前需读取旧 name）
///
/// 返回单行单列（entity.name），若 entity_id 不存在则 [`rusqlite::Error::QueryReturnedNoRows`]，
/// 由调用方映射为 [`Error::NotFound`]。
const SELECT_ENTITY_NAME_SQL: &str = "SELECT name FROM entity WHERE id = ?";

/// 统计 event_entity_relation 中引用该 entity 的关系行数（affected_relations）
///
/// 重命名仅修改 entity.name，entity_id 不变，因此关系行本身不需要 UPDATE。
/// 但关系行数反映了「受影响的关系数量」（外键关联自动反映新 name）。
const COUNT_AFFECTED_RELATIONS_SQL: &str =
    "SELECT COUNT(*) FROM event_entity_relation WHERE entity_id = ?";

/// 统计受影响的 event 数量（affected_events）
///
/// 通过 event_entity_relation JOIN 计算去重后的 event_id 数量。
/// 同一 event 可能被多条 relation 引用（如多 entity 共同参与一个 event），
/// 故用 `COUNT(DISTINCT event_id)` 去重。
const COUNT_AFFECTED_EVENTS_SQL: &str =
    "SELECT COUNT(DISTINCT event_id) FROM event_entity_relation WHERE entity_id = ?";

/// 统计受影响的 chunks（affected_chunks）数量
///
/// 项目 schema 无独立 chunks 表，[`knowledge_event`] 的 `content` / `summary` / `title`
/// 三列作为 chunk_text 全文索引的代理字段（事件正文 + 摘要 + 标题）。
///
/// 使用 `instr(text, ?) > 0` 而非 `LIKE '%?%'`，避免 old_name 含 `%` / `_` 通配符时
/// 误匹配（instr 是字节级查找，无通配符语义，更安全）。
///
/// 参数：`[old_name, old_name, old_name]`（content / summary / title 各一个）
const COUNT_AFFECTED_CHUNKS_SQL: &str = r#"
SELECT COUNT(*) FROM knowledge_event
WHERE instr(content, ?) > 0
   OR instr(summary, ?) > 0
   OR instr(title, ?) > 0
"#;

/// UPDATE knowledge_event.content（执行重命名时，将 content 中旧 name 替换为新 name）
///
/// SQLite `REPLACE(X, Y, Z)`：在 X 中查找 Y，替换为 Z。
///
/// 参数：`[old_name, new_name, old_name]`
/// - 第 1 个 `?` = old_name：REPLACE 的 Y 参数（要被替换的旧值）
/// - 第 2 个 `?` = new_name：REPLACE 的 Z 参数（替换为的新值）
/// - 第 3 个 `?` = old_name：WHERE 子句的 instr 查找（仅更新含旧 name 的行）
const UPDATE_EVENT_CONTENT_SQL: &str =
    "UPDATE knowledge_event SET content = REPLACE(content, ?, ?) WHERE instr(content, ?) > 0";

/// UPDATE knowledge_event.summary（执行重命名时，将 summary 中旧 name 替换为新 name）
///
/// 参数顺序同 [`UPDATE_EVENT_CONTENT_SQL`]：`[old_name, new_name, old_name]`
const UPDATE_EVENT_SUMMARY_SQL: &str =
    "UPDATE knowledge_event SET summary = REPLACE(summary, ?, ?) WHERE instr(summary, ?) > 0";

/// UPDATE knowledge_event.title（执行重命名时，将 title 中旧 name 替换为新 name）
///
/// 参数顺序同 [`UPDATE_EVENT_CONTENT_SQL`]：`[old_name, new_name, old_name]`
const UPDATE_EVENT_TITLE_SQL: &str =
    "UPDATE knowledge_event SET title = REPLACE(title, ?, ?) WHERE instr(title, ?) > 0";

// ---------------------------------------------------------------------------
// merge_entities / merge_entities_with_conflict_report — 合并实体
// ---------------------------------------------------------------------------

/// 合并实体（增强版，spec §三 12.4.1）
///
/// 在 11.4.2 [`merge_entities`] 基础上新增**冲突检测 + 冲突报告**：
/// 1. **冲突检测**：source 和 target 都关联同一 event 时，识别为冲突
/// 2. **冲突去重**：删除 source 的冲突关系（保留 target 的），避免 UPDATE 后产生重复对
/// 3. **冲突报告**：返回冲突的 event_id 列表，供调用方做审计 / UI 提示
///
/// ## 流程（单事务原子性）
/// 1. 校验 `source_id != target_id`（自合并是 no-op 且会误删 source）
/// 2. **查询冲突 event_id 列表**（source 和 target 都关联的 event）— 12.4.1 新增
/// 3. 删除 source 与 target 重复的关系（保留 target 的，避免 UPDATE 后重复）
/// 4. UPDATE event_entity_relation SET entity_id = target_id WHERE entity_id = source_id
/// 5. DELETE FROM entity WHERE id = source_id
/// 6. 返回步骤 2 查询到的冲突 event_id 列表
///
/// ## 参数
/// - `conn`：SQLite 连接（调用方持有，便于复用 + 单测注入 in-memory DB）
/// - `source_id`：被合并的实体 ID（合并后删除）
/// - `target_id`：合并目标的实体 ID（保留）
///
/// ## 返回
/// 成功返回 `Ok(Vec<String>)`（冲突的 event_id 列表，可能为空）；
/// 失败返回 `Err(Error)`（事务自动 ROLLBACK）
///
/// ## 与 [`merge_entities`] 的关系
/// [`merge_entities`] 是本函数的薄包装（丢弃返回的冲突列表），
/// 保留原 11.4.2 签名以保证 Tauri command 层 `entity_merge` 向后兼容。
pub fn merge_entities_with_conflict_report(
    conn: &Connection,
    source_id: &str,
    target_id: &str,
) -> Result<Vec<String>> {
    // 自合并保护：source_id == target_id 时直接返回错误（避免误删 source）
    if source_id == target_id {
        return Err(Error::invalid_argument(
            "source_id 与 target_id 不能相同".to_string(),
            "merge_entities_with_conflict_report",
        ));
    }

    conn.execute_batch("BEGIN")?;
    let result = merge_entities_with_conflict_report_inner(conn, source_id, target_id);
    match result {
        Ok(conflicts) => {
            conn.execute_batch("COMMIT")?;
            Ok(conflicts)
        }
        Err(e) => {
            // 回滚事务（忽略回滚自身的错误）
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// merge_entities_with_conflict_report 内部实现（在事务内执行）
///
/// 返回冲突的 event_id 列表（可能为空）。
fn merge_entities_with_conflict_report_inner(
    conn: &Connection,
    source_id: &str,
    target_id: &str,
) -> Result<Vec<String>> {
    // 1. 查询冲突的 event_id 列表（source 和 target 都关联的 event）— 12.4.1 冲突检测
    //    必须在 DELETE 之前查询，否则 source 的关系已被删除，无法检测冲突。
    //    使用 block scope 提前释放 prepared statement 的 borrow，避免与后续 DELETE/UPDATE 冲突
    let conflict_event_ids: Vec<String> = {
        let mut stmt = conn.prepare(SELECT_CONFLICT_EVENT_IDS_SQL)?;
        let rows = stmt.query_map(
            rusqlite::params![source_id, target_id],
            |row| row.get::<_, String>(0),
        )?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // 2. 删除 source 与 target 重复的关系（保留 target 的，避免 UPDATE 后产生重复对）
    conn.execute(
        DELETE_DUPLICATE_RELATIONS_SQL,
        rusqlite::params![source_id, target_id],
    )?;

    // 3. 转移 source 的剩余关系到 target
    conn.execute(
        TRANSFER_RELATIONS_SQL,
        rusqlite::params![target_id, source_id],
    )?;

    // 4. 删除 source entity
    conn.execute(DELETE_ENTITY_SQL, rusqlite::params![source_id])?;

    // 5. 返回冲突的 event_id 列表（供调用方做审计 / UI 提示）
    Ok(conflict_event_ids)
}

/// 合并实体：将 source_entity 的所有关系转移到 target_entity，然后删除 source_entity
///
/// ## 向后兼容（11.4.2 → 12.4.1）
/// 12.4.1 重构后，本函数委托给 [`merge_entities_with_conflict_report`]，
/// 丢弃返回的冲突 event_id 列表。原 11.4.2 调用方（含 Tauri command `entity_merge`）
/// 无需改动。
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
    // 12.4.1：委托给增强版，丢弃冲突列表（保持原签名向后兼容）
    merge_entities_with_conflict_report(conn, source_id, target_id).map(|_| ())
}

// ---------------------------------------------------------------------------
// split_entity / split_entity_with_strategy — 拆分实体
// ---------------------------------------------------------------------------

/// 拆分策略（spec §三 12.4.1）
///
/// 控制 [`split_entity_with_strategy`] 如何将 source entity 的关系分配到新实体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitStrategy {
    /// 轮询分配（11.4.2 默认）
    ///
    /// 按 event_id ASC 排序后，将第 i 条关系分配给 `new_entity_ids[i % n]`。
    /// 优点：实现简单、稳定可复现；缺点：忽略 event 的语义信息。
    RoundRobin,

    /// 按实体类型匹配（12.4.1 新增）
    ///
    /// 根据 event 中已有的非 source 实体类型作为「签名」聚类：
    /// - 同签名的 event 分配到同一新 entity（避免相同上下文的 event 被拆散）
    /// - 首次出现的签名 → 顺序分配到下一个新 entity（round-robin over unique signatures）
    /// - 签名数量超过新实体数量时，按 `signature_index % n` 取模回绕
    ///
    /// 适用场景：source entity 是「同名异实」混淆（如两个张三），
    /// 拆分时希望按 event 的上下文类型聚类（如「张三+组织」聚到 A，「张三+地点」聚到 B）。
    ByEntityType,
}

/// 拆分实体（增强版，spec §三 12.4.1）
///
/// 在 11.4.2 [`split_entity`] 基础上新增 [`SplitStrategy`] 策略选择：
/// - [`SplitStrategy::RoundRobin`]：与 11.4.2 行为一致（按 event_id ASC 轮询）
/// - [`SplitStrategy::ByEntityType`]：按 event 中已有的非 source 实体类型聚类分配
///
/// ## 流程（单事务原子性）
/// 1. 校验 `new_names` 非空
/// 2. 查询 source entity 的 entity_type_id（用于继承到新 entity + ByEntityType 排除自身类型）
/// 3. 为每个 new_name 新建 entity（保留 source 的 entity_type_id）
/// 4. 查询 source 的所有 event_entity_relation（按 event_id 升序，确保分配稳定）
/// 5. 根据 `strategy` 分配关系到新 entity：
///    - `RoundRobin`：relation[i] → new_entity_ids[i % n]
///    - `ByEntityType`：按 event 的非 source 类型签名聚类，同签名 → 同新 entity
/// 6. **不删除 source entity**（保留历史）
///
/// ## 参数
/// - `conn`：SQLite 连接
/// - `source_id`：被拆分的实体 ID（保留不删除）
/// - `new_names`：新实体名称列表（不可为空）
/// - `strategy`：拆分策略（见 [`SplitStrategy`]）
///
/// ## 返回
/// 成功返回 `Ok(Vec<String>)`（新建的 entity_id 列表，顺序与 new_names 对应）；
/// 失败返回 `Err(Error)`（事务自动 ROLLBACK）
pub fn split_entity_with_strategy(
    conn: &Connection,
    source_id: &str,
    new_names: &[String],
    strategy: SplitStrategy,
) -> Result<Vec<String>> {
    if new_names.is_empty() {
        return Err(Error::invalid_argument(
            "new_names 不能为空".to_string(),
            "split_entity_with_strategy",
        ));
    }

    conn.execute_batch("BEGIN")?;
    let result = split_entity_with_strategy_inner(conn, source_id, new_names, strategy);
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

/// split_entity_with_strategy 内部实现（在事务内执行）
fn split_entity_with_strategy_inner(
    conn: &Connection,
    source_id: &str,
    new_names: &[String],
    strategy: SplitStrategy,
) -> Result<Vec<String>> {
    // 1. 查询 source entity 的 entity_type_id（用于继承到新 entity + ByEntityType 排除自身类型）
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

    // 3. 查询 source 的所有关系（按 event_id 升序，保证分配稳定可复现）
    //    返回 (relation_id, event_id) 对，ByEntityType 需要 event_id 查询签名
    let relation_rows: Vec<(String, String)> = {
        let mut stmt = conn.prepare(SELECT_SOURCE_RELATIONS_SQL)?;
        let rows = stmt.query_map(rusqlite::params![source_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // 4. 根据 strategy 分配关系到新 entity
    let n = new_names.len();
    match strategy {
        SplitStrategy::RoundRobin => {
            // RoundRobin：relation[i] → new_entity_ids[i % n]
            for (i, (rel_id, _event_id)) in relation_rows.iter().enumerate() {
                let target_entity_id = &new_entity_ids[i % n];
                conn.execute(
                    UPDATE_RELATION_ENTITY_SQL,
                    rusqlite::params![target_entity_id, rel_id],
                )?;
            }
        }
        SplitStrategy::ByEntityType => {
            // ByEntityType：按 event 中已有的非 source 实体类型签名聚类
            //
            // 签名构造：查询 event 中除 source 外的其他 entity 的 entity_type_id（去重 + 排除 source 自身类型），
            // 用逗号拼接作为签名。同签名的 event 分配到同一新 entity。
            //
            // 签名 → 新 entity 索引的映射：首次出现的签名按顺序分配（0, 1, 2, ...），
            // 签名数超过 n 时按 `signature_index % n` 取模回绕。
            let mut signature_to_idx: HashMap<String, usize> = HashMap::new();
            let mut next_idx: usize = 0;

            for (rel_id, event_id) in relation_rows.iter() {
                // 查询 event 中除 source 外的其他 entity 的 entity_type_id
                // （排除 source 自身类型，避免签名退化为常量 default_person,default_person,...）
                let signature = {
                    let mut stmt = conn.prepare(SELECT_EVENT_OTHER_ENTITY_TYPES_SQL)?;
                    let rows = stmt.query_map(
                        rusqlite::params![event_id, source_id, &entity_type_id],
                        |row| row.get::<_, String>(0),
                    )?;
                    let mut types: Vec<String> = rows.filter_map(|r| r.ok()).collect();
                    types.sort(); // 排序保证签名的多类型集合顺序无关
                    types.join(",")
                };

                // 查表得到/分配新 entity 索引
                let idx = *signature_to_idx.entry(signature).or_insert_with(|| {
                    let assigned = next_idx % n;
                    next_idx += 1;
                    assigned
                });

                let target_entity_id = &new_entity_ids[idx];
                conn.execute(
                    UPDATE_RELATION_ENTITY_SQL,
                    rusqlite::params![target_entity_id, rel_id],
                )?;
            }
        }
    }

    // 5. 不删除 source entity（保留历史，spec §三 11.4.2 / 12.4.1 明确要求）
    Ok(new_entity_ids)
}

/// 拆分实体：新建 new_names 对应的实体，round-robin 分配 source entity 的关系
///
/// ## 向后兼容（11.4.2 → 12.4.1）
/// 12.4.1 重构后，本函数等价于 `split_entity_with_strategy(conn, source_id, new_names, SplitStrategy::RoundRobin)`。
/// 原 11.4.2 调用方（含 Tauri command `entity_split`）无需改动。
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
    // 12.4.1：委托给增强版，默认使用 RoundRobin（保持原签名向后兼容）
    split_entity_with_strategy(conn, source_id, new_names, SplitStrategy::RoundRobin)
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
// Sub-Step 12.4.2 — 重命名全局影响预览 + 事务执行
// ---------------------------------------------------------------------------

/// 重命名影响预览结果（spec §三 12.4.2）
///
/// 描述重命名实体后会受影响的全局范围：
/// - [`affected_events`](Self::affected_events)：受影响的 event 数量（通过 event_entity_relation JOIN）
/// - [`affected_relations`](Self::affected_relations)：受影响的 event_entity_relation 行数
/// - [`affected_chunks`](Self::affected_chunks)：受影响的 chunk 数量（knowledge_event.content/summary/title 含旧 name）
///
/// 由 [`preview_entity_rename_impact`]（仅查询不修改）与 [`execute_entity_rename`]（事务执行后返回实际数量）
/// 共用，前端通过对比两个返回值可验证预览准确性。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameImpactPreview {
    /// 受影响的 event 数量（DISTINCT event_id，通过 event_entity_relation JOIN 计算）
    pub affected_events: usize,
    /// 受影响的 event_entity_relation 行数（引用该 entity 的所有关系行）
    pub affected_relations: usize,
    /// 受影响的 chunk 数量（knowledge_event.content/summary/title 含旧 name 的行数，
    /// 项目 schema 无独立 chunks 表，三列作为 chunk_text 全文索引的代理字段）
    pub affected_chunks: usize,
}

/// 查询 entity 当前 name，若不存在返回 [`Error::NotFound`]
///
/// 内部辅助函数，由 [`preview_entity_rename_impact`] / [`execute_entity_rename`] 共用。
/// 返回 `Ok(String)`（旧 name）或 `Err(Error::NotFound)`。
fn query_entity_name(conn: &Connection, entity_id: &str) -> Result<String> {
    conn.query_row(SELECT_ENTITY_NAME_SQL, rusqlite::params![entity_id], |row| {
        row.get::<_, String>(0)
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Error::not_found("entity", entity_id),
        other => Error::Db(other),
    })
}

/// 统计重命名影响（不执行重命名）
///
/// 在执行重命名前预览受影响的 events / relations / chunks 数量，供 UI 显示
/// 「受影响事件: N / 受影响关系: N / 受影响文本块: N」让用户确认后再执行。
///
/// ## 影响范围
/// - **affected_events**：通过 event_entity_relation JOIN 计算 DISTINCT event_id 数量
///   （重命名不修改 entity_id，关系行本身不 UPDATE，但 JOIN 查询会反映新 name）
/// - **affected_relations**：event_entity_relation 中引用该 entity 的行数
///   （外键关联，重命名后这些关系自动指向新 name）
/// - **affected_chunks**：knowledge_event 中 content / summary / title 含旧 name 的行数
///   （这些行的文本需要 REPLACE 更新，等效于「全文索引重建」）
///
/// ## 参数
/// - `conn`：SQLite 连接
/// - `entity_id`：要重命名的实体 ID（必须存在，否则返回 [`Error::NotFound`]）
/// - `new_name`：新名称（预览阶段不使用，仅用于日志；保留参数为与 [`execute_entity_rename`] 签名对称）
///
/// ## 返回
/// 成功返回 `Ok(RenameImpactPreview)`；失败返回 `Err(Error)`
///
/// ## 副作用
/// **无**（纯 SELECT 查询，不修改任何数据）
pub fn preview_entity_rename_impact(
    conn: &Connection,
    entity_id: &str,
    new_name: &str,
) -> Result<RenameImpactPreview> {
    // 1. 查询 entity 当前 name（用于统计 chunk_text 含旧 name 的行数）
    //    若 entity_id 不存在，返回 NotFound 错误（前置校验）
    let old_name = query_entity_name(conn, entity_id)?;

    // 2. 统计 affected_relations（引用该 entity 的关系行数）
    let affected_relations: i64 = conn.query_row(
        COUNT_AFFECTED_RELATIONS_SQL,
        rusqlite::params![entity_id],
        |row| row.get(0),
    )?;

    // 3. 统计 affected_events（DISTINCT event_id，通过 event_entity_relation 计算）
    let affected_events: i64 = conn.query_row(
        COUNT_AFFECTED_EVENTS_SQL,
        rusqlite::params![entity_id],
        |row| row.get(0),
    )?;

    // 4. 统计 affected_chunks（content/summary/title 含旧 name 的 knowledge_event 行数）
    //    使用 instr() 而非 LIKE，避免 old_name 含 % / _ 通配符时误匹配
    let affected_chunks: i64 = conn.query_row(
        COUNT_AFFECTED_CHUNKS_SQL,
        rusqlite::params![&old_name, &old_name, &old_name],
        |row| row.get(0),
    )?;

    log::debug!(
        "preview_entity_rename_impact: entity_id={}, new_name={}, old_name={}, \
         affected_events={}, affected_relations={}, affected_chunks={}",
        entity_id,
        new_name,
        old_name,
        affected_events,
        affected_relations,
        affected_chunks
    );

    Ok(RenameImpactPreview {
        affected_events: affected_events as usize,
        affected_relations: affected_relations as usize,
        affected_chunks: affected_chunks as usize,
    })
}

/// 执行重命名（事务原子性，spec §三 12.4.2）
///
/// 在单个 SQLite 事务中执行：
/// 1. 查询 entity 当前 name（old_name），若不存在返回 [`Error::NotFound`]（事务回滚）
/// 2. UPDATE entity SET name = new_name, normalized_name = ..., updated_time = ... WHERE id = ?
/// 3. UPDATE knowledge_event.content = REPLACE(content, old_name, new_name) WHERE instr(content, old_name) > 0
/// 4. UPDATE knowledge_event.summary = REPLACE(summary, old_name, new_name) WHERE instr(summary, old_name) > 0
/// 5. UPDATE knowledge_event.title = REPLACE(title, old_name, new_name) WHERE instr(title, old_name) > 0
/// 6. 统计实际受影响数量并返回 [`RenameImpactPreview`]
///
/// 任一步失败则 ROLLBACK，所有修改回滚（entity.name + knowledge_event 文本均不变）。
///
/// ## 设计决策
/// - **事务原子性**：3 步 UPDATE 在同一事务中，保证 entity.name 与 knowledge_event 文本同步更新
///   （避免出现 entity.name 已更新但 chunk_text 仍含旧 name 的不一致状态）
/// - **不更新 event_entity_relation**：表通过 entity_id 外键引用 entity，重命名不修改 entity_id，
///   关系行自动反映新 name（无需 UPDATE，减少事务开销）
/// - **chunk_text 代理字段**：项目 schema 无独立 chunks 表，使用 knowledge_event 的
///   content / summary / title 三列作为 chunk_text 全文索引的代理字段
/// - **instr() 而非 LIKE**：避免 old_name 含 `%` / `_` 通配符时误匹配
///
/// ## 参数
/// - `conn`：SQLite 连接
/// - `entity_id`：要重命名的实体 ID（必须存在，否则返回 [`Error::NotFound`]）
/// - `new_name`：新名称（将自动归一化为 normalized_name = trim + lowercase）
///
/// ## 返回
/// 成功返回 `Ok(RenameImpactPreview)`（含实际受影响数量，应与 [`preview_entity_rename_impact`] 一致）；
/// 失败返回 `Err(Error)`（事务自动 ROLLBACK）
pub fn execute_entity_rename(
    conn: &Connection,
    entity_id: &str,
    new_name: &str,
) -> Result<RenameImpactPreview> {
    conn.execute_batch("BEGIN")?;
    let result = execute_entity_rename_inner(conn, entity_id, new_name);
    match result {
        Ok(preview) => {
            conn.execute_batch("COMMIT")?;
            Ok(preview)
        }
        Err(e) => {
            // 回滚事务（忽略回滚自身的错误）
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// execute_entity_rename 内部实现（在事务内执行）
///
/// 流程见 [`execute_entity_rename`] 文档。返回实际受影响数量。
fn execute_entity_rename_inner(
    conn: &Connection,
    entity_id: &str,
    new_name: &str,
) -> Result<RenameImpactPreview> {
    // 1. 查询 entity 当前 name（old_name）
    //    若 entity_id 不存在，返回 NotFound 错误（事务将由外层 ROLLBACK）
    let old_name = query_entity_name(conn, entity_id)?;

    // 2. 统计 affected_relations / affected_events / affected_chunks（在 UPDATE 之前统计，
    //    因为 UPDATE knowledge_event 后 chunk_text 已被 REPLACE，instr() 找不到旧 name）
    let affected_relations: i64 = conn.query_row(
        COUNT_AFFECTED_RELATIONS_SQL,
        rusqlite::params![entity_id],
        |row| row.get(0),
    )?;
    let affected_events: i64 = conn.query_row(
        COUNT_AFFECTED_EVENTS_SQL,
        rusqlite::params![entity_id],
        |row| row.get(0),
    )?;
    let affected_chunks: i64 = conn.query_row(
        COUNT_AFFECTED_CHUNKS_SQL,
        rusqlite::params![&old_name, &old_name, &old_name],
        |row| row.get(0),
    )?;

    // 3. UPDATE entity SET name = new_name, normalized_name = ..., updated_time = ... WHERE id = ?
    //    normalized_name 与 DefaultEntityNormalizer 一致：trim + lowercase
    let normalized = new_name.trim().to_lowercase();
    conn.execute(
        RENAME_ENTITY_SQL,
        rusqlite::params![new_name, &normalized, FIXED_TIMESTAMP, entity_id],
    )?;

    // 4. UPDATE knowledge_event.content（REPLACE 旧 name 为新 name）
    //    参数顺序：[old_name, new_name, old_name]（REPLACE 第 2/3 参数 + WHERE instr 参数）
    conn.execute(
        UPDATE_EVENT_CONTENT_SQL,
        rusqlite::params![&old_name, new_name, &old_name],
    )?;

    // 5. UPDATE knowledge_event.summary
    conn.execute(
        UPDATE_EVENT_SUMMARY_SQL,
        rusqlite::params![&old_name, new_name, &old_name],
    )?;

    // 6. UPDATE knowledge_event.title
    conn.execute(
        UPDATE_EVENT_TITLE_SQL,
        rusqlite::params![&old_name, new_name, &old_name],
    )?;

    log::debug!(
        "execute_entity_rename: entity_id={}, old_name={}, new_name={}, \
         affected_events={}, affected_relations={}, affected_chunks={}",
        entity_id,
        old_name,
        new_name,
        affected_events,
        affected_relations,
        affected_chunks
    );

    Ok(RenameImpactPreview {
        affected_events: affected_events as usize,
        affected_relations: affected_relations as usize,
        affected_chunks: affected_chunks as usize,
    })
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
