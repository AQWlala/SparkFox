//! ATOMIC 检索策略 — 基于 `event_entity_relation` 表的原子事件检索
//!
//! ## 流程
//! 1. 从 query 提取实体（复用 [`crate::jieba_ner::JiebaNer`]）
//! 2. 通过实体文本查找 `entity.id`（按 `name` / `normalized_name` 匹配）
//! 3. SQL JOIN `event_entity_relation` + `entity` + `entity_type`：通过 `entity_id` 找
//!    `event_id`，同时取出 entity 的 `name` 与 entity_type 的 `type` 字段
//! 4. 返回 [`SearchHit`] 列表，`hop=1`，`via_entities=[匹配的 EntityRef]`，`chunk_span=None`
//!
//! ## top_k
//! 默认 `top_k=10`，可通过 [`AtomicStrategy::new_with_top_k`] 自定义。
//! 通过 SQL `LIMIT` 在数据库层裁剪，避免传输多余行。
//!
//! ## 无匹配处理
//! 若 query 无法提取实体，或实体文本在 `entity` 表中无匹配，返回空 `Vec`。
//!
//! ## Sync 约束说明
//! `rusqlite::Connection` 是 `Send` 但不是 `Sync`（内部使用 `RefCell`）。
//! [`SearchStrategy`] trait 要求 `Send + Sync`，因此 [`AtomicStrategy`] 用
//! `std::sync::Mutex<Connection>` 包装 `Connection`，使整体满足 `Send + Sync`。
//!
//! 由于 [`AtomicStrategy::search`] 内部全部是同步 `rusqlite` 调用（不跨 `await`
//! 点持有锁），使用 `std::sync::Mutex` 即可（无需 `tokio::sync::Mutex`）。

use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use rusqlite::Connection;

use sparkfox_core::{Error, Result};

use crate::jieba_ner::JiebaNer;
use super::{EntityRef, SearchHit, SearchResult, SearchStrategy};

/// 通过 `entity_id` JOIN `event_entity_relation` + `entity` + `entity_type` 找 `event` 的 SQL 模板
///
/// ## SELECT 列
/// - `e.id` / `e.title` / `e.summary` / `e.chunk_id` / `e.content`：事件字段
/// - `ent.id` / `et.type` / `ent.name`：用于构造 [`EntityRef`]（U-02 修复）
///
/// ## JOIN 链
/// ```sql
/// knowledge_event e
/// JOIN event_entity_relation r ON e.id = r.event_id   -- 事件 ↔ 实体关系
/// JOIN entity ent ON r.entity_id = ent.id             -- 实体本体（取 name）
/// JOIN entity_type et ON ent.entity_type_id = et.id   -- 实体类型字典（取 type）
/// ```
///
/// ## 占位符
/// `{placeholders}` 由调用方按 `entity_id` 数量填充为 `?, ?, ...`，
/// `LIMIT` 由 `top_k` 参数填充（参数化）。
const SQL_ATOMIC_SEARCH_TEMPLATE: &str = r#"
SELECT DISTINCT e.id, e.title, e.summary, e.chunk_id, e.content,
       ent.id AS entity_id, et.type AS entity_type, ent.name AS entity_name
FROM knowledge_event e
JOIN event_entity_relation r ON e.id = r.event_id
JOIN entity ent ON r.entity_id = ent.id
JOIN entity_type et ON ent.entity_type_id = et.id
WHERE r.entity_id IN ({placeholders})
ORDER BY e.created_time DESC
LIMIT ?
"#;

/// 查找与给定实体文本匹配的 `entity.id` 列表的 SQL 模板
const SQL_FIND_ENTITY_IDS_TEMPLATE: &str = r#"
SELECT DISTINCT id FROM entity
WHERE name IN ({placeholders}) OR normalized_name IN ({placeholders})
"#;

/// ATOMIC 检索策略
///
/// 单跳检索：query → jieba 实体抽取 → `entity_id` 查找 → JOIN
/// `event_entity_relation` → [`SearchHit`]。
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::{AtomicStrategy, SearchStrategy};
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// let strategy = AtomicStrategy::new(conn);
/// let result = strategy.search("张三去了哪里").await?;
/// for hit in &result.hits {
///     println!("{}: {}", hit.event_id, hit.title);
/// }
/// ```
pub struct AtomicStrategy {
    /// SQLite 连接（`Mutex` 包装以满足 `Sync` 约束）
    conn: Mutex<Connection>,
    /// jieba NER 分词器（用于从 query 提取实体；`JiebaNer` 内部 `Send + Sync`）
    jieba: JiebaNer,
    /// 返回结果的最大行数（SQL LIMIT）
    top_k: usize,
}

impl AtomicStrategy {
    /// 创建默认 `top_k=10` 的 [`AtomicStrategy`]
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k: 10,
        }
    }

    /// 创建指定 `top_k` 的 [`AtomicStrategy`]
    ///
    /// `top_k` 控制 SQL `LIMIT` 子句，限制返回的 [`SearchHit`] 数量。
    pub fn new_with_top_k(conn: Connection, top_k: usize) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k,
        }
    }

    /// 从 query 提取实体文本（用于 SQL 匹配 `entity.name`）
    ///
    /// 复用 [`JiebaNer::extract`]，仅取 `text` 字段（不含偏移与类型）。
    fn extract_query_entities(&self, query: &str) -> Vec<String> {
        self.jieba.extract(query).into_iter().map(|e| e.text).collect()
    }

    /// 查找与给定实体文本匹配的 `entity.id` 列表
    ///
    /// 匹配规则：`entity.name IN (...) OR entity.normalized_name IN (...)`
    /// （同时匹配原始名与归一化名，覆盖大小写 / 全半角差异）。
    fn find_entity_ids(&self, entity_texts: &[String]) -> Result<Vec<String>> {
        if entity_texts.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = entity_texts
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = SQL_FIND_ENTITY_IDS_TEMPLATE.replace("{placeholders}", &placeholders);

        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "AtomicStrategy::find_entity_ids")
        })?;
        let mut stmt = conn.prepare(&sql).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "AtomicStrategy::find_entity_ids")
        })?;

        // 参数顺序：name IN (...) 然后 normalized_name IN (...)
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(entity_texts.len() * 2);
        for t in entity_texts {
            params.push(t);
        }
        for t in entity_texts {
            params.push(t);
        }

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "AtomicStrategy::find_entity_ids"))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "AtomicStrategy::find_entity_ids")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 列表 JOIN `event_entity_relation` + `entity` + `entity_type` 查找事件
    ///
    /// ## SQL 行结构
    /// 每行包含一个 (event, entity) 对（即使 `DISTINCT` 也按 (event_id, entity_id) 元组去重）：
    /// - `e.id` / `e.title` / `e.summary` / `e.chunk_id` / `e.content`：事件字段
    /// - `ent.id` / `et.type` / `ent.name`：用于构造 [`EntityRef`]（U-02 修复）
    ///
    /// ## via_entities 填充策略（U-02）
    /// 每行构造一个 `EntityRef`，作为该 hit 的 `via_entities`（单元素 Vec）。
    /// 同一 event 可能通过多个 entity 匹配（返回多行 / 多个 hit），调用方按需合并去重。
    ///
    /// ## hop / chunk_span 填充策略
    /// - `hop = Some(1)`：ATOMIC 是单跳检索（query → entity → event）
    /// - `chunk_span = None`：ATOMIC 不涉及 chunk 位置信息，未来 MULTI / VECTOR 策略可填充
    fn find_events(&self, entity_ids: &[String]) -> Result<Vec<SearchHit>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = entity_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = SQL_ATOMIC_SEARCH_TEMPLATE.replace("{placeholders}", &placeholders);

        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "AtomicStrategy::find_events")
        })?;
        let mut stmt = conn.prepare(&sql).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "AtomicStrategy::find_events")
        })?;

        let mut params: Vec<&dyn rusqlite::ToSql> = entity_ids
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();
        let top_k_i64 = i64::try_from(self.top_k).unwrap_or(i64::MAX);
        params.push(&top_k_i64);

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                let event_id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let chunk_id: Option<String> = row.get(3)?;
                let _content: String = row.get(4)?;
                // U-02：构造 EntityRef（entity_id / entity_type / name）
                let entity_id: String = row.get(5)?;
                let entity_type: String = row.get(6)?;
                let entity_name: String = row.get(7)?;
                Ok(SearchHit {
                    event_id,
                    title,
                    summary,
                    chunk_id,
                    score: 1.0,
                    hop: Some(1u8),
                    via_entities: vec![EntityRef {
                        entity_id,
                        entity_type,
                        name: entity_name,
                    }],
                    chunk_span: None,
                })
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "AtomicStrategy::find_events"))?;

        let mut hits = Vec::new();
        for row in rows {
            hits.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "AtomicStrategy::find_events")
            })?);
        }
        Ok(hits)
    }
}

#[async_trait]
impl SearchStrategy for AtomicStrategy {
    async fn search(&self, query: &str) -> Result<SearchResult> {
        let start = Instant::now();

        // 1. 从 query 提取实体文本
        let entity_texts = self.extract_query_entities(query);
        if entity_texts.is_empty() {
            return Ok(SearchResult {
                hits: Vec::new(),
                latency_ms: start.elapsed().as_millis() as u64,
                strategy_name: "atomic".to_string(),
            });
        }

        // 2. 查找匹配的 entity_id
        let entity_ids = self.find_entity_ids(&entity_texts)?;
        if entity_ids.is_empty() {
            return Ok(SearchResult {
                hits: Vec::new(),
                latency_ms: start.elapsed().as_millis() as u64,
                strategy_name: "atomic".to_string(),
            });
        }

        // 3. JOIN event_entity_relation 查找事件
        let hits = self.find_events(&entity_ids)?;

        Ok(SearchResult {
            hits,
            latency_ms: start.elapsed().as_millis() as u64,
            strategy_name: "atomic".to_string(),
        })
    }

    fn name(&self) -> &str {
        "atomic"
    }
}
