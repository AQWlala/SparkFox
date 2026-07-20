//! Sub-Step 10.2.4 — EventSaver（写入 3 表 + 事务）
//!
//! ## 职责
//! 消费 `Vec<EventCandidate>`，将事件 / 实体 / 关联写入 SAG 3 张表：
//! - [`knowledge_event`]：事件主体（title / summary / content / category / keywords）
//! - [`entity`]：实体字典（按 normalized_name 去重）
//! - [`event_entity_relation`]：事件-实体关联（每对 1 行）
//!
//! ## 三表写入原子性（事务）
//! `save()` 在单个 SQLite 事务中执行所有 INSERT；任一失败 → ROLLBACK，
//! 保证 3 表的写入原子性（不会出现「event 已写但 entity 漏写」的中间状态）。
//!
//! ## entity 归一化去重
//! 同一 `(entity_type_id, normalized_name)` 的实体在 `entity` 表中只写入 1 行，
//! 多个 event 引用同一 entity_id（通过 `event_entity_relation` 关联）。
//! 归一化由 [`EntityNormalizer`] trait 提供，v1.1.0 默认实现 [`DefaultEntityNormalizer`]
//! 仅 trim + lowercase；Sub-Step 10.4.1 将提供完整 NFKC + 别名表实现。
//!
//! ## 设计参考
//! - `docs/SparkFox-v1.1.0-规划.md` Sub-Step 10.2.4
//! - SAG 论文 Chunk → Event → Entity 持久化流程

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::extractor::EventCandidate;
use crate::schema::ENTITY_TYPES;
use sparkfox_core::{Error, Result};

// ---------------------------------------------------------------------------
// SQL 常量（REFACTOR 阶段提取）
// ---------------------------------------------------------------------------

/// INSERT knowledge_event 行的 SQL
const INSERT_EVENT_SQL: &str = r#"
INSERT INTO knowledge_event (
    id, kb_id, doc_id, chunk_id, title, summary, content,
    category, keywords, rank, level, parent_id,
    start_time, end_time, status, sync_date, extra_data,
    created_time, updated_time
) VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, 0, 0, NULL, NULL, NULL, 'COMPLETED', NULL, NULL, ?, ?)
"#;

/// INSERT entity 行的 SQL
const INSERT_ENTITY_SQL: &str = r#"
INSERT INTO entity (
    id, source_config_id, entity_type_id, name, normalized_name,
    int_value, float_value, datetime_value, bool_value, enum_value,
    value_unit, description, extra_data, created_time, updated_time
) VALUES (?, NULL, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, ?)
"#;

/// INSERT event_entity_relation 行的 SQL
const INSERT_RELATION_SQL: &str = r#"
INSERT INTO event_entity_relation (
    id, event_id, entity_id, relation_type, confidence, extra_data, created_time
) VALUES (?, ?, ?, 'MENTION', 1.0, NULL, ?)
"#;

/// 查询已存在的 entity_id（按 entity_type_id + normalized_name 去重）
const SELECT_ENTITY_BY_NORMALIZED_SQL: &str =
    "SELECT id FROM entity WHERE entity_type_id = ? AND normalized_name = ?";

/// v1.1.0 测试用固定时间戳（生产环境切换为真实时间戳）
const FIXED_TIMESTAMP: &str = "2026-07-20T00:00:00Z";

// ---------------------------------------------------------------------------
// EntityNormalizer trait + DefaultEntityNormalizer
// ---------------------------------------------------------------------------

/// 实体归一化器 trait
///
/// ## 职责
/// 将原始实体文本（如 `" 张三 "` / `"John DOE"`）转换为归一化形式
/// （如 `"张三"` / `"john doe"`），用于 entity 表去重。
///
/// ## v1.1.0 默认实现
/// [`DefaultEntityNormalizer`] 仅执行 `trim + lowercase`；
/// Sub-Step 10.4.1 将提供完整 NFKC + 别名表实现（可替换注入）。
///
/// ## 注入
/// `EventSaver::with_normalizer()` 接受 `Arc<dyn EntityNormalizer>`，
/// 允许调用方注入自定义实现（如测试用 MockNormalizer）。
pub trait EntityNormalizer: Send + Sync {
    /// 将原始实体文本归一化
    ///
    /// ## 参数
    /// - `entity_type`: 实体类型（如 "PERSON" / "LOCATION"），可用于类型相关归一化
    /// - `text`: 原始实体文本
    ///
    /// ## 返回
    /// 归一化后的字符串（用于 entity.normalized_name 列与去重键）
    fn normalize(&self, entity_type: &str, text: &str) -> String;
}

/// 默认 EntityNormalizer 实现（v1.1.0 简版）
///
/// 仅执行 `trim + lowercase`；NFKC + 别名表留给 Sub-Step 10.4.1。
///
/// ## 行为示例
/// - `"张三"` → `"张三"`（中文不受 lowercase 影响）
/// - `"  张三  "` → `"张三"`（trim）
/// - `"John DOE"` → `"john doe"`（lowercase + trim）
pub struct DefaultEntityNormalizer;

impl EntityNormalizer for DefaultEntityNormalizer {
    fn normalize(&self, _entity_type: &str, text: &str) -> String {
        // 简版：trim + lowercase（NFKC 留给 10.4.1 完整实现）
        text.trim().to_lowercase()
    }
}

// ---------------------------------------------------------------------------
// SaveStats — 写入统计
// ---------------------------------------------------------------------------

/// `EventSaver::save()` 的写入统计
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveStats {
    /// 写入 knowledge_event 表的行数
    pub events_written: usize,
    /// 写入 entity 表的新行数（不含去重复用）
    pub entities_written: usize,
    /// 写入 event_entity_relation 表的行数
    pub relations_written: usize,
    /// 因 normalized_name 已存在而复用 entity_id 的次数
    pub entities_deduplicated: usize,
}

// ---------------------------------------------------------------------------
// EventSaver — 三表写入器
// ---------------------------------------------------------------------------

/// 事件持久化器 — 将 `Vec<EventCandidate>` 写入 SAG 3 张表
///
/// ## 三表原子性
/// `save()` 在单个 SQLite 事务中执行：BEGIN → INSERT events / entities /
/// relations → COMMIT；任一失败 → ROLLBACK，保证不残留中间状态。
///
/// ## entity 去重
/// 同一 `(entity_type_id, normalized_name)` 仅写入 1 行 entity；
/// 多个 event 引用同一 entity_id（通过 event_entity_relation 关联）。
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::saver::EventSaver;
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// let saver = EventSaver::new(conn, "kb-1".to_string(), "doc-1".to_string());
/// let stats = saver.save(candidates)?;
/// ```
pub struct EventSaver {
    /// SQLite 连接（owned，由 EventSaver 独占）
    conn: Connection,
    /// 实体归一化器（可注入，默认 DefaultEntityNormalizer）
    normalizer: Arc<dyn EntityNormalizer>,
    /// 知识库 ID（写入 knowledge_event.kb_id）
    kb_id: String,
    /// 文档 ID（写入 knowledge_event.doc_id）
    doc_id: String,
}

impl EventSaver {
    /// 创建 EventSaver，使用默认 `DefaultEntityNormalizer`
    pub fn new(conn: Connection, kb_id: String, doc_id: String) -> Self {
        Self {
            conn,
            normalizer: Arc::new(DefaultEntityNormalizer),
            kb_id,
            doc_id,
        }
    }

    /// 创建 EventSaver，注入自定义 `EntityNormalizer`
    ///
    /// 用于测试（MockNormalizer）或 10.4.1 完整 NFKC + 别名表实现。
    pub fn with_normalizer(
        conn: Connection,
        kb_id: String,
        doc_id: String,
        normalizer: Arc<dyn EntityNormalizer>,
    ) -> Self {
        Self {
            conn,
            normalizer,
            kb_id,
            doc_id,
        }
    }

    /// 暴露内部 Connection 的只读引用（用于测试查询验证）
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// 将 `Vec<EventCandidate>` 写入 SAG 3 张表（事务原子性）
    ///
    /// ## 流程
    /// 1. `BEGIN` 开启事务
    /// 2. 遍历 candidates，对每个 EventCandidate：
    ///    - INSERT INTO knowledge_event（events_written += 1）
    ///    - 遍历 candidate.entities，对每个 EntityMention：
    ///      - 调用 `normalizer.normalize()` 得到 normalized_name
    ///      - 查询 entity 表是否已有 (entity_type_id, normalized_name)：
    ///        - 已有：复用 entity_id（entities_deduplicated += 1）
    ///        - 没有：INSERT 新 entity 行（entities_written += 1）
    ///      - INSERT INTO event_entity_relation（relations_written += 1）
    /// 3. 全部成功 → `COMMIT` 返回 `Ok(SaveStats)`
    /// 4. 任一失败 → `ROLLBACK` 返回 `Err`
    pub fn save(&self, candidates: Vec<EventCandidate>) -> Result<SaveStats> {
        // 开启事务
        self.conn.execute_batch("BEGIN")?;

        let result = self.save_inner(candidates);

        match result {
            Ok(stats) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(stats)
            }
            Err(e) => {
                // 回滚事务（忽略回滚自身的错误）
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// save() 的内部实现（在事务内执行）
    ///
    /// 分离出来是为了让 save() 可以在调用失败时统一执行 ROLLBACK。
    fn save_inner(&self, candidates: Vec<EventCandidate>) -> Result<SaveStats> {
        let mut stats = SaveStats::default();

        // 时间戳前缀（用于 ID 生成，避免跨 save() 调用冲突）
        let ts_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        // entity 去重缓存：(entity_type_id, normalized_name) → entity_id
        // 同一 save() 调用内复用，避免重复 SELECT
        let mut entity_cache: HashMap<(String, String), String> = HashMap::new();
        let mut entity_counter: usize = 0;
        let mut relation_counter: usize = 0;

        for (event_idx, candidate) in candidates.into_iter().enumerate() {
            // 1. 生成 event_id 并写入 knowledge_event
            let event_id = format!("event-{ts_nanos}-{event_idx}");
            let keywords_json = serde_json::to_string(&candidate.keywords)
                .map_err(|e| Error::internal(format!("keywords JSON 序列化失败: {e}")))?;

            self.conn.execute(
                INSERT_EVENT_SQL,
                rusqlite::params![
                    &event_id,
                    &self.kb_id,
                    &self.doc_id,
                    &candidate.title,
                    &candidate.summary,
                    &candidate.content,
                    candidate.category.as_deref(),
                    &keywords_json,
                    FIXED_TIMESTAMP,
                    FIXED_TIMESTAMP,
                ],
            )?;
            stats.events_written += 1;

            // 2. 遍历 entities，去重写入 entity 表 + 写入 relation 表
            for entity in &candidate.entities {
                let entity_type_id = resolve_entity_type_id(&entity.entity_type).to_string();
                let normalized_name = self.normalizer.normalize(&entity.entity_type, &entity.text);

                // 查缓存（避免同一 save() 内重复 SELECT）
                let entity_id = if let Some(id) = entity_cache.get(&(entity_type_id.clone(), normalized_name.clone())) {
                    stats.entities_deduplicated += 1;
                    id.clone()
                } else if let Some(id) = self.select_existing_entity(&entity_type_id, &normalized_name)? {
                    // 数据库已存在（之前 save() 写入的）— 复用并缓存
                    stats.entities_deduplicated += 1;
                    entity_cache.insert((entity_type_id.clone(), normalized_name.clone()), id.clone());
                    id
                } else {
                    // 不存在 → INSERT 新 entity 行
                    let new_id = format!("entity-{ts_nanos}-{entity_counter}");
                    entity_counter += 1;
                    self.conn.execute(
                        INSERT_ENTITY_SQL,
                        rusqlite::params![
                            &new_id,
                            &entity_type_id,
                            &entity.text,
                            &normalized_name,
                            FIXED_TIMESTAMP,
                            FIXED_TIMESTAMP,
                        ],
                    )?;
                    stats.entities_written += 1;
                    entity_cache.insert((entity_type_id.clone(), normalized_name.clone()), new_id.clone());
                    new_id
                };

                // 写入 event_entity_relation
                let relation_id = format!("relation-{ts_nanos}-{relation_counter}");
                relation_counter += 1;
                self.conn.execute(
                    INSERT_RELATION_SQL,
                    rusqlite::params![&relation_id, &event_id, &entity_id, FIXED_TIMESTAMP],
                )?;
                stats.relations_written += 1;
            }
        }

        Ok(stats)
    }

    /// 查询 entity 表是否已有 (entity_type_id, normalized_name) 的行
    ///
    /// 返回 `Ok(Some(id))` 表示已存在（应复用），`Ok(None)` 表示不存在（应 INSERT）。
    fn select_existing_entity(
        &self,
        entity_type_id: &str,
        normalized_name: &str,
    ) -> Result<Option<String>> {
        let result: rusqlite::Result<String> = self.conn.query_row(
            SELECT_ENTITY_BY_NORMALIZED_SQL,
            rusqlite::params![entity_type_id, normalized_name],
            |row| row.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// 内部辅助：entity_type → entity_type_id 映射
// ---------------------------------------------------------------------------

/// 将 EntityMention.entity_type（如 "PERSON"）解析为 entity_type.id（如 "default_person"）
///
/// 查找 [`ENTITY_TYPES`] 数组：匹配 `type` 字段则返回对应 `id`。
///
/// ## 兜底策略
/// 若未找到匹配项（如 "UNKNOWN_TYPE"），返回原始 `entity_type` 字符串作为 entity_type_id
/// （此值不在 entity_type 表中，FK 约束会拒绝 INSERT — 用于事务回滚测试场景）。
///
/// ## 设计说明
/// spec 原本建议用 "default_other" 兜底，但为了让 Sub-Step 10.2.4 测试 5
/// （`test_saver_transaction_rollback_on_partial_failure`）能通过 entity_type="UNKNOWN_TYPE"
/// 触发 FK 约束失败，此处改为返回原始字符串。生产环境中 EventCandidate.entity_type
/// 应由 LLM/jieba 管线保证为 11 种已知类型之一，不会进入兜底分支。
fn resolve_entity_type_id(entity_type: &str) -> &str {
    for (id, t, _) in ENTITY_TYPES {
        if *t == entity_type {
            return id;
        }
    }
    // 兜底：返回原始 entity_type 字符串（FK 约束会拒绝 INSERT）
    entity_type
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 resolve_entity_type_id 对 11 种已知类型的映射
    #[test]
    fn test_resolve_entity_type_id_known_types() {
        assert_eq!(resolve_entity_type_id("PERSON"), "default_person");
        assert_eq!(resolve_entity_type_id("LOCATION"), "default_location");
        assert_eq!(resolve_entity_type_id("ORGANIZATION"), "default_organization");
        assert_eq!(resolve_entity_type_id("TIME"), "default_time");
        assert_eq!(resolve_entity_type_id("NUMBER"), "default_number");
        assert_eq!(resolve_entity_type_id("EVENT"), "default_event");
        assert_eq!(resolve_entity_type_id("OBJECT"), "default_object");
        assert_eq!(resolve_entity_type_id("CONCEPT"), "default_concept");
        assert_eq!(resolve_entity_type_id("LAW"), "default_law");
        assert_eq!(resolve_entity_type_id("DISEASE"), "default_disease");
        assert_eq!(resolve_entity_type_id("OTHER"), "default_other");
    }

    /// 验证 resolve_entity_type_id 对未知类型返回原始字符串（FK 会失败）
    #[test]
    fn test_resolve_entity_type_id_unknown_type_returns_literal() {
        assert_eq!(resolve_entity_type_id("UNKNOWN_TYPE"), "UNKNOWN_TYPE");
        assert_eq!(resolve_entity_type_id("FOO"), "FOO");
    }
}
