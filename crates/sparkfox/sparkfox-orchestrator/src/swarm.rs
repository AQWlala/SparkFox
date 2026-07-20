//! Swarm — 蜂群编排 + 组织编排融合
//!
//! 双主控 + 蜂群 worker + persona 自进化设计：
//! - 主星·编排者（Orchestrator）：注册为 agent，dispatch DAG
//! - 化身·灵魂分身（Persona）：persona 自进化
//! - 星尘群（Worker）：蜂群并发执行 DAG ready 节点
//! - 星魂（Reviewer）：评审节点
//!
//! NOTICE: OpenAkita AGPL，清洁室重写 — 仅借鉴 DAG + 蜂群思路，不拷贝代码。
//!
//! 设计说明（与任务规格的细微调整）：
//! - agents 用 `tokio::sync::RwLock`，因为 `tick` 跨 await 持有 agent 槽位锁
//! - dag 用 `std::sync::RwLock`，DAG 操作短暂，无需跨 await 持锁；
//!   这样 `dispatch`（同步方法）可在任何上下文调用，包括异步运行时内

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use sparkfox_agent::{AgentProfile, AgentStatus};
use sparkfox_core::{Error, Result};
use tokio::sync::RwLock as AsyncRwLock;

use crate::dag::{Dag, DagNodeStatus};

/// Agent 槽位 — 包装 AgentProfile + 当前任务
pub struct AgentSlot {
    /// Agent 配置
    pub profile: AgentProfile,
    /// 当前正在执行的任务（DagNode.id），空闲时为 None
    pub current_task: Option<String>,
}

impl AgentSlot {
    /// 创建新槽位
    pub fn new(profile: AgentProfile) -> Self {
        Self {
            profile,
            current_task: None,
        }
    }

    /// 是否空闲（profile 状态为 Idle 且无当前任务）
    pub fn is_idle(&self) -> bool {
        self.profile.status == AgentStatus::Idle && self.current_task.is_none()
    }
}

/// Swarm — 蜂群调度器
///
/// 持有所有注册的 Agent 与当前 DAG，`tick` 推进一轮调度。
pub struct Swarm {
    agents: HashMap<String, Arc<AsyncRwLock<AgentSlot>>>,
    dag: Arc<RwLock<Dag>>,
}

impl Swarm {
    /// 创建空 Swarm
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            dag: Arc::new(RwLock::new(Dag::new())),
        }
    }

    /// 注册 Agent — 按 `profile.id` 字符串索引
    pub fn register(&mut self, profile: AgentProfile) {
        let id = profile.id.to_string();
        let slot = AgentSlot::new(profile);
        self.agents.insert(id, Arc::new(AsyncRwLock::new(slot)));
    }

    /// 派发 DAG — 替换当前 DAG；校验无环
    ///
    /// 同步方法，可在任何上下文调用（包括异步运行时内）。
    pub fn dispatch(&self, dag: Dag) -> Result<()> {
        // 校验 DAG 无环（toposort 失败表示有环）
        dag.topological_order()?;
        let mut guard = self
            .dag
            .write()
            .map_err(|e| Error::internal(format!("DAG 锁获取失败: {e}")))?;
        *guard = dag;
        Ok(())
    }

    /// 推进一轮调度 — 返回本轮完成的节点数
    ///
    /// 流程：
    /// 1. 收集所有 ready 节点（短暂持 dag 读锁）
    /// 2. 对每个 ready 节点：
    ///    a. 找到对应 agent，标记为 Running（异步持 agent 锁）
    ///    b. 标记节点为 Running（短暂持 dag 写锁）
    ///    c. 模拟执行（Phase 1 占位 — 后续接入 LLM 异步执行）
    ///    d. 标记节点为 Completed（短暂持 dag 写锁）
    ///    e. 恢复 agent 为 Idle（异步持 agent 锁）
    pub async fn tick(&self) -> Result<usize> {
        // 第一阶段：收集 ready 节点 + 对应 agent_id（短暂持 dag 读锁）
        let ready: Vec<(String, String)> = {
            let dag = self
                .dag
                .read()
                .map_err(|e| Error::internal(format!("DAG 读锁获取失败: {e}")))?;
            dag.ready_nodes()
                .into_iter()
                .filter_map(|node_id| {
                    let node = dag.node(&node_id)?;
                    Some((node_id, node.agent_id.clone()))
                })
                .collect()
        };

        let mut completed = 0usize;
        for (node_id, agent_id) in ready {
            // 找到对应 agent；未注册则跳过
            let slot = match self.agents.get(&agent_id) {
                Some(s) => s.clone(),
                None => continue,
            };

            // a. 标记 agent Running
            {
                let mut slot_guard = slot.write().await;
                slot_guard.profile.transition(AgentStatus::Running);
                slot_guard.current_task = Some(node_id.clone());
            }

            // b. 标记节点 Running
            {
                let mut dag = self
                    .dag
                    .write()
                    .map_err(|e| Error::internal(format!("DAG 写锁获取失败: {e}")))?;
                if let Some(node) = dag.node_mut(&node_id) {
                    node.status = DagNodeStatus::Running;
                }
            }

            // c. 模拟执行（Phase 1 占位 — 后续接入 LLM 异步执行）

            // d. 标记节点 Completed
            {
                let mut dag = self
                    .dag
                    .write()
                    .map_err(|e| Error::internal(format!("DAG 写锁获取失败: {e}")))?;
                if let Some(node) = dag.node_mut(&node_id) {
                    node.status = DagNodeStatus::Completed;
                }
            }

            // e. 恢复 agent Idle
            {
                let mut slot_guard = slot.write().await;
                slot_guard.profile.transition(AgentStatus::Idle);
                slot_guard.current_task = None;
            }

            completed += 1;
        }
        Ok(completed)
    }

    /// 注册 Agent 数量
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for Swarm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparkfox_agent::AgentRole;

    fn make_profile(name: &str, role: AgentRole) -> AgentProfile {
        AgentProfile::new(name.to_string(), role, "system".to_string())
    }

    fn make_node(id: &str, agent_id: &str, task: &str) -> crate::dag::DagNode {
        crate::dag::DagNode {
            id: id.to_string(),
            agent_id: agent_id.to_string(),
            task: task.to_string(),
            status: DagNodeStatus::Pending,
        }
    }

    #[test]
    fn register_agent_increments_count() {
        let mut swarm = Swarm::new();
        assert_eq!(swarm.agent_count(), 0);
        swarm.register(make_profile("agent-1", AgentRole::Worker));
        assert_eq!(swarm.agent_count(), 1);
        swarm.register(make_profile("agent-2", AgentRole::Worker));
        assert_eq!(swarm.agent_count(), 2);
    }

    #[test]
    fn dispatch_valid_dag_succeeds() {
        let mut swarm = Swarm::new();
        swarm.register(make_profile("agent-1", AgentRole::Orchestrator));
        let agent_id = swarm.agents.keys().next().unwrap().clone();
        let mut dag = Dag::new();
        dag.add_node(make_node("n1", &agent_id, "task 1"));
        swarm.dispatch(dag).expect("无环 DAG 应派发成功");
    }

    #[test]
    fn dispatch_cyclic_dag_fails() {
        let mut swarm = Swarm::new();
        swarm.register(make_profile("agent-1", AgentRole::Worker));
        let agent_id = swarm.agents.keys().next().unwrap().clone();
        let mut dag = Dag::new();
        dag.add_node(make_node("a", &agent_id, "A"));
        dag.add_node(make_node("b", &agent_id, "B"));
        dag.add_edge(
            "a",
            "b",
            crate::dag::DagEdge {
                edge_type: crate::dag::EdgeType::Depends,
            },
        )
        .unwrap();
        dag.add_edge(
            "b",
            "a",
            crate::dag::DagEdge {
                edge_type: crate::dag::EdgeType::Depends,
            },
        )
        .unwrap();
        let err = swarm.dispatch(dag).unwrap_err();
        assert!(format!("{err}").contains("环"), "错误信息: {err}");
    }

    #[tokio::test]
    async fn tick_completes_chain() {
        let mut swarm = Swarm::new();
        swarm.register(make_profile("agent-1", AgentRole::Worker));
        let agent_id = swarm.agents.keys().next().unwrap().clone();
        let mut dag = Dag::new();
        dag.add_node(make_node("n1", &agent_id, "task 1"));
        dag.add_node(make_node("n2", &agent_id, "task 2"));
        dag.add_edge(
            "n1",
            "n2",
            crate::dag::DagEdge {
                edge_type: crate::dag::EdgeType::Depends,
            },
        )
        .unwrap();
        swarm.dispatch(dag).expect("派发成功");

        // 第一轮：只有 n1 ready（n2 依赖 n1）
        let c1 = swarm.tick().await.expect("tick 应成功");
        assert_eq!(c1, 1, "第一轮应完成 1 个节点");

        // 验证 n1 已完成、n2 仍 Pending
        {
            let dag = swarm.dag.read().unwrap();
            assert_eq!(dag.node("n1").unwrap().status, DagNodeStatus::Completed);
            assert_eq!(dag.node("n2").unwrap().status, DagNodeStatus::Pending);
        }

        // 第二轮：n2 ready（n1 已完成）
        let c2 = swarm.tick().await.expect("tick 应成功");
        assert_eq!(c2, 1, "第二轮应完成 1 个节点");
        {
            let dag = swarm.dag.read().unwrap();
            assert_eq!(dag.node("n2").unwrap().status, DagNodeStatus::Completed);
        }

        // 第三轮：无 ready 节点
        let c3 = swarm.tick().await.expect("tick 应成功");
        assert_eq!(c3, 0, "第三轮应无节点完成");
    }

    #[tokio::test]
    async fn tick_with_unregistered_agent_skips() {
        let swarm = Swarm::new();
        // 不注册任何 agent，但 dag 引用了不存在的 agent
        let mut dag = Dag::new();
        dag.add_node(make_node("n1", "agent-missing", "task 1"));
        swarm.dispatch(dag).expect("派发成功");

        // tick 应完成 0 个节点（agent 未注册）
        let c = swarm.tick().await.expect("tick 应成功");
        assert_eq!(c, 0);
    }

    #[tokio::test]
    async fn tick_restores_agent_to_idle() {
        let mut swarm = Swarm::new();
        swarm.register(make_profile("agent-1", AgentRole::Worker));
        let agent_id = swarm.agents.keys().next().unwrap().clone();
        let mut dag = Dag::new();
        dag.add_node(make_node("n1", &agent_id, "task 1"));
        swarm.dispatch(dag).expect("派发成功");

        let _ = swarm.tick().await.unwrap();

        // 验证 agent 恢复 Idle
        let slot = swarm.agents.get(&agent_id).unwrap();
        let slot_guard = slot.read().await;
        assert_eq!(slot_guard.profile.status, AgentStatus::Idle);
        assert!(slot_guard.current_task.is_none());
    }
}
