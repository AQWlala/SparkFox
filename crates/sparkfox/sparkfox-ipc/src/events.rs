//! SparkFox IPC Tauri events — 后端 Rust 推送到前端 React 的事件载荷
//!
//! Task 7.1.3：定义 Tauri event 载荷结构体，后端通过 `app_handle.emit(event, payload)`
//! 推送，前端通过 `listen(event, callback)` 订阅。
//!
//! # 设计原则
//! - 所有 event 载荷实现 `serde::Serialize`（Tauri emit 要求）
//! - 字段使用 `snake_case`（Rust 惯例），前端通过 `camelCase` 转换消费
//! - 载荷结构体命名为 `XxxPayload`，对应 event 名为 `xxx_yyy`（snake_case）
//!
//! # 事件清单
//! | event 名             | 载荷结构体           | 对接 view       | 说明                  |
//! |----------------------|----------------------|-----------------|-----------------------|
//! | `thought_pushed`     | `ThoughtPushed`      | ChatView        | 思考过程推送          |
//! | `citation_added`     | `CitationAdded`      | ChatView        | 引用追加              |
//! | `hotspot_updated`    | `HotspotUpdated`     | HotspotView     | 热点更新              |
//! | `monitor_updated`    | `MonitorUpdated`     | MonitorView     | 监控数据更新          |
//! | `memory_changed`     | `MemoryChanged`      | MemoryView      | 记忆变更通知          |
//! | `agent_status`       | `AgentStatus`        | agentStore      | Agent 状态变更        |

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 思考过程推送 — ChatView 实时展示 Agent 思考链
///
/// event 名：`thought_pushed`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtPushed {
    /// 会话 id
    pub session_id: String,
    /// 思考序号（同会话内递增）
    pub seq: u32,
    /// 思考内容文本
    pub content: String,
    /// 是否为本次思考的最后一段
    pub is_final: bool,
}

/// 引用追加 — ChatView 在回答中追加知识库引用
///
/// event 名：`citation_added`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationAdded {
    /// 会话 id
    pub session_id: String,
    /// 引用 id（唯一标识）
    pub citation_id: String,
    /// 引用来源文档标题
    pub title: String,
    /// 引用来源 URL 或文件路径
    pub source: String,
    /// 引用文本片段
    pub snippet: String,
}

/// 热点更新 — HotspotView 实时展示热点变化
///
/// event 名：`hotspot_updated`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotUpdated {
    /// 热点 id
    pub hotspot_id: String,
    /// 热点标题
    pub title: String,
    /// 热度值（0.0-1.0）
    pub score: f32,
    /// 更新时间戳（Unix 毫秒）
    pub updated_at: i64,
}

/// 监控数据更新 — MonitorView 实时刷新监控指标
///
/// event 名：`monitor_updated`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorUpdated {
    /// 监控指标名（cpu / memory / disk / network / ...）
    pub metric: String,
    /// 当前值
    pub value: f64,
    /// 单位（% / MB / Mbps / ...）
    pub unit: String,
    /// 采样时间戳（Unix 毫秒）
    pub timestamp: i64,
}

/// 记忆变更通知 — MemoryView 增量更新记忆列表
///
/// event 名：`memory_changed`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChanged {
    /// 变更类型（"put" / "delete" / "update"）
    pub change_type: String,
    /// 记忆层（0-5）
    pub layer: u8,
    /// 变更的记忆 id
    pub memory_id: String,
}

/// Agent 状态变更 — agentStore 同步 Agent 运行状态
///
/// event 名：`agent_status`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent id
    pub agent_id: String,
    /// 新状态（"idle" / "running" / "waiting" / "error" / "stopped"）
    pub status: String,
    /// 状态变更时间戳（Unix 毫秒）
    pub changed_at: i64,
}

// ============================================================================
// 测试 — 验证 event 载荷可序列化 / 反序列化
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thought_pushed_serialize() {
        let payload = ThoughtPushed {
            session_id: "sess-1".into(),
            seq: 42,
            content: "正在分析...".into(),
            is_final: false,
        };
        let json = serde_json::to_string(&payload).expect("序列化失败");
        assert!(json.contains("sess-1"));
        assert!(json.contains("正在分析"));
        // 反序列化往返
        let back: ThoughtPushed = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back.session_id, "sess-1");
        assert_eq!(back.seq, 42);
        assert!(!back.is_final);
    }

    #[test]
    fn test_citation_added_serialize() {
        let payload = CitationAdded {
            session_id: "sess-1".into(),
            citation_id: "cit-1".into(),
            title: "SparkFox 设计文档".into(),
            source: "docs/spec.md".into(),
            snippet: "SparkFox 是 ...".into(),
        };
        let json = serde_json::to_string(&payload).expect("序列化失败");
        let back: CitationAdded = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back.citation_id, "cit-1");
        assert_eq!(back.title, "SparkFox 设计文档");
    }

    #[test]
    fn test_hotspot_updated_serialize() {
        let payload = HotspotUpdated {
            hotspot_id: "hot-1".into(),
            title: "热点话题".into(),
            score: 0.87,
            updated_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&payload).expect("序列化失败");
        let back: HotspotUpdated = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back.hotspot_id, "hot-1");
        assert!((back.score - 0.87).abs() < 1e-6);
    }

    #[test]
    fn test_monitor_updated_serialize() {
        let payload = MonitorUpdated {
            metric: "cpu".into(),
            value: 75.5,
            unit: "%".into(),
            timestamp: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&payload).expect("序列化失败");
        let back: MonitorUpdated = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back.metric, "cpu");
        assert!((back.value - 75.5).abs() < 1e-6);
    }

    #[test]
    fn test_memory_changed_serialize() {
        let payload = MemoryChanged {
            change_type: "put".into(),
            layer: 3,
            memory_id: "mem-1".into(),
        };
        let json = serde_json::to_string(&payload).expect("序列化失败");
        let back: MemoryChanged = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back.change_type, "put");
        assert_eq!(back.layer, 3);
    }

    #[test]
    fn test_agent_status_serialize() {
        let payload = AgentStatus {
            agent_id: "agent-1".into(),
            status: "running".into(),
            changed_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&payload).expect("序列化失败");
        let back: AgentStatus = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back.agent_id, "agent-1");
        assert_eq!(back.status, "running");
    }
}
