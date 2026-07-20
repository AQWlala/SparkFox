//! SparkFox Graph — 通用图遍历引擎（A-02 P0 修复）
//!
//! ## A-02 P0 修复：sparkfox-graph 降级
//!
//! 修正前问题：sparkfox-knowledge 与 sparkfox-graph 表结构双轨冲突，
//! `knowledge_event` 与 `graph_node`/`graph_edge` 字段重叠，无单一 SoT，
//! 数据冗余 + 同步困难。
//!
//! 修正后方案：
//! 1. **sparkfox-knowledge 为唯一 SoT**（Source of Truth）
//! 2. **sparkfox-graph 不再维护独立 node/edge 表**
//! 3. 通过 [`GraphBackend`] trait 抽象图遍历操作
//! 4. [`KnowledgeGraphBackend`] 反向引用 sparkfox-knowledge 的
//!    `entity` / `knowledge_event` / `event_entity_relation` 表
//!
//! ## 模块结构
//!
//! - [`backend`]：[`GraphBackend`] trait + 图类型（`GraphNode` / `GraphEdge` / `Graph`）
//! - [`knowledge_backend`]：[`KnowledgeGraphBackend`] 实现（反向引用 SAG 3 表）
//! - [`graph`]：[`PetgraphBackend`] 实现（petgraph 内存图 + SQLite 持久化）
//! - [`traversal`]：MDRM 5 维多跳遍历（OpenAkita 清洁室重写 + R-07 LIMIT 阀门）
//! - [`extractor`]：实体抽取（Task 8.13，v1.0.0 占位）
//! - [`relation`]：关系抽取（Task 8.14，v1.0.0 占位）
//!
//! ## 字段对齐
//!
//! 所有 SQL 查询字段严格对齐 `sparkfox-knowledge/src/schema.rs` 的 SAG 6 表 DDL：
//! - `entity`: id / name / normalized_name / description / extra_data / ...
//! - `knowledge_event`: id / title / summary / content / ...
//! - `event_entity_relation`: id / event_id / entity_id / relation_type / confidence / extra_data
//!
//! 参考：SAG 论文 arXiv:2606.15971 + SAG-Benchmark (MIT License, verified 2026-07-19)。

#![forbid(unsafe_code)]

pub mod backend;
pub mod extractor;
pub mod graph;
pub mod knowledge_backend;
pub mod relation;
pub mod traversal;

pub use backend::{
    Graph, GraphBackend, GraphEdge, GraphEdgeType, GraphNode, GraphNodeType,
};
pub use extractor::{Entity, EntityExtractor};
pub use graph::PetgraphBackend;
pub use knowledge_backend::KnowledgeGraphBackend;
pub use relation::{Relation, RelationExtractor};
pub use traversal::{multi_hop_traverse, TraversalConfig, TraversalDirection};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
