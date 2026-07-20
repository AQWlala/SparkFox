//! DAG 编排数据结构 — 融合 OpenAkita 组织编排 + Pangu Nebula 蜂群模式
//!
//! DAG 为主，其他编排模式（蜂群/组织/流水线）作为策略插件。
//! NOTICE: OpenAkita AGPL，清洁室重写 — 仅借鉴 DAG + 蜂群思路，不拷贝代码。

use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use sparkfox_core::{Error, Result};

/// DAG 节点 — 一个可执行任务，关联到某个 Agent
#[derive(Debug, Clone)]
pub struct DagNode {
    /// 节点唯一 Id
    pub id: String,
    /// 关联 AgentProfile.id（Id<AgentId> 的字符串形式）
    pub agent_id: String,
    /// 任务描述
    pub task: String,
    /// 节点状态
    pub status: DagNodeStatus,
}

/// DAG 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagNodeStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}

/// DAG 边 — 描述节点间的关系
#[derive(Debug, Clone)]
pub struct DagEdge {
    /// 边类型
    pub edge_type: EdgeType,
}

/// 边类型 — Depends/Parallel/Sequential
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// 依赖 — 必须等待 from 完成后才能执行 to
    Depends,
    /// 并行 — from 和 to 可并行执行（语义提示，不算依赖）
    Parallel,
    /// 顺序 — from 完成后立即执行 to（语义上等同 Depends，调度策略可区分）
    Sequential,
}

/// DAG — 有向无环图，支持拓扑排序与就绪节点查询
pub struct Dag {
    graph: DiGraph<DagNode, DagEdge>,
    node_index: HashMap<String, NodeIndex>,
}

impl Dag {
    /// 创建空 DAG
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
        }
    }

    /// 添加节点 — 返回 NodeIndex；若 Id 已存在会覆盖索引（调用方自行保证唯一性）
    pub fn add_node(&mut self, node: DagNode) -> NodeIndex {
        let idx = self.graph.add_node(node.clone());
        // 若同 Id 节点已存在，索引会被覆盖 — 保持最新 entry
        self.node_index.insert(node.id, idx);
        idx
    }

    /// 添加边 — 从 from 节点到 to 节点
    ///
    /// 错误：from/to 不存在时返回 NotFound
    pub fn add_edge(&mut self, from: &str, to: &str, edge: DagEdge) -> Result<()> {
        let from_idx = self
            .node_index
            .get(from)
            .copied()
            .ok_or_else(|| Error::not_found("DagNode", from))?;
        let to_idx = self
            .node_index
            .get(to)
            .copied()
            .ok_or_else(|| Error::not_found("DagNode", to))?;
        self.graph.add_edge(from_idx, to_idx, edge);
        Ok(())
    }

    /// 拓扑排序 — 检测环，有环时返回 Error
    ///
    /// 返回节点 Id 的拓扑顺序（依赖在前，被依赖在后）
    pub fn topological_order(&self) -> Result<Vec<String>> {
        use petgraph::algo::toposort;
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order
                .into_iter()
                .map(|idx| self.graph[idx].id.clone())
                .collect()),
            Err(_) => Err(Error::internal("DAG 存在环，无法拓扑排序")),
        }
    }

    /// 返回所有依赖已完成的节点 Id（仅 Pending 状态）
    ///
    /// 依赖判定：所有入边的源节点状态均为 Completed
    /// Parallel 边不算依赖（并行语义）
    pub fn ready_nodes(&self) -> Vec<String> {
        let mut ready = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if node.status != DagNodeStatus::Pending {
                continue;
            }
            let mut deps_ok = true;
            for edge in self.graph.edges_directed(idx, petgraph::Direction::Incoming) {
                let edge_type = edge.weight().edge_type;
                if edge_type == EdgeType::Parallel {
                    // Parallel 边不算依赖
                    continue;
                }
                let src_status = self.graph[edge.source()].status;
                if src_status != DagNodeStatus::Completed {
                    deps_ok = false;
                    break;
                }
            }
            if deps_ok {
                ready.push(node.id.clone());
            }
        }
        ready
    }

    /// 获取节点引用
    pub fn node(&self, id: &str) -> Option<&DagNode> {
        self.node_index.get(id).map(|idx| &self.graph[*idx])
    }

    /// 获取节点可变引用
    pub fn node_mut(&mut self, id: &str) -> Option<&mut DagNode> {
        self.node_index
            .get(id)
            .map(|idx| &mut self.graph[*idx])
    }

    /// 节点数量
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// 边数量
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: &str, agent: &str, task: &str) -> DagNode {
        DagNode {
            id: id.to_string(),
            agent_id: agent.to_string(),
            task: task.to_string(),
            status: DagNodeStatus::Pending,
        }
    }

    #[test]
    fn add_node_and_query() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "agent-1", "task A"));
        dag.add_node(n("b", "agent-2", "task B"));
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.node("a").unwrap().task, "task A");
        assert_eq!(dag.node("b").unwrap().agent_id, "agent-2");
        assert!(dag.node("missing").is_none());
    }

    #[test]
    fn node_mut_updates_status() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "agent-1", "task A"));
        dag.node_mut("a").unwrap().status = DagNodeStatus::Running;
        assert_eq!(dag.node("a").unwrap().status, DagNodeStatus::Running);
    }

    #[test]
    fn add_edge_success() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "agent-1", "task A"));
        dag.add_node(n("b", "agent-2", "task B"));
        dag.add_edge(
            "a",
            "b",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn add_edge_missing_node_fails() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "agent-1", "task A"));
        let err = dag
            .add_edge(
                "a",
                "missing",
                DagEdge {
                    edge_type: EdgeType::Depends,
                },
            )
            .unwrap_err();
        assert!(format!("{err}").contains("missing"), "错误信息: {err}");
    }

    #[test]
    fn topological_order_acyclic() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.add_node(n("b", "g2", "B"));
        dag.add_node(n("c", "g3", "C"));
        dag.add_edge(
            "a",
            "b",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        dag.add_edge(
            "b",
            "c",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        let order = dag.topological_order().expect("无环应成功");
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        assert!(pos_a < pos_b, "a 必须在 b 之前");
        assert!(pos_b < pos_c, "b 必须在 c 之前");
    }

    #[test]
    fn topological_order_cyclic_fails() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.add_node(n("b", "g2", "B"));
        dag.add_edge(
            "a",
            "b",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        dag.add_edge(
            "b",
            "a",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        let err = dag.topological_order().unwrap_err();
        assert!(format!("{err}").contains("环"), "错误信息: {err}");
    }

    #[test]
    fn ready_nodes_no_deps() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.add_node(n("b", "g2", "B"));
        let ready = dag.ready_nodes();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn ready_nodes_with_depends_edge() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.add_node(n("b", "g2", "B"));
        dag.add_edge(
            "a",
            "b",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        // a 无入边 → ready
        let ready = dag.ready_nodes();
        assert_eq!(ready, vec!["a".to_string()]);

        // a 完成后 b 应 ready
        dag.node_mut("a").unwrap().status = DagNodeStatus::Completed;
        let ready = dag.ready_nodes();
        assert_eq!(ready, vec!["b".to_string()]);
    }

    #[test]
    fn ready_nodes_parallel_edge_ignored() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.add_node(n("b", "g2", "B"));
        // Parallel 边不算依赖
        dag.add_edge(
            "a",
            "b",
            DagEdge {
                edge_type: EdgeType::Parallel,
            },
        )
        .unwrap();
        let ready = dag.ready_nodes();
        assert_eq!(ready.len(), 2, "Parallel 边不应阻止 b ready");
    }

    #[test]
    fn ready_nodes_skips_non_pending() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.node_mut("a").unwrap().status = DagNodeStatus::Running;
        let ready = dag.ready_nodes();
        assert!(ready.is_empty(), "Running 节点不应 ready");
    }

    #[test]
    fn ready_nodes_failed_dep_blocks() {
        let mut dag = Dag::new();
        dag.add_node(n("a", "g1", "A"));
        dag.add_node(n("b", "g2", "B"));
        dag.add_edge(
            "a",
            "b",
            DagEdge {
                edge_type: EdgeType::Depends,
            },
        )
        .unwrap();
        // a Failed 时 b 不应 ready
        dag.node_mut("a").unwrap().status = DagNodeStatus::Failed;
        let ready = dag.ready_nodes();
        assert!(ready.is_empty(), "依赖 Failed 节点不应 ready");
    }
}
