//! SparkFox IPC Tauri commands — 前端 React 调用后端 Rust 的命令骨架
//!
//! Task 7.1.2：定义约 10 个 Tauri command，覆盖 6 个 store（agentStore /
//! memoryStore / monitorStore / hotspotStore / sceneStore + ChatView）。
//!
//! Sub-Step 11.4.2：新增 3 个 entity 编辑 command（`entity_merge` /
//! `entity_split` / `entity_rename`），调用 `sparkfox_knowledge::entity_ops`
//! free function，持久化到 entity 表 + event_entity_relation 表。
//!
//! Sub-Step 12.4.2：新增 2 个重命名影响预览 command（`preview_entity_rename_impact` /
//! `execute_entity_rename`），调用 `sparkfox_knowledge::entity_ops` 的同名 free function，
//! 支持重命名前预览受影响 events / relations / chunks 数量，确认后事务执行。
//!
//! # 设计原则
//! - **占位实现**：Task 7.1 的 10 个 command 不实际调用 sparkfox-knowledge /
//!   sparkfox-memory 等业务 crate，避免循环依赖。仅定义 command 签名 + 返回占位结果。
//! - **11.4.2 实装**：3 个 entity command 不再是占位，直接调用
//!   `sparkfox_knowledge::entity_ops::{merge_entities, split_entity, rename_entity}`，
//!   通过 `tauri::State<Mutex<rusqlite::Connection>>` 注入 SQLite 连接。
//! - **12.4.2 增强**：2 个 rename impact preview command 调用
//!   `sparkfox_knowledge::entity_ops::{preview_entity_rename_impact, execute_entity_rename}`，
//!   返回 [`RenameImpactPreview`]（含 affected_events / affected_relations / affected_chunks）。
//! - **标准签名**：所有 command 返回 `Result<T, String>`（Tauri 2 命令标准签名）。
//! - **异步**：所有 command 为 `async fn`，便于后续接入真实异步业务逻辑。
//!
//! # 命令清单
//! | 命令                  | 对接 store       | 说明                                       |
//! |-----------------------|------------------|--------------------------------------------|
//! | `knowledge_search`    | knowledge/RAG    | 知识库检索（query + mode）                 |
//! | `memory_put`          | memoryStore      | 写入记忆条目                               |
//! | `memory_get`          | memoryStore      | 按 id 读取记忆条目                         |
//! | `memory_list`         | memoryStore      | 列出某层全部记忆                           |
//! | `agent_list`          | agentStore       | 列出全部 Agent                             |
//! | `agent_create`        | agentStore       | 创建新 Agent                               |
//! | `monitor_stats`       | monitorStore     | 获取监控统计                               |
//! | `monitor_ack`         | monitorStore     | 确认告警                                   |
//! | `hotspot_track`       | hotspotStore     | 上报热点事件                               |
//! | `hotspot_list`        | hotspotStore     | 列出近期热点                               |
//! | `entity_merge`        | KnowledgeGraph   | 合并实体（11.4.2 实装）                    |
//! | `entity_split`        | KnowledgeGraph   | 拆分实体（11.4.2 实装）                    |
//! | `entity_rename`       | KnowledgeGraph   | 重命名实体（11.4.2 实装，向后兼容保留）    |
//! | `preview_entity_rename_impact` | KnowledgeGraph | 重命名影响预览（12.4.2，仅查询不修改）|
//! | `execute_entity_rename`        | KnowledgeGraph | 执行重命名（12.4.2，事务原子性）      |

#![forbid(unsafe_code)]

use serde_json::Value;

// Sub-Step 11.4.2: entity 编辑命令直接调用 sparkfox-knowledge::entity_ops free function
// Sub-Step 12.4.2: preview_entity_rename_impact / execute_entity_rename 命令调用同名 free function
use sparkfox_knowledge::entity_ops;
// Sub-Step 12.4.2: RenameImpactPreview 作为 Tauri command 返回值类型（serde::Serialize 派生）
pub use sparkfox_knowledge::entity_ops::RenameImpactPreview;

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
// Sub-Step 11.4.2 — entity 编辑命令（实装，非占位）
// ============================================================================

/// 合并实体 — 将 source_entity 的所有 event_entity_relation 关系转移到
/// target_entity，然后删除 source_entity（含其残留 relations）。
///
/// # 参数
/// - `db`: Tauri managed state（`Mutex<rusqlite::Connection>`）
/// - `source_entity_id`: 被合并的源实体 id（合并后删除）
/// - `target_entity_id`: 合并目标实体 id（保留并接收关系）
///
/// # 返回
/// 成功返回 `Ok(())`；失败返回 `Err(String)`（含错误原因）
///
/// # 设计
/// 直接调用 [`entity_ops::merge_entities`] free function，避免在 Tauri command
/// 层重复实现 SQL 逻辑。事务原子性由 entity_ops 保证（BEGIN/COMMIT/ROLLBACK）。
#[tauri::command]
pub async fn entity_merge(
    db: tauri::State<'_, std::sync::Mutex<rusqlite::Connection>>,
    source_entity_id: String,
    target_entity_id: String,
) -> Result<(), String> {
    log::debug!(
        "entity_merge: source={source_entity_id}, target={target_entity_id}"
    );
    let conn = db.lock().map_err(|e| format!("Mutex lock 失败: {e}"))?;
    entity_ops::merge_entities(&conn, &source_entity_id, &target_entity_id)
        .map_err(|e| format!("entity_merge 失败: {e}"))
}

/// 拆分实体 — 将一个源实体拆分为多个新实体（关系按 round-robin 分配）。
///
/// # 参数
/// - `db`: Tauri managed state（`Mutex<rusqlite::Connection>`）
/// - `source_entity_id`: 被拆分的源实体 id（保留，不删除）
/// - `new_names`: 新实体名称列表（至少 1 个）
///
/// # 返回
/// 成功返回 `Ok(Vec<String>)`（新建实体 id 列表）；失败返回 `Err(String)`
///
/// # 设计
/// 直接调用 [`entity_ops::split_entity`] free function。新实体继承源实体的
/// `entity_type_id`；关系按 `ORDER BY event_id ASC` round-robin 分配以保证可重现。
#[tauri::command]
pub async fn entity_split(
    db: tauri::State<'_, std::sync::Mutex<rusqlite::Connection>>,
    source_entity_id: String,
    new_names: Vec<String>,
) -> Result<Vec<String>, String> {
    log::debug!(
        "entity_split: source={source_entity_id}, new_names={:?}",
        new_names
    );
    let conn = db.lock().map_err(|e| format!("Mutex lock 失败: {e}"))?;
    entity_ops::split_entity(&conn, &source_entity_id, &new_names)
        .map_err(|e| format!("entity_split 失败: {e}"))
}

/// 重命名实体 — 更新 entity.name + entity.normalized_name（保留 id 不变）。
///
/// # 参数
/// - `db`: Tauri managed state（`Mutex<rusqlite::Connection>`）
/// - `entity_id`: 待重命名的实体 id
/// - `new_name`: 新名称（将自动归一化为 normalized_name）
///
/// # 返回
/// 成功返回 `Ok(())`；失败返回 `Err(String)`
///
/// # 设计
/// 直接调用 [`entity_ops::rename_entity`] free function。归一化使用
/// `DefaultEntityNormalizer`（trim + lowercase），与 EventSaver 写入侧一致。
#[tauri::command]
pub async fn entity_rename(
    db: tauri::State<'_, std::sync::Mutex<rusqlite::Connection>>,
    entity_id: String,
    new_name: String,
) -> Result<(), String> {
    log::debug!("entity_rename: id={entity_id}, new_name={new_name}");
    let conn = db.lock().map_err(|e| format!("Mutex lock 失败: {e}"))?;
    entity_ops::rename_entity(&conn, &entity_id, &new_name)
        .map_err(|e| format!("entity_rename 失败: {e}"))
}

// ============================================================================
// Sub-Step 12.4.2 — 重命名影响预览 + 事务执行命令（实装，非占位）
// ============================================================================

/// 重命名影响预览 — 查询重命名后会受影响的 events / relations / chunks 数量，不执行重命名。
///
/// # 参数
/// - `db`: Tauri managed state（`Mutex<rusqlite::Connection>`）
/// - `entity_id`: 待重命名的实体 id
/// - `new_name`: 新名称（预览阶段仅用于日志，不参与查询）
///
/// # 返回
/// 成功返回 `Ok(RenameImpactPreview)`，含三个字段：
/// - `affected_events`: 受影响的 event 数量（DISTINCT event_id）
/// - `affected_relations`: 受影响的 event_entity_relation 行数
/// - `affected_chunks`: 受影响的 knowledge_event 行数（content/summary/title 含旧 name）
///
/// 失败返回 `Err(String)`（如 entity_id 不存在时返回 NotFound 错误信息）
///
/// # 设计
/// 直接调用 [`entity_ops::preview_entity_rename_impact`] free function，纯 SELECT 查询，
/// 不修改任何数据。前端 UI 显示「受影响事件: N / 受影响关系: N / 受影响文本块: N」后
/// 由用户确认是否调用 [`execute_entity_rename`] 执行重命名。
#[tauri::command]
pub async fn preview_entity_rename_impact(
    db: tauri::State<'_, std::sync::Mutex<rusqlite::Connection>>,
    entity_id: String,
    new_name: String,
) -> Result<RenameImpactPreview, String> {
    log::debug!(
        "preview_entity_rename_impact: id={entity_id}, new_name={new_name}"
    );
    let conn = db.lock().map_err(|e| format!("Mutex lock 失败: {e}"))?;
    entity_ops::preview_entity_rename_impact(&conn, &entity_id, &new_name)
        .map_err(|e| format!("preview_entity_rename_impact 失败: {e}"))
}

/// 执行重命名（事务原子性）— 在单个 SQLite 事务中更新 entity.name + knowledge_event 文本，
/// 任一步失败则 ROLLBACK，保证 entity.name 与 chunk_text 同步更新。
///
/// # 参数
/// - `db`: Tauri managed state（`Mutex<rusqlite::Connection>`）
/// - `entity_id`: 待重命名的实体 id
/// - `new_name`: 新名称（将自动归一化为 normalized_name = trim + lowercase）
///
/// # 返回
/// 成功返回 `Ok(RenameImpactPreview)`（含实际受影响数量，应与 [`preview_entity_rename_impact`] 一致）；
/// 失败返回 `Err(String)`（事务自动 ROLLBACK）
///
/// # 设计
/// 直接调用 [`entity_ops::execute_entity_rename`] free function。事务原子性由 entity_ops 保证
/// （BEGIN/COMMIT/ROLLBACK）。重命名流程：
/// 1. UPDATE entity SET name = new_name, normalized_name = ..., updated_time = ...
/// 2. UPDATE knowledge_event.content = REPLACE(content, old_name, new_name) WHERE instr > 0
/// 3. UPDATE knowledge_event.summary = REPLACE(summary, old_name, new_name) WHERE instr > 0
/// 4. UPDATE knowledge_event.title = REPLACE(title, old_name, new_name) WHERE instr > 0
///
/// event_entity_relation 无需 UPDATE（通过 entity_id 外键引用，重命名后自动反映新 name）。
#[tauri::command]
pub async fn execute_entity_rename(
    db: tauri::State<'_, std::sync::Mutex<rusqlite::Connection>>,
    entity_id: String,
    new_name: String,
) -> Result<RenameImpactPreview, String> {
    log::debug!(
        "execute_entity_rename: id={entity_id}, new_name={new_name}"
    );
    let conn = db.lock().map_err(|e| format!("Mutex lock 失败: {e}"))?;
    entity_ops::execute_entity_rename(&conn, &entity_id, &new_name)
        .map_err(|e| format!("execute_entity_rename 失败: {e}"))
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
