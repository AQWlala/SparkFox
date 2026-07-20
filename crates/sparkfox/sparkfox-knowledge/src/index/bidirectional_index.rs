//! Sub-Step 11.6.2 — 双向索引（entity ↔ event）实现（spec §三 11.6.2）
//!
//! ## 设计动机
//! 在内存中构建 entity → events 和 event → entities 双向 HashMap 索引，
//! 替代 multi-hop BFS 扩展中每次 SQL JOIN 查询，将 O(N) SQL 查询降为 O(1)
//! HashMap 查找。
//!
//! ## 适用场景
//! - **MultiStrategy BFS 扩展**：entity → events → entities → events → ...
//! - **Multi1Strategy 单跳检索**：entity → events
//! - **HopllmStrategy LLM 引导**：entity → events → entities
//!
//! ## 构建
//! - 从 SQLite `event_entity_relation` 表一次性加载
//! - 启动期一次性构建，运行期只读（暂不支持增量更新）
//!
//! ## 内存估算
//! - 每条关系约 2KB（含两个 String + HashSet 节点开销）
//! - 100k 关系约 200MB 内存
//!
//! ## 线程安全
//! `HashMap<String, HashSet<String>>` 是 `Send + Sync`（无内部可变性），
//! `BidirectionalIndex` 通过 `&self` 提供只读查询，无需外部同步。
//!
//! ## 与 11.6.1 HnswIndex 的关系
//! - HnswIndex（11.6.1）：向量索引，解决"找相似 entity"问题
//! - BidirectionalIndex（11.6.2，本模块）：图结构索引，解决"找关联 event"问题
//! - 两者互补，共同构成 multi-hop BFS 扩展的索引层
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use sparkfox_core::{Error, Result};

/// 从 `event_entity_relation` 表加载所有 (event_id, entity_id) 关系的 SQL
///
/// 仅选择双向索引必需的两列，避免无关列的 IO 开销。
const SQL_LOAD_ALL_RELATIONS: &str =
    "SELECT event_id, entity_id FROM event_entity_relation";

/// 双向索引（spec §三 11.6.2）
///
/// 在内存中构建 entity ↔ event 双向索引，加速 multi-hop BFS 扩展。
/// 替代每次 SQL JOIN 查询，将 O(N) SQL 查询降为 O(1) HashMap 查找。
///
/// ## 内部结构
/// - `entity_to_events`：entity_id → 关联的 event_id 集合（去重）
/// - `event_to_entities`：event_id → 关联的 entity_id 集合（去重）
/// - `relation_count`：原始关系行数（不去重，与 SQL COUNT(*) 一致）
///
/// ## 查询语义
/// - `get_events_by_entity(eid)` 返回 `Option<&HashSet<String>>`
///   - `Some(set)`：entity 存在，set 为其关联的 event_ids
///   - `None`：entity 不存在
///
/// ## 示例
/// ```ignore
/// use sparkfox_knowledge::index::bidirectional_index::BidirectionalIndex;
///
/// let conn = rusqlite::Connection::open_in_memory().unwrap();
/// // ... 初始化 schema + 数据 ...
/// let index = BidirectionalIndex::from_connection(&conn).unwrap();
///
/// if let Some(events) = index.get_events_by_entity("ent-0-0") {
///     println!("ent-0-0 关联 {} 个 event", events.len());
/// }
/// ```
pub struct BidirectionalIndex {
    /// entity_id → 关联的 event_id 集合（去重）
    entity_to_events: HashMap<String, HashSet<String>>,
    /// event_id → 关联的 entity_id 集合（去重）
    event_to_entities: HashMap<String, HashSet<String>>,
    /// 原始关系行数（不去重，与 SQL COUNT(*) 一致）
    ///
    /// 注意：与 `entity_to_events` / `event_to_entities` 内 HashSet 大小之和不必然相等：
    /// 若同一 (event_id, entity_id) 对在表中出现多次，HashSet 去重后变小，但
    /// `relation_count` 与 SQL `COUNT(*)` 一致。fixture 无重复时三者相等。
    relation_count: usize,
}

impl BidirectionalIndex {
    /// 新建空索引
    ///
    /// ## 返回
    /// 空的 `BidirectionalIndex`（`entity_count()=0` / `event_count()=0` /
    /// `relation_count()=0` / `is_empty()=true`）
    pub fn new() -> Self {
        Self {
            entity_to_events: HashMap::new(),
            event_to_entities: HashMap::new(),
            relation_count: 0,
        }
    }

    /// 从 SQLite `event_entity_relation` 表一次性构建索引
    ///
    /// ## 参数
    /// - `conn`：SQLite 连接（须已创建 SAG schema，见 [`crate::schema::ALL_SAG_DDL`]）
    ///
    /// ## 返回
    /// 加载完毕的 `BidirectionalIndex`
    ///
    /// ## 错误
    /// - SQL prepare / 查询失败：返回 `Storage` 错误
    /// - 字段类型不匹配（理论不应发生）：返回 `Storage` 错误
    pub fn from_connection(conn: &Connection) -> Result<Self> {
        let mut index = Self::new();

        let mut stmt = conn
            .prepare(SQL_LOAD_ALL_RELATIONS)
            .map_err(|e| Error::storage(
                format!("prepare 失败: {e}"),
                "BidirectionalIndex::from_connection",
            ))?;

        let rows = stmt
            .query_map([], |row| {
                let event_id: String = row.get(0)?;
                let entity_id: String = row.get(1)?;
                Ok((event_id, entity_id))
            })
            .map_err(|e| Error::storage(
                format!("query_map 失败: {e}"),
                "BidirectionalIndex::from_connection",
            ))?;

        for row in rows {
            let (event_id, entity_id) = row.map_err(|e| Error::storage(
                format!("读取行失败: {e}"),
                "BidirectionalIndex::from_connection",
            ))?;
            index.add_relation(entity_id, event_id);
        }

        Ok(index)
    }

    /// 添加一条 entity → event 关系（内部使用，构建期专用）
    ///
    /// 将关系同时插入双向 HashMap：
    /// - `entity_to_events[entity_id].insert(event_id)`
    /// - `event_to_entities[event_id].insert(entity_id)`
    /// - `relation_count` += 1
    ///
    /// 若 (entity_id, event_id) 已存在，HashSet::insert 会返回 false 但不报错，
    /// `relation_count` 仍递增（与 SQL COUNT(*) 语义一致）。
    fn add_relation(&mut self, entity_id: String, event_id: String) {
        self.entity_to_events
            .entry(entity_id.clone())
            .or_insert_with(HashSet::new)
            .insert(event_id.clone());

        self.event_to_entities
            .entry(event_id)
            .or_insert_with(HashSet::new)
            .insert(entity_id);

        self.relation_count += 1;
    }

    /// 查询 entity 关联的 events
    ///
    /// ## 参数
    /// - `entity_id`：实体 ID（如 `"ent-0-0"`）
    ///
    /// ## 返回
    /// - `Some(&HashSet<String>)`：该 entity 关联的所有 event_id（去重）
    /// - `None`：entity 不存在于索引（无任何关系）
    pub fn get_events_by_entity(&self, entity_id: &str) -> Option<&HashSet<String>> {
        self.entity_to_events.get(entity_id)
    }

    /// 查询 event 关联的 entities
    ///
    /// ## 参数
    /// - `event_id`：事件 ID（如 `"evt-0"`）
    ///
    /// ## 返回
    /// - `Some(&HashSet<String>)`：该 event 关联的所有 entity_id（去重）
    /// - `None`：event 不存在于索引（无任何关系）
    pub fn get_entities_by_event(&self, event_id: &str) -> Option<&HashSet<String>> {
        self.event_to_entities.get(event_id)
    }

    /// 索引中 entity 总数（去重）
    ///
    /// 与 `event_count` 一样，是去重计数（HashMap::len），不是关系总数。
    pub fn entity_count(&self) -> usize {
        self.entity_to_events.len()
    }

    /// 索引中 event 总数（去重）
    ///
    /// 与 `entity_count` 一样，是去重计数（HashMap::len），不是关系总数。
    pub fn event_count(&self) -> usize {
        self.event_to_entities.len()
    }

    /// 索引中关系总数（不去重，与 SQL COUNT(*) 一致）
    ///
    /// 注意：与 `entity_count` / `event_count` 不同：
    /// - `entity_count` / `event_count` 是去重计数
    /// - `relation_count` 是原始关系行数
    /// - 例：1 个 entity 关联 N 个 event，entity_count=1，relation_count=N
    pub fn relation_count(&self) -> usize {
        self.relation_count
    }

    /// 索引是否为空
    ///
    /// 当且仅当 `relation_count() == 0` 时返回 true（隐含 entity_count 与 event_count 也为 0）。
    pub fn is_empty(&self) -> bool {
        self.relation_count == 0
    }
}

/// 默认实现（空索引，等价于 [`BidirectionalIndex::new`]）
impl Default for BidirectionalIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_index() {
        let index = BidirectionalIndex::new();
        assert_eq!(index.entity_count(), 0);
        assert_eq!(index.event_count(), 0);
        assert_eq!(index.relation_count(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_default_equals_new() {
        let index = BidirectionalIndex::default();
        assert_eq!(index.entity_count(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_add_relation_updates_both_directions() {
        let mut index = BidirectionalIndex::new();
        index.add_relation("ent-1".to_string(), "evt-1".to_string());

        assert_eq!(index.entity_count(), 1);
        assert_eq!(index.event_count(), 1);
        assert_eq!(index.relation_count(), 1);
        assert!(!index.is_empty());

        let events = index.get_events_by_entity("ent-1").unwrap();
        assert!(events.contains("evt-1"));

        let entities = index.get_entities_by_event("evt-1").unwrap();
        assert!(entities.contains("ent-1"));
    }

    #[test]
    fn test_add_relation_dedupes_hashset_but_increments_count() {
        // 同一 (entity, event) 对重复添加：
        // - HashSet 去重（events.len() 仍为 1）
        // - relation_count 累加（与 SQL COUNT(*) 一致）
        let mut index = BidirectionalIndex::new();
        index.add_relation("ent-1".to_string(), "evt-1".to_string());
        index.add_relation("ent-1".to_string(), "evt-1".to_string());

        let events = index.get_events_by_entity("ent-1").unwrap();
        assert_eq!(events.len(), 1, "HashSet 应去重");

        // relation_count 与 SQL COUNT(*) 语义一致（计入重复行）
        assert_eq!(index.relation_count(), 2, "relation_count 应累加");
    }

    #[test]
    fn test_get_returns_none_for_missing_key() {
        let index = BidirectionalIndex::new();
        assert!(index.get_events_by_entity("missing").is_none());
        assert!(index.get_entities_by_event("missing").is_none());
    }
}
