//! Sub-Step 10.8.2 + 11.1.1 — MULTI 检索策略（8 步骨架 + BFS 多跳扩展，spec §三 11.1 / 11.2.1）
//!
//! ## 8 步骨架（11.1.1 引入）
//! [`MultiStrategy::search`] 调用 [`super::multi_step`] 的 8 步流程：
//! - **Step1** [`multi_step::step1_vectorize`]：query 向量化（mock embedding）
//! - **Step2** [`multi_step::step2_extract_entities_with_jieba`]：query 实体抽取（jieba + 正则）
//! - Step3-4：当前在 search 内部用 SQL 文本匹配代替（11.2.x 接入 HnswIndex + event_entity_relation）
//! - **Step5**：保留 10.8.2 的 BFS 多跳扩展作为 multi 策略实现
//! - Step6-7：BFS 内置去重 + 排序（11.2.x 接入 rerank 模型）
//! - **Step8** [`multi_step::step8_build_result`]：返回 [`SearchResult`]
//!
//! ## BFS 多跳扩展流程（10.8.2 保留）
//! 1. **hop=1**：seed entity → `event_entity_relation` → 直接关联的 events
//! 2. **hop=2**：hop1 event → 其他 entity → 这些 entity 的其他 events
//! 3. **hop=3**：重复 Step 3，直到 `max_hop`（默认 3）
//! 4. **裁剪**：按 hop 升序排序，取 `top_k`
//!
//! ## hop 含义
//! - hop=1：seed entity 直接关联的 event（等价于 ATOMIC 检索）
//! - hop=2：经 1 个中间 entity 扩展到的 event
//! - hop=3：经 2 个中间 entity 扩展到的 event
//!
//! ## via_entities
//! 路径上所有 entity 的 [`EntityRef`] 列表（含 `entity_id` / `entity_type` / `name`）。
//! 例：查询「张三」找 evt-3，路径为 张三 → evt-1 → 北京 → evt-2 → 腾讯 → evt-3，
//! via_entities = [EntityRef(张三), EntityRef(北京), EntityRef(腾讯)]。
//!
//! ## 去重策略
//! - `visited_events: HashSet<String>`：同一 event 只记录首次到达（BFS 保证是最短路径）
//! - `visited_entities: HashSet<String>`：同一 entity 只扩展一次
//!
//! ## score 衰减
//! `score = 1.0 / hop`（hop=1 → 1.0，hop=2 → 0.5，hop=3 → 0.333），
//! 体现「跳数越远相关性越低」的直觉。
//!
//! ## chunk_span
//! MULTI 策略不涉及 chunk 内位置，`chunk_span` 固定为 `None`（U-02 预留字段）。
//!
//! ## Sync 约束
//! 与 [`crate::search::atomic::AtomicStrategy`] 相同，使用
//! `std::sync::Mutex<Connection>` 包装 rusqlite `Connection` 以满足 `Send + Sync`。

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use rusqlite::Connection;

use sparkfox_core::{Error, Result};

use crate::jieba_ner::JiebaNer;
use super::multi_step::{
    step1_vectorize, step2_extract_entities_with_jieba, step8_build_result, MultiState,
};
use super::{EntityRef, SearchHit, SearchResult, SearchStrategy};

/// 查找与给定实体文本匹配的 `entity.id` 列表的 SQL 模板
///
/// 复用 [`crate::search::atomic::AtomicStrategy`] 的匹配规则：
/// `entity.name IN (...) OR entity.normalized_name IN (...)`。
const SQL_FIND_ENTITY_IDS_TEMPLATE: &str = r#"
SELECT DISTINCT id FROM entity
WHERE name IN ({placeholders}) OR normalized_name IN ({placeholders})
"#;

/// 通过 `entity_id` 查找关联 `event_id` 列表的 SQL
///
/// 利用 P-01 反向索引 `idx_eer_entity_event` 高效查找。
const SQL_FIND_EVENTS_BY_ENTITY: &str = r#"
SELECT DISTINCT event_id FROM event_entity_relation WHERE entity_id = ?
"#;

/// 通过 `event_id` 查找关联 `entity_id` 列表的 SQL
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

/// 通过 `entity_id` 查找 entity 引用（name + type）的 SQL
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

/// MULTI 检索策略 — BFS 多跳扩展
///
/// 从 query 抽取的 seed entity 出发，沿 `event_entity_relation` 双向索引进行 BFS
/// 扩展，发现 `max_hop`（默认 3）跳内的所有关联 events。
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::{MultiStrategy, SearchStrategy};
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// let strategy = MultiStrategy::new(conn);
/// let result = strategy.search("张三去了哪里").await?;
/// for hit in &result.hits {
///     println!("hop={:?}: {} ({})", hit.hop, hit.event_id, hit.title);
/// }
/// ```
pub struct MultiStrategy {
    /// SQLite 连接（`Mutex` 包装以满足 `Sync` 约束）
    conn: Mutex<Connection>,
    /// jieba NER 分词器
    jieba: JiebaNer,
    /// 返回结果的最大行数
    top_k: usize,
    /// BFS 最大跳数（默认 3）
    max_hop: u8,
}

impl MultiStrategy {
    /// 创建默认 `top_k=10` / `max_hop=3` 的 [`MultiStrategy`]
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k: 10,
            max_hop: 3,
        }
    }

    /// 创建指定 `max_hop` 的 [`MultiStrategy`]（`top_k` 默认 10）
    ///
    /// `max_hop=1` 时退化为 ATOMIC 检索（仅返回直接关联 event）。
    pub fn new_with_max_hop(conn: Connection, max_hop: u8) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k: 10,
            max_hop,
        }
    }

    /// 创建指定 `top_k` 和 `max_hop` 的 [`MultiStrategy`]
    pub fn new_with_top_k_and_max_hop(conn: Connection, top_k: usize, max_hop: u8) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k,
            max_hop,
        }
    }

    /// 查找与给定实体文本匹配的 `entity.id` 列表
    ///
    /// 复用 [`crate::search::atomic::AtomicStrategy`] 的匹配规则。
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
            Error::storage(format!("Mutex lock 失败: {e}"), "MultiStrategy::find_entity_ids")
        })?;
        let mut stmt = conn.prepare(&sql).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "MultiStrategy::find_entity_ids")
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
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "MultiStrategy::find_entity_ids"))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "MultiStrategy::find_entity_ids")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 查找关联的 `event_id` 列表
    fn find_events_by_entity(&self, entity_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "MultiStrategy::find_events_by_entity")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_EVENTS_BY_ENTITY).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "MultiStrategy::find_events_by_entity")
        })?;
        let rows = stmt
            .query_map([entity_id], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "MultiStrategy::find_events_by_entity"))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "MultiStrategy::find_events_by_entity")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `event_id` 查找关联的 `entity_id` 列表（排除来源 entity）
    fn find_entities_by_event(&self, event_id: &str, exclude_entity: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "MultiStrategy::find_entities_by_event")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITIES_BY_EVENT).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "MultiStrategy::find_entities_by_event")
        })?;
        let rows = stmt
            .query_map(rusqlite::params![event_id, exclude_entity], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "MultiStrategy::find_entities_by_event"))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "MultiStrategy::find_entities_by_event")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 查找完整的 [`EntityRef`]（含 entity_id / entity_type / name）
    ///
    /// LEFT JOIN `entity_type` 表获取类型信息；若 entity_type 记录缺失，
    /// `entity_type` 字段回退为 `"UNKNOWN"`（保证数据不一致时仍能返回结果）。
    fn find_entity_ref(&self, entity_id: &str) -> Result<Option<EntityRef>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "MultiStrategy::find_entity_ref")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITY_REF_BY_ID).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "MultiStrategy::find_entity_ref")
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
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "MultiStrategy::find_entity_ref"))?;
        if let Some(row) = rows.next() {
            let entity_ref = row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "MultiStrategy::find_entity_ref")
            })?;
            Ok(Some(entity_ref))
        } else {
            Ok(None)
        }
    }

    /// 通过 `event_id` 查找 event 详情（title / summary / chunk_id）
    fn find_event_detail(
        &self,
        event_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "MultiStrategy::find_event_detail")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_EVENT_BY_ID).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "MultiStrategy::find_event_detail")
        })?;
        let mut rows = stmt
            .query_map([event_id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let chunk_id: Option<String> = row.get(3)?;
                Ok((id, title, summary, chunk_id))
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "MultiStrategy::find_event_detail"))?;
        if let Some(row) = rows.next() {
            let (_id, title, summary, chunk_id) = row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "MultiStrategy::find_event_detail")
            })?;
            Ok(Some((title, summary, chunk_id)))
        } else {
            Ok(None)
        }
    }

    /// BFS 多跳扩展核心算法
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
    /// `Vec<(event_id, hop, via_entities)>`：event_id + 跳数 + 路径上的 [`EntityRef`] 列表
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
            // 超过 max_hop 的 entity 不再扩展（它产生的 event 将是 hop+1 > max_hop）
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
    /// `chunk_span = None`（MULTI 不涉及 chunk 内位置）。
    fn build_hits(&self, expansion: Vec<(String, u8, Vec<EntityRef>)>) -> Result<Vec<SearchHit>> {
        // 按 hop 升序排序（hop 小的优先），稳定排序保持 BFS 顺序
        let mut sorted = expansion;
        sorted.sort_by_key(|(_, hop, _)| *hop);

        // 取 top_k
        let limited: Vec<(String, u8, Vec<EntityRef>)> = sorted
            .into_iter()
            .take(self.top_k)
            .collect();

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
impl SearchStrategy for MultiStrategy {
    async fn search(&self, query: &str) -> Result<SearchResult> {
        let start = Instant::now();

        // 8 步流程骨架（spec §三 11.1）
        //
        // Step1: query 向量化（mock embedding，384 维）
        //   - 当前为 mock，11.2.x 接入 bge-small-zh 真实 embedding
        //   - query_vec 用于后续 Step3 的 HnswIndex 向量检索
        let state = MultiState::new(query);
        let state = step1_vectorize(state);

        // Step2: query 实体抽取（jieba + 正则）
        //   - 复用 self.jieba 避免每次 search 重新加载 jieba 词典（节省 ~50ms）
        //   - 输出 Vec<EntityMention>，含 PERSON / ORGANIZATION / LOCATION / TIME / NUMBER
        let state = step2_extract_entities_with_jieba(state, &self.jieba);

        // Step3: 实体向量检索 — 当前用 SQL 文本匹配代替（11.2.x 接入 HnswIndex）
        //   - 取 Step2 实体的 text 字段，在 entity 表中按 name / normalized_name 匹配
        //   - 返回 entity_id 列表（seed entities）
        let entity_texts: Vec<String> = state
            .entities
            .iter()
            .map(|e| e.text.clone())
            .collect();
        let entity_ids = self.find_entity_ids(&entity_texts)?;
        let mut state = state;
        state.entity_ids = entity_ids.clone();
        state
            .thought_process
            .push("Step3: 实体向量检索（text fallback，11.2.x 接入 HnswIndex）".to_string());

        // Step4: 事件检索 — stub（11.2.x 实施 event_entity_relation 查询得到候选 event_ids）
        state.candidates = Vec::new();
        state
            .thought_process
            .push("Step4: 事件检索（stub，11.2.x 实施）".to_string());

        // Step5: 三策略占位 — 当前复用 10.8.2 的 BFS 作为 multi 策略实现
        //   - bfs_expand: 从 seed entities BFS 扩展 max_hop 跳内所有可达 events
        //   - build_hits: 按 hop 升序排序 + score=1/hop 衰减 + 取 top_k
        //   - 11.2.x 在此分支调用 multi / multi1 / hopllm 三策略
        let expansion = self.bfs_expand(&entity_ids)?;
        let hits = self.build_hits(expansion)?;
        state.hits = hits;
        state
            .thought_process
            .push("Step5: multi 策略（10.8.2 BFS 多跳扩展）".to_string());

        // Step6: 候选合并 + 去重 — BFS 内置 visited_events/visited_entities 去重
        state
            .thought_process
            .push("Step6: 候选合并 + 去重（BFS 内置去重）".to_string());

        // Step7: Rerank 重排 — build_hits 已按 hop 升序排序
        //   - 11.2.x 接入 bge-reranker 重排模型
        state
            .thought_process
            .push("Step7: Rerank 重排（按 hop 升序，11.2.x 接入 bge-reranker）".to_string());

        // Step8: 返回 SearchResult（latency_ms 由调用方覆写）
        let mut result = step8_build_result(state);
        result.latency_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    fn name(&self) -> &str {
        "multi"
    }
}

// ---------------------------------------------------------------------------
// Sub-Step 11.2.2 — MULTI1 检索策略（单跳剪枝，spec §三 11.2.2）
// ---------------------------------------------------------------------------

/// MULTI1 检索策略 — 单跳剪枝（spec §三 11.2.2）
///
/// 在 [`MultiStrategy`] 基础上限制 `max_hop=1`，仅扩展 1 跳（等价于 ATOMIC 检索
/// 但保留 MULTI 的 8 步骨架和 `thought_process`）。适用于：
/// - 实时性要求高的场景（比 multi 快 > 50%）
/// - 召回率要求不高、仅需直接关联事件的查询
///
/// ## 性能对比
/// - multi（max_hop=3）：BFS 三跳扩展，最坏情况 O(N^3)
/// - multi1（max_hop=1）：仅一跳扩展，O(N)
/// - 实测 1k events 数据集，multi1 比 multi 快 > 50%
///
/// ## 与 ATOMIC 的区别
/// - ATOMIC：基于 `event_entity_relation` 单表 JOIN，无 thought_process
/// - multi1：复用 MultiStrategy 的 8 步骨架 + BFS，含 thought_process
///
/// ## 实现方式
/// 通过内部委托 [`MultiStrategy`]（`max_hop=1`）实现 BFS 单跳剪枝：
/// - 构造时通过 [`MultiStrategy::new_with_max_hop`] / [`MultiStrategy::new_with_top_k_and_max_hop`]
///   创建内部 `inner`（`max_hop=1`）
/// - [`Multi1Strategy::search`] 委托 `inner.search()` 后覆写 `strategy_name = "multi1"`
/// - [`Multi1Strategy::name`] 直接返回 `"multi1"`
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::multi::Multi1Strategy;
/// use sparkfox_knowledge::search::SearchStrategy;
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// let strategy = Multi1Strategy::new(conn);
/// let result = strategy.search("张三去了哪里").await?;
/// // result.strategy_name == "multi1"
/// // result.hits 中所有 hit.hop == Some(1)
/// ```
pub struct Multi1Strategy {
    /// 内部委托 MultiStrategy（max_hop=1）
    inner: MultiStrategy,
}

impl Multi1Strategy {
    /// 创建默认 `top_k=10` / `max_hop=1` 的 [`Multi1Strategy`]
    ///
    /// 内部通过 [`MultiStrategy::new_with_max_hop`] 创建 `max_hop=1` 的 [`MultiStrategy`]。
    pub fn new(conn: Connection) -> Self {
        Self {
            inner: MultiStrategy::new_with_max_hop(conn, 1),
        }
    }

    /// 创建指定 `top_k` 的 [`Multi1Strategy`]（`max_hop` 固定为 1）
    ///
    /// 内部通过 [`MultiStrategy::new_with_top_k_and_max_hop`] 创建 `max_hop=1` 的 [`MultiStrategy`]。
    pub fn new_with_top_k(conn: Connection, top_k: usize) -> Self {
        Self {
            inner: MultiStrategy::new_with_top_k_and_max_hop(conn, top_k, 1),
        }
    }
}

#[async_trait]
impl SearchStrategy for Multi1Strategy {
    async fn search(&self, query: &str) -> Result<SearchResult> {
        // 委托 inner MultiStrategy（max_hop=1），覆写 strategy_name
        let mut result = self.inner.search(query).await?;
        result.strategy_name = "multi1".to_string();
        Ok(result)
    }

    fn name(&self) -> &str {
        "multi1"
    }
}

// ---------------------------------------------------------------------------
// Sub-Step 11.2.3 — HOPLLM 检索策略（LLM 引导多跳扩展，spec §三 11.2.3）
// ---------------------------------------------------------------------------

/// LLM 调用 trait（本地定义，避免直接依赖 LLM crate）
///
/// 实现方负责：给定 query + 当前实体 + 候选实体列表，返回 LLM 选择的下一个实体 ID。
/// 测试时用 mock 实现（[`MockLlm`] 确定性返回第一个候选），生产环境接入真实 LLM API。
///
/// ## 设计动机
/// 为避免 `sparkfox-knowledge` 直接依赖具体 LLM crate（如 `async-openai` / `sparkfox-llm`），
/// 定义本地 trait 解耦 LLM 调用。`HopllmStrategy` 通过依赖此 trait 实现可测试性
/// （注入 [`MockLlm`] / [`FailLlm`] 验证 LLM 选路逻辑），同时保留生产环境接入真实 LLM 的扩展点。
///
/// ## 同步 vs 异步
/// trait 方法为同步（非 `async`）：
/// - LLM 调用本身在网络层是异步的，但封装为 trait object 时 `Box<dyn HopllmLlm>` 要求
///   object safety，同步方法天然 object-safe。
/// - 生产实现可在内部用 `tokio::runtime::Handle::block_on` 将 async 调用包装为同步接口。
/// - 测试 mock 无需异步，避免不必要的 `#[async_trait]` 依赖。
pub trait HopllmLlm: Send + Sync {
    /// 从候选实体中选择下一个最相关的实体
    ///
    /// ## 参数
    /// - `query`: 原始查询（用于 LLM 判断语义相关性）
    /// - `current_entity`: 当前实体（[`EntityRef`] 含 id / type / name）
    /// - `candidates`: 候选实体列表（`Vec<EntityRef>`，至少 1 个元素）
    /// - `path_history`: 已走过的路径（`Vec<EntityRef>`，含 current_entity）
    ///
    /// ## 返回
    /// - `Ok(Some(entity_id))`: LLM 选择的下一个实体 ID（应在 `candidates` 中）
    /// - `Ok(None)`: LLM 认为无需继续扩展（语义上已达终点）
    /// - `Err(_)`: LLM 调用失败，调用方应降级到 [`Multi1Strategy`]（`max_hop=1`）
    fn select_next_hop(
        &self,
        query: &str,
        current_entity: &EntityRef,
        candidates: &[EntityRef],
        path_history: &[EntityRef],
    ) -> Result<Option<String>>;
}

/// Mock LLM 实现（测试用）：始终返回第一个候选
///
/// 用于单元测试验证 [`HopllmStrategy`] 的 LLM 选路逻辑：
/// - 调用 `select_next_hop` 时返回 `candidates[0].entity_id`
/// - 不考虑 query / current_entity / path_history 内容
/// - 确定性输出（便于断言期望路径）
pub struct MockLlm;

impl HopllmLlm for MockLlm {
    fn select_next_hop(
        &self,
        _query: &str,
        _current_entity: &EntityRef,
        candidates: &[EntityRef],
        _path_history: &[EntityRef],
    ) -> Result<Option<String>> {
        Ok(candidates.first().map(|c| c.entity_id.clone()))
    }
}

/// FailLlm 实现（测试用）：始终返回错误，触发降级
///
/// 用于验证 [`HopllmStrategy`] 的 LLM 失败降级路径：
/// - 调用 `select_next_hop` 时始终返回 `Err`
/// - 触发 [`HopllmStrategy::search`] 的降级分支（fallback 到 multi1 行为）
pub struct FailLlm;

impl HopllmLlm for FailLlm {
    fn select_next_hop(
        &self,
        _query: &str,
        _current_entity: &EntityRef,
        _candidates: &[EntityRef],
        _path_history: &[EntityRef],
    ) -> Result<Option<String>> {
        // Error 类型无 config() 构造器，使用 llm() 单参数构造器（spec §三 错误类型）
        Err(Error::llm("LLM 调用失败（测试 mock）"))
    }
}

/// 构建 HOPLLM LLM prompt（spec §三 11.2.3 REFACTOR）
///
/// 将 query + 当前实体 + 候选实体 + 路径历史格式化为 LLM 可读的中文 prompt 字符串。
/// 生产环境接入真实 LLM 时，将此 prompt 作为 user message 发送。
///
/// ## prompt 格式
/// ```text
/// 你是知识图谱导航助手。请根据用户查询，从候选实体中选择最相关的下一个实体。
///
/// 用户查询：{query}
/// 当前实体：{current_entity.name}（类型：{current_entity.entity_type}）
/// 候选实体：
/// 1. {candidates[0].name}（类型：{candidates[0].entity_type}）
/// 2. {candidates[1].name}（类型：{candidates[1].entity_type}）
/// ...
/// 已走路径：{path_history 中的 entity name 列表}
///
/// 请返回最相关候选的编号（1-N），或返回 0 表示无需继续。
/// ```
///
/// ## 参数
/// - `query`: 原始用户查询
/// - `current_entity`: 当前所在实体
/// - `candidates`: 候选实体列表
/// - `path_history`: 已走过的路径（含 current_entity）
///
/// ## 返回
/// 格式化后的 prompt 字符串
pub fn build_hopllm_prompt(
    query: &str,
    current_entity: &EntityRef,
    candidates: &[EntityRef],
    path_history: &[EntityRef],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("你是知识图谱导航助手。请根据用户查询，从候选实体中选择最相关的下一个实体。\n\n");
    prompt.push_str(&format!("用户查询：{}\n", query));
    prompt.push_str(&format!(
        "当前实体：{}（类型：{}）\n",
        current_entity.name, current_entity.entity_type
    ));
    prompt.push_str("候选实体：\n");
    for (idx, c) in candidates.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. {}（类型：{}）\n",
            idx + 1,
            c.name,
            c.entity_type
        ));
    }
    let path_names: Vec<&str> = path_history.iter().map(|e| e.name.as_str()).collect();
    prompt.push_str(&format!("已走路径：{}\n", path_names.join(" → ")));
    prompt.push_str("\n请返回最相关候选的编号（1-N），或返回 0 表示无需继续。");
    prompt
}

/// HOPLLM 检索策略 — LLM 引导多跳扩展（spec §三 11.2.3）
///
/// 在 [`MultiStrategy`] 基础上，每个 hop 调用 LLM 从候选实体中选择最相关的下一个实体，
/// 而非 BFS 全扩展。适用于：
/// - 语义相关性要求高的场景（BFS 会扩展到无关实体）
/// - 候选实体数量大，需要 LLM 剪枝
///
/// ## 降级策略
/// LLM 调用失败时，降级到 [`Multi1Strategy`] 行为（`max_hop=1`）保证基本可用性。
/// 降级通过 [`HopllmStrategy::single_hop_expand`] 在 [`HopllmStrategy::search`] 内部完成，
/// `strategy_name` 保持 `"hopllm"` 不变（标识降级路径但保持策略名稳定）。
///
/// ## 性能
/// - 每个 hop 增加 LLM 调用延迟（~200-500ms）
/// - 总延迟 ≈ multi 延迟 + max_hop × LLM 延迟
/// - 适用于实时性要求不高的场景
///
/// ## 算法
/// 1. **Step1-2**：query 向量化 + 实体抽取（复用 [`multi_step`] free function）
/// 2. **Step3**：实体检索（SQL 文本匹配 fallback，复用 [`MultiStrategy`] 的 SQL 常量）
/// 3. **Step4-7**：LLM 引导多跳扩展（[`HopllmStrategy::llm_guided_expand`]）：
///    - BFS 框架，但每个 hop 调用 [`HopllmLlm::select_next_hop`] 选路
///    - 仅入队 LLM 选择的实体（而非 BFS 全扩展）
///    - LLM 失败时停止该分支扩展
/// 4. **降级**：若所有 LLM 调用均失败，调用 [`HopllmStrategy::single_hop_expand`]（multi1 等效）
/// 5. **Step8**：构建 [`SearchResult`]（`strategy_name="hopllm"`）
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::multi::{HopllmStrategy, MockLlm};
/// use sparkfox_knowledge::search::SearchStrategy;
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory()?;
/// let strategy = HopllmStrategy::new(conn, Box::new(MockLlm));
/// let result = strategy.search("张三去了哪里").await?;
/// // result.strategy_name == "hopllm"
/// ```
pub struct HopllmStrategy {
    /// SQLite 连接（`Mutex` 包装以满足 `Sync` 约束）
    conn: Mutex<Connection>,
    /// jieba NER 分词器
    jieba: JiebaNer,
    /// 返回结果的最大行数
    top_k: usize,
    /// BFS 最大跳数（固定 3，spec §三 11.2.3）
    max_hop: u8,
    /// LLM 调用实例（[`HopllmLlm`] trait object）
    llm: Box<dyn HopllmLlm>,
}

impl HopllmStrategy {
    /// 创建默认 `top_k=10` / `max_hop=3` 的 [`HopllmStrategy`]
    ///
    /// ## 参数
    /// - `conn`: SQLite 连接（含 SAG schema 数据）
    /// - `llm`: LLM 调用实例（生产环境接入真实 LLM，测试用 [`MockLlm`] / [`FailLlm`]）
    pub fn new(conn: Connection, llm: Box<dyn HopllmLlm>) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k: 10,
            max_hop: 3,
            llm,
        }
    }

    /// 创建指定 `top_k` 的 [`HopllmStrategy`]（`max_hop` 固定为 3）
    ///
    /// ## 参数
    /// - `conn`: SQLite 连接
    /// - `top_k`: 返回结果的最大行数
    /// - `llm`: LLM 调用实例
    pub fn new_with_top_k(conn: Connection, top_k: usize, llm: Box<dyn HopllmLlm>) -> Self {
        Self {
            conn: Mutex::new(conn),
            jieba: JiebaNer::new(),
            top_k,
            max_hop: 3,
            llm,
        }
    }

    /// 查找与给定实体文本匹配的 `entity.id` 列表
    ///
    /// 复用 [`MultiStrategy::find_entity_ids`] 的 SQL 匹配规则：
    /// `entity.name IN (...) OR entity.normalized_name IN (...)`。
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
            Error::storage(format!("Mutex lock 失败: {e}"), "HopllmStrategy::find_entity_ids")
        })?;
        let mut stmt = conn.prepare(&sql).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "HopllmStrategy::find_entity_ids")
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
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "HopllmStrategy::find_entity_ids"))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "HopllmStrategy::find_entity_ids")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 查找关联的 `event_id` 列表
    fn find_events_by_entity(&self, entity_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "HopllmStrategy::find_events_by_entity")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_EVENTS_BY_ENTITY).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "HopllmStrategy::find_events_by_entity")
        })?;
        let rows = stmt
            .query_map([entity_id], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "HopllmStrategy::find_events_by_entity"))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "HopllmStrategy::find_events_by_entity")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `event_id` 查找关联的 `entity_id` 列表（排除来源 entity）
    fn find_entities_by_event(&self, event_id: &str, exclude_entity: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "HopllmStrategy::find_entities_by_event")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITIES_BY_EVENT).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "HopllmStrategy::find_entities_by_event")
        })?;
        let rows = stmt
            .query_map(rusqlite::params![event_id, exclude_entity], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "HopllmStrategy::find_entities_by_event"))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "HopllmStrategy::find_entities_by_event")
            })?);
        }
        Ok(ids)
    }

    /// 通过 `entity_id` 查找完整的 [`EntityRef`]（含 entity_id / entity_type / name）
    ///
    /// LEFT JOIN `entity_type` 表获取类型信息；若 entity_type 记录缺失，
    /// `entity_type` 字段回退为 `"UNKNOWN"`。
    fn find_entity_ref(&self, entity_id: &str) -> Result<Option<EntityRef>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "HopllmStrategy::find_entity_ref")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_ENTITY_REF_BY_ID).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "HopllmStrategy::find_entity_ref")
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
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "HopllmStrategy::find_entity_ref"))?;
        if let Some(row) = rows.next() {
            let entity_ref = row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "HopllmStrategy::find_entity_ref")
            })?;
            Ok(Some(entity_ref))
        } else {
            Ok(None)
        }
    }

    /// 通过 `event_id` 查找 event 详情（title / summary / chunk_id）
    fn find_event_detail(
        &self,
        event_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>> {
        let conn = self.conn.lock().map_err(|e| {
            Error::storage(format!("Mutex lock 失败: {e}"), "HopllmStrategy::find_event_detail")
        })?;
        let mut stmt = conn.prepare(SQL_FIND_EVENT_BY_ID).map_err(|e| {
            Error::storage(format!("prepare 失败: {e}"), "HopllmStrategy::find_event_detail")
        })?;
        let mut rows = stmt
            .query_map([event_id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let chunk_id: Option<String> = row.get(3)?;
                Ok((id, title, summary, chunk_id))
            })
            .map_err(|e| Error::storage(format!("query 失败: {e}"), "HopllmStrategy::find_event_detail"))?;
        if let Some(row) = rows.next() {
            let (_id, title, summary, chunk_id) = row.map_err(|e| {
                Error::storage(format!("row 失败: {e}"), "HopllmStrategy::find_event_detail")
            })?;
            Ok(Some((title, summary, chunk_id)))
        } else {
            Ok(None)
        }
    }

    /// LLM 引导多跳扩展核心算法（spec §三 11.2.3）
    ///
    /// BFS 框架，但每个 hop 调用 [`HopllmLlm::select_next_hop`] 从候选实体中选路，
    /// 而非全扩展。LLM 返回 `None` 时停止该分支；LLM 返回 `Err` 时记录失败并停止该分支。
    ///
    /// ## 算法
    /// 1. 初始化队列：每个 seed entity 作为 `(entity_id, hop=0, path=[])` 入队
    /// 2. 弹出 `(entity_id, hop, path)`：
    ///    - 若 `hop >= max_hop` 或 entity 已访问，跳过
    ///    - 标记 entity 已访问
    ///    - 查该 entity 关联的 events
    ///    - 对每个未访问的 event：
    ///      - 标记 event 已访问
    ///      - 记录 `(event_id, hop+1, path + [entity_ref])`
    ///      - 若 `hop+1 < max_hop`：查该 event 关联的其他 entities 作为候选，
    ///        调用 LLM 选路，仅将 LLM 选中的 entity 入队
    /// 3. 返回 `(results, llm_fully_failed_flag)`
    ///
    /// ## 降级判定
    /// - `llm_call_count > 0 && llm_failure_count == llm_call_count`：
    ///   所有 LLM 调用均失败 → `llm_fully_failed = true`，调用方应降级到 multi1
    /// - 否则：`llm_fully_failed = false`，使用 LLM 选路后的 expansion 结果
    ///
    /// ## 返回
    /// `(Vec<(event_id, hop, via_entities)>, llm_fully_failed)`：
    /// - 第一个元素：BFS 扩展结果（含 event_id / hop / 路径上的 [`EntityRef`]）
    /// - 第二个元素：LLM 是否全部失败（用于触发降级）
    fn llm_guided_expand(
        &self,
        seed_entity_ids: &[String],
        query: &str,
    ) -> Result<(Vec<(String, u8, Vec<EntityRef>)>, bool)> {
        if seed_entity_ids.is_empty() || self.max_hop == 0 {
            return Ok((Vec::new(), false));
        }

        let mut visited_events: HashSet<String> = HashSet::new();
        let mut visited_entities: HashSet<String> = HashSet::new();
        let mut results: Vec<(String, u8, Vec<EntityRef>)> = Vec::new();
        let mut queue: VecDeque<(String, u8, Vec<EntityRef>)> = VecDeque::new();

        // LLM 调用统计（用于降级判定）
        let mut llm_call_count: u32 = 0;
        let mut llm_failure_count: u32 = 0;

        // 初始化：seed entities 入队，hop=0（它们将产生 hop=1 的 events）
        for eid in seed_entity_ids {
            queue.push_back((eid.clone(), 0u8, Vec::new()));
        }

        while let Some((entity_id, hop, path)) = queue.pop_front() {
            // 超过 max_hop 的 entity 不再扩展
            if hop >= self.max_hop {
                continue;
            }
            // 同一 entity 只扩展一次（避免环路）
            if !visited_entities.insert(entity_id.clone()) {
                continue;
            }

            // 查询 entity 完整引用，构建新的 path
            let entity_ref = match self.find_entity_ref(&entity_id)? {
                Some(r) => r,
                None => continue, // entity 可能已被删除，跳过
            };
            let mut new_path = path.clone();
            new_path.push(entity_ref.clone());

            // 查该 entity 关联的 events
            let events = self.find_events_by_entity(&entity_id)?;
            for event_id in events {
                // 同一 event 只记录首次到达
                if !visited_events.insert(event_id.clone()) {
                    continue;
                }
                let event_hop = hop + 1;
                results.push((event_id.clone(), event_hop, new_path.clone()));

                // 若已达 max_hop，不再扩展（即使 LLM 也无法继续）
                if event_hop >= self.max_hop {
                    continue;
                }

                // 查该 event 关联的其他 entities 作为 LLM 候选
                let candidate_ids = self.find_entities_by_event(&event_id, &entity_id)?;
                if candidate_ids.is_empty() {
                    continue;
                }

                // 构建 candidate EntityRefs（跳过查询失败的 entity）
                let mut candidates: Vec<EntityRef> = Vec::new();
                for cid in &candidate_ids {
                    if let Some(r) = self.find_entity_ref(cid)? {
                        candidates.push(r);
                    }
                }
                if candidates.is_empty() {
                    continue;
                }

                // 调用 LLM 选路
                // 生产环境实现可在 HopllmLlm::select_next_hop 内部调用 build_hopllm_prompt 构建 prompt
                // 测试 mock（MockLlm/FailLlm）忽略 prompt 内容，仅按既定策略返回
                llm_call_count += 1;
                match self.llm.select_next_hop(query, &entity_ref, &candidates, &new_path) {
                    Ok(Some(selected_id)) => {
                        // 仅入队 LLM 选择的实体（剪枝：跳过其他候选）
                        queue.push_back((selected_id, event_hop, new_path.clone()));
                    }
                    Ok(None) => {
                        // LLM 认为无需继续扩展，停止该分支
                    }
                    Err(_) => {
                        // LLM 调用失败，记录并停止该分支
                        llm_failure_count += 1;
                    }
                }
            }
        }

        // 降级判定：所有 LLM 调用均失败 → 触发 multi1 降级
        let llm_fully_failed = llm_call_count > 0 && llm_failure_count == llm_call_count;
        Ok((results, llm_fully_failed))
    }

    /// 单跳扩展（multi1 等效降级路径，spec §三 11.2.3 降级策略）
    ///
    /// LLM 全部失败时使用此方法作为降级：仅扩展 1 跳（`hop=1`），
    /// 行为等价于 [`Multi1Strategy`]（`max_hop=1` 的 BFS）。
    ///
    /// ## 算法
    /// 1. 对每个 seed entity：查其关联的 events
    /// 2. 每个 event 记录为 `(event_id, hop=1, [entity_ref])`
    /// 3. 不继续扩展（无 hop=2/3）
    ///
    /// ## 返回
    /// `Vec<(event_id, hop=1, via_entities=[seed_entity_ref])>`
    fn single_hop_expand(
        &self,
        seed_entity_ids: &[String],
    ) -> Result<Vec<(String, u8, Vec<EntityRef>)>> {
        let mut visited_events: HashSet<String> = HashSet::new();
        let mut results: Vec<(String, u8, Vec<EntityRef>)> = Vec::new();

        for eid in seed_entity_ids {
            let entity_ref = match self.find_entity_ref(eid)? {
                Some(r) => r,
                None => continue,
            };
            let events = self.find_events_by_entity(eid)?;
            for event_id in events {
                if !visited_events.insert(event_id.clone()) {
                    continue;
                }
                results.push((event_id, 1u8, vec![entity_ref.clone()]));
            }
        }
        Ok(results)
    }

    /// 将扩展结果转换为 [`SearchHit`] 列表
    ///
    /// 按 hop 升序排序（hop=1 优先），取 `top_k`。
    /// `score = 1.0 / hop`（跳数衰减，与 [`MultiStrategy::build_hits`] 一致）。
    /// `chunk_span = None`（HOPLLM 不涉及 chunk 内位置）。
    fn build_hits(&self, expansion: Vec<(String, u8, Vec<EntityRef>)>) -> Result<Vec<SearchHit>> {
        // 按 hop 升序排序（hop 小的优先），稳定排序保持 BFS 顺序
        let mut sorted = expansion;
        sorted.sort_by_key(|(_, hop, _)| *hop);

        // 取 top_k
        let limited: Vec<(String, u8, Vec<EntityRef>)> = sorted.into_iter().take(self.top_k).collect();

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
impl SearchStrategy for HopllmStrategy {
    async fn search(&self, query: &str) -> Result<SearchResult> {
        let start = Instant::now();

        // Step1: query 向量化（mock embedding，384 维）
        let state = MultiState::new(query);
        let state = step1_vectorize(state);

        // Step2: query 实体抽取（jieba + 正则）
        let state = step2_extract_entities_with_jieba(state, &self.jieba);

        // Step3: 实体检索 — 当前用 SQL 文本匹配代替（11.2.x 接入 HnswIndex）
        let entity_texts: Vec<String> = state
            .entities
            .iter()
            .map(|e| e.text.clone())
            .collect();
        let entity_ids = self.find_entity_ids(&entity_texts)?;

        // Step4-7: LLM 引导多跳扩展（核心算法）
        let (expansion, llm_fully_failed) = self.llm_guided_expand(&entity_ids, query)?;

        // 降级检查：若 LLM 全程失败且无 expansion，降级到 multi1（single_hop_expand）
        let final_expansion = if llm_fully_failed && expansion.is_empty() && !entity_ids.is_empty()
        {
            // LLM 全程失败 → 降级到 multi1 行为（single_hop_expand）
            self.single_hop_expand(&entity_ids)?
        } else {
            // 正常路径或部分失败（仍使用 LLM 选路结果）
            expansion
        };

        // Step8: 构建 SearchResult
        let hits = self.build_hits(final_expansion)?;
        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResult {
            hits,
            latency_ms,
            strategy_name: "hopllm".to_string(),
        })
    }

    fn name(&self) -> &str {
        "hopllm"
    }
}
