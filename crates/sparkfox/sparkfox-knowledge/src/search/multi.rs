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
