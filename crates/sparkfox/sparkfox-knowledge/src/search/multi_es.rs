//! Sub-Step 12.1.1 — MULTI_ES 策略 ES-first 实现（spec §三 12.1.1）
//!
//! MULTI_ES 策略是与 MULTI / MULTI1 / HOPLLM 并列的第 4 种 SAG 检索策略。
//! ES-first 表示先用实体检索（Entity Search first）缩小候选集，再做多跳扩展。
//!
//! ## 算法（ES-first）
//! 1. **Step1（ES-first）**：query 直接作为 entity_name 查询 `entity` 表（跳过 Step2 NER 抽取）
//!    - SQL：`SELECT DISTINCT id FROM entity WHERE name LIKE '%query%' OR normalized_name = query`
//!    - 若无匹配实体，降级到 [`MultiStrategy`] 行为（jieba NER + BFS 扩展）
//! 2. **Step2-Step8**：用 Step1 匹配到的 entities 作为种子，复用 [`MultiStrategy`] 的 BFS 扩展逻辑
//!    - BFS 扩展 `max_hop` 跳内所有可达 events
//!    - 按 hop 升序排序 + `score = 1.0 / hop` 衰减 + 取 `top_k`
//!
//! ## 与其他策略对比
//! - **MULTI**：query → entity extraction（jieba NER）→ BFS 扩展（默认）
//! - **MULTI1**：query → entity extraction → 单跳剪枝（快速）
//! - **HOPLLM**：query → entity extraction → LLM 引导跳数（智能）
//! - **MULTI_ES**：query → **直接实体检索**（跳过 extraction）→ BFS 扩展（ES-first）
//!
//! ## 适用场景
//! - query 本身就是实体名（如「张三」）
//! - query 包含明确实体（无需 NER 抽取，加速检索）
//! - query 为缩写/简称（LIKE 匹配部分实体名）
//!
//! ## 降级策略
//! 若 ES-first 直接实体检索无匹配（query 不是任何 entity name 的子串，
//! 也不等于任何 normalized_name），则降级到 [`MultiStrategy`] 行为：
//! - 调用 jieba NER 从 query 抽取实体
//! - 用抽取到的实体文本作为 seeds 调用 `find_entity_ids` 匹配 entity 表
//! - 若 NER 也无结果，返回空 `hits`（不报错）
//!
//! `strategy_name` 始终保持 `"multi_es"`（降级不改变策略名，便于上层诊断）。
//!
//! ## 设计决策 — 方案 A（复制 Step3-Step8 逻辑）
//! spec §三 12.1.1 提出两种实现方案：
//! - **方案 A（采用）**：复制 [`MultiStrategy`] 的 Step3-Step8 BFS 扩展逻辑到本模块
//! - **方案 B（弃用）**：在 [`MultiStrategy`] 中新增 `search_with_seed_entities` 方法
//!
//! 选择方案 A 的原因：
//! - 避免修改 `multi.rs`（与并行 subagent 12.x 冲突）
//! - 保持模块独立性，便于后续重构
//! - 代码重复有限（BFS 核心算法 ~100 行），可接受
//!
//! ## Sync 约束
//! 与 [`MultiStrategy`] 相同，使用 `std::sync::Mutex<Connection>` 包装 rusqlite `Connection`
//! 以满足 `Send + Sync`。
//!
//! ## License
//! AGPL-3.0-only

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use rusqlite::Connection;

use sparkfox_core::{Error, Result};

use crate::jieba_ner::JiebaNer;
use super::multi_step::{step1_vectorize, step2_extract_entities_with_jieba, MultiState};
use super::{EntityRef, SearchHit, SearchResult, SearchStrategy};

// ---------------------------------------------------------------------------
// SQL 常量（与 multi.rs 保持一致，独立定义以避免跨模块依赖）
// ---------------------------------------------------------------------------

/// ES-first 直接实体检索 SQL（spec §三 12.1.1 Step1）
///
/// 用 query 直接作为 entity_name 查询 `entity` 表：
/// - `name LIKE '%query%'`：entity name 包含 query 子串（支持缩写/简称匹配）
/// - `normalized_name = query`：归一化名等于 query（精确匹配）
///
/// 两个条件用 OR 连接，任一命中即返回。
///
/// ## 参数绑定
/// - 第一个 `?` 绑定 `format!("%{}%", query)`（LIKE 通配符包裹）
/// - 第二个 `?` 绑定 `query`（精确匹配 normalized_name）
const SQL_FIND_ENTITY_IDS_BY_QUERY: &str = r#"
SELECT DISTINCT id FROM entity
WHERE name LIKE ? OR normalized_name = ?
"#;

/// 通过 `entity_id` 查找关联 `event_id` 列表的 SQL（与 multi.rs 一致）
///
/// 利用 P-01 反向索引 `idx_eer_entity_event` 高效查找。
const SQL_FIND_EVENTS_BY_ENTITY: &str = r#"
SELECT DISTINCT event_id FROM event_entity_relation WHERE entity_id = ?
"#;

/// 通过 `event_id` 查找关联 `entity_id` 列表的 SQL（与 multi.rs 一致）
///
/// 利用 P-01 正向索引 `idx_eer_event_entity` 高效查找。
/// `AND entity_id != ?` 排除来源 entity，避免回环。
const SQL_FIND_ENTITIES_BY_EVENT: &str = r#"
SELECT DISTINCT entity_id FROM event_entity_relation WHERE event_id = ? AND entity_id != ?
"#;

/// 通过 `event_id` 查找 event 详情（title / summary / chunk_id）
const SQL_FIND_EVENT_BY_ID: &str = r#"
SELECT id, title, summary, chunk_id FROM knowledge_event WHERE id = ?
"#;

/// 通过 `entity_id` 查找 entity 引用（name + type）的 SQL（与 multi.rs 一致）
///
/// LEFT JOIN `entity_type` 以获取 `entity_type.type`（如 "PERSON"）。
/// 若 `entity_type_id` 无对应记录（数据不一致），`entity_type` 返回 NULL，
/// 此时代码层回退为 `"UNKNOWN"`（避免因数据问题阻断检索）。
const SQL_FIND_ENTITY_REF_BY_ID: &str = r#"
SELECT e.id, et.type, e.name
FROM entity e
LEFT JOIN entity_type et ON e.entity_type_id = et.id
WHERE e.id = ?
"#;

/// 查找与给定实体文本匹配的 `entity.id` 列表的 SQL 模板（降级路径使用）
///
/// 复用 [`crate::search::atomic::AtomicStrategy`] 的匹配规则：
/// `entity.name IN (...) OR entity.normalized_name IN (...)`。
const SQL_FIND_ENTITY_IDS_TEMPLATE: &str = r#"
SELECT DISTINCT id FROM entity
WHERE name IN ({placeholders}) OR normalized_name IN ({placeholders})
"#;

/// MULTI_ES 默认最大跳数（与 [`super::multi::MAX_HOP`] 一致）
const DEFAULT_MAX_HOP: u8 = 3;

/// MULTI_ES 默认 top_k（与 [`super::multi::MultiStrategy`] 一致）
const DEFAULT_TOP_K: usize = 10;

// ---------------------------------------------------------------------------
// MultiEsStrategy
// ---------------------------------------------------------------------------

/// MULTI_ES 检索策略 — ES-first 多跳扩展（spec §三 12.1.1）
///
/// ES-first 多跳检索：先用实体检索（Entity Search first）缩小候选集，
/// 再复用 [`MultiStrategy`] 的 BFS 扩展完成多跳。
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::{MultiEsStrategy, SearchStrategy};
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// let strategy = MultiEsStrategy::new(conn);
/// let result = strategy.search("张三").await?;
/// // result.strategy_name == "multi_es"
/// for hit in &result.hits {
///     println!("hop={:?}: {} ({})", hit.hop, hit.event_id, hit.title);
/// }
/// ```
///
/// ## 与 [`MultiStrategy`] 的区别
/// - [`MultiStrategy`]：query → jieba NER 抽取 entity → entity 表匹配 → BFS 扩展
/// - [`MultiEsStrategy`]：query → **直接 entity 表 LIKE 匹配** → BFS 扩展（跳过 NER）
///
/// 当 ES-first 无匹配时降级到 [`MultiStrategy`] 等效行为（jieba NER + BFS）。
pub struct MultiEsStrategy {
    /// SQLite 连接（`Mutex` 包装以满足 `Sync` 约束）
    conn: Mutex<Connection>,
    /// jieba NER 分词器（仅用于 ES-first 失败时的降级路径）
    jieba: JiebaNer,
    /// 返回结果的最大行数
    top_k: usize,
    /// BFS 最大跳数（默认 3）
    max_hop: u8,
}

impl MultiEsStrategy {
    /// 创建默认 `top_k=10` / `max_hop=3` 的 [`MultiEsStrategy`]
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k: DEFAULT_TOP_K,
            max_hop: DEFAULT_MAX_HOP,
        }
    }

    /// 创建指定 `max_hop` 的 [`MultiEsStrategy`]（`top_k` 默认 10）
    ///
    /// `max_hop=1` 时退化为 ATOMIC 检索（仅返回直接关联 event）。
    pub fn new_with_max_hop(conn: Connection, max_hop: u8) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k: DEFAULT_TOP_K,
            max_hop,
        }
    }

    /// Builder 方法：设置 `max_hop`（链式调用）
    ///
    /// ## 用法
    /// ```ignore
    /// let strategy = MultiEsStrategy::new(conn).with_max_hop(2);
    /// ```
    pub fn with_max_hop(mut self, max_hop: u8) -> Self {
        self.max_hop = max_hop;
        self
    }

    /// Builder 方法：设置 `top_k`（链式调用）
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    /// ES-first Step1：用 query 直接作为 entity_name 查询 entity 表
    ///
    /// SQL：`SELECT DISTINCT id FROM entity WHERE name LIKE '%query%' OR normalized_name = query`
    ///
    /// ## 参数
    /// - `query`: 用户查询字符串（直接作为 entity name 匹配）
    ///
    /// ## 返回
    /// `Vec<String>`：匹配到的 entity_id 列表（可能为空，空时触发降级）
    ///
    /// ## 匹配规则
    /// - `name LIKE '%query%'`：entity name 包含 query 子串
    ///   - 例：query="张三" 匹配 name="张三"（精确子串）
    ///   - 例：query="张" 匹配 name="张三"（部分子串）
    /// - `normalized_name = query`：归一化名等于 query（精确匹配）
    fn find_entity_ids_by_query(&self, query: &str) -> Result<Vec<String>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        // LIKE 通配符包裹 query（spec §三 12.1.1 Step1）
        let like_pattern = format!("%{}%", query);

        let conn = self.conn.lock().map_err(|e| {
            Error::storage(
                format!("Mutex lock 失败: {e}"),
                "MultiEsStrategy::find_entity_ids_by_query",
            )
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITY_IDS_BY_QUERY).map_err(|e| {
            Error::storage(
                format!("prepare 失败: {e}"),
                "MultiEsStrategy::find_entity_ids_by_query",
            )
        })?;

        let rows = stmt
            .query_map(
                rusqlite::params![like_pattern, query],
                |row| {
                    let id: String = row.get(0)?;
                    Ok(id)
                },
            )
            .map_err(|e| {
                Error::storage(
                    format!("query 失败: {e}"),
                    "MultiEsStrategy::find_entity_ids_by_query",
                )
            })?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(
                    format!("row 失败: {e}"),
                    "MultiEsStrategy::find_entity_ids_by_query",
                )
            })?);
        }
        Ok(ids)
    }

    /// 降级路径：通过实体文本列表匹配 entity.id（与 [`MultiStrategy::find_entity_ids`] 等效）
    ///
    /// 用于 ES-first 失败时，从 jieba NER 抽取的实体文本匹配 entity 表。
    /// SQL：`SELECT DISTINCT id FROM entity WHERE name IN (...) OR normalized_name IN (...)`
    fn find_entity_ids_by_texts(&self, entity_texts: &[String]) -> Result<Vec<String>> {
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
            Error::storage(
                format!("Mutex lock 失败: {e}"),
                "MultiEsStrategy::find_entity_ids_by_texts",
            )
        })?;
        let mut stmt = conn.prepare(&sql).map_err(|e| {
            Error::storage(
                format!("prepare 失败: {e}"),
                "MultiEsStrategy::find_entity_ids_by_texts",
            )
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
            .map_err(|e| {
                Error::storage(
                    format!("query 失败: {e}"),
                    "MultiEsStrategy::find_entity_ids_by_texts",
                )
            })?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(
                    format!("row 失败: {e}"),
                    "MultiEsStrategy::find_entity_ids_by_texts",
                )
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 查找关联的 `event_id` 列表（与 [`MultiStrategy::find_events_by_entity`] 等效）
    fn find_events_by_entity(&self, entity_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(
                format!("Mutex lock 失败: {e}"),
                "MultiEsStrategy::find_events_by_entity",
            )
        })?;
        let mut stmt = conn.prepare(SQL_FIND_EVENTS_BY_ENTITY).map_err(|e| {
            Error::storage(
                format!("prepare 失败: {e}"),
                "MultiEsStrategy::find_events_by_entity",
            )
        })?;
        let rows = stmt
            .query_map([entity_id], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| {
                Error::storage(
                    format!("query 失败: {e}"),
                    "MultiEsStrategy::find_events_by_entity",
                )
            })?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(
                    format!("row 失败: {e}"),
                    "MultiEsStrategy::find_events_by_entity",
                )
            })?);
        }
        Ok(ids)
    }

    /// 通过 `event_id` 查找关联的 `entity_id` 列表（排除来源 entity）
    ///
    /// 与 [`MultiStrategy::find_entities_by_event`] 等效。
    fn find_entities_by_event(
        &self,
        event_id: &str,
        exclude_entity: &str,
    ) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(
                format!("Mutex lock 失败: {e}"),
                "MultiEsStrategy::find_entities_by_event",
            )
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITIES_BY_EVENT).map_err(|e| {
            Error::storage(
                format!("prepare 失败: {e}"),
                "MultiEsStrategy::find_entities_by_event",
            )
        })?;
        let rows = stmt
            .query_map(rusqlite::params![event_id, exclude_entity], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| {
                Error::storage(
                    format!("query 失败: {e}"),
                    "MultiEsStrategy::find_entities_by_event",
                )
            })?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(
                    format!("row 失败: {e}"),
                    "MultiEsStrategy::find_entities_by_event",
                )
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 查找完整的 [`EntityRef`]（含 entity_id / entity_type / name）
    ///
    /// 与 [`MultiStrategy::find_entity_ref`] 等效。LEFT JOIN `entity_type` 表获取类型信息；
    /// 若 entity_type 记录缺失，`entity_type` 字段回退为 `"UNKNOWN"`。
    fn find_entity_ref(&self, entity_id: &str) -> Result<Option<EntityRef>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(
                format!("Mutex lock 失败: {e}"),
                "MultiEsStrategy::find_entity_ref",
            )
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITY_REF_BY_ID).map_err(|e| {
            Error::storage(
                format!("prepare 失败: {e}"),
                "MultiEsStrategy::find_entity_ref",
            )
        })?;
        let mut rows = stmt
            .query_map([entity_id], |row| {
                let id: String = row.get(0)?;
                let entity_type: Option<String> = row.get(1)?;
                let name: String = row.get(2)?;
                Ok(EntityRef {
                    entity_id: id,
                    entity_type: entity_type.unwrap_or_else(|| "UNKNOWN".to_string()),
                    name,
                })
            })
            .map_err(|e| {
                Error::storage(
                    format!("query 失败: {e}"),
                    "MultiEsStrategy::find_entity_ref",
                )
            })?;
        if let Some(row) = rows.next() {
            let entity_ref = row.map_err(|e| {
                Error::storage(
                    format!("row 失败: {e}"),
                    "MultiEsStrategy::find_entity_ref",
                )
            })?;
            Ok(Some(entity_ref))
        } else {
            Ok(None)
        }
    }

    /// 通过 `event_id` 查找 event 详情（title / summary / chunk_id）
    ///
    /// 与 [`MultiStrategy::find_event_detail`] 等效。
    fn find_event_detail(
        &self,
        event_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(
                format!("Mutex lock 失败: {e}"),
                "MultiEsStrategy::find_event_detail",
            )
        })?;
        let mut stmt = conn.prepare(SQL_FIND_EVENT_BY_ID).map_err(|e| {
            Error::storage(
                format!("prepare 失败: {e}"),
                "MultiEsStrategy::find_event_detail",
            )
        })?;
        let mut rows = stmt
            .query_map([event_id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let chunk_id: Option<String> = row.get(3)?;
                Ok((id, title, summary, chunk_id))
            })
            .map_err(|e| {
                Error::storage(
                    format!("query 失败: {e}"),
                    "MultiEsStrategy::find_event_detail",
                )
            })?;
        if let Some(row) = rows.next() {
            let (_id, title, summary, chunk_id) = row.map_err(|e| {
                Error::storage(
                    format!("row 失败: {e}"),
                    "MultiEsStrategy::find_event_detail",
                )
            })?;
            Ok(Some((title, summary, chunk_id)))
        } else {
            Ok(None)
        }
    }

    /// BFS 多跳扩展核心算法（复制自 [`MultiStrategy::bfs_expand`]，方案 A）
    ///
    /// 从 `seed_entity_ids` 出发，沿 `event_entity_relation` 双向索引扩展，
    /// 返回 `max_hop` 跳内所有可达的 events 及其路径信息。
    ///
    /// ## 算法
    /// 1. 初始化队列：每个 seed entity 作为 (entity_id, hop=0, path=[]) 入队
    /// 2. 弹出 (entity_id, hop, path)：
    ///    - 若 `hop >= max_hop` 或 entity 已访问，跳过
    ///    - 标记 entity 已访问
    ///    - 查该 entity 关联的 events
    ///    - 对每个未访问的 event：
    ///      - 标记 event 已访问
    ///      - 记录 (event_id, hop+1, path + [entity_ref])
    ///      - 查该 event 关联的其他 entities（排除当前 entity）
    ///      - 将这些 entities 以 (other_entity_id, hop+1, new_path) 入队
    /// 3. 返回所有结果
    ///
    /// ## 去重
    /// - `visited_events`：同一 event 只记录首次到达（BFS 最短路径）
    /// - `visited_entities`：同一 entity 只扩展一次
    ///
    /// ## 返回
    /// `Vec<(event_id, hop, via_entities)>`：BFS 扩展结果
    ///
    /// ## 与 [`MultiStrategy::bfs_expand`] 的区别
    /// MULTI_ES 当前不实现 R-07 三道 LIMIT 阀门（spec §三 12.1.1 未要求），
    /// 仅依赖 `max_hop` 限制扩展深度。若后续需要阀门保护，可参考 multi.rs 实现。
    fn bfs_expand(
        &self,
        seed_entity_ids: &[String],
    ) -> Result<Vec<(String, u8, Vec<EntityRef>)>> {
        if seed_entity_ids.is_empty() || self.max_hop == 0 {
            return Ok(Vec::new());
        }

        let mut visited_events: HashSet<String> = HashSet::new();
        let mut visited_entities: HashSet<String> = HashSet::new();
        let mut results: Vec<(String, u8, Vec<EntityRef>)> = Vec::new();
        let mut queue: VecDeque<(String, u8, Vec<EntityRef>)> = VecDeque::new();

        // 初始化：seed entities 入队，hop=0（它们将产生 hop=1 的 events）
        for eid in seed_entity_ids {
            queue.push_back((eid.clone(), 0u8, Vec::new()));
        }

        while let Some((entity_id, hop, path)) = queue.pop_front() {
            // 阀门 1: max_hop（BFS 扩展深度上限）
            if hop >= self.max_hop {
                continue;
            }

            // 同一 entity 只扩展一次（避免环路）
            if !visited_entities.insert(entity_id.clone()) {
                continue;
            }

            // 查询 entity 完整引用（entity_id + entity_type + name），构建新的 path
            let entity_ref = match self.find_entity_ref(&entity_id)? {
                Some(r) => r,
                None => continue, // entity 可能已被删除，跳过
            };
            let mut new_path = path.clone();
            new_path.push(entity_ref);

            // 查该 entity 关联的 events
            let events = self.find_events_by_entity(&entity_id)?;

            for event_id in events {
                // 同一 event 只记录首次到达
                if !visited_events.insert(event_id.clone()) {
                    continue;
                }
                let event_hop = hop + 1;
                results.push((event_id.clone(), event_hop, new_path.clone()));

                // 查该 event 关联的其他 entities，继续扩展
                let next_entities = self.find_entities_by_event(&event_id, &entity_id)?;
                for next_entity_id in next_entities {
                    queue.push_back((next_entity_id, event_hop, new_path.clone()));
                }
            }
        }

        Ok(results)
    }

    /// 将 BFS 扩展结果转换为 [`SearchHit`] 列表
    ///
    /// 按 hop 升序排序（hop=1 优先），取 `top_k`。
    /// `score = 1.0 / hop`（跳数衰减）。
    /// `chunk_span = None`（MULTI_ES 不涉及 chunk 内位置）。
    ///
    /// 与 [`MultiStrategy::build_hits`] 等效。
    fn build_hits(
        &self,
        expansion: Vec<(String, u8, Vec<EntityRef>)>,
    ) -> Result<Vec<SearchHit>> {
        // 按 hop 升序排序（hop 小的优先），稳定排序保持 BFS 顺序
        let mut sorted = expansion;
        sorted.sort_by_key(|(_, hop, _)| *hop);

        // 取 top_k
        let limited: Vec<(String, u8, Vec<EntityRef>)> =
            sorted.into_iter().take(self.top_k).collect();

        let mut hits = Vec::with_capacity(limited.len());
        for (event_id, hop, via_entities) in limited {
            let (title, summary, chunk_id) = match self.find_event_detail(&event_id)? {
                Some(detail) => detail,
                None => continue, // event 可能已被删除（外键级联）
            };
            let score = 1.0 / hop as f64;
            hits.push(SearchHit {
                event_id,
                title,
                summary,
                chunk_id,
                score,
                hop: Some(hop),
                via_entities,
                chunk_span: None,
            });
        }
        Ok(hits)
    }
}

#[async_trait]
impl SearchStrategy for MultiEsStrategy {
    async fn search(&self, query: &str) -> Result<SearchResult> {
        let start = Instant::now();

        // ====================================================================
        // Step1（ES-first）：query 直接作为 entity_name 查询 entity 表
        // ====================================================================
        // 跳过 Step2 jieba NER 抽取，直接用 query 作为 entity name 在 entity 表中
        // 进行 LIKE + normalized_name 精确匹配。这是 MULTI_ES 与 MULTI 的核心区别。
        //
        // 匹配规则（SQL_FIND_ENTITY_IDS_BY_QUERY）：
        // - name LIKE '%query%'：entity name 包含 query 子串
        // - normalized_name = query：归一化名等于 query
        let entity_ids = self.find_entity_ids_by_query(query)?;

        // ====================================================================
        // 降级路径：ES-first 无匹配时降级到 MultiStrategy 行为
        // ====================================================================
        // 若 ES-first 直接检索无结果（query 不是任何 entity name 的子串，
        // 也不等于任何 normalized_name），则降级到 MultiStrategy 等效行为：
        // - Step1：query 向量化（mock embedding）
        // - Step2：jieba NER 抽取实体
        // - Step3：用 NER 抽取的实体文本匹配 entity 表
        // - Step5-8：BFS 扩展 + build_hits
        //
        // 降级不改变 strategy_name（保持 "multi_es"），便于上层诊断。
        let entity_ids = if entity_ids.is_empty() {
            // 降级到 MultiStrategy 行为：jieba NER 抽取 + entity 表匹配
            let state = MultiState::new(query);
            let state = step1_vectorize(state);
            let state = step2_extract_entities_with_jieba(state, &self.jieba);

            let entity_texts: Vec<String> = state
                .entities
                .iter()
                .map(|e| e.text.clone())
                .collect();
            self.find_entity_ids_by_texts(&entity_texts)?
        } else {
            entity_ids
        };

        // ====================================================================
        // Step5-7：BFS 多跳扩展（复制自 MultiStrategy::bfs_expand，方案 A）
        // ====================================================================
        // 从 seed entities 出发，沿 event_entity_relation 双向索引扩展 max_hop 跳内
        // 所有可达 events。BFS 内置去重（visited_events / visited_entities）。
        let expansion = self.bfs_expand(&entity_ids)?;

        // ====================================================================
        // Step8：构建 SearchResult
        // ====================================================================
        // - 按 hop 升序排序 + score=1/hop 衰减 + 取 top_k
        // - strategy_name = "multi_es"（降级不改变）
        // - latency_ms 由 Instant::now() 测量
        let hits = self.build_hits(expansion)?;
        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResult {
            hits,
            latency_ms,
            strategy_name: "multi_es".to_string(),
        })
    }

    fn name(&self) -> &str {
        "multi_es"
    }
}
