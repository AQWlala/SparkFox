//! SparkFox Agent — Agent 引擎
//!
//! AgentProfile + DAG 编排基础。
//! 设计参考 OpenAkita 的 AgentProfile（AGPL，清洁室重写）。
//!
//! 角色映射"双主控 + 蜂群 worker + persona 自进化"设计：
//! - Orchestrator → 主星·编排者
//! - Worker       → 星尘群
//! - Persona      → 化身·灵魂分身
//! - Reviewer     → 星魂

#![forbid(unsafe_code)]

pub mod profile;

pub use profile::{AgentProfile, AgentRole, AgentStatus};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
