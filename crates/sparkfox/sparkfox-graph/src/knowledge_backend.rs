//! KnowledgeGraphBackend — 反向引用 sparkfox-knowledge 的 event/entity 表
//!
//! 【A-02 P0 修复】实现 [`GraphBackend`](crate::GraphBackend) trait，
//! 所有图操作直接查询 sparkfox-knowledge 的 SAG 3 表：
//!
//! - `entity` 表 → [`GraphNode`]（[`Entity`](crate::GraphNodeType::Entity) 类型）
//! - `knowledge_event` 表 → [`GraphNode`]（[`Event`](crate::GraphNodeType::Event) 类型）
//! - `event_entity_relation` 表 → [`GraphEdge`]（[`EventEntity`](crate::GraphEdgeType::EventEntity) 类型）
//!
//! ## SoT 原则
//!
//! 本实现 **不创建任何新表**，仅读取 sparkfox-knowledge 已有的 SAG 表。
//! sparkfox-knowledge 为唯一 SoT（Source of Truth），sparkfox-graph 仅作为通用图遍历引擎。
//!
//! ## 字段映射（对齐 sparkfox-knowledge/src/schema.rs）
//!
//! | sparkfox-graph 类型 | sparkfox-knowledge 表 | 字段映射 |
//! |---|---|---|
//! | `GraphNode(Entity)` | `entity` | id / name → label / normalized_name + description + extra_data → properties |
//! | `GraphNode(Event)` | `knowledge_event` | id / title → label / summary + content → properties |
//! | `GraphEdge(EventEntity)` | `event_entity_relation` | id / event_id → source / entity_id → target / relation_type + confidence + extra_data → properties |

use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::{params, Connection};
use tokio::sync::Mutex;

use sparkfox_core::{Error, Result};

use crate::{Graph, GraphBackend, GraphEdge, GraphEdgeType, GraphNode, GraphNodeType};

/// 【A-02 P0 修复】KnowledgeGraphBackend — 反向引用 sparkfox-knowledge 的 SAG 表
///
/// 持有 `Arc<Mutex<Connection>>`（与 sparkfox-knowledge 共享同一 SQLite 连接），
/// 所有图操作直接查询 `entity` / `knowledge_event` / `event_entity_relation` 表。
///
/// ## 创建方式
///
/// - [`KnowledgeGraphBackend::new`]：从 `Arc<Mutex<Connection>>` 创建（推荐，共享连接）
/// - [`KnowledgeGraphBackend::from_conn`]：从 `Connection` 创建（独占连接，用于测试）
pub struct KnowledgeGraphBackend {
    conn: Arc<Mutex<Connection>>,
}

impl KnowledgeGraphBackend {
    /// 从 `Arc<Mutex<Connection>>` 创建（推荐方式，与 sparkfox-knowledge 共享连接）
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 从 `Connection` 创建（独占连接，主要用于测试）
    pub fn from_conn(conn: Connection) -> Self {
        Self::new(Arc::new(Mutex::new(conn)))
    }
}

#[async_trait]
impl GraphBackend for KnowledgeGraphBackend {
    async fn get_node(&self, id: &str) -> Result<Option<GraphNode>> {
        let conn = self.conn.lock().await;
        // 1. 优先查 entity 表
        if let Some(node) = Self::query_entity(&conn, id)? {
            return Ok(Some(node));
        }
        // 2. 其次查 knowledge_event 表
        if let Some(node) = Self::query_event(&conn, id)? {
            return Ok(Some(node));
        }
        Ok(None)
    }

    async fn get_neighbors(&self, node_id: &str, max_depth: u8) -> Result<Vec<GraphNode>> {
        let conn = self.conn.lock().await;
        let mut visited = std::collections::HashSet::new();
        visited.insert(node_id.to_string());
        let mut current_layer = vec![node_id.to_string()];
        let mut all_neighbors = Vec::new();

        for _depth in 0..max_depth {
            let mut next_layer = Vec::new();
            for nid in &current_layer {
                // 通过 event_entity_relation 双向索引查邻居
                let neighbors = Self::query_neighbors(&conn, nid)?;
                for n in neighbors {
                    if !visited.contains(&n.id) {
                        visited.insert(n.id.clone());
                        next_layer.push(n.id.clone());
                        all_neighbors.push(n);
                    }
                }
            }
            current_layer = next_layer;
            if current_layer.is_empty() {
                break;
            }
        }
        Ok(all_neighbors)
    }

    async fn subgraph(&self, root: &str, max_depth: u8) -> Result<Graph> {
        let conn = self.conn.lock().await;
        let mut visited_nodes = std::collections::HashSet::new();
        let mut visited_edges = std::collections::HashSet::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // 获取根节点
        if let Some(root_node) = Self::query_any_node(&conn, root)? {
            visited_nodes.insert(root.to_string());
            nodes.push(root_node);
        }

        // BFS 扩展
        let mut current_layer = vec![root.to_string()];
        for _depth in 0..max_depth {
            let mut next_layer = Vec::new();
            for nid in &current_layer {
                let node_edges = Self::query_edges(&conn, nid)?;
                for edge in node_edges {
                    let edge_key = edge.id.clone();
                    if !visited_edges.contains(&edge_key) {
                        visited_edges.insert(edge_key);
                        edges.push(edge.clone());
                        // 添加对端节点
                        let other = if edge.source == *nid {
                            &edge.target
                        } else {
                            &edge.source
                        };
                        if !visited_nodes.contains(other) {
                            visited_nodes.insert(other.clone());
                            if let Some(n) = Self::query_any_node(&conn, other)? {
                                nodes.push(n);
                                next_layer.push(other.clone());
                            }
                        }
                    }
                }
            }
            current_layer = next_layer;
            if current_layer.is_empty() {
                break;
            }
        }

        Ok(Graph { nodes, edges })
    }

    async fn get_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let conn = self.conn.lock().await;
        Self::query_edges(&conn, node_id)
    }

    fn backend_name(&self) -> &'static str {
        "knowledge_graph_backend"
    }
}

impl KnowledgeGraphBackend {
    /// 查询 entity 表（对应 GraphNode::Entity）
    ///
    /// SQL 字段对齐 sparkfox-knowledge/src/schema.rs DDL_ENTITY：
    /// id / name / normalized_name / description / extra_data
    fn query_entity(conn: &Connection, id: &str) -> Result<Option<GraphNode>> {
        let mut stmt = conn
            .prepare("SELECT id, name, normalized_name, description, extra_data FROM entity WHERE id = ?1")
            .map_err(|e| Error::internal(format!("prepare entity 查询失败: {e}")))?;
        let mut rows = stmt
            .query(params![id])
            .map_err(|e| Error::internal(format!("entity 查询失败: {e}")))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| Error::internal(format!("entity row 失败: {e}")))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| Error::internal(format!("entity id 失败: {e}")))?;
            let name: String = row.get(1).unwrap_or_default();
            let normalized_name: Option<String> = row.get(2).ok();
            let description: Option<String> = row.get(3).ok();
            let extra_data: Option<String> = row.get(4).ok();
            let properties = serde_json::json!({
                "name": name,
                "normalized_name": normalized_name,
                "description": description,
                "extra_data": extra_data,
            });
            return Ok(Some(GraphNode {
                id,
                node_type: GraphNodeType::Entity,
                label: name,
                properties,
            }));
        }
        Ok(None)
    }

    /// 查询 knowledge_event 表（对应 GraphNode::Event）
    ///
    /// SQL 字段对齐 sparkfox-knowledge/src/schema.rs DDL_KNOWLEDGE_EVENT：
    /// id / title / summary / content
    fn query_event(conn: &Connection, id: &str) -> Result<Option<GraphNode>> {
        let mut stmt = conn
            .prepare("SELECT id, title, summary, content FROM knowledge_event WHERE id = ?1")
            .map_err(|e| Error::internal(format!("prepare event 查询失败: {e}")))?;
        let mut rows = stmt
            .query(params![id])
            .map_err(|e| Error::internal(format!("event 查询失败: {e}")))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| Error::internal(format!("event row 失败: {e}")))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| Error::internal(format!("event id 失败: {e}")))?;
            let title: String = row.get(1).unwrap_or_default();
            let summary: Option<String> = row.get(2).ok();
            let content: Option<String> = row.get(3).ok();
            let properties = serde_json::json!({
                "title": title,
                "summary": summary,
                "content": content,
            });
            return Ok(Some(GraphNode {
                id,
                node_type: GraphNodeType::Event,
                label: title,
                properties,
            }));
        }
        Ok(None)
    }

    /// 查询任意节点（先 entity，后 knowledge_event）
    fn query_any_node(conn: &Connection, id: &str) -> Result<Option<GraphNode>> {
        if let Some(n) = Self::query_entity(conn, id)? {
            return Ok(Some(n));
        }
        Self::query_event(conn, id)
    }

    /// 通过 event_entity_relation 表查邻居（双向）
    ///
    /// 利用 schema.rs 的双向复合索引：
    /// - idx_eer_event_entity（event_id, entity_id）— 正向
    /// - idx_eer_entity_event（entity_id, event_id）— 反向
    fn query_neighbors(conn: &Connection, node_id: &str) -> Result<Vec<GraphNode>> {
        let mut stmt = conn
            .prepare(
                r#"SELECT 
                    CASE WHEN eer.event_id = ?1 THEN eer.entity_id ELSE eer.event_id END AS neighbor_id
                   FROM event_entity_relation eer
                   WHERE eer.event_id = ?1 OR eer.entity_id = ?1"#,
            )
            .map_err(|e| Error::internal(format!("prepare neighbors 失败: {e}")))?;
        let neighbor_ids: Vec<String> = stmt
            .query_map(params![node_id], |row| row.get(0))
            .map_err(|e| Error::internal(format!("neighbors 查询失败: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        let mut neighbors = Vec::new();
        for nid in neighbor_ids {
            if let Some(n) = Self::query_any_node(conn, &nid)? {
                neighbors.push(n);
            }
        }
        Ok(neighbors)
    }

    /// 查询节点的所有关联边（event_entity_relation 表）
    ///
    /// SQL 字段对齐 sparkfox-knowledge/src/schema.rs DDL_EVENT_ENTITY_RELATION：
    /// id / event_id / entity_id / relation_type / confidence / extra_data
    fn query_edges(conn: &Connection, node_id: &str) -> Result<Vec<GraphEdge>> {
        let mut stmt = conn
            .prepare(
                r#"SELECT id, event_id, entity_id, relation_type, confidence, extra_data
                   FROM event_entity_relation
                   WHERE event_id = ?1 OR entity_id = ?1"#,
            )
            .map_err(|e| Error::internal(format!("prepare edges 失败: {e}")))?;
        let edges = stmt
            .query_map(params![node_id], |row| {
                let id: String = row.get(0)?;
                let event_id: String = row.get(1)?;
                let entity_id: String = row.get(2)?;
                let relation_type: Option<String> = row.get(3)?;
                let confidence: Option<f64> = row.get(4)?;
                let extra_data: Option<String> = row.get(5)?;
                Ok(GraphEdge {
                    id,
                    source: event_id,
                    target: entity_id,
                    edge_type: GraphEdgeType::EventEntity,
                    properties: serde_json::json!({
                        "relation_type": relation_type,
                        "confidence": confidence,
                        "extra_data": extra_data,
                    }),
                })
            })
            .map_err(|e| Error::internal(format!("edges 查询失败: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造与 sparkfox-knowledge/src/schema.rs 字段一致的内存测试库
    ///
    /// 为简化测试 INSERT，NOT NULL 约束的 created_time/updated_time 使用默认值，
    /// entity_type_id 放宽为可空（测试不验证 FK）。
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        // DDL 字段名严格对齐 sparkfox-knowledge/src/schema.rs（仅放宽 NOT NULL 以简化测试插入）
        conn.execute_batch(
            r#"
            CREATE TABLE entity (
                id TEXT PRIMARY KEY,
                source_config_id TEXT,
                entity_type_id TEXT,
                name TEXT NOT NULL,
                normalized_name TEXT NOT NULL,
                description TEXT,
                extra_data TEXT,
                created_time TEXT NOT NULL DEFAULT '2026-07-19T00:00:00Z',
                updated_time TEXT NOT NULL DEFAULT '2026-07-19T00:00:00Z'
            );
            CREATE TABLE knowledge_event (
                id TEXT PRIMARY KEY,
                kb_id TEXT NOT NULL DEFAULT 'test_kb',
                doc_id TEXT NOT NULL DEFAULT 'test_doc',
                title TEXT NOT NULL,
                summary TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                created_time TEXT NOT NULL DEFAULT '2026-07-19T00:00:00Z',
                updated_time TEXT NOT NULL DEFAULT '2026-07-19T00:00:00Z'
            );
            CREATE TABLE event_entity_relation (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                relation_type TEXT,
                confidence REAL NOT NULL DEFAULT 1.0,
                extra_data TEXT,
                created_time TEXT NOT NULL DEFAULT '2026-07-19T00:00:00Z'
            );
            "#,
        )
        .unwrap();
        // 插入测试数据
        conn.execute(
            "INSERT INTO entity (id, name, normalized_name, description) VALUES ('e1', '北京', '北京', '中国首都')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entity (id, name, normalized_name) VALUES ('e2', '上海', '上海')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_event (id, title, summary, content) VALUES ('ev1', '北京会议', '北京举办的会议', '2026 年北京 AI 峰会')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, relation_type, confidence) VALUES ('r1', 'ev1', 'e1', 'subject', 0.95)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, relation_type, confidence) VALUES ('r2', 'ev1', 'e2', 'mention', 0.8)",
            [],
        )
        .unwrap();
        conn
    }

    #[tokio::test]
    async fn test_get_node_entity() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        let node = backend.get_node("e1").await.unwrap();
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.node_type, GraphNodeType::Entity);
        assert_eq!(node.label, "北京");
        // 验证 properties 含 description
        assert_eq!(node.properties["description"], "中国首都");
    }

    #[tokio::test]
    async fn test_get_node_event() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        let node = backend.get_node("ev1").await.unwrap();
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.node_type, GraphNodeType::Event);
        assert_eq!(node.label, "北京会议");
        // 验证 properties 含 summary 和 content
        assert_eq!(node.properties["summary"], "北京举办的会议");
        assert_eq!(node.properties["content"], "2026 年北京 AI 峰会");
    }

    #[tokio::test]
    async fn test_get_node_not_found() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        let node = backend.get_node("nonexistent").await.unwrap();
        assert!(node.is_none());
    }

    #[tokio::test]
    async fn test_get_neighbors_1_hop() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // ev1 关联 e1 和 e2（1 跳）
        let neighbors = backend.get_neighbors("ev1", 1).await.unwrap();
        assert_eq!(neighbors.len(), 2, "ev1 应有 2 个直连邻居（e1 + e2）");
        let labels: Vec<_> = neighbors.iter().map(|n| n.label.as_str()).collect();
        assert!(labels.contains(&"北京"), "邻居应含北京");
        assert!(labels.contains(&"上海"), "邻居应含上海");
    }

    #[tokio::test]
    async fn test_get_neighbors_2_hop() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // e1 → ev1 → e2（2 跳）
        let neighbors = backend.get_neighbors("e1", 2).await.unwrap();
        assert!(
            neighbors.iter().any(|n| n.id == "ev1"),
            "2 跳邻居应含 ev1"
        );
        assert!(
            neighbors.iter().any(|n| n.id == "e2"),
            "2 跳邻居应含 e2"
        );
    }

    #[tokio::test]
    async fn test_get_neighbors_excludes_start() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // 起始节点 e1 不应出现在邻居列表中
        let neighbors = backend.get_neighbors("e1", 2).await.unwrap();
        assert!(
            !neighbors.iter().any(|n| n.id == "e1"),
            "邻居列表不应含起始节点 e1"
        );
    }

    #[tokio::test]
    async fn test_get_edges() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        let edges = backend.get_edges("ev1").await.unwrap();
        assert_eq!(edges.len(), 2, "ev1 应有 2 条关联边（r1 + r2）");
        assert!(
            edges.iter().all(|e| e.edge_type == GraphEdgeType::EventEntity),
            "所有边类型应为 EventEntity"
        );
        // 验证边字段
        let r1 = edges.iter().find(|e| e.id == "r1").unwrap();
        assert_eq!(r1.source, "ev1");
        assert_eq!(r1.target, "e1");
        assert_eq!(r1.properties["relation_type"], "subject");
        assert_eq!(r1.properties["confidence"], 0.95);
    }

    #[tokio::test]
    async fn test_get_edges_entity_side() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // 从 entity 侧查边（验证双向查询）
        let edges = backend.get_edges("e1").await.unwrap();
        assert_eq!(edges.len(), 1, "e1 应有 1 条关联边（r1）");
        assert_eq!(edges[0].id, "r1");
    }

    #[tokio::test]
    async fn test_subgraph_from_event() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        let graph = backend.subgraph("ev1", 1).await.unwrap();
        // 节点: ev1 + e1 + e2 = 3
        assert_eq!(graph.nodes.len(), 3, "子图应含 3 个节点");
        // 边: r1 + r2 = 2
        assert_eq!(graph.edges.len(), 2, "子图应含 2 条边");
    }

    #[tokio::test]
    async fn test_subgraph_from_entity_2_hop() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // e1 → ev1 → e2（2 跳子图）
        let graph = backend.subgraph("e1", 2).await.unwrap();
        // 节点: e1 + ev1 + e2 = 3
        assert_eq!(graph.nodes.len(), 3, "2 跳子图应含 3 个节点");
        // 边: r1 + r2 = 2
        assert_eq!(graph.edges.len(), 2, "2 跳子图应含 2 条边");
    }

    #[tokio::test]
    async fn test_subgraph_nonexistent_root() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // 不存在的根节点应返回空 Graph（不报错）
        let graph = backend.subgraph("nonexistent", 1).await.unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[tokio::test]
    async fn test_backend_name() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        assert_eq!(backend.backend_name(), "knowledge_graph_backend");
    }

    #[tokio::test]
    async fn test_get_neighbors_max_depth_zero() {
        let conn = setup_test_db();
        let backend = KnowledgeGraphBackend::from_conn(conn);
        // max_depth = 0 应返回空邻居列表
        let neighbors = backend.get_neighbors("ev1", 0).await.unwrap();
        assert!(neighbors.is_empty(), "max_depth=0 应返回空邻居列表");
    }
}
