//! LLM 审计日志 — S-01 P0 修复（Task 7.2.2）
//!
//! 每次 LLM 调用记录审计日志，便于追溯私密文档外泄。
//! 审计日志**仅本地存储，不同步跨设备**（隐私保护，spec S-01 要求）。
//!
//! ## 隐私设计
//! - `doc_hash` 仅存 SHA256，不存文档原文
//! - 不记录 prompt / completion 内容（仅记 token 数）
//! - 表 `llm_audit_log` 不进入 CRDT 同步通道（仅本地 SQLite）
//!
//! ## DDL
//! 与 Task 3.1 SAG schema 中 `llm_audit_log` 表定义一致：
//! ```sql
//! CREATE TABLE IF NOT EXISTS llm_audit_log (
//!     id TEXT PRIMARY KEY,
//!     timestamp TEXT NOT NULL,
//!     doc_hash TEXT,
//!     llm_provider TEXT NOT NULL,
//!     model TEXT NOT NULL,
//!     prompt_tokens INTEGER,
//!     completion_tokens INTEGER,
//!     status TEXT NOT NULL,
//!     error_msg TEXT,
//!     extra_data TEXT
//! );
//! ```

#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use rusqlite::{params, Connection};
use tokio::sync::Mutex;

use sparkfox_core::{Error, Result};

/// LLM 审计日志条目
///
/// 一次 LLM 调用对应一条记录（成功/失败/超时）。
/// `doc_hash` 用于追溯私密文档外泄（仅存 SHA256，不存原文）。
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// UUID（主键）
    pub id: String,
    /// ISO 8601 时间戳（UTC）
    pub timestamp: String,
    /// 文档 SHA256（不存原文）— 用于追溯哪份文档被发往 LLM
    pub doc_hash: Option<String>,
    /// LLM 供应方：openai / anthropic / local / qwen / ...
    pub llm_provider: String,
    /// 模型名：gpt-4 / claude-3-opus / qwen-max / ...
    pub model: String,
    /// prompt token 数（粗估 len/4，真实数由 provider 返回）
    pub prompt_tokens: Option<i32>,
    /// completion token 数
    pub completion_tokens: Option<i32>,
    /// 状态：success / failed / timeout
    pub status: String,
    /// 失败时的错误信息
    pub error_msg: Option<String>,
    /// 额外数据（JSON 字符串，如 request_id / latency_ms）
    pub extra_data: Option<String>,
}

impl AuditEntry {
    /// 创建成功条目
    pub fn success(
        provider: &str,
        model: &str,
        doc_hash: Option<String>,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            doc_hash,
            llm_provider: provider.to_string(),
            model: model.to_string(),
            prompt_tokens: Some(prompt_tokens),
            completion_tokens: Some(completion_tokens),
            status: "success".to_string(),
            error_msg: None,
            extra_data: None,
        }
    }

    /// 创建失败条目
    pub fn failure(
        provider: &str,
        model: &str,
        doc_hash: Option<String>,
        error: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            doc_hash,
            llm_provider: provider.to_string(),
            model: model.to_string(),
            prompt_tokens: None,
            completion_tokens: None,
            status: "failed".to_string(),
            error_msg: Some(error.to_string()),
            extra_data: None,
        }
    }

    /// 创建超时条目
    pub fn timeout(provider: &str, model: &str, doc_hash: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            doc_hash,
            llm_provider: provider.to_string(),
            model: model.to_string(),
            prompt_tokens: None,
            completion_tokens: None,
            status: "timeout".to_string(),
            error_msg: Some("LLM 调用超时".to_string()),
            extra_data: None,
        }
    }
}

/// LLM 调用审计日志记录器（线程安全）
///
/// 基于 `rusqlite::Connection` + `tokio::sync::Mutex` 实现线程安全。
/// 表 `llm_audit_log` 仅本地存储，不参与 CRDT 跨设备同步（隐私保护）。
///
/// # 用法
/// ```no_run
/// # use sparkfox_security::{AuditEntry, LlmAuditLogger};
/// # use rusqlite::Connection;
/// # async fn run() -> sparkfox_core::Result<()> {
/// let conn = Connection::open_in_memory()?;
/// let logger = LlmAuditLogger::from_conn(conn).await?;
/// logger.log(AuditEntry::success("openai", "gpt-4", None, 100, 200)).await?;
/// let recent = logger.recent(10).await?;
/// # Ok(())
/// # }
/// ```
pub struct LlmAuditLogger {
    conn: Arc<Mutex<Connection>>,
}

impl LlmAuditLogger {
    /// 用已存在的 `Arc<Mutex<Connection>>` 包装（不建表，调用方需保证表存在）
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 从裸 `Connection` 创建（同步建表，确保 `llm_audit_log` 存在）
    ///
    /// DDL 与 Task 3.1 SAG schema 中 `llm_audit_log` 表一致。
    /// 包含 timestamp / provider 索引以加速审计 UI 查询。
    pub async fn from_conn(conn: Connection) -> Result<Self> {
        // 建表 + 索引（如果不存在）
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS llm_audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                doc_hash TEXT,
                llm_provider TEXT NOT NULL,
                model TEXT NOT NULL,
                prompt_tokens INTEGER,
                completion_tokens INTEGER,
                status TEXT NOT NULL,
                error_msg TEXT,
                extra_data TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON llm_audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_provider ON llm_audit_log(llm_provider);
            "#,
        )
        .map_err(|e| Error::internal(format!("建 llm_audit_log 表失败: {e}")))?;
        Ok(Self::new(Arc::new(Mutex::new(conn))))
    }

    /// 记录审计日志（INSERT）
    pub async fn log(&self, entry: AuditEntry) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO llm_audit_log (id, timestamp, doc_hash, llm_provider, model, prompt_tokens, completion_tokens, status, error_msg, extra_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.timestamp,
                entry.doc_hash,
                entry.llm_provider,
                entry.model,
                entry.prompt_tokens,
                entry.completion_tokens,
                entry.status,
                entry.error_msg,
                entry.extra_data,
            ],
        )
        .map_err(|e| Error::internal(format!("审计日志写入失败: {e}")))?;
        Ok(())
    }

    /// 查询最近 N 条日志（用于审计 UI，按时间倒序）
    pub async fn recent(&self, limit: u32) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, doc_hash, llm_provider, model, prompt_tokens, completion_tokens, status, error_msg, extra_data FROM llm_audit_log ORDER BY timestamp DESC LIMIT ?1",
            )
            .map_err(|e| Error::internal(format!("审计日志查询失败: {e}")))?;
        let entries = stmt
            .query_map(params![limit as i64], |row| {
                Ok(AuditEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    doc_hash: row.get(2)?,
                    llm_provider: row.get(3)?,
                    model: row.get(4)?,
                    prompt_tokens: row.get(5)?,
                    completion_tokens: row.get(6)?,
                    status: row.get(7)?,
                    error_msg: row.get(8)?,
                    extra_data: row.get(9)?,
                })
            })
            .map_err(|e| Error::internal(format!("审计日志映射失败: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// 统计日志总数
    pub async fn count(&self) -> Result<u64> {
        let conn = self.conn.lock().await;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM llm_audit_log", [], |row| row.get(0))
            .map_err(|e| Error::internal(format!("审计日志计数失败: {e}")))?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助：创建一个 in-memory logger，自动建表
    async fn make_logger() -> LlmAuditLogger {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        LlmAuditLogger::from_conn(conn)
            .await
            .expect("建 logger 失败")
    }

    // T-01：成功调用 → 审计日志记录 status=success + token 数
    #[tokio::test]
    async fn test_audit_log_success() {
        let logger = make_logger().await;
        let entry = AuditEntry::success("openai", "gpt-4", None, 100, 200);
        logger.log(entry).await.expect("写入成功日志失败");

        let recent = logger.recent(10).await.expect("查询失败");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].status, "success");
        assert_eq!(recent[0].llm_provider, "openai");
        assert_eq!(recent[0].model, "gpt-4");
        assert_eq!(recent[0].prompt_tokens, Some(100));
        assert_eq!(recent[0].completion_tokens, Some(200));
        assert!(recent[0].error_msg.is_none());
        assert!(recent[0].doc_hash.is_none());
    }

    // T-02：失败调用 → 审计日志记录 status=failed + error_msg
    #[tokio::test]
    async fn test_audit_log_failure() {
        let logger = make_logger().await;
        let entry = AuditEntry::failure("anthropic", "claude-3", None, "rate limit exceeded");
        logger.log(entry).await.expect("写入失败日志失败");

        let recent = logger.recent(10).await.expect("查询失败");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].status, "failed");
        assert_eq!(recent[0].error_msg.as_deref(), Some("rate limit exceeded"));
        assert!(recent[0].prompt_tokens.is_none());
        assert!(recent[0].completion_tokens.is_none());
    }

    // T-03（doc_hash 覆盖）：含 doc_hash → 审计日志记录 doc_hash（不存原文）
    #[tokio::test]
    async fn test_audit_log_doc_hash() {
        let logger = make_logger().await;
        // 模拟文档 SHA256（不存原文）— 64 字符 hex
        let doc_hash = "a3f5b8e9c1d2f4a6b8e0d2c4f6a8b0e2d4f6a8b0c2e4d6f8a0b2c4e6d8f0a2c4".to_string();
        assert_eq!(doc_hash.len(), 64, "测试前置：doc_hash 应为 64 字符 SHA256 hex");
        let entry = AuditEntry::success("local", "qwen-max", Some(doc_hash.clone()), 50, 80);
        logger.log(entry).await.expect("写入 doc_hash 日志失败");

        let recent = logger.recent(10).await.expect("查询失败");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].doc_hash.as_deref(), Some(doc_hash.as_str()));
        // 验证不存原文：doc_hash 字段为 64 字符 hex（SHA256）
        assert_eq!(recent[0].doc_hash.as_ref().unwrap().len(), 64);
    }

    // T-04：审计日志仅本地，无跨设备同步接口（验证 recent/count 仅查本地，无 sync 方法）
    #[tokio::test]
    async fn test_audit_log_no_sync_interface() {
        let logger = make_logger().await;
        // 仅本地：连续写入 3 条，recent/count 应只反映本地写入
        for i in 0..3 {
            logger
                .log(AuditEntry::success("openai", "gpt-4", None, i, i * 2))
                .await
                .unwrap();
        }
        // count 仅查本地
        let total = logger.count().await.expect("count 失败");
        assert_eq!(total, 3, "审计日志 count 应仅反映本地条目");

        // recent 仅查本地
        let recent = logger.recent(100).await.expect("recent 失败");
        assert_eq!(recent.len(), 3);

        // LlmAuditLogger API 不暴露 sync / push / pull 等跨设备方法
        // （静态保证：本模块无任何 sync 相关方法，编译期可证）
    }

    // T-05：多次调用后 count 正确
    #[tokio::test]
    async fn test_audit_log_count() {
        let logger = make_logger().await;
        for i in 0..10 {
            let entry = AuditEntry::success("openai", "gpt-4", None, i, i);
            logger.log(entry).await.unwrap();
        }
        assert_eq!(logger.count().await.unwrap(), 10);

        // 失败条目也应计入
        logger
            .log(AuditEntry::failure("openai", "gpt-4", None, "err"))
            .await
            .unwrap();
        assert_eq!(logger.count().await.unwrap(), 11);
    }

    // T-06：recent(N) 返回最近 N 条（按时间倒序）
    #[tokio::test]
    async fn test_audit_log_recent_limit() {
        let logger = make_logger().await;
        // 写入 5 条，每条间隔 1ms 保证 timestamp 单调
        for i in 0..5 {
            logger
                .log(AuditEntry::success("openai", "gpt-4", None, i, i))
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        let recent3 = logger.recent(3).await.expect("recent(3) 失败");
        assert_eq!(recent3.len(), 3, "recent(3) 应返回 3 条");

        // 时间倒序：第一条应为最后写入的（prompt_tokens=4）
        assert_eq!(recent3[0].prompt_tokens, Some(4));
        assert_eq!(recent3[1].prompt_tokens, Some(3));
        assert_eq!(recent3[2].prompt_tokens, Some(2));
    }

    // T-07：并发调用无丢失（tokio::join! 3 个并发写入）
    #[tokio::test]
    async fn test_audit_log_concurrent() {
        let logger = Arc::new(make_logger().await);

        // 3 个并发任务，各写 10 条
        let logger_clone1 = logger.clone();
        let logger_clone2 = logger.clone();
        let logger_clone3 = logger.clone();

        let task1 = tokio::spawn(async move {
            for i in 0..10 {
                logger_clone1
                    .log(AuditEntry::success("openai", "gpt-4", None, i, i))
                    .await
                    .unwrap();
            }
        });
        let task2 = tokio::spawn(async move {
            for i in 0..10 {
                logger_clone2
                    .log(AuditEntry::success("anthropic", "claude-3", None, i, i))
                    .await
                    .unwrap();
            }
        });
        let task3 = tokio::spawn(async move {
            for i in 0..10 {
                logger_clone3
                    .log(AuditEntry::success("local", "qwen-max", None, i, i))
                    .await
                    .unwrap();
            }
        });

        // 等待 3 个并发任务完成（unwrap JoinHandle 确保任务无 panic）
        let (h1, h2, h3) = tokio::join!(task1, task2, task3);
        h1.unwrap();
        h2.unwrap();
        h3.unwrap();

        // 无丢失：应有 30 条
        let total = logger.count().await.expect("count 失败");
        assert_eq!(total, 30, "并发写入不应丢失任何条目");

        // 各 provider 各 10 条
        let recent = logger.recent(100).await.unwrap();
        let openai_count = recent
            .iter()
            .filter(|e| e.llm_provider == "openai")
            .count();
        let anthropic_count = recent
            .iter()
            .filter(|e| e.llm_provider == "anthropic")
            .count();
        let local_count = recent.iter().filter(|e| e.llm_provider == "local").count();
        assert_eq!(openai_count, 10);
        assert_eq!(anthropic_count, 10);
        assert_eq!(local_count, 10);
    }

    // T-08：表删除后 from_conn 自动重建
    #[tokio::test]
    async fn test_audit_log_table_recreate() {
        // 1. 第一次创建 logger，建表 + 写入数据
        let conn = Connection::open_in_memory().expect("open sqlite");
        let logger = LlmAuditLogger::from_conn(conn).await.expect("第一次建 logger 失败");
        logger
            .log(AuditEntry::success("openai", "gpt-4", None, 1, 2))
            .await
            .unwrap();
        assert_eq!(logger.count().await.unwrap(), 1);

        // 2. 取出 conn，手动 DROP 表（模拟表损坏/被删）
        let conn = Arc::try_unwrap(logger.conn)
            .expect("Arc 应唯一")
            .into_inner();
        conn.execute("DROP TABLE llm_audit_log", [])
            .expect("DROP 表失败");

        // 3. 再次 from_conn：应自动重建表（CREATE TABLE IF NOT EXISTS）
        let logger2 = LlmAuditLogger::from_conn(conn)
            .await
            .expect("from_conn 应自动重建表");
        // 表已重建，count 应为 0（旧数据已随 DROP 丢失）
        assert_eq!(logger2.count().await.unwrap(), 0, "重建后表应为空");

        // 4. 重建后写入应正常
        logger2
            .log(AuditEntry::success("openai", "gpt-4", None, 3, 4))
            .await
            .unwrap();
        assert_eq!(logger2.count().await.unwrap(), 1);
    }
}
