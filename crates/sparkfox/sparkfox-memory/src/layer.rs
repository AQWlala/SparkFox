//! 【A-01 P0 修复】6 层记忆 + L3 子层 — SAG 表映射 SoT
//!
//! 原 v1.0.0 方案错误地将 event/entity 映射到 L1（Working Memory，TTL 过期），
//! 本模块修正为 L3（Episodic/Semantic/GraphNode/GraphEdge/EventEntity）。
//!
//! ## 6 层记忆 + SAG 表映射（A-01 P0 修正）
//!
//! | SAG 表 | 6 层记忆映射 | 说明 |
//! |---|---|---|
//! | knowledge_event | L3 Episodic | 情节记忆（事件） |
//! | entity | L3 Semantic + L3 GraphNode | 语义记忆 + 图节点 |
//! | event_entity_relation | L3 GraphEdge | 图边 |
//! | event_entity_embedding | L3 EventEntity | 关联级嵌入 |
//! | knowledge_chunk | L0 Raw | 原始分块 |
//!
//! 设计参考：Pangu Nebula 6 层架构 + OpenAkita 三层记忆 + SAG schema。
//!
//! 注意：本模块定义的 `MemoryLayer` **枚举** 用于 SAG 表映射 / vector_insert 表名选择。
//! `sparkfox_core::MemoryLayer` **trait**（const LAYER + name()）仍然保留，
//! 通过全限定路径 `sparkfox_core::MemoryLayer` 引用，二者不冲突。

use std::str::FromStr;

/// 【A-01 P0 修复】6 层记忆 + L3 子层
///
/// SAG 表映射：
/// - knowledge_event → L3Episodic（情节记忆）
/// - entity → L3Semantic + L3GraphNode（语义记忆 + 图节点）
/// - event_entity_relation → L3GraphEdge（图边）
/// - event_entity_embedding → L3EventEntity（关联级嵌入）
/// - knowledge_chunk → L0Raw（原始分块）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryLayer {
    L0Raw,           // 原始数据 — knowledge_chunk 映射于此
    L1Working,       // 工作记忆（TTL 过期）
    L2Core,          // 核心事实
    L3Episodic,      // 情节记忆 — SAG knowledge_event 映射于此
    L3Semantic,      // 语义记忆 — SAG entity 映射于此
    L3GraphNode,     // 图节点 — entity 同义（sparkfox-graph 引用）
    L3GraphEdge,     // 图边 — SAG event_entity_relation 映射于此
    L3EventEntity,   // 关联级嵌入 — SAG event_entity_embedding 映射于此
    L4Persona,       // 人格
    L5Meta,          // 元认知
}

impl MemoryLayer {
    /// 返回层级的字符串标识（持久化 / 跨进程 IPC 用）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::L0Raw => "L0_raw",
            Self::L1Working => "L1_working",
            Self::L2Core => "L2_core",
            Self::L3Episodic => "L3_episodic",
            Self::L3Semantic => "L3_semantic",
            Self::L3GraphNode => "L3_graph_node",
            Self::L3GraphEdge => "L3_graph_edge",
            Self::L3EventEntity => "L3_event_entity",
            Self::L4Persona => "L4_persona",
            Self::L5Meta => "L5_meta",
        }
    }

    /// 对应的 SQLite 向量表名（用于 vector_insert，A-03 协同）。
    ///
    /// - L3GraphNode 复用 `vec_l3_entity`（entity 表即图节点存储）
    /// - L3GraphEdge 复用 `vec_l3_event_entity`（关系级嵌入即关联级嵌入）
    pub fn vector_table_name(&self) -> &'static str {
        match self {
            Self::L0Raw => "vec_l0",
            Self::L1Working => "vec_l1",
            Self::L2Core => "vec_l2",
            Self::L3Episodic => "vec_l3_event",
            Self::L3Semantic => "vec_l3_entity",
            Self::L3GraphNode => "vec_l3_entity",       // GraphNode 复用 entity 表
            Self::L3GraphEdge => "vec_l3_event_entity",  // GraphEdge 复用 event_entity 表
            Self::L3EventEntity => "vec_l3_event_entity",
            Self::L4Persona => "vec_l4",
            Self::L5Meta => "vec_l5",
        }
    }
}

/// 反序列化 / 字符串还原（as_str 的逆运算）。
impl FromStr for MemoryLayer {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "L0_raw" => Ok(Self::L0Raw),
            "L1_working" => Ok(Self::L1Working),
            "L2_core" => Ok(Self::L2Core),
            "L3_episodic" => Ok(Self::L3Episodic),
            "L3_semantic" => Ok(Self::L3Semantic),
            "L3_graph_node" => Ok(Self::L3GraphNode),
            "L3_graph_edge" => Ok(Self::L3GraphEdge),
            "L3_event_entity" => Ok(Self::L3EventEntity),
            "L4_persona" => Ok(Self::L4Persona),
            "L5_meta" => Ok(Self::L5Meta),
            other => Err(format!("未知 MemoryLayer 标识: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证所有 10 个枚举值存在（编译期 + 运行期双重保障）。
    #[test]
    fn test_memory_layer_variants() {
        let variants = [
            MemoryLayer::L0Raw,
            MemoryLayer::L1Working,
            MemoryLayer::L2Core,
            MemoryLayer::L3Episodic,
            MemoryLayer::L3Semantic,
            MemoryLayer::L3GraphNode,
            MemoryLayer::L3GraphEdge,
            MemoryLayer::L3EventEntity,
            MemoryLayer::L4Persona,
            MemoryLayer::L5Meta,
        ];
        assert_eq!(variants.len(), 10, "MemoryLayer 应有 10 个变体");

        // 各变体互不相同（hashable + eq）
        let mut seen = std::collections::HashSet::new();
        for v in variants {
            assert!(seen.insert(v), "重复变体: {v:?}");
        }
    }

    /// 验证 vector_table_name 映射（A-03 vector_insert 协同）。
    #[test]
    fn test_vector_table_name_mapping() {
        assert_eq!(MemoryLayer::L0Raw.vector_table_name(), "vec_l0");
        assert_eq!(MemoryLayer::L1Working.vector_table_name(), "vec_l1");
        assert_eq!(MemoryLayer::L2Core.vector_table_name(), "vec_l2");
        assert_eq!(MemoryLayer::L3Episodic.vector_table_name(), "vec_l3_event");
        assert_eq!(MemoryLayer::L3Semantic.vector_table_name(), "vec_l3_entity");
        assert_eq!(MemoryLayer::L3GraphNode.vector_table_name(), "vec_l3_entity");
        assert_eq!(MemoryLayer::L3GraphEdge.vector_table_name(), "vec_l3_event_entity");
        assert_eq!(MemoryLayer::L3EventEntity.vector_table_name(), "vec_l3_event_entity");
        assert_eq!(MemoryLayer::L4Persona.vector_table_name(), "vec_l4");
        assert_eq!(MemoryLayer::L5Meta.vector_table_name(), "vec_l5");
    }

    /// 验证 as_str + FromStr 往返一致性。
    #[test]
    fn test_as_str_roundtrip() {
        let variants = [
            MemoryLayer::L0Raw,
            MemoryLayer::L1Working,
            MemoryLayer::L2Core,
            MemoryLayer::L3Episodic,
            MemoryLayer::L3Semantic,
            MemoryLayer::L3GraphNode,
            MemoryLayer::L3GraphEdge,
            MemoryLayer::L3EventEntity,
            MemoryLayer::L4Persona,
            MemoryLayer::L5Meta,
        ];
        for v in variants {
            let s = v.as_str();
            let back: MemoryLayer = s.parse().expect("as_str 产物应可被 FromStr 还原");
            assert_eq!(v, back, "往返失败: {v:?} -> {s} -> {back:?}");
        }

        // 未知标识应返回 Err
        let bad: Result<MemoryLayer, String> = "L9_unknown".parse();
        assert!(bad.is_err());
    }

    /// 验证 SAG 4 表 + knowledge_chunk 映射一致性（A-01 P0 修正核心）。
    ///
    /// 原方案错误：event/entity → L1Working（TTL 过期，会丢失）。
    /// 修正后：event/entity → L3（持久化情节/语义记忆）。
    #[test]
    fn test_sag_mapping_consistency() {
        // knowledge_event → L3Episodic（情节记忆）
        assert_eq!(MemoryLayer::L3Episodic.as_str(), "L3_episodic");
        assert_eq!(MemoryLayer::L3Episodic.vector_table_name(), "vec_l3_event");

        // entity → L3Semantic + L3GraphNode（语义记忆 + 图节点）
        assert_eq!(MemoryLayer::L3Semantic.as_str(), "L3_semantic");
        assert_eq!(MemoryLayer::L3Semantic.vector_table_name(), "vec_l3_entity");
        assert_eq!(MemoryLayer::L3GraphNode.as_str(), "L3_graph_node");
        assert_eq!(MemoryLayer::L3GraphNode.vector_table_name(), "vec_l3_entity");

        // event_entity_relation → L3GraphEdge（图边）
        assert_eq!(MemoryLayer::L3GraphEdge.as_str(), "L3_graph_edge");
        assert_eq!(MemoryLayer::L3GraphEdge.vector_table_name(), "vec_l3_event_entity");

        // event_entity_embedding → L3EventEntity（关联级嵌入）
        assert_eq!(MemoryLayer::L3EventEntity.as_str(), "L3_event_entity");
        assert_eq!(MemoryLayer::L3EventEntity.vector_table_name(), "vec_l3_event_entity");

        // knowledge_chunk → L0Raw（原始分块）
        assert_eq!(MemoryLayer::L0Raw.as_str(), "L0_raw");
        assert_eq!(MemoryLayer::L0Raw.vector_table_name(), "vec_l0");

        // 关键修正断言：event/entity 不应映射到 L1Working
        assert_ne!(
            MemoryLayer::L3Episodic,
            MemoryLayer::L1Working,
            "A-01 修正：event 不应映射到 L1Working"
        );
        assert_ne!(
            MemoryLayer::L3Semantic,
            MemoryLayer::L1Working,
            "A-01 修正：entity 不应映射到 L1Working"
        );
    }
}
