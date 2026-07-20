//! Sub-Step 11.6.1 — 向量索引模块（spec §三 11.6.1）
//!
//! ## 双引擎策略
//! `sparkfox-knowledge` 提供独立于 `sparkfox-store` 的 HnswIndex 实现，作为
//! `sqlite-vec` 的补充向量检索引擎：
//! - **sqlite-vec**（在 `sparkfox-store` 中）：SQL 集成，适合 < 1k 向量的轻量场景
//! - **HnswIndex**（本模块）：基于 `hnsw_rs` 纯 Rust HNSW 算法，适合 >= 1k 向量的
//!   快速 kNN 检索场景
//!
//! ## 与 `sparkfox_store::vector_index::hnsw::HnswIndex` 的关系
//! `sparkfox-store` 中的 `HnswIndex` 因 `hnswlib-rs` (C++ binding) Windows 编译失败，
//! 当前为 HashMap + 暴力 cosine 占位实现。本模块的 `HnswIndex` 使用 `hnsw_rs`
//! （pure Rust，Windows 可编译）的真实 HNSW 算法，作为知识库层的独立实现。
//!
//! ## 循环依赖规避
//! `sparkfox-store` 已依赖 `sparkfox-knowledge`（用于 `ALL_SAG_DDL` 迁移），
//! 反向依赖会形成循环。因此本模块在 `sparkfox-knowledge` 内独立实现 HnswIndex，
//! 不依赖 `sparkfox-store`。集成测试中可通过 adapter 桥接两个实现。

// Sub-Step 11.6.2: BidirectionalIndex（entity ↔ event 双向 HashMap 索引，加速 multi-hop BFS）
// Sub-Step 11.7.1: IndexOptimizer（HnswIndex 参数调优 + 启动期预热，外部顾问式优化）
pub mod bidirectional_index;
pub mod hnsw_index;
pub mod index_optimizer;

pub use bidirectional_index::BidirectionalIndex;
pub use hnsw_index::HnswIndex;
pub use index_optimizer::{BenchmarkResult, HnswParams, IndexOptimizer};
