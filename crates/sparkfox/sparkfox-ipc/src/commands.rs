//! SparkFox IPC Tauri commands — 前端 React 调用后端 Rust 的命令骨架
//!
//! Task 7.1.2：定义约 10 个 Tauri command，覆盖 6 个 store（agentStore /
//! memoryStore / monitorStore / hotspotStore / sceneStore + ChatView）。
//!
//! # 设计原则
//! - **占位实现**：不实际调用 sparkfox-knowledge / sparkfox-memory 等业务 crate，
//!   避免循环依赖。仅定义 command 签名 + 返回占位结果。
//! - **标准签名**：所有 command 返回 `Result<T, String>`（Tauri 2 命令标准签名）。
//! - **异步**：所有 command 为 `async fn`，便于后续接入真实异步业务逻辑。
//!
//! # 命令清单
//! | 命令                  | 对接 store       | 说明                        |
//! |-----------------------|------------------|-----------------------------|
//! | `knowledge_search`    | knowledge/RAG    | 知识库检索（query + mode）  |
//! | `memory_put`          | memoryStore      | 写入记忆条目                |
//! | `memory_get`          | memoryStore      | 按 id 读取记忆条目          |
//! | `memory_list`         | memoryStore      | 列出某层全部记忆            |
//! | `agent_list`          | agentStore       | 列出全部 Agent              |
//! | `agent_create`        | agentStore       | 创建新 Agent                |
//! | `monitor_stats`       | monitorStore     | 获取监控统计                |
//! | `monitor_ack`         | monitorStore     | 确认告警                    |
//! | `hotspot_track`       | hotspotStore     | 上报热点事件                |
//! | `hotspot_list`        | hotspotStore     | 列出近期热点                |

#![forbid(unsafe_code)]

use serde_json::Value;

/// 知识库检索 — 调用 sparkfox-knowledge::RagEngine（占位）
///
/// # 参数
/// - `query`: 检索查询字符串
/// - `mode`: 检索模式（"vector" / "keyword" / "hybrid"）
///
/// # 返回
/// 匹配的文档片段列表（占位：返回空数组）
#[tauri::command]
pub async fn knowledge_search(query: String, mode: String) -> Result<Vec<Value>, String> {
    log::debug!("knowledge_search: query={query}, mode={mode}");
    // 占位：Phase 1 阶段接入 sparkfox-knowledge::RagEngine
    Ok(vec![])
}

/// 写入记忆条目 — 调用 sparkfox-memory（占位）
///
/// # 参数
/// - `layer`: 记忆层（0-5，对应 L0..L5）
/// - `entry`: 记忆条目（JSON，结构由 sparkfox-memory 定义）
///
/// # 返回
/// 写入成功返回 `Ok(())`
#[tauri::command]
pub async fn memory_put(layer: u8, entry: Value) -> Result<(), String> {
    log::debug!("memory_put: layer={layer}, entry={entry}");
    // 占位：Phase 1 阶段接入 sparkfox-memory
    Ok(())
}

/// 按 id 读取记忆条目 — 调用 sparkfox-memory（占位）
///
/// # 参数
/// - `layer`: 记忆层
/// - `id`: 记忆条目 id
///
/// # 返回
/// 找到则返回 `Some(Value)`，未找到返回 `None`（占位：始终返回 `None`）
#[tauri::command]
pub async fn memory_get(layer: u8, id: String) -> Result<Option<Value>, String> {
    log::debug!("memory_get: layer={layer}, id={id}");
    // 占位：Phase 1 阶段接入 sparkfox-memory
    Ok(None)
}

/// 列出某层全部记忆 — 调用 sparkfox-memory（占位）
///
/// # 参数
/// - `layer`: 记忆层
///
/// # 返回
/// 该层全部记忆条目列表（占位：返回空数组）
#[tauri::command]
pub async fn memory_list(layer: u8) -> Result<Vec<Value>, String> {
    log::debug!("memory_list: layer={layer}");
    // 占位：Phase 1 阶段接入 sparkfox-memory
    Ok(vec![])
}

/// 列出全部 Agent — 调用 sparkfox-agent（占位）
///
/// # 返回
/// Agent 元信息列表（占位：返回空数组）
#[tauri::command]
pub async fn agent_list() -> Result<Vec<Value>, String> {
    log::debug!("agent_list");
    // 占位：Phase 1 阶段接入 sparkfox-agent
    Ok(vec![])
}

/// 创建新 Agent — 调用 sparkfox-agent（占位）
///
/// # 参数
/// - `name`: Agent 名称
/// - `config`: Agent 配置（JSON）
///
/// # 返回
/// 新创建的 Agent 元信息（占位：返回基本 JSON）
#[tauri::command]
pub async fn agent_create(name: String, config: Value) -> Result<Value, String> {
    log::debug!("agent_create: name={name}, config={config}");
    // 占位：Phase 1 阶段接入 sparkfox-agent
    Ok(serde_json::json!({
        "id": "placeholder-agent-id",
        "name": name,
        "status": "created",
    }))
}

/// 获取监控统计 — 调用 sparkfox-monitor（占位）
///
/// # 返回
/// 监控统计数据（占位：返回空对象）
#[tauri::command]
pub async fn monitor_stats() -> Result<Value, String> {
    log::debug!("monitor_stats");
    // 占位：Phase 1 阶段接入 sparkfox-monitor
    Ok(serde_json::json!({}))
}

/// 确认告警 — 调用 sparkfox-monitor（占位）
///
/// # 参数
/// - `alert_id`: 告警 id
///
/// # 返回
/// 确认成功返回 `Ok(())`
#[tauri::command]
pub async fn monitor_ack(alert_id: String) -> Result<(), String> {
    log::debug!("monitor_ack: alert_id={alert_id}");
    // 占位：Phase 1 阶段接入 sparkfox-monitor
    Ok(())
}

/// 上报热点事件 — 调用 sparkfox-hotspot（占位）
///
/// # 参数
/// - `event`: 热点事件（JSON，结构由 sparkfox-hotspot 定义）
///
/// # 返回
/// 上报成功返回 `Ok(())`
#[tauri::command]
pub async fn hotspot_track(event: Value) -> Result<(), String> {
    log::debug!("hotspot_track: event={event}");
    // 占位：Phase 1 阶段接入 sparkfox-hotspot
    Ok(())
}

/// 列出近期热点 — 调用 sparkfox-hotspot（占位）
///
/// # 返回
/// 近期热点列表（占位：返回空数组）
#[tauri::command]
pub async fn hotspot_list() -> Result<Vec<Value>, String> {
    log::debug!("hotspot_list");
    // 占位：Phase 1 阶段接入 sparkfox-hotspot
    Ok(vec![])
}

// ============================================================================
// 测试 — 验证 command 占位返回值
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// knowledge_search 占位返回空数组
    #[tokio::test]
    async fn test_knowledge_search_placeholder() {
        let result = knowledge_search("test query".into(), "vector".into())
            .await
            .expect("占位 command 应返回 Ok");
        assert!(result.is_empty(), "占位应返回空数组");
    }

    /// memory_put 占位返回 Ok(())
    #[tokio::test]
    async fn test_memory_put_placeholder() {
        memory_put(3, serde_json::json!({"content": "test"}))
            .await
            .expect("占位 command 应返回 Ok");
    }

    /// memory_get 占位返回 None
    #[tokio::test]
    async fn test_memory_get_placeholder() {
        let result = memory_get(3, "nonexistent-id".into())
            .await
            .expect("占位 command 应返回 Ok");
        assert!(result.is_none(), "占位应返回 None");
    }

    /// memory_list 占位返回空数组
    #[tokio::test]
    async fn test_memory_list_placeholder() {
        let result = memory_list(3)
            .await
            .expect("占位 command 应返回 Ok");
        assert!(result.is_empty(), "占位应返回空数组");
    }

    /// agent_list 占位返回空数组
    #[tokio::test]
    async fn test_agent_list_placeholder() {
        let result = agent_list().await.expect("占位 command 应返回 Ok");
        assert!(result.is_empty(), "占位应返回空数组");
    }

    /// agent_create 占位返回带 name 的 JSON
    #[tokio::test]
    async fn test_agent_create_placeholder() {
        let result = agent_create("TestAgent".into(), serde_json::json!({}))
            .await
            .expect("占位 command 应返回 Ok");
        assert_eq!(result["name"], "TestAgent");
        assert_eq!(result["status"], "created");
    }

    /// monitor_stats 占位返回空对象
    #[tokio::test]
    async fn test_monitor_stats_placeholder() {
        let result = monitor_stats().await.expect("占位 command 应返回 Ok");
        assert!(result.is_object(), "占位应返回 JSON 对象");
    }

    /// monitor_ack 占位返回 Ok(())
    #[tokio::test]
    async fn test_monitor_ack_placeholder() {
        monitor_ack("alert-1".into())
            .await
            .expect("占位 command 应返回 Ok");
    }

    /// hotspot_track 占位返回 Ok(())
    #[tokio::test]
    async fn test_hotspot_track_placeholder() {
        hotspot_track(serde_json::json!({"type": "click"}))
            .await
            .expect("占位 command 应返回 Ok");
    }

    /// hotspot_list 占位返回空数组
    #[tokio::test]
    async fn test_hotspot_list_placeholder() {
        let result = hotspot_list().await.expect("占位 command 应返回 Ok");
        assert!(result.is_empty(), "占位应返回空数组");
    }
}
