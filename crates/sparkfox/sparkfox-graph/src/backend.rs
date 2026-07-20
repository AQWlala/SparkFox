//! GraphBackend trait — sparkfox-graph 降级为通用图遍历引擎（A-02 P0 修复）
//!
//! ## A-02 P0 修复设计原则
//!
//! 1. **sparkfox-graph 不再维护独立 node/edge 表**（移除双轨制）
//! 2. **sparkfox-knowledge 为唯一 SoT**（Source of Truth）：
//!    - `entity` 表 → `GraphNode`（Entity 类型）
//!    - `knowledge_event` 表 → `GraphNode`（Event 类型）
//!    - `event_entity_relation` 表 → `GraphEdge`（EventEntity 类型）
//! 3. **GraphBackend trait** 抽象图遍历操作，实现方（如 [`KnowledgeGraphBackend`](crate::KnowledgeGraphBackend)）
//!    反向引用 sparkfox-knowledge 的表，不创建新表
//!
//! ## 与 spec 2.0 §6.4 的关系
//!
//! 字段名严格对齐 `sparkfox-knowledge/src/schema.rs` 的 SAG 6 表 DDL：
//! - `entity`: id / name / normalized_name / description / extra_data / ...
//! - `knowledge_event`: id / title / summary / content / ...
//! - `event_entity_relation`: id / event_id / entity_id / relation_type / confidence / extra_data
//!
//! 参考：SAG 论文 arXiv:2606.15971 + SAG-Benchmark (MIT License, verified 2026-07-19)。

use async_trait::async_trait;
use sparkfox_core::Result;

/// 图节点（统一抽象，对应 sparkfox-knowledge 的 entity 或 knowledge_event）
///
/// - `node_type = Entity` → 对应 `entity` 表
/// - `node_type = Event` → 对应 `knowledge_event` 表
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// 节点 ID（entity.id 或 knowledge_event.id）
    pub id: String,
    /// 节点类型（Entity / Event）
    pub node_type: GraphNodeType,
    /// 显示名称（entity.name 或 knowledge_event.title）
    pub label: String,
    /// 额外属性（JSON，包含原表的 description / extra_data / summary / content 等）
    pub properties: serde_json::Value,
}

/// 图节点类型
///
/// 对应 sparkfox-knowledge 的两类节点表：
/// - [`Entity`](GraphNodeType::Entity) → `entity` 表（L3 Semantic + L3 GraphNode）
/// - [`Event`](GraphNodeType::Event) → `knowledge_event` 表（L3 Episodic）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphNodeType {
    /// 对应 sparkfox-knowledge `entity` 表（L3 Semantic + L3 GraphNode）
    Entity,
    /// 对应 sparkfox-knowledge `knowledge_event` 表（L3 Episodic）
    Event,
}

/// 图边（对应 sparkfox-knowledge 的 event_entity_relation）
///
/// SAG 关联表 `event_entity_relation` 描述 event ↔ entity 的二元关系，
/// 在图模型中表示为无向边（双向索引见 schema.rs 的 idx_eer_event_entity / idx_eer_entity_event）。
#[derive(Debug, Clone)]
pub struct GraphEdge {
    /// 边 ID（event_entity_relation.id）
    pub id: String,
    /// 起点（event_id）
    pub source: String,
    /// 终点（entity_id）
    pub target: String,
    /// 边类型（当前仅 EventEntity，未来可扩展 EntityEntity 等）
    pub edge_type: GraphEdgeType,
    /// 额外属性（JSON，包含 relation_type / confidence / extra_data）
    pub properties: serde_json::Value,
}

/// 图边类型
///
/// 当前仅支持 [`EventEntity`](GraphEdgeType::EventEntity)（来自 event_entity_relation 表），
/// 未来可扩展 EntityEntity（entity ↔ entity 关系）等。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphEdgeType {
    /// event ↔ entity 关系（来自 event_entity_relation 表）
    EventEntity,
}

/// 子图（subgraph 查询结果）
///
/// 由 [`GraphBackend::subgraph`] 返回，包含从根节点出发 max_depth 跳内的所有节点和边。
#[derive(Debug, Clone)]
pub struct Graph {
    /// 子图包含的节点列表
    pub nodes: Vec<GraphNode>,
    /// 子图包含的边列表
    pub edges: Vec<GraphEdge>,
}

/// 【A-02 P0 修复】GraphBackend trait — 通用图遍历接口
///
/// sparkfox-graph 降级为通用图遍历引擎，不再维护独立表。
/// 实现方（如 [`KnowledgeGraphBackend`](crate::KnowledgeGraphBackend)）反向引用
/// sparkfox-knowledge 的 `entity` / `knowledge_event` / `event_entity_relation` 表。
///
/// ## 实现约定
///
/// - `get_node` 优先查 `entity` 表，其次查 `knowledge_event` 表
/// - `get_neighbors` 通过 `event_entity_relation` 表的双向索引进行 BFS 扩展
/// - `subgraph` 返回根节点 + max_depth 跳内的所有节点和边
/// - 所有方法都是 `async`（实现方通常持有 `Arc<Mutex<Connection>>`）
///
/// ## 示例
///
/// ```no_run
/// use sparkfox_graph::{GraphBackend, KnowledgeGraphBackend};
/// use rusqlite::Connection;
/// # async fn demo() -> sparkfox_core::Result<()> {
/// let conn = Connection::open_in_memory().unwrap();
/// let backend = KnowledgeGraphBackend::from_conn(conn);
/// let node = backend.get_node("e1").await?;
/// # Ok(()) }
/// ```
#[async_trait]
pub trait GraphBackend: Send + Sync {
    /// 获取节点（优先查 `entity` 表，其次 `knowledge_event` 表）
    ///
    /// 返回 `Ok(None)` 表示两张表均未找到该 ID。
    async fn get_node(&self, id: &str) -> Result<Option<GraphNode>>;

    /// 获取节点的邻居（BFS，最多 `max_depth` 跳）
    ///
    /// - 对 entity 节点：通过 `event_entity_relation` 查关联的 event 节点
    /// - 对 event 节点：通过 `event_entity_relation` 查关联的 entity 节点
    /// - `max_depth = 1` 表示仅直连邻居，`max_depth = 2` 表示 2 跳内所有节点
    /// - 返回结果不含起始节点本身
    async fn get_neighbors(&self, node_id: &str, max_depth: u8) -> Result<Vec<GraphNode>>;

    /// 获取子图（从 `root` 出发，`max_depth` 跳内的所有节点和边）
    ///
    /// - 若 `root` 节点不存在，返回空 Graph（不返回错误）
    /// - 节点和边均去重
    async fn subgraph(&self, root: &str, max_depth: u8) -> Result<Graph>;

    /// 获取节点的所有关联边（通过 `event_entity_relation` 表）
    ///
    /// 查询条件：`event_id = node_id OR entity_id = node_id`
    async fn get_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>>;

    /// 后端名称（用于日志和调试）
    fn backend_name(&self) -> &'static str;
}

#[cfg(test)]
mod trait_tests {
    use super::*;

    /// 验证 GraphNodeType 枚举值存在与相等性。
    #[test]
    fn test_graph_node_type_enum() {
        assert_eq!(GraphNodeType::Entity, GraphNodeType::Entity);
        assert_eq!(GraphNodeType::Event, GraphNodeType::Event);
        assert_ne!(GraphNodeType::Entity, GraphNodeType::Event);
    }

    /// 验证 GraphEdgeType 枚举值存在与相等性。
    #[test]
    fn test_graph_edge_type_enum() {
        assert_eq!(GraphEdgeType::EventEntity, GraphEdgeType::EventEntity);
    }

    /// 验证 Graph 结构可构造。
    #[test]
    fn test_graph_struct_construction() {
        let graph = Graph {
            nodes: vec![],
            edges: vec![],
        };
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }
}
