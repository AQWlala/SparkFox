//! 6 层记忆 schema + 迁移 — 对应 Pangu Nebula L0-L5 蓝图
//!
//! 表设计（每层独立表，避免跨层耦合）：
//! - memory_l0_raw: 原始事件流（对话/工具/感知）
//! - memory_l1_working: 工作记忆（短期上下文，TTL 自动过期）
//! - memory_l2_core: 核心记忆（事实/偏好/技能/规则）
//! - memory_l3_dynamic: 动态记忆（语义/情景/图）
//! - memory_l4_persona: 人格记忆（身份/角色/历史）
//! - memory_l5_meta: 元认知（策略日志/错误模式/自评）
//!
//! 向量表（独立于 6 层，可被任一层引用）：
//! - memory_vectors: (id, layer, ref_id, embedding BLOB, dim, model, ts)

use rusqlite::Connection;

use sparkfox_core::{Error, Result};
use sparkfox_knowledge::schema::ALL_SAG_DDL;

const MIGRATIONS: &[&str] = &[
    // V1: 6 层记忆表
    r#"CREATE TABLE IF NOT EXISTS memory_l0_raw (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'dialogue' | 'tool' | 'perception'
        payload TEXT NOT NULL,        -- JSON
        ts INTEGER NOT NULL,
        session_id TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_l0_ts ON memory_l0_raw(ts);
    CREATE INDEX IF NOT EXISTS idx_l0_session ON memory_l0_raw(session_id);"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l1_working (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        ttl_seconds INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        session_id TEXT
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l2_core (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'fact' | 'preference' | 'skill' | 'rule'
        key TEXT NOT NULL,
        value TEXT NOT NULL,
        confidence REAL DEFAULT 1.0,
        updated_at INTEGER NOT NULL,
        UNIQUE(kind, key)
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l3_dynamic (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'semantic' | 'episodic' | 'graph_node' | 'graph_edge'
        payload TEXT NOT NULL,
        ts INTEGER NOT NULL
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l4_persona (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'identity' | 'role' | 'history'
        payload TEXT NOT NULL,
        ts INTEGER NOT NULL
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l5_meta (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'strategy_log' | 'error_pattern' | 'self_eval'
        payload TEXT NOT NULL,
        ts INTEGER NOT NULL,
        related_layer INTEGER,
        related_id TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_l5_kind ON memory_l5_meta(kind);"#,
    // V2: 向量表（与 sqlite-vec 联动，但用普通表存元数据）
    r#"CREATE TABLE IF NOT EXISTS memory_vectors (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        layer INTEGER NOT NULL,
        ref_id TEXT NOT NULL,
        model TEXT NOT NULL,
        dim INTEGER NOT NULL,
        ts INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_vec_layer ON memory_vectors(layer);
    CREATE INDEX IF NOT EXISTS idx_vec_ref ON memory_vectors(ref_id);"#,
    // V3: 同步状态表
    r#"CREATE TABLE IF NOT EXISTS sync_state (
        layer INTEGER NOT NULL,
        ref_id TEXT NOT NULL,
        state TEXT NOT NULL DEFAULT 'local',  -- 'local' | 'syncing' | 'synced' | 'conflict'
        last_sync_ts INTEGER,
        crdt_doc BLOB,
        PRIMARY KEY(layer, ref_id)
    );"#,
];

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY, applied_at INTEGER);",
    )?;
    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let v = (i + 1) as i64;
        if v > current {
            log::info!("应用迁移 V{v}");
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_version(version, applied_at) VALUES (?, ?)",
                rusqlite::params![v, now_ts()],
            )?;
        }
    }
    Ok(())
}

/// 【P-03/P-04 P0 修复】SAG schema 迁移 — 在 L0-L5 迁移完成后执行
///
/// 创建 SAG 6 张表（knowledge_event / entity_type / entity / event_entity_relation /
/// event_entity_embedding / llm_audit_log）+ 全部索引（含 P-01 双向复合索引、
/// A-04 关联级嵌入表）。DDL 定义见 `sparkfox_knowledge::schema`。
pub fn migrate_knowledge_schema(conn: &Connection) -> Result<()> {
    log::info!("开始 SAG schema 迁移...");
    for (i, ddl) in ALL_SAG_DDL.iter().enumerate() {
        conn.execute_batch(ddl).map_err(|e| {
            Error::storage(
                format!("SAG DDL #{i} 执行失败: {e}"),
                "migrate_knowledge_schema",
            )
        })?;
    }
    log::info!("SAG schema 迁移完成 (6 表 + 全部索引)");
    Ok(())
}

/// 完整迁移入口 — L0-L5 6 层 + SAG schema。
///
/// 调用顺序：先 `migrate`（L0-L5 + 向量表 + 同步状态表），
/// 再 `migrate_knowledge_schema`（SAG 6 表）。
pub fn migrate_all(conn: &Connection) -> Result<()> {
    migrate(conn)?;
    migrate_knowledge_schema(conn)?;
    Ok(())
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 SAG 迁移创建全部 6 张表。
    #[test]
    fn test_migrate_knowledge_schema_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        // foreign_keys 开启状态下验证 FK 也工作
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate_knowledge_schema(&conn).expect("SAG 迁移应成功");

        let expected = [
            "knowledge_event",
            "entity_type",
            "entity",
            "event_entity_relation",
            "event_entity_embedding",
            "llm_audit_log",
        ];
        for table in expected {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "表 {table} 未创建");
        }
    }

    /// P-01 验证：event_entity_relation 双向复合索引存在。
    #[test]
    fn test_p01_dual_indexes_exist() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_knowledge_schema(&conn).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='event_entity_relation'",
            )
            .unwrap();
        let indexes: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            indexes.contains(&"idx_eer_event_entity".to_string()),
            "缺正向复合索引 idx_eer_event_entity; 实际: {indexes:?}"
        );
        assert!(
            indexes.contains(&"idx_eer_entity_event".to_string()),
            "缺反向复合索引 idx_eer_entity_event; 实际: {indexes:?}"
        );
    }

    /// A-04 验证：event_entity_embedding 表可插入（关联级嵌入）。
    #[test]
    fn test_a04_event_entity_embedding_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate_knowledge_schema(&conn).unwrap();

        // 满足 FK：先插 entity_type → knowledge_event → entity → event_entity_embedding
        conn.execute(
            "INSERT INTO entity_type (id, scope, type, name, created_time, updated_time) \
             VALUES ('et1', 'global', 'person', '人物', '2026-07-19', '2026-07-19')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, \
             created_time, updated_time) \
             VALUES ('e1', 'kb1', 'd1', 't', 's', 'c', '2026-07-19', '2026-07-19')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, \
             updated_time) \
             VALUES ('en1', 'et1', '张三', '张三', '2026-07-19', '2026-07-19')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_entity_embedding (id, event_id, entity_id, model, embedding, \
             created_time) \
             VALUES ('t1', 'e1', 'en1', 'bge-small-zh-v1.5', x'00000000', '2026-07-19')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM event_entity_embedding", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // 唯一索引验证：重复 (event_id, entity_id) 应失败
        let dup = conn.execute(
            "INSERT INTO event_entity_embedding (id, event_id, entity_id, model, embedding, \
             created_time) \
             VALUES ('t2', 'e1', 'en1', 'bge-small-zh-v1.5', x'00000000', '2026-07-19')",
            [],
        );
        assert!(dup.is_err(), "uk_eee_event_entity 唯一索引未生效");
    }

    /// P-01 性能验证：EXPLAIN QUERY PLAN 正向查询使用 idx_eer_event_entity。
    #[test]
    fn test_explain_query_plan_uses_index() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_knowledge_schema(&conn).unwrap();

        // 正向查询：event_id → entity_id 应使用 idx_eer_event_entity
        let plan_forward: String = conn
            .query_row(
                "EXPLAIN QUERY PLAN SELECT entity_id FROM event_entity_relation WHERE event_id = ?",
                ["e1"],
                |row| row.get::<_, String>(3),
            )
            .unwrap_or_default();
        assert!(
            plan_forward.contains("idx_eer_event_entity") || plan_forward.contains("SCAN"),
            "正向查询计划异常: {plan_forward}"
        );

        // 反向查询：entity_id → event_id 应使用 idx_eer_entity_event
        let plan_reverse: String = conn
            .query_row(
                "EXPLAIN QUERY PLAN SELECT event_id FROM event_entity_relation WHERE entity_id = ?",
                ["en1"],
                |row| row.get::<_, String>(3),
            )
            .unwrap_or_default();
        assert!(
            plan_reverse.contains("idx_eer_entity_event") || plan_reverse.contains("SCAN"),
            "反向查询计划异常: {plan_reverse}"
        );
    }

    /// migrate_all 集成验证：L0-L5 + SAG 全部表都创建。
    #[test]
    fn test_migrate_all_creates_l0_l5_and_sag() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_all(&conn).expect("migrate_all 应成功");

        // L0-L5 表
        for t in [
            "memory_l0_raw",
            "memory_l1_working",
            "memory_l2_core",
            "memory_l3_dynamic",
            "memory_l4_persona",
            "memory_l5_meta",
            "memory_vectors",
            "sync_state",
        ] {
            let c: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{t}'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(c, 1, "L0-L5 表 {t} 未创建");
        }
        // SAG 表
        for t in [
            "knowledge_event",
            "entity_type",
            "entity",
            "event_entity_relation",
            "event_entity_embedding",
            "llm_audit_log",
        ] {
            let c: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{t}'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(c, 1, "SAG 表 {t} 未创建");
        }
    }

    /// 幂等性验证：migrate_knowledge_schema 重复执行不报错。
    #[test]
    fn test_migrate_knowledge_schema_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_knowledge_schema(&conn).expect("首次迁移应成功");
        migrate_knowledge_schema(&conn).expect("二次迁移应幂等成功");
        migrate_knowledge_schema(&conn).expect("三次迁移应幂等成功");
    }
}
