//! Sub-Step 10.7.1 — SearchHit 多跳元数据扩展共用类型（U-02，spec §三 10.9.1）
//!
//! 本模块定义 SAG 检索结果中跨多跳追溯所需的实体引用类型 [`EntityRef`]。
//!
//! ## U-02 背景
//! v1.1.0 引入 MULTI 多跳检索策略后，原 `SearchHit.via_entities: Vec<String>`（仅含 `entity_id`）
//! 不足以表达"实体是什么类型 / 叫什么名字"，导致调用方需二次查询 `entity` / `entity_type` 表
//! 才能渲染路径。U-02 修复扩展为 `Vec<EntityRef>`，把 `entity_id` / `entity_type` / `name`
//! 一次性带出，便于上层 UI 直接展示多跳路径。
//!
//! ## 字段来源（atomic.rs::find_events SQL JOIN）
//! - `entity_id`：`entity.id`（UUID 字符串）
//! - `entity_type`：`entity_type.type`（如 `"PERSON"` / `"LOCATION"` / `"ORGANIZATION"`）
//! - `name`：`entity.name`（原始名，未归一化；归一化名可通过 `entity.normalized_name` 二次查询）
//!
//! ## 设计取舍
//! - 不包含 `normalized_name`：避免冗余，`name` 已足够展示（归一化名仅用于 SQL 匹配）
//! - 不包含 `confidence` / `relation_type`：这些是 `event_entity_relation` 表的列，
//!   表征"事件-实体"关系而非"实体"本身，未来如需可在 [`SearchHit`] 上层另设字段
//! - 派生 `PartialEq`：便于测试断言与去重
//! - 派生 `Serialize` / `Deserialize`：便于 API 返回 / 跨设备 CRDT 同步

/// 实体引用 — SAG 检索结果中 `via_entities` 路径节点的结构化类型
///
/// 用于 [`super::SearchHit::via_entities`] 字段，表征"该 hit 是通过哪些实体命中的"。
/// 每个节点携带完整类型信息，调用方无需二次查询 `entity` / `entity_type` 表即可渲染多跳路径。
///
/// ## 字段
/// - `entity_id`：实体 ID（对应 `entity.id`，UUID 字符串）
/// - `entity_type`：实体类型（对应 `entity_type.type`，如 `"PERSON"` / `"LOCATION"`）
/// - `name`：实体名称（对应 `entity.name`，原始未归一化名）
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::EntityRef;
///
/// let e = EntityRef {
///     entity_id: "ent-1".to_string(),
///     entity_type: "PERSON".to_string(),
///     name: "张三".to_string(),
/// };
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EntityRef {
    /// 实体 ID（对应 `entity.id`，UUID 字符串）
    pub entity_id: String,
    /// 实体类型（对应 `entity_type.type`，如 `"PERSON"` / `"LOCATION"` / `"ORGANIZATION"`）
    pub entity_type: String,
    /// 实体名称（对应 `entity.name`，原始未归一化名）
    pub name: String,
}
