//! PetgraphBackend — petgraph + SQLite 持久化的图存储实现
//!
//! 作为 [`GraphBackend`](crate::GraphBackend) trait 的另一个实现，
//! 与 [`KnowledgeGraphBackend`](crate::KnowledgeGraphBackend) 互补：
//!
//! - [`KnowledgeGraphBackend`](crate::KnowledgeGraphBackend)：反向引用 sparkfox-knowledge
//!   的 SAG 3 表（SoT 在 sparkfox-knowledge）
//! - [`PetgraphBackend`](crate::PetgraphBackend)：自维护 petgraph 内存图 + SQLite 持久化
//!   （SoT 在本后端的 graph_node/graph_edge 表）
//!
//! ## 适用场景
//!
//! - 需要独立图存储（不依赖 sparkfox-knowledge 的 entity/event 表）
//! - 测试 / 演示 / 离线分析
//! - 临时子图缓存
//!
//! ## 持久化设计
//!
//! 使用 SQLite 两张表：
//! - `graph_node`：id / node_type / label / properties (JSON)
//! - `graph_edge`：id / source / target / edge_type / properties (JSON)
//!
//! [`PetgraphBackend::persist`] 将内存图全量写入 SQLite，
//! [`PetgraphBackend::load`] 从 SQLite 还原内存图。
//! 简单全量替换策略，适合中小规模图（< 10k 节点）。
//!
//! ## StableDiGraph 选型
//!
//! 使用 `petgraph::stable_graph::StableDiGraph`（非 `DiGraph`）：
//! 删除节点/边后其他索引保持不变，`node_index: HashMap<String, NodeIndex>` 不会失效。

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use rusqlite::{params, Connection};
use tokio::sync::Mutex;

use sparkfox_core::{Error, Result};

use crate::{Graph, GraphBackend, GraphEdge, GraphEdgeType, GraphNode, GraphNodeType};

/// SQLite schema 初始化 SQL（IF NOT EXISTS，幂等）
const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS graph_node (
    id TEXT PRIMARY KEY,
    node_type TEXT NOT NULL,
    label TEXT NOT NULL,
    properties TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS graph_edge (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    target TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    properties TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_graph_edge_source ON graph_edge(source);
CREATE INDEX IF NOT EXISTS idx_graph_edge_target ON graph_edge(target);
"#;

/// PetgraphBackend — petgraph 内存图 + SQLite 持久化
///
/// 详见模块级文档。
pub struct PetgraphBackend {
    /// petgraph 有向图（StableDiGraph 支持安全的节点/边删除，索引稳定）
    graph: StableDiGraph<GraphNode, GraphEdge>,
    /// 节点 ID → NodeIndex 映射（加速查找）
    node_index: HashMap<String, NodeIndex>,
    /// SQLite 连接（持久化用，Arc<Mutex> 支持异步共享）
    conn: Arc<Mutex<Connection>>,
}

impl PetgraphBackend {
    /// 从 `Connection` 创建（独占连接）
    ///
    /// 自动创建 schema（IF NOT EXISTS，幂等）。
    pub fn new(conn: Connection) -> Result<Self> {
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| Error::internal(format!("初始化 graph schema 失败: {e}")))?;
        Ok(Self {
            graph: StableDiGraph::new(),
            node_index: HashMap::new(),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 从 `Arc<Mutex<Connection>>` 创建（共享连接，推荐方式）
    pub async fn from_arc(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        {
            let c = conn.lock().await;
            c.execute_batch(SCHEMA_SQL)
                .map_err(|e| Error::internal(format!("初始化 graph schema 失败: {e}")))?;
        }
        Ok(Self {
            graph: StableDiGraph::new(),
            node_index: HashMap::new(),
            conn,
        })
    }

    /// 添加节点（若 ID 已存在则更新权重并返回原索引）
    pub fn add_node(&mut self, node: GraphNode) -> Result<NodeIndex> {
        if let Some(&idx) = self.node_index.get(&node.id) {
            // 已存在，更新权重
            if let Some(n) = self.graph.node_weight_mut(idx) {
                *n = node;
            }
            return Ok(idx);
        }
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        Ok(idx)
    }

    /// 添加边（要求 from / to 节点均已存在）
    pub fn add_edge(&mut self, from: &str, to: &str, edge: GraphEdge) -> Result<EdgeIndex> {
        let from_idx = *self
            .node_index
            .get(from)
            .ok_or_else(|| Error::not_found("node", from))?;
        let to_idx = *self
            .node_index
            .get(to)
            .ok_or_else(|| Error::not_found("node", to))?;
        Ok(self.graph.add_edge(from_idx, to_idx, edge))
    }

    /// 删除节点（同时自动删除关联的边）
    ///
    /// 使用 StableDiGraph，删除后其他节点/边的索引保持不变。
    pub fn remove_node(&mut self, id: &str) -> Result<()> {
        let idx = self
            .node_index
            .remove(id)
            .ok_or_else(|| Error::not_found("node", id))?;
        self.graph.remove_node(idx);
        Ok(())
    }

    /// 删除首条 from → to 的边
    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<()> {
        let from_idx = *self
            .node_index
            .get(from)
            .ok_or_else(|| Error::not_found("node", from))?;
        let to_idx = *self
            .node_index
            .get(to)
            .ok_or_else(|| Error::not_found("node", to))?;
        let edge_idx = self
            .graph
            .find_edge(from_idx, to_idx)
            .ok_or_else(|| Error::not_found("edge", format!("{from}->{to}")))?;
        self.graph.remove_edge(edge_idx);
        Ok(())
    }

    /// 持久化内存图到 SQLite（全量替换）
    ///
    /// 单事务内：清空 graph_node / graph_edge，然后全量写入当前内存图。
    pub async fn persist(&self) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let tx = conn
            .transaction()
            .map_err(|e| Error::internal(format!("开启事务失败: {e}")))?;
        // 清空旧数据
        tx.execute("DELETE FROM graph_edge", [])
            .map_err(|e| Error::internal(format!("清空 graph_edge 失败: {e}")))?;
        tx.execute("DELETE FROM graph_node", [])
            .map_err(|e| Error::internal(format!("清空 graph_node 失败: {e}")))?;

        // 写入节点（遍历 node_index 保证仅写入活动节点）
        for (_id, &idx) in &self.node_index {
            let node = &self.graph[idx];
            let node_type = node_type_to_str(node.node_type);
            tx.execute(
                "INSERT INTO graph_node (id, node_type, label, properties) VALUES (?1, ?2, ?3, ?4)",
                params![node.id, node_type, node.label, node.properties.to_string()],
            )
            .map_err(|e| Error::internal(format!("插入 graph_node 失败: {e}")))?;
        }

        // 写入边（遍历所有活动边）
        for edge_idx in self.graph.edge_indices() {
            let edge = self
                .graph
                .edge_weight(edge_idx)
                .ok_or_else(|| Error::internal("内部错误：edge_weight 缺失"))?;
            let (source_idx, target_idx) = self
                .graph
                .edge_endpoints(edge_idx)
                .ok_or_else(|| Error::internal("内部错误：edge_endpoints 缺失"))?;
            let source_id = &self.graph[source_idx].id;
            let target_id = &self.graph[target_idx].id;
            let edge_type = edge_type_to_str(edge.edge_type);
            tx.execute(
                "INSERT INTO graph_edge (id, source, target, edge_type, properties) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![edge.id, source_id, target_id, edge_type, edge.properties.to_string()],
            )
            .map_err(|e| Error::internal(format!("插入 graph_edge 失败: {e}")))?;
        }

        tx.commit()
            .map_err(|e| Error::internal(format!("提交事务失败: {e}")))?;
        Ok(())
    }

    /// 从 SQLite 还原内存图
    ///
    /// 自动创建 schema（IF NOT EXISTS），然后读取全量数据构建内存图。
    pub async fn load(conn: Connection) -> Result<Self> {
        let mut backend = Self::new(conn)?;

        // 在锁内收集所有数据，锁释放后再调用 add_node / add_edge（避免 mut borrow 冲突）
        let (nodes, edges) = {
            let conn = backend.conn.lock().await;

            // 加载节点
            let mut stmt = conn
                .prepare("SELECT id, node_type, label, properties FROM graph_node")
                .map_err(|e| Error::internal(format!("prepare graph_node 失败: {e}")))?;
            let nodes: Vec<(String, String, String, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                .map_err(|e| Error::internal(format!("query graph_node 失败: {e}")))?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);

            // 加载边
            let mut stmt = conn
                .prepare("SELECT id, source, target, edge_type, properties FROM graph_edge")
                .map_err(|e| Error::internal(format!("prepare graph_edge 失败: {e}")))?;
            let edges: Vec<(String, String, String, String, String)> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .map_err(|e| Error::internal(format!("query graph_edge 失败: {e}")))?
                .filter_map(|r| r.ok())
                .collect();

            (nodes, edges)
        }; // conn 锁在此释放

        // 现在可以 mutably borrow backend
        for (id, node_type, label, props_str) in nodes {
            let nt = parse_node_type(&node_type)?;
            let properties =
                serde_json::from_str(&props_str).unwrap_or(serde_json::Value::Null);
            backend.add_node(GraphNode {
                id,
                node_type: nt,
                label,
                properties,
            })?;
        }

        for (id, source, target, edge_type, props_str) in edges {
            let et = parse_edge_type(&edge_type)?;
            let properties =
                serde_json::from_str(&props_str).unwrap_or(serde_json::Value::Null);
            // 克隆 source/target 用于 &str 引用，原始值 move 进 GraphEdge
            let edge = GraphEdge {
                id,
                source: source.clone(),
                target: target.clone(),
                edge_type: et,
                properties,
            };
            backend.add_edge(&source, &target, edge)?;
        }

        Ok(backend)
    }

    /// 当前内存图节点数
    pub fn node_count(&self) -> usize {
        self.node_index.len()
    }

    /// 当前内存图边数
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[async_trait]
impl GraphBackend for PetgraphBackend {
    async fn get_node(&self, id: &str) -> Result<Option<GraphNode>> {
        let Some(&idx) = self.node_index.get(id) else {
            return Ok(None);
        };
        Ok(self.graph.node_weight(idx).cloned())
    }

    async fn get_neighbors(&self, node_id: &str, max_depth: u8) -> Result<Vec<GraphNode>> {
        let Some(&start) = self.node_index.get(node_id) else {
            return Ok(vec![]);
        };
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        visited.insert(start);
        let mut current = vec![start];
        let mut result = Vec::new();

        for _ in 0..max_depth {
            let mut next = Vec::new();
            for &n in &current {
                // 无向邻居（兼顾入边 + 出边）
                for neighbor in self.graph.neighbors_undirected(n) {
                    if visited.insert(neighbor) {
                        if let Some(node) = self.graph.node_weight(neighbor) {
                            result.push(node.clone());
                        }
                        next.push(neighbor);
                    }
                }
            }
            current = next;
            if current.is_empty() {
                break;
            }
        }
        Ok(result)
    }

    async fn subgraph(&self, root: &str, max_depth: u8) -> Result<Graph> {
        let Some(&start) = self.node_index.get(root) else {
            return Ok(Graph {
                nodes: vec![],
                edges: vec![],
            });
        };
        let mut visited_nodes: HashSet<NodeIndex> = HashSet::new();
        let mut visited_edges: HashSet<EdgeIndex> = HashSet::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        visited_nodes.insert(start);
        if let Some(n) = self.graph.node_weight(start) {
            nodes.push(n.clone());
        }

        let mut current = vec![start];
        for _ in 0..max_depth {
            let mut next = Vec::new();
            for &n in &current {
                // StableDiGraph 无 edges_undirected，需双向遍历
                // visited_edges 去重保证每条边仅入队一次
                for dir in [petgraph::Direction::Outgoing, petgraph::Direction::Incoming] {
                    for edge_ref in self.graph.edges_directed(n, dir) {
                        let edge_idx = edge_ref.id();
                        if visited_edges.insert(edge_idx) {
                            if let Some(e) = self.graph.edge_weight(edge_idx) {
                                edges.push(e.clone());
                            }
                            let other = if edge_ref.source() == n {
                                edge_ref.target()
                            } else {
                                edge_ref.source()
                            };
                            if visited_nodes.insert(other) {
                                if let Some(node) = self.graph.node_weight(other) {
                                    nodes.push(node.clone());
                                }
                                next.push(other);
                            }
                        }
                    }
                }
            }
            current = next;
            if current.is_empty() {
                break;
            }
        }
        Ok(Graph { nodes, edges })
    }

    async fn get_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let Some(&idx) = self.node_index.get(node_id) else {
            return Ok(vec![]);
        };
        let mut edges = Vec::new();
        let mut seen: HashSet<EdgeIndex> = HashSet::new();
        // 双向遍历，去重
        for dir in [petgraph::Direction::Outgoing, petgraph::Direction::Incoming] {
            for edge_ref in self.graph.edges_directed(idx, dir) {
                let eid = edge_ref.id();
                if seen.insert(eid) {
                    if let Some(e) = self.graph.edge_weight(eid) {
                        edges.push(e.clone());
                    }
                }
            }
        }
        Ok(edges)
    }

    fn backend_name(&self) -> &'static str {
        "petgraph_backend"
    }
}

/// GraphNodeType → SQLite 存储字符串
fn node_type_to_str(nt: GraphNodeType) -> &'static str {
    match nt {
        GraphNodeType::Entity => "Entity",
        GraphNodeType::Event => "Event",
    }
}

/// 字符串 → GraphNodeType
fn parse_node_type(s: &str) -> Result<GraphNodeType> {
    match s {
        "Entity" => Ok(GraphNodeType::Entity),
        "Event" => Ok(GraphNodeType::Event),
        other => Err(Error::internal(format!("未知节点类型: {other}"))),
    }
}

/// GraphEdgeType → SQLite 存储字符串
fn edge_type_to_str(et: GraphEdgeType) -> &'static str {
    match et {
        GraphEdgeType::EventEntity => "EventEntity",
    }
}

/// 字符串 → GraphEdgeType
fn parse_edge_type(s: &str) -> Result<GraphEdgeType> {
    match s {
        "EventEntity" => Ok(GraphEdgeType::EventEntity),
        other => Err(Error::internal(format!("未知边类型: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造无 created_time 的简单节点
    fn make_node(id: &str, nt: GraphNodeType, label: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: nt,
            label: label.to_string(),
            properties: serde_json::json!({}),
        }
    }

    /// 构造带 confidence 的边
    fn make_edge(id: &str, src: &str, tgt: &str, confidence: f64) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: src.to_string(),
            target: tgt.to_string(),
            edge_type: GraphEdgeType::EventEntity,
            properties: serde_json::json!({ "confidence": confidence }),
        }
    }

    /// 构造测试图：ev1 → e1, ev1 → e2
    fn setup_backend() -> PetgraphBackend {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("e1", GraphNodeType::Entity, "北京"))
            .unwrap();
        b.add_node(make_node("e2", GraphNodeType::Entity, "上海"))
            .unwrap();
        b.add_node(make_node("ev1", GraphNodeType::Event, "北京会议"))
            .unwrap();
        b.add_edge("ev1", "e1", make_edge("r1", "ev1", "e1", 0.95))
            .unwrap();
        b.add_edge("ev1", "e2", make_edge("r2", "ev1", "e2", 0.8))
            .unwrap();
        b
    }

    // 同步版本的 get_node，简化单测
    impl PetgraphBackend {
        fn get_node_sync(&self, id: &str) -> Option<GraphNode> {
            self.node_index
                .get(id)
                .and_then(|&idx| self.graph.node_weight(idx).cloned())
        }
    }

    #[test]
    fn test_add_node() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        let idx = b
            .add_node(make_node("n1", GraphNodeType::Entity, "节点1"))
            .unwrap();
        assert!(b.graph.node_weight(idx).is_some());
        assert_eq!(b.graph.node_weight(idx).unwrap().label, "节点1");
        assert_eq!(b.node_count(), 1);
    }

    #[test]
    fn test_add_node_update_existing() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "旧"))
            .unwrap();
        // 再次添加同 id，应更新而非新增
        b.add_node(make_node("n1", GraphNodeType::Entity, "新"))
            .unwrap();
        assert_eq!(b.node_count(), 1, "节点数量应为 1（更新而非新增）");
        let node = b.get_node_sync("n1").unwrap();
        assert_eq!(node.label, "新");
    }

    #[test]
    fn test_add_edge() {
        let b = setup_backend();
        assert_eq!(b.edge_count(), 2, "应有 2 条边");
    }

    #[test]
    fn test_add_edge_missing_node() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "n1"))
            .unwrap();
        let r = b.add_edge("n1", "missing", make_edge("e1", "n1", "missing", 1.0));
        assert!(r.is_err(), "缺失节点应报错");
    }

    #[test]
    fn test_remove_node() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "n1"))
            .unwrap();
        b.add_node(make_node("n2", GraphNodeType::Entity, "n2"))
            .unwrap();
        b.remove_node("n1").unwrap();
        assert!(b.get_node_sync("n1").is_none(), "n1 应已删除");
        // StableDiGraph 保证 n2 的索引不变
        assert!(b.get_node_sync("n2").is_some(), "n2 应仍存在");
        assert_eq!(b.node_count(), 1);
    }

    #[test]
    fn test_remove_node_not_found() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        let r = b.remove_node("missing");
        assert!(r.is_err());
    }

    #[test]
    fn test_remove_node_with_edges() {
        // 删除节点应级联删除其关联边
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "n1"))
            .unwrap();
        b.add_node(make_node("n2", GraphNodeType::Entity, "n2"))
            .unwrap();
        b.add_edge("n1", "n2", make_edge("e1", "n1", "n2", 1.0))
            .unwrap();
        assert_eq!(b.edge_count(), 1);
        b.remove_node("n1").unwrap();
        assert_eq!(b.edge_count(), 0, "删除节点应级联删除关联边");
    }

    #[test]
    fn test_remove_edge() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "n1"))
            .unwrap();
        b.add_node(make_node("n2", GraphNodeType::Entity, "n2"))
            .unwrap();
        b.add_edge("n1", "n2", make_edge("e1", "n1", "n2", 1.0))
            .unwrap();
        assert_eq!(b.edge_count(), 1);
        b.remove_edge("n1", "n2").unwrap();
        assert_eq!(b.edge_count(), 0);
    }

    #[test]
    fn test_remove_edge_not_found() {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "n1"))
            .unwrap();
        b.add_node(make_node("n2", GraphNodeType::Entity, "n2"))
            .unwrap();
        let r = b.remove_edge("n1", "n2"); // 没有边
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_get_node() {
        let b = setup_backend();
        let node = b.get_node("e1").await.unwrap();
        assert!(node.is_some());
        assert_eq!(node.unwrap().label, "北京");
    }

    #[tokio::test]
    async fn test_get_node_not_found() {
        let b = setup_backend();
        let node = b.get_node("missing").await.unwrap();
        assert!(node.is_none());
    }

    #[tokio::test]
    async fn test_get_neighbors_1_hop() {
        let b = setup_backend();
        let neighbors = b.get_neighbors("ev1", 1).await.unwrap();
        assert_eq!(neighbors.len(), 2, "ev1 应有 2 个直连邻居");
        let labels: Vec<_> = neighbors.iter().map(|n| n.label.as_str()).collect();
        assert!(labels.contains(&"北京"));
        assert!(labels.contains(&"上海"));
    }

    #[tokio::test]
    async fn test_get_neighbors_2_hop() {
        let b = setup_backend();
        // e1 → ev1 → e2（2 跳）
        let neighbors = b.get_neighbors("e1", 2).await.unwrap();
        assert!(neighbors.iter().any(|n| n.id == "ev1"), "应含 ev1");
        assert!(neighbors.iter().any(|n| n.id == "e2"), "应含 e2");
    }

    #[tokio::test]
    async fn test_get_neighbors_excludes_start() {
        let b = setup_backend();
        let neighbors = b.get_neighbors("e1", 2).await.unwrap();
        assert!(
            !neighbors.iter().any(|n| n.id == "e1"),
            "邻居列表不应含起始节点"
        );
    }

    #[tokio::test]
    async fn test_get_neighbors_max_depth_zero() {
        let b = setup_backend();
        let neighbors = b.get_neighbors("ev1", 0).await.unwrap();
        assert!(neighbors.is_empty(), "max_depth=0 应返回空");
    }

    #[tokio::test]
    async fn test_subgraph_from_event() {
        let b = setup_backend();
        let g = b.subgraph("ev1", 1).await.unwrap();
        assert_eq!(g.nodes.len(), 3, "子图应含 3 节点");
        assert_eq!(g.edges.len(), 2, "子图应含 2 边");
    }

    #[tokio::test]
    async fn test_subgraph_from_entity_2_hop() {
        let b = setup_backend();
        let g = b.subgraph("e1", 2).await.unwrap();
        assert_eq!(g.nodes.len(), 3, "2 跳子图应含 3 节点");
        assert_eq!(g.edges.len(), 2, "2 跳子图应含 2 边");
    }

    #[tokio::test]
    async fn test_subgraph_nonexistent_root() {
        let b = setup_backend();
        let g = b.subgraph("missing", 1).await.unwrap();
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[tokio::test]
    async fn test_get_edges() {
        let b = setup_backend();
        let edges = b.get_edges("ev1").await.unwrap();
        assert_eq!(edges.len(), 2, "ev1 应有 2 条关联边");
    }

    #[tokio::test]
    async fn test_get_edges_entity_side() {
        let b = setup_backend();
        // 从 entity 侧查边（双向）
        let edges = b.get_edges("e1").await.unwrap();
        assert_eq!(edges.len(), 1, "e1 应有 1 条关联边");
        assert_eq!(edges[0].id, "r1");
    }

    #[tokio::test]
    async fn test_backend_name() {
        let b = setup_backend();
        assert_eq!(b.backend_name(), "petgraph_backend");
    }

    #[tokio::test]
    async fn test_persist_then_load_roundtrip() {
        // 使用临时文件验证 persist + load 往返一致性
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_graph.db");
        let conn = Connection::open(&db_path).unwrap();

        // 创建并写入
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("e1", GraphNodeType::Entity, "北京"))
            .unwrap();
        b.add_node(make_node("e2", GraphNodeType::Entity, "上海"))
            .unwrap();
        b.add_node(make_node("ev1", GraphNodeType::Event, "北京会议"))
            .unwrap();
        b.add_edge("ev1", "e1", make_edge("r1", "ev1", "e1", 0.95))
            .unwrap();
        b.add_edge("ev1", "e2", make_edge("r2", "ev1", "e2", 0.8))
            .unwrap();
        b.persist().await.unwrap();
        drop(b); // 关闭连接，确保数据落盘

        // 重新打开并加载
        let conn2 = Connection::open(&db_path).unwrap();
        let b2 = PetgraphBackend::load(conn2).await.unwrap();
        assert_eq!(b2.node_count(), 3, "应加载 3 个节点");
        assert_eq!(b2.edge_count(), 2, "应加载 2 条边");

        // 验证节点字段
        let e1 = b2.get_node("e1").await.unwrap().unwrap();
        assert_eq!(e1.label, "北京");
        assert_eq!(e1.node_type, GraphNodeType::Entity);
        let ev1 = b2.get_node("ev1").await.unwrap().unwrap();
        assert_eq!(ev1.label, "北京会议");
        assert_eq!(ev1.node_type, GraphNodeType::Event);

        // 验证边字段
        let edges = b2.get_edges("ev1").await.unwrap();
        assert_eq!(edges.len(), 2);
        let r1 = edges.iter().find(|e| e.id == "r1").unwrap();
        assert_eq!(r1.source, "ev1");
        assert_eq!(r1.target, "e1");
        assert_eq!(r1.edge_type, GraphEdgeType::EventEntity);
        assert_eq!(r1.properties["confidence"], 0.95);
    }

    #[tokio::test]
    async fn test_load_empty_db() {
        // 加载空库（仅 schema）应返回空图
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("empty.db");
        let conn = Connection::open(&db_path).unwrap();
        // 仅初始化 schema
        conn.execute_batch(SCHEMA_SQL).unwrap();
        drop(conn);

        let conn2 = Connection::open(&db_path).unwrap();
        let b = PetgraphBackend::load(conn2).await.unwrap();
        assert_eq!(b.node_count(), 0);
        assert_eq!(b.edge_count(), 0);
    }

    #[tokio::test]
    async fn test_persist_clears_old_data() {
        // 验证 persist 是全量替换：先写入 A，persist，再删除 A 添加 B，persist，应只剩 B
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("replace.db");
        let conn = Connection::open(&db_path).unwrap();

        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("old1", GraphNodeType::Entity, "旧1"))
            .unwrap();
        b.add_node(make_node("old2", GraphNodeType::Entity, "旧2"))
            .unwrap();
        b.add_edge("old1", "old2", make_edge("olde", "old1", "old2", 1.0))
            .unwrap();
        b.persist().await.unwrap();

        // 删除旧数据，添加新数据
        b.remove_node("old1").unwrap();
        b.remove_node("old2").unwrap();
        b.add_node(make_node("new1", GraphNodeType::Entity, "新1"))
            .unwrap();
        b.persist().await.unwrap();
        drop(b);

        // 加载验证
        let conn2 = Connection::open(&db_path).unwrap();
        let b2 = PetgraphBackend::load(conn2).await.unwrap();
        assert_eq!(b2.node_count(), 1, "应只剩 1 个节点（new1）");
        assert_eq!(b2.edge_count(), 0, "应无边");
        assert!(b2.get_node("new1").await.unwrap().is_some());
        assert!(b2.get_node("old1").await.unwrap().is_none());
    }
}
