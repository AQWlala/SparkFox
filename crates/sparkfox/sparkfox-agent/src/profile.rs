//! AgentProfile — Agent 配置文件
//!
//! 定义 SparkFox Agent 的角色、状态和系统提示词。
//! 设计参考 OpenAkita 的 AgentProfile（AGPL，清洁室重写）。
//!
//! 角色映射"双主控 + 蜂群 worker + persona 自进化"设计：
//! - Orchestrator → 主星·编排者（DAG 顶层调度）
//! - Worker       → 星尘群（蜂群并发执行）
//! - Persona      → 化身·灵魂分身（persona 自进化）
//! - Reviewer     → 星魂（评审节点）

use serde::{Deserialize, Serialize};
use sparkfox_core::{AgentId, Id};

/// Agent 配置文件 — 描述 Agent 的角色、状态和系统提示词
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Agent 唯一 Id（强类型，防止与其他实体 Id 混用）
    pub id: Id<AgentId>,
    /// Agent 名称（人类可读）
    pub name: String,
    /// Agent 角色
    pub role: AgentRole,
    /// Agent 当前状态
    pub status: AgentStatus,
    /// 系统提示词（定义 Agent 的人格与能力边界）
    pub system_prompt: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Agent 角色 — 对应"双主控 + 蜂群 worker + persona 自进化"设计
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgentRole {
    /// 主星·编排者 — DAG 顶层调度
    Orchestrator,
    /// 星尘群 — 蜂群 worker，执行具体子任务（默认角色）
    #[default]
    Worker,
    /// 化身·灵魂分身 — persona 自进化角色
    Persona,
    /// 星魂 — 评审角色
    Reviewer,
}

/// Agent 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgentStatus {
    /// 空闲（默认状态）
    #[default]
    Idle,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
    /// 错误
    Error,
}

impl AgentProfile {
    /// 创建新的 AgentProfile — 自动生成 Id 并设置状态为 Idle
    pub fn new(name: String, role: AgentRole, system_prompt: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Id::new(),
            name,
            role,
            status: AgentStatus::Idle,
            system_prompt,
            created_at: now,
            updated_at: now,
        }
    }

    /// 状态转换 — 更新 status 并刷新 updated_at
    pub fn transition(&mut self, new_status: AgentStatus) {
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_has_idle_status() {
        let profile = AgentProfile::new(
            "test-agent".to_string(),
            AgentRole::Worker,
            "You are a test agent".to_string(),
        );
        assert_eq!(profile.status, AgentStatus::Idle);
        assert_eq!(profile.role, AgentRole::Worker);
        assert_eq!(profile.name, "test-agent");
        assert_eq!(profile.system_prompt, "You are a test agent");
        // 新建时 created_at == updated_at
        assert_eq!(profile.created_at, profile.updated_at);
    }

    #[test]
    fn transition_updates_status_and_timestamp() {
        let mut profile = AgentProfile::new(
            "test-agent".to_string(),
            AgentRole::Worker,
            "You are a test agent".to_string(),
        );
        let original_updated = profile.updated_at;
        // 等待时间推进，确保 updated_at 改变
        std::thread::sleep(std::time::Duration::from_millis(10));
        profile.transition(AgentStatus::Running);
        assert_eq!(profile.status, AgentStatus::Running);
        assert!(profile.updated_at > original_updated);
    }

    #[test]
    fn profile_serialization_roundtrip() {
        let profile = AgentProfile::new(
            "serde-agent".to_string(),
            AgentRole::Orchestrator,
            "You orchestrate".to_string(),
        );
        let json = serde_json::to_string(&profile).expect("序列化成功");
        let decoded: AgentProfile = serde_json::from_str(&json).expect("反序列化成功");
        assert_eq!(profile.id, decoded.id);
        assert_eq!(profile.name, decoded.name);
        assert_eq!(profile.role, decoded.role);
        assert_eq!(profile.status, decoded.status);
        assert_eq!(profile.system_prompt, decoded.system_prompt);
    }

    #[test]
    fn role_serialization_roundtrip() {
        for role in [
            AgentRole::Orchestrator,
            AgentRole::Worker,
            AgentRole::Persona,
            AgentRole::Reviewer,
        ] {
            let json = serde_json::to_string(&role).expect("序列化成功");
            let decoded: AgentRole = serde_json::from_str(&json).expect("反序列化成功");
            assert_eq!(role, decoded, "角色序列化往返失败: {json}");
        }
    }

    #[test]
    fn status_serialization_roundtrip() {
        for status in [
            AgentStatus::Idle,
            AgentStatus::Running,
            AgentStatus::Paused,
            AgentStatus::Stopped,
            AgentStatus::Error,
        ] {
            let json = serde_json::to_string(&status).expect("序列化成功");
            let decoded: AgentStatus = serde_json::from_str(&json).expect("反序列化成功");
            assert_eq!(status, decoded, "状态序列化往返失败: {json}");
        }
    }

    #[test]
    fn role_default_is_worker() {
        assert_eq!(AgentRole::default(), AgentRole::Worker);
    }

    #[test]
    fn status_default_is_idle() {
        assert_eq!(AgentStatus::default(), AgentStatus::Idle);
    }
}
