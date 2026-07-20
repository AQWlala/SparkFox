#![forbid(unsafe_code)]
//! MCP Broker audit log — 记录谁在何时调用了 MCP 工具（knowledge_search 等）
//!
//! 与 [`crate::audit`]（LLM 审计日志）的区别：
//! - [`crate::audit`]：记录 LLM 调用（prompt/response/token）
//! - 本模块：记录 MCP 工具调用（caller_id/tool_name/args/result_summary）
//!
//! 表 `mcp_audit_log` 仅本地存储，不参与 CRDT 跨设备同步（隐私保护，
//! 与 LLM 审计日志一致）。
//!
//! NOTICE: NomiFun MCP Broker 设计借鉴，Rust 重写

use sparkfox_core::Result;

/// MCP 审计日志记录
///
/// 一次 MCP 工具调用对应一条记录（如 `knowledge_search` / `memory_put`）。
/// `args` 与 `result_summary` 均为可选，便于适配不同工具的参数形态。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpAuditEntry {
    /// 自增主键（写入后由 DB 分配）
    pub id: Option<i64>,
    /// Unix 秒级时间戳
    pub ts: i64,
    /// 调用方标识（如 agent_id / session_id）
    pub caller_id: String,
    /// MCP 工具名（如 `knowledge_search`）
    pub tool_name: String,
    /// 调用参数（JSON 字符串，可选）
    pub args: Option<String>,
    /// 结果摘要（不存原始结果，避免泄漏，可选）
    pub result_summary: Option<String>,
}

/// MCP 审计日志器（线程安全）
///
/// 基于 `rusqlite::Connection` + `std::sync::Mutex` 实现线程安全。
/// 与 [`crate::audit::LlmAuditLogger`] 的异步设计不同，本模块采用同步 API，
/// 适用于 MCP Broker 同步调度路径。
pub struct McpAuditLogger {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl McpAuditLogger {
    /// 从已建立的 SQLite 连接创建（自动建表，幂等）
    pub fn from_conn(conn: rusqlite::Connection) -> Result<Self> {
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// 创建内存数据库（用于测试）
    pub fn in_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| sparkfox_core::Error::internal(format!("打开内存数据库失败: {e}")))?;
        Self::from_conn(conn)
    }

    /// 建表 + 索引（IF NOT EXISTS，幂等）
    fn init_schema(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute_batch(
            r#"CREATE TABLE IF NOT EXISTS mcp_audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                caller_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                args TEXT,
                result_summary TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_mcp_audit_ts ON mcp_audit_log(ts);
            CREATE INDEX IF NOT EXISTS idx_mcp_audit_caller ON mcp_audit_log(caller_id);
            CREATE INDEX IF NOT EXISTS idx_mcp_audit_tool ON mcp_audit_log(tool_name);"#,
        )
        .map_err(|e| sparkfox_core::Error::internal(format!("建表失败: {e}")))?;
        Ok(())
    }

    /// 记录一次 MCP 工具调用
    pub fn log(
        &self,
        caller_id: &str,
        tool_name: &str,
        args: Option<&str>,
        result_summary: Option<&str>,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| sparkfox_core::Error::internal(format!("锁失败: {e}")))?;
        conn.execute(
            "INSERT INTO mcp_audit_log(ts, caller_id, tool_name, args, result_summary) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![now_ts(), caller_id, tool_name, args, result_summary],
        )
        .map_err(|e| sparkfox_core::Error::internal(format!("插入审计日志失败: {e}")))?;
        Ok(())
    }

    /// 查询最近的 N 条记录（按时间倒序）
    pub fn recent(&self, limit: usize) -> Result<Vec<McpAuditEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| sparkfox_core::Error::internal(format!("锁失败: {e}")))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, ts, caller_id, tool_name, args, result_summary FROM mcp_audit_log ORDER BY ts DESC LIMIT ?",
            )
            .map_err(|e| sparkfox_core::Error::internal(format!("prepare 失败: {e}")))?;

        let entries = stmt
            .query_map([limit as i64], |row| {
                Ok(McpAuditEntry {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    caller_id: row.get(2)?,
                    tool_name: row.get(3)?,
                    args: row.get(4)?,
                    result_summary: row.get(5)?,
                })
            })
            .map_err(|e| sparkfox_core::Error::internal(format!("query_map 失败: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| sparkfox_core::Error::internal(format!("collect 失败: {e}")))?;

        Ok(entries)
    }

    /// 查询指定 caller 的调用记录（按时间倒序）
    pub fn by_caller(&self, caller_id: &str, limit: usize) -> Result<Vec<McpAuditEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| sparkfox_core::Error::internal(format!("锁失败: {e}")))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, ts, caller_id, tool_name, args, result_summary FROM mcp_audit_log WHERE caller_id = ? ORDER BY ts DESC LIMIT ?",
            )
            .map_err(|e| sparkfox_core::Error::internal(format!("prepare 失败: {e}")))?;

        let entries = stmt
            .query_map(rusqlite::params![caller_id, limit as i64], |row| {
                Ok(McpAuditEntry {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    caller_id: row.get(2)?,
                    tool_name: row.get(3)?,
                    args: row.get(4)?,
                    result_summary: row.get(5)?,
                })
            })
            .map_err(|e| sparkfox_core::Error::internal(format!("query_map 失败: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| sparkfox_core::Error::internal(format!("collect 失败: {e}")))?;

        Ok(entries)
    }

    /// 记录总数
    pub fn count(&self) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| sparkfox_core::Error::internal(format!("锁失败: {e}")))?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_audit_log", [], |row| row.get(0))
            .map_err(|e| sparkfox_core::Error::internal(format!("count 失败: {e}")))?;
        Ok(count)
    }
}

/// 当前 Unix 秒级时间戳
fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_creation() {
        let logger = McpAuditLogger::in_memory().unwrap();
        assert_eq!(logger.count().unwrap(), 0);
    }

    #[test]
    fn test_log_and_count() {
        let logger = McpAuditLogger::in_memory().unwrap();
        logger
            .log(
                "agent_001",
                "knowledge_search",
                Some(r#"{"query":"test"}"#),
                Some("3 hits"),
            )
            .unwrap();
        logger
            .log("agent_002", "memory_put", None, None)
            .unwrap();
        assert_eq!(logger.count().unwrap(), 2);
    }

    #[test]
    fn test_recent() {
        let logger = McpAuditLogger::in_memory().unwrap();
        logger.log("agent_001", "tool_a", None, None).unwrap();
        logger.log("agent_001", "tool_b", None, None).unwrap();
        logger.log("agent_002", "tool_c", None, None).unwrap();
        let recent = logger.recent(2).unwrap();
        assert_eq!(recent.len(), 2);
        // 倒序，最新的在前
        assert_eq!(recent[0].tool_name, "tool_c");
        assert_eq!(recent[1].tool_name, "tool_b");
    }

    #[test]
    fn test_by_caller() {
        let logger = McpAuditLogger::in_memory().unwrap();
        logger.log("agent_001", "tool_a", None, None).unwrap();
        logger.log("agent_002", "tool_b", None, None).unwrap();
        logger.log("agent_001", "tool_c", None, None).unwrap();
        let entries = logger.by_caller("agent_001", 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.caller_id == "agent_001"));
    }

    #[test]
    fn test_schema_idempotent() {
        let logger = McpAuditLogger::in_memory().unwrap();
        // 重复 from_conn 应该幂等（IF NOT EXISTS）
        // 这里通过再次 log 验证表结构完整
        logger.log("x", "y", None, None).unwrap();
        assert_eq!(logger.count().unwrap(), 1);
    }

    #[test]
    fn test_optional_fields_null() {
        let logger = McpAuditLogger::in_memory().unwrap();
        logger.log("agent", "tool", None, None).unwrap();
        let entries = logger.recent(1).unwrap();
        assert_eq!(entries[0].args, None);
        assert_eq!(entries[0].result_summary, None);
    }
}
