//! SparkFox Orchestrator — DAG 编排（RFC-002 编排协调）
//!
//! 融合 OpenAkita 组织编排 + Pangu Nebula 蜂群模式，形成 DAG 结构。
//! DAG 为主，其他编排模式（蜂群/组织/流水线）作为策略插件。
//!
//! NOTICE: OpenAkita AGPL，清洁室重写 — 仅借鉴 DAG + 蜂群思路，不拷贝代码。

#![forbid(unsafe_code)]

pub mod dag;
pub mod swarm;

pub use dag::{Dag, DagEdge, DagNode, DagNodeStatus, EdgeType};
pub use swarm::{AgentSlot, Swarm};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
