//! Sub-Step 12.2.1 — 超边检测算法（spec §三 12.2.1）
//!
//! ## SAG 核心创新 — 超边（Hyperedge）
//! 传统语义图（如 RAG/知识图谱）的边是二元的：一条边连接 2 个节点
//! （如 entity ↔ event）。SAG（Semantic Agentic Graph）引入超边：
//! **>2 个 event 共享 >2 个 entity 时，自动形成一条多元超边**。
//!
//! 超边表达「多事件-多实体共现」的语义聚合，超越传统二元关系：
//! - 二元边：`evt-0 → ent-0`（一对一）
//! - SAG 超边：`{evt-0, evt-1, evt-2} ⇄ {ent-0, ent-1, ent-2}`（多对多）
//!
//! ## 边界设计（严格 >2）
//! - **min_events = 3**：超过 2 个 event 共现（≥3）
//! - **min_entities = 3**：超过 2 个 entity 共现（≥3）
//! - 若阈值放宽到 ≥2，超边退化为普通二元边，丧失 SAG 创新意义
//! - 故严格 >2 保证超边的「多元」语义
//!
//! ## 算法思路（detect_from_relations）
//! 1. 输入：`relations: Vec<(event_id, entity_id)>`（event-entity 二元关系）
//! 2. 构建 `entity → Vec<event>` 反向索引
//! 3. 对每个 entity 的 event 列表，找所有 size ≥ 3 的 event 子集，
//!    这些 event 共享该 entity
//! 4. 用 `HashMap<sorted_event_subset, HashSet<entity>>` 聚合：
//!    哪些 event 集合共享了哪些 entity
//! 5. 筛选：event 数 > 2 且 entity 数 > 2 的集合 → 形成超边
//! 6. 返回 `Vec<Hyperedge>`
//!
//! ## 复杂度
//! - 设 n = 单个 entity 关联的 event 数（最大值）
//! - 子集生成：O(2^n)（仅 size ≥ 3 的子集）
//! - 总复杂度：O(E × 2^n)，E = entity 数
//! - 测试场景（n ≤ 5）：毫秒级
//! - 生产场景（n > 20）：需启用 `max_subset_size` 限制（v1.1.0+ 优化）
//!
//! ## 超边 ID 生成
//! `id = format!("he_{:x}", hash(sorted_events ++ sorted_entities))`
//! 使用 `DefaultHasher`（稳定哈希，非加密），保证同输入产生同 ID（幂等）。
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use rusqlite::Connection;
use sparkfox_core::{Error, Result};

/// 从 `event_entity_relation` 表加载所有 (event_id, entity_id) 关系的 SQL
///
/// 与 `BidirectionalIndex`（11.6.2）共用同一 SQL，保证数据源一致。
const SQL_LOAD_ALL_RELATIONS: &str =
    "SELECT event_id, entity_id FROM event_entity_relation";

/// 超边（SAG 核心创新）— >2 个 event 共享 >2 个 entity 时自动形成
///
/// ## 字段
/// - `id`：超边唯一 ID（如 `"he_<hash>"`，由成员 events + entities 哈希生成）
/// - `member_events`：成员 event IDs（≥3，已排序保证幂等）
/// - `member_entities`：成员 entity IDs（≥3，已排序保证幂等）
///
/// ## 语义
/// 超边表达「所有 member_events 都关联所有 member_entities」的多元共现关系：
/// ```text
/// ∀ evt ∈ member_events, ∀ ent ∈ member_entities:
///     (evt, ent) ∈ event_entity_relation
/// ```
///
/// ## 幂等性
/// 同一关系集合多次调用 `detect_from_relations` 返回相同 ID 的超边
/// （因 `member_events` / `member_entities` 排序后哈希）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperedge {
    /// 超边唯一 ID（如 `"he_<hash>"`）
    pub id: String,
    /// 成员 event IDs（≥3，已排序）
    pub member_events: Vec<String>,
    /// 成员 entity IDs（≥3，已排序）
    pub member_entities: Vec<String>,
}

/// 超边检测器 — 从二元关系检测多元超边
///
/// ## 默认阈值
/// - `min_events = 3`（>2）
/// - `min_entities = 3`（>2）
///
/// ## 使用方式
/// ```ignore
/// use sparkfox_knowledge::hyperedge::HyperedgeDetector;
///
/// let detector = HyperedgeDetector::new();
/// let relations = vec![
///     ("evt-0".to_string(), "ent-0".to_string()),
///     ("evt-0".to_string(), "ent-1".to_string()),
///     // ... 更多关系
/// ];
/// let hyperedges = detector.detect_from_relations(&relations);
/// for he in &hyperedges {
///     println!("超边 {}: {} events × {} entities",
///              he.id, he.member_events.len(), he.member_entities.len());
/// }
/// ```
pub struct HyperedgeDetector {
    /// 最小 event 数（>2，默认 3）
    min_events: usize,
    /// 最小 entity 数（>2，默认 3）
    min_entities: usize,
}

impl HyperedgeDetector {
    /// 创建默认检测器（min_events=3, min_entities=3）
    ///
    /// ## 返回
    /// 配置为默认阈值的 `HyperedgeDetector`：
    /// - `min_events = 3`（>2，即 ≥3）
    /// - `min_entities = 3`（>2，即 ≥3）
    pub fn new() -> Self {
        Self {
            min_events: 3,
            min_entities: 3,
        }
    }

    /// 创建可配置检测器
    ///
    /// ## 参数
    /// - `min_events`：最小 event 数（应 > 2，否则退化为二元边）
    /// - `min_entities`：最小 entity 数（应 > 2，否则退化为二元边）
    ///
    /// ## 注意
    /// 调用方应保证 `min_events > 2` 且 `min_entities > 2`，否则违背 SAG 超边语义。
    /// 本构造函数不做硬性校验，允许实验性调参。
    pub fn with_thresholds(min_events: usize, min_entities: usize) -> Self {
        Self {
            min_events,
            min_entities,
        }
    }

    /// 从 SQLite 数据库检测所有超边
    ///
    /// ## 参数
    /// - `conn`：SQLite 连接（须已创建 SAG schema，见 [`crate::schema::ALL_SAG_DDL`]）
    ///
    /// ## 流程
    /// 1. SQL 查询 `event_entity_relation` 表加载所有 (event_id, entity_id) 关系
    /// 2. 调用 [`detect_from_relations`](Self::detect_from_relations) 内存版算法
    ///
    /// ## 错误
    /// - SQL prepare / 查询失败：返回 `Storage` 错误
    /// - 字段类型不匹配（理论不应发生）：返回 `Storage` 错误
    pub fn detect_hyperedges(&self, conn: &Connection) -> Result<Vec<Hyperedge>> {
        let mut stmt = conn
            .prepare(SQL_LOAD_ALL_RELATIONS)
            .map_err(|e| Error::storage(
                format!("prepare 失败: {e}"),
                "HyperedgeDetector::detect_hyperedges",
            ))?;

        let rows = stmt
            .query_map([], |row| {
                let event_id: String = row.get(0)?;
                let entity_id: String = row.get(1)?;
                Ok((event_id, entity_id))
            })
            .map_err(|e| Error::storage(
                format!("query_map 失败: {e}"),
                "HyperedgeDetector::detect_hyperedges",
            ))?;

        let mut relations: Vec<(String, String)> = Vec::new();
        for row in rows {
            let (event_id, entity_id) = row.map_err(|e| Error::storage(
                format!("读取行失败: {e}"),
                "HyperedgeDetector::detect_hyperedges",
            ))?;
            relations.push((event_id, entity_id));
        }

        Ok(self.detect_from_relations(&relations))
    }

    /// 从给定的 event-entity 关系列表检测超边（内存版，用于测试）
    ///
    /// ## 参数
    /// - `relations`：`(event_id, entity_id)` 二元关系列表
    ///
    /// ## 返回
    /// 检测到的所有超边（按 ID 排序，保证幂等）
    ///
    /// ## 算法
    /// 1. 构建 `entity → Vec<event>` 反向索引（去重）
    /// 2. 调用 [`find_shared_entities`] 聚合 `event_subset → 共享 entities` 映射
    /// 3. 筛选：event 数 ≥ `min_events` 且 entity 数 ≥ `min_entities` → 形成超边
    /// 4. 按 ID 排序返回（幂等）
    ///
    /// ## SAG 核心创新体现
    /// 传统二元图把 `event × entity` 关系展开为 N×M 条独立边，丢失「共现」语义。
    /// 本方法通过子集聚合，发现「3+ event 共享 3+ entity」的多元共现结构，
    /// 将其表达为单条超边，是 SAG 超越传统二元图的核心创新。
    pub fn detect_from_relations(&self, relations: &[(String, String)]) -> Vec<Hyperedge> {
        // Step 1: 构建 entity → Vec<event> 反向索引（去重，BTreeSet 保证排序）
        let mut entity_to_events: HashMap<String, BTreeSet<String>> = HashMap::new();
        for (event_id, entity_id) in relations {
            entity_to_events
                .entry(entity_id.clone())
                .or_insert_with(BTreeSet::new)
                .insert(event_id.clone());
        }

        // Step 2: 聚合 event_subset → 共享 entities（提取到 find_shared_entities 函数）
        let shared_map = find_shared_entities(&entity_to_events, self.min_events);

        // Step 3: 筛选 event 数 ≥ min_events 且 entity 数 ≥ min_entities 的子集 → 超边
        let mut hyperedges: Vec<Hyperedge> = shared_map
            .into_iter()
            .filter_map(|(member_events, entities)| {
                if entities.len() < self.min_entities {
                    return None;
                }
                // 排序 member_entities 保证幂等
                let mut member_entities: Vec<String> = entities.into_iter().collect();
                member_entities.sort();
                let id = generate_hyperedge_id(&member_events, &member_entities);
                Some(Hyperedge {
                    id,
                    member_events,
                    member_entities,
                })
            })
            .collect();

        // Step 4: 按 ID 排序，保证返回顺序幂等
        hyperedges.sort_by(|a, b| a.id.cmp(&b.id));
        hyperedges
    }
}

/// 默认实现（等价于 [`HyperedgeDetector::new`]）
impl Default for HyperedgeDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 内部辅助函数
// ---------------------------------------------------------------------------

/// 计算共享实体映射（SAG 超边检测核心步骤）
///
/// ## 输入
/// - `entity_to_events`：`entity_id → 关联 event_id 集合` 反向索引（BTreeSet 已排序）
/// - `min_events`：最小 event 数阈值（>2）
///
/// ## 输出
/// `HashMap<sorted_event_subset, HashSet<entity_id>>`：
/// - key：排序后的 event 子集（Vec<String>，长度 ≥ `min_events`）
/// - value：共享该 event 子集的所有 entity 集合
///
/// ## 算法
/// 1. 遍历每个 entity，取其关联的 event 集合
/// 2. 若 event 数 < `min_events`，跳过（无法形成超边）
/// 3. 否则生成所有 size ≥ `min_events` 的 event 子集
/// 4. 对每个子集，把当前 entity 加入其共享 entity 集合
///
/// ## SAG 核心创新体现
/// 此函数实现了 SAG 超边检测的核心：从二元关系聚合多元共现结构。
/// 传统二元图只能表达「单 event ↔ 单 entity」关系；本函数通过子集枚举
/// 与聚合，发现「多 event 共享多 entity」的多元结构，是 SAG 超越传统
/// 二元图的关键步骤。
///
/// ## 复杂度
/// - 设 n = 单个 entity 关联的 event 数（最大值）
/// - 子集生成：O(2^n)
/// - 总复杂度：O(E × 2^n)，E = entity 数
///
/// ## 示例
/// ```ignore
/// use std::collections::{BTreeSet, HashMap};
/// use sparkfox_knowledge::hyperedge::find_shared_entities;
///
/// let mut idx: HashMap<String, BTreeSet<String>> = HashMap::new();
/// idx.insert("ent-0".into(), ["evt-0","evt-1","evt-2"].into_iter().map(String::from).collect());
/// idx.insert("ent-1".into(), ["evt-0","evt-1","evt-2"].into_iter().map(String::from).collect());
/// idx.insert("ent-2".into(), ["evt-0","evt-1","evt-2"].into_iter().map(String::from).collect());
///
/// let shared = find_shared_entities(&idx, 3);
/// // shared = { ["evt-0","evt-1","evt-2"] => {"ent-0","ent-1","ent-2"} }
/// ```
fn find_shared_entities(
    entity_to_events: &HashMap<String, BTreeSet<String>>,
    min_events: usize,
) -> HashMap<Vec<String>, HashSet<String>> {
    let mut shared_map: HashMap<Vec<String>, HashSet<String>> = HashMap::new();

    for (entity_id, events) in entity_to_events {
        // 仅当 event 数 ≥ min_events 时才生成子集
        if events.len() < min_events {
            continue;
        }

        let events_vec: Vec<String> = events.iter().cloned().collect();

        // 生成所有 size ≥ min_events 的 event 子集
        for subset in generate_subsets_at_least(&events_vec, min_events) {
            shared_map
                .entry(subset)
                .or_insert_with(HashSet::new)
                .insert(entity_id.clone());
        }
    }

    shared_map
}

/// 生成 Vec 的所有 size ≥ `min_size` 的子集
///
/// ## 参数
/// - `items`：输入 Vec（已排序，子集保持原顺序）
/// - `min_size`：最小子集大小（含）
///
/// ## 返回
/// 所有满足 size ≥ `min_size` 的子集，每个子集为 `Vec<String>`（保持排序）
///
/// ## 算法
/// 使用位掩码枚举所有 2^n 子集，过滤 size ≥ `min_size`。
/// - 复杂度：O(2^n × n)
/// - 适合 n ≤ 20 的场景（生产环境 n > 20 应改用迭代式深度限制）
///
/// ## 示例
/// ```ignore
/// let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
/// let subsets = generate_subsets_at_least(&items, 2);
/// // 返回 4 个子集：[a,b], [a,c], [b,c], [a,b,c]
/// ```
fn generate_subsets_at_least(items: &[String], min_size: usize) -> Vec<Vec<String>> {
    let n = items.len();
    if n < min_size {
        return Vec::new();
    }

    let total = 1usize << n; // 2^n
    let mut result: Vec<Vec<String>> = Vec::new();

    // mask 从 1 开始（跳过空集），到 2^n - 1 结束
    for mask in 1..total {
        let size = mask.count_ones() as usize;
        if size < min_size {
            continue;
        }

        // 按 mask 提取子集，保持原顺序（items 已排序）
        let subset: Vec<String> = (0..n)
            .filter(|&i| (mask >> i) & 1 == 1)
            .map(|i| items[i].clone())
            .collect();

        result.push(subset);
    }

    result
}

/// 生成超边 ID（基于成员 events + entities 的哈希）
///
/// ## 算法
/// 1. 输入：`member_events`（已排序）+ `member_entities`（已排序）
/// 2. 用 `DefaultHasher` 哈希所有成员字符串
/// 3. 返回 `format!("he_{:x}", hash)`
///
/// ## 幂等性
/// - 同一 `(member_events, member_entities)` 集合产生相同 ID
/// - `member_events` / `member_entities` 必须已排序，否则哈希不稳定
///
/// ## 不使用加密哈希
/// `DefaultHasher`（SipHash）足够稳定且速度快；超边 ID 不需要抗碰撞，
/// 因输入由关系表派生，不存在恶意构造场景。
fn generate_hyperedge_id(member_events: &[String], member_entities: &[String]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for evt in member_events {
        evt.hash(&mut hasher);
    }
    // 分隔符避免 events 与 entities 边界混淆
    0xFEEDBEEFu64.hash(&mut hasher);
    for ent in member_entities {
        ent.hash(&mut hasher);
    }
    format!("he_{:x}", hasher.finish())
}
