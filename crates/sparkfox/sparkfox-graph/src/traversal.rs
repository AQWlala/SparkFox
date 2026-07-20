//! 多跳遍历 — OpenAkita MDRM 5 维清洁室重写
//!
//! ## OpenAkita MDRM 借鉴（AGPL-3.0 合规声明）
//!
//! 本实现借鉴 OpenAkita MDRM（Multi-hop Dynamic Relation Model）5 维过滤思路，
//! **所有代码均为 SparkFox Contributors 独立编写**（清洁室重写，未拷贝 OpenAkita 源码）。
//! 详见 crate 根 `NOTICE` 文件。
//!
//! ## 5 维过滤
//!
//! 1. **深度维度**（Depth）：BFS 逐层扩展，达 `max_depth` 停止
//! 2. **节点类型维度**（Entity）：扩展时过滤 `node_type` 不在白名单的节点
//! 3. **边类型维度**（Relation）：扩展时过滤 `edge_type` 不在白名单的边
//! 4. **时间维度**（Temporal，可选）：节点 `created_time` 不在时间范围的过滤
//! 5. **权重维度**（Semantic，可选）：边 `confidence` < `min_weight` 的过滤
//!
//! ## R-07 LIMIT 阀门
//!
//! 多跳扩展必须设最大节点数限制（默认 1000），防止图遍历爆炸。
//! 当结果节点数达 [`TraversalConfig::max_nodes`] 或遍历边数达
//! [`TraversalConfig::max_edges`] 时，立即停止扩展并返回当前结果。

#![forbid(unsafe_code)]

use std::collections::HashSet;

use sparkfox_core::{Error, Result};

use crate::{GraphBackend, GraphEdge, GraphEdgeType, GraphNode, GraphNodeType};

/// 遍历方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDirection {
    /// 仅出边（`edge.source == current_node`）
    Outgoing,
    /// 仅入边（`edge.target == current_node`）
    Incoming,
    /// 双向（默认）
    Both,
}

/// 遍历配置 — MDRM 5 维过滤参数
///
/// 详见模块级文档。
#[derive(Debug, Clone)]
pub struct TraversalConfig {
    /// 最大 BFS 深度（默认 3）
    pub max_depth: u8,
    /// 最大节点数阀门 R-07（默认 1000）
    ///
    /// 当结果集达此值时立即停止扩展。
    pub max_nodes: usize,
    /// 最大边数阀门（默认 5000）
    ///
    /// 当遍历过的边数达此值时立即停止扩展。
    pub max_edges: usize,
    /// 遍历方向（默认 [`Both`](TraversalDirection::Both)）
    pub direction: TraversalDirection,
    /// 节点类型白名单（`None` 表示不过滤）
    pub node_type_filter: Option<Vec<GraphNodeType>>,
    /// 边类型白名单（`None` 表示不过滤）
    pub edge_type_filter: Option<Vec<GraphEdgeType>>,
    /// 时间范围 `(start_iso, end_iso)`，ISO8601 字符串词法比较
    ///
    /// v1.0.0 基本实现：从 `GraphNode.properties["created_time"]` 读取时间戳，
    /// 词法比较 ISO8601 字符串。若节点无 `created_time` 属性，视为通过（不阻塞扩展）。
    pub time_range: Option<(String, String)>,
    /// 最小边权重阈值（`None` 表示不过滤）
    ///
    /// 从 `GraphEdge.properties["confidence"]` 读取权重（f64 转 f32 比较）。
    pub min_weight: Option<f32>,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_nodes: 1000,
            max_edges: 5000,
            direction: TraversalDirection::Both,
            node_type_filter: None,
            edge_type_filter: None,
            time_range: None,
            min_weight: None,
        }
    }
}

impl TraversalConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大深度
    pub fn with_max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth;
        self
    }

    /// 设置 R-07 节点数阀门
    pub fn with_max_nodes(mut self, n: usize) -> Self {
        self.max_nodes = n;
        self
    }

    /// 设置最大边数阀门
    pub fn with_max_edges(mut self, n: usize) -> Self {
        self.max_edges = n;
        self
    }

    /// 设置遍历方向
    pub fn with_direction(mut self, dir: TraversalDirection) -> Self {
        self.direction = dir;
        self
    }

    /// 设置节点类型白名单
    pub fn with_node_type_filter(mut self, types: Vec<GraphNodeType>) -> Self {
        self.node_type_filter = Some(types);
        self
    }

    /// 设置边类型白名单
    pub fn with_edge_type_filter(mut self, types: Vec<GraphEdgeType>) -> Self {
        self.edge_type_filter = Some(types);
        self
    }

    /// 设置时间范围（ISO8601 字符串）
    pub fn with_time_range(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.time_range = Some((start.into(), end.into()));
        self
    }

    /// 设置最小边权重阈值
    pub fn with_min_weight(mut self, w: f32) -> Self {
        self.min_weight = Some(w);
        self
    }
}

/// MDRM 5 维多跳遍历
///
/// 从 `root_id` 出发，按 [`TraversalConfig`] 进行 BFS 扩展，返回所有可达节点
/// （含根节点；根节点也需通过 `node_type_filter` / `time_range` 过滤）。
///
/// ## R-07 LIMIT 阀门
///
/// 当结果集节点数达 `config.max_nodes` 或遍历边数达 `config.max_edges` 时，
/// 立即停止扩展并返回当前结果。
///
/// ## 5 维过滤顺序
///
/// 1. 边方向过滤（`direction`）
/// 2. 边类型过滤（`edge_type_filter`）
/// 3. 边权重过滤（`min_weight`）
/// 4. 节点类型过滤（`node_type_filter`）
/// 5. 节点时间过滤（`time_range`）
///
/// ## 错误
///
/// - 根节点不存在：返回 `Err(Error::NotFound)`
/// - 后端查询错误：透传
pub async fn multi_hop_traverse(
    backend: &dyn GraphBackend,
    root_id: &str,
    config: &TraversalConfig,
) -> Result<Vec<GraphNode>> {
    // 获取根节点
    let root = backend
        .get_node(root_id)
        .await?
        .ok_or_else(|| Error::not_found("node", root_id))?;

    // 根节点也需通过 node_type / time 过滤
    if !node_type_matches(&root, config) {
        return Ok(vec![]);
    }
    if !time_range_matches(&root, config) {
        return Ok(vec![]);
    }

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(root_id.to_string());
    let mut result: Vec<GraphNode> = vec![root.clone()];
    let mut edge_count = 0usize;
    let mut current_layer: Vec<String> = vec![root_id.to_string()];

    for _depth in 0..config.max_depth {
        let mut next_layer: Vec<String> = Vec::new();
        for nid in &current_layer {
            let edges = backend.get_edges(nid).await?;
            for edge in edges {
                edge_count += 1;
                // R-07 边数阀门
                if edge_count > config.max_edges {
                    return Ok(result);
                }
                // 1. 方向过滤
                if !edge_direction_matches(&edge, nid, config.direction) {
                    continue;
                }
                // 2. 边类型过滤
                if !edge_type_matches(&edge, config) {
                    continue;
                }
                // 3. 权重过滤
                if !weight_matches(&edge, config) {
                    continue;
                }
                // 确定对端节点
                let other = if &edge.source == nid {
                    &edge.target
                } else {
                    &edge.source
                };
                if !visited.contains(other) {
                    if let Some(other_node) = backend.get_node(other).await? {
                        // 4. 节点类型过滤
                        if !node_type_matches(&other_node, config) {
                            visited.insert(other.clone());
                            continue;
                        }
                        // 5. 时间范围过滤
                        if !time_range_matches(&other_node, config) {
                            visited.insert(other.clone());
                            continue;
                        }
                        visited.insert(other.clone());
                        result.push(other_node);
                        // R-07 节点数阀门
                        if result.len() >= config.max_nodes {
                            return Ok(result);
                        }
                        next_layer.push(other.clone());
                    }
                }
            }
        }
        current_layer = next_layer;
        if current_layer.is_empty() {
            break;
        }
    }
    Ok(result)
}

/// 节点类型过滤（维度 2：Entity）
fn node_type_matches(node: &GraphNode, config: &TraversalConfig) -> bool {
    match &config.node_type_filter {
        Some(types) => types.contains(&node.node_type),
        None => true,
    }
}

/// 边类型过滤（维度 3：Relation）
fn edge_type_matches(edge: &GraphEdge, config: &TraversalConfig) -> bool {
    match &config.edge_type_filter {
        Some(types) => types.contains(&edge.edge_type),
        None => true,
    }
}

/// 边方向过滤
fn edge_direction_matches(
    edge: &GraphEdge,
    current_id: &str,
    direction: TraversalDirection,
) -> bool {
    match direction {
        TraversalDirection::Outgoing => edge.source == current_id,
        TraversalDirection::Incoming => edge.target == current_id,
        TraversalDirection::Both => true,
    }
}

/// 边权重过滤（维度 5：Semantic）
///
/// 从 `edge.properties["confidence"]` 读取权重，缺失视为 0.0。
fn weight_matches(edge: &GraphEdge, config: &TraversalConfig) -> bool {
    match config.min_weight {
        Some(min) => {
            let w = edge
                .properties
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            w >= min
        }
        None => true,
    }
}

/// 时间范围过滤（维度 4：Temporal）
///
/// 从 `node.properties["created_time"]` 读取时间戳，词法比较 ISO8601 字符串。
/// 若节点无 `created_time` 属性，放行（不阻塞扩展）。
fn time_range_matches(node: &GraphNode, config: &TraversalConfig) -> bool {
    let Some((start, end)) = &config.time_range else {
        return true;
    };
    let Some(t) = node.properties.get("created_time").and_then(|v| v.as_str()) else {
        // 节点无时间属性，放行
        return true;
    };
    t >= start.as_str() && t <= end.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PetgraphBackend;
    use rusqlite::Connection;

    /// 构造节点（可选 created_time）
    fn make_node(id: &str, nt: GraphNodeType, label: &str, created: Option<&str>) -> GraphNode {
        let mut props = serde_json::json!({});
        if let Some(c) = created {
            props["created_time"] = serde_json::json!(c);
        }
        GraphNode {
            id: id.to_string(),
            node_type: nt,
            label: label.to_string(),
            properties: props,
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

    /// 构造测试图：
    /// ```text
    ///   e1 <──r1(0.95)── ev1 ──r2(0.5)──> e2
    ///                    │
    ///                    └──r3(0.3)──> e3
    /// ```
    /// ev1 是 Event，eN 是 Entity；所有边 source=ev1，target=eN。
    fn setup_backend() -> PetgraphBackend {
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node(
            "e1",
            GraphNodeType::Entity,
            "北京",
            Some("2026-01-01T00:00:00Z"),
        ))
        .unwrap();
        b.add_node(make_node(
            "e2",
            GraphNodeType::Entity,
            "上海",
            Some("2026-06-01T00:00:00Z"),
        ))
        .unwrap();
        b.add_node(make_node(
            "e3",
            GraphNodeType::Entity,
            "广州",
            Some("2026-12-01T00:00:00Z"),
        ))
        .unwrap();
        b.add_node(make_node(
            "ev1",
            GraphNodeType::Event,
            "会议",
            Some("2026-05-01T00:00:00Z"),
        ))
        .unwrap();
        // 注意：source=ev1, target=e1（边方向 ev1 → e1）
        b.add_edge("ev1", "e1", make_edge("r1", "ev1", "e1", 0.95))
            .unwrap();
        b.add_edge("ev1", "e2", make_edge("r2", "ev1", "e2", 0.5))
            .unwrap();
        b.add_edge("ev1", "e3", make_edge("r3", "ev1", "e3", 0.3))
            .unwrap();
        b
    }

    #[tokio::test]
    async fn test_default_config_traverses_all() {
        let b = setup_backend();
        let result = multi_hop_traverse(&b, "ev1", &TraversalConfig::default())
            .await
            .unwrap();
        // ev1 + e1 + e2 + e3 = 4
        assert_eq!(result.len(), 4, "默认配置应遍历所有 4 个节点");
    }

    #[tokio::test]
    async fn test_depth_limit_one() {
        let b = setup_backend();
        let cfg = TraversalConfig::new().with_max_depth(1);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        // ev1 + e1 + e2 + e3 = 4（1 跳即到达所有直连邻居）
        assert_eq!(result.len(), 4, "depth=1 应到达 ev1 的所有直连邻居");
    }

    #[tokio::test]
    async fn test_depth_limit_zero() {
        let b = setup_backend();
        let cfg = TraversalConfig::new().with_max_depth(0);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        // 仅根节点
        assert_eq!(result.len(), 1, "depth=0 仅根节点");
        assert_eq!(result[0].id, "ev1");
    }

    #[tokio::test]
    async fn test_depth_limit_two_hop() {
        // 2 跳场景：e1 → ev1 → e2/e3
        let b = setup_backend();
        let cfg = TraversalConfig::new().with_max_depth(2);
        let result = multi_hop_traverse(&b, "e1", &cfg).await.unwrap();
        // e1 + ev1 + e2 + e3 = 4
        assert_eq!(result.len(), 4, "2 跳应到达所有节点");
        let ids: Vec<_> = result.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"e1"));
        assert!(ids.contains(&"ev1"));
        assert!(ids.contains(&"e2"));
        assert!(ids.contains(&"e3"));
    }

    #[tokio::test]
    async fn test_max_nodes_valve_basic() {
        let b = setup_backend();
        // R-07：max_nodes=2 应在收集到 2 个节点时停止
        let cfg = TraversalConfig::new().with_max_nodes(2);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 2, "R-07 阀门应在 2 节点时停止");
        assert_eq!(result[0].id, "ev1", "根节点应在结果中");
    }

    #[tokio::test]
    async fn test_max_nodes_valve_large_graph() {
        // 构造 200 节点的星形图，验证 R-07 提前停止
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("root", GraphNodeType::Event, "root", None))
            .unwrap();
        for i in 0..200 {
            let id = format!("n{i}");
            b.add_node(make_node(&id, GraphNodeType::Entity, &format!("节点{i}"), None))
                .unwrap();
            b.add_edge("root", &id, make_edge(&format!("e{i}"), "root", &id, 1.0))
                .unwrap();
        }
        // max_nodes = 50
        let cfg = TraversalConfig::new().with_max_nodes(50);
        let result = multi_hop_traverse(&b, "root", &cfg).await.unwrap();
        assert_eq!(result.len(), 50, "应在 50 节点时停止（R-07）");
        assert_eq!(result[0].id, "root", "根节点应在首位");
    }

    #[tokio::test]
    async fn test_max_edges_valve() {
        // 构造多边图，max_edges=2 应在遍历 2 条边后停止
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("root", GraphNodeType::Event, "root", None))
            .unwrap();
        for i in 0..10 {
            let id = format!("n{i}");
            b.add_node(make_node(&id, GraphNodeType::Entity, &format!("n{i}"), None))
                .unwrap();
            b.add_edge("root", &id, make_edge(&format!("e{i}"), "root", &id, 1.0))
                .unwrap();
        }
        // max_edges = 2 → 仅遍历 2 条边
        let cfg = TraversalConfig::new().with_max_edges(2);
        let result = multi_hop_traverse(&b, "root", &cfg).await.unwrap();
        // root + 2 个邻居 = 3
        assert_eq!(result.len(), 3, "max_edges=2 应在 2 条边后停止");
    }

    #[tokio::test]
    async fn test_node_type_filter_only_event() {
        let b = setup_backend();
        // 仅 Event 类型（只有 ev1）
        let cfg = TraversalConfig::new().with_node_type_filter(vec![GraphNodeType::Event]);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 1, "Event 过滤应只剩 ev1");
        assert_eq!(result[0].id, "ev1");
    }

    #[tokio::test]
    async fn test_node_type_filter_excludes_root() {
        let b = setup_backend();
        // 仅 Entity 类型，根节点 ev1 是 Event，应被过滤（返回空）
        let cfg = TraversalConfig::new().with_node_type_filter(vec![GraphNodeType::Entity]);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert!(result.is_empty(), "根节点不通过过滤时应返回空");
    }

    #[tokio::test]
    async fn test_node_type_filter_multiple_types() {
        let b = setup_backend();
        let cfg = TraversalConfig::new()
            .with_node_type_filter(vec![GraphNodeType::Entity, GraphNodeType::Event]);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 4, "Entity+Event 过滤应包含所有");
    }

    #[tokio::test]
    async fn test_edge_type_filter_pass() {
        let b = setup_backend();
        let cfg = TraversalConfig::new().with_edge_type_filter(vec![GraphEdgeType::EventEntity]);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 4, "EventEntity 过滤应包含所有");
    }

    #[tokio::test]
    async fn test_min_weight_filter() {
        let b = setup_backend();
        // confidence >= 0.6 → 仅 r1(0.95) 通过，e1 加入；r2/r3 被过滤
        let cfg = TraversalConfig::new().with_min_weight(0.6);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        // ev1 + e1 = 2（r2/r3 被过滤）
        assert_eq!(result.len(), 2);
        let ids: Vec<_> = result.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"ev1"));
        assert!(ids.contains(&"e1"));
        assert!(!ids.contains(&"e2"));
        assert!(!ids.contains(&"e3"));
    }

    #[tokio::test]
    async fn test_min_weight_filter_zero() {
        let b = setup_backend();
        // min_weight = 0 应通过所有边（confidence 都 >= 0）
        let cfg = TraversalConfig::new().with_min_weight(0.0);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 4);
    }

    #[tokio::test]
    async fn test_time_range_filter() {
        let b = setup_backend();
        // 仅 2026-01-01 ~ 2026-06-30 的节点
        let cfg = TraversalConfig::new()
            .with_time_range("2026-01-01T00:00:00Z", "2026-06-30T00:00:00Z");
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        // ev1 (2026-05) + e1 (2026-01) + e2 (2026-06) = 3（e3 在 2026-12 被过滤）
        assert_eq!(result.len(), 3);
        let ids: Vec<_> = result.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"ev1"));
        assert!(ids.contains(&"e1"));
        assert!(ids.contains(&"e2"));
        assert!(!ids.contains(&"e3"), "e3 在 2026-12 应被时间过滤");
    }

    #[tokio::test]
    async fn test_time_range_excludes_root() {
        let b = setup_backend();
        // ev1 created_time = 2026-05-01，时间范围设为 2026-06-01 ~ 2026-12-31
        // 根节点 ev1 不在范围内，应返回空
        let cfg = TraversalConfig::new()
            .with_time_range("2026-06-01T00:00:00Z", "2026-12-31T00:00:00Z");
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert!(result.is_empty(), "根节点不在时间范围应返回空");
    }

    #[tokio::test]
    async fn test_direction_outgoing() {
        let b = setup_backend();
        // ev1 出边：ev1→e1, ev1→e2, ev1→e3
        let cfg = TraversalConfig::new().with_direction(TraversalDirection::Outgoing);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 4, "Outgoing 方向 ev1 应到达所有 e1/e2/e3");
    }

    #[tokio::test]
    async fn test_direction_incoming() {
        let b = setup_backend();
        // ev1 入边：无（所有边都是 ev1→x，没有 x→ev1）
        let cfg = TraversalConfig::new().with_direction(TraversalDirection::Incoming);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        assert_eq!(result.len(), 1, "Incoming 方向 ev1 无入边，仅根节点");
        assert_eq!(result[0].id, "ev1");
    }

    #[tokio::test]
    async fn test_direction_incoming_from_entity() {
        let b = setup_backend();
        // 从 e1 看，入边：ev1→e1（target=e1），出边：无
        let cfg = TraversalConfig::new().with_direction(TraversalDirection::Incoming);
        let result = multi_hop_traverse(&b, "e1", &cfg).await.unwrap();
        // e1 + ev1 = 2（Incoming 方向找到 ev1→e1）
        assert_eq!(result.len(), 2);
        let ids: Vec<_> = result.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"e1"));
        assert!(ids.contains(&"ev1"));
    }

    #[tokio::test]
    async fn test_root_not_found() {
        let b = setup_backend();
        let r = multi_hop_traverse(&b, "missing", &TraversalConfig::default()).await;
        assert!(r.is_err(), "根节点不存在应报错");
    }

    #[tokio::test]
    async fn test_combined_filters() {
        let b = setup_backend();
        // min_weight=0.4 + Entity/Event 双类型过滤
        let cfg = TraversalConfig::new()
            .with_min_weight(0.4)
            .with_node_type_filter(vec![GraphNodeType::Entity, GraphNodeType::Event]);
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        // r1(0.95)→e1, r2(0.5)→e2 通过；r3(0.3)→e3 被过滤
        // ev1 + e1 + e2 = 3
        assert_eq!(result.len(), 3);
        let ids: Vec<_> = result.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"ev1"));
        assert!(ids.contains(&"e1"));
        assert!(ids.contains(&"e2"));
        assert!(!ids.contains(&"e3"));
    }

    #[tokio::test]
    async fn test_combined_filters_weight_and_time() {
        let b = setup_backend();
        // min_weight=0.4 + 时间范围 2026-01 ~ 2026-06
        let cfg = TraversalConfig::new()
            .with_min_weight(0.4)
            .with_time_range("2026-01-01T00:00:00Z", "2026-06-30T00:00:00Z");
        let result = multi_hop_traverse(&b, "ev1", &cfg).await.unwrap();
        // 通过 weight 的边：r1(0.95)→e1, r2(0.5)→e2；r3(0.3) 被过滤
        // 通过 time 的节点：ev1(2026-05), e1(2026-01), e2(2026-06)；e3(2026-12) 被过滤
        // ev1 + e1 + e2 = 3
        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn test_default_config_values() {
        let cfg = TraversalConfig::default();
        assert_eq!(cfg.max_depth, 3);
        assert_eq!(cfg.max_nodes, 1000, "R-07 默认 1000");
        assert_eq!(cfg.max_edges, 5000);
        assert_eq!(cfg.direction, TraversalDirection::Both);
        assert!(cfg.node_type_filter.is_none());
        assert!(cfg.edge_type_filter.is_none());
        assert!(cfg.time_range.is_none());
        assert!(cfg.min_weight.is_none());
    }

    #[tokio::test]
    async fn test_no_created_time_passes_time_filter() {
        // 节点无 created_time 属性，时间过滤应放行
        let conn = Connection::open_in_memory().unwrap();
        let mut b = PetgraphBackend::new(conn).unwrap();
        b.add_node(make_node("root", GraphNodeType::Event, "root", None))
            .unwrap();
        b.add_node(make_node("n1", GraphNodeType::Entity, "n1", None))
            .unwrap();
        b.add_edge("root", "n1", make_edge("e1", "root", "n1", 1.0))
            .unwrap();

        let cfg = TraversalConfig::new().with_time_range("2026-01-01", "2026-12-31");
        let result = multi_hop_traverse(&b, "root", &cfg).await.unwrap();
        // 无 created_time 视为通过，应遍历所有
        assert_eq!(result.len(), 2, "无 created_time 应通过时间过滤");
    }
}
