//! VectorIndex trait — 向量索引抽象（sqlite-vec + hnswlib-rs 双实现，P-02 P0 修复）
//!
//! 选择策略（按规模自动选择后端）：
//! - `< 1k` 向量：[`SqliteVecIndex`](sqlite_vec::SqliteVecIndex)（内存 HashMap + 暴力 cosine，零额外依赖）
//! - `>= 1k` 向量：[`HnswIndex`](hnsw::HnswIndex)（HNSW 算法，10 万向量 <50ms）
//!
//! spec v2.0 Task 1.5；P-02 P0 修复。

use sparkfox_core::Result;

/// 向量索引抽象
///
/// 实现者需保证线程安全（`Send + Sync`），支持并发插入/检索/删除。
pub trait VectorIndex: Send + Sync {
    /// 插入向量（id 唯一；重复插入为 upsert 语义）
    fn insert(&self, id: &str, vector: &[f32]) -> Result<()>;
    /// 检索 k 个最近邻；`filter` 可选后置过滤（按 `ref_ids` 白名单）
    fn search(&self, query: &[f32], k: usize, filter: Option<&VectorFilter>) -> Result<Vec<VectorMatch>>;
    /// 删除向量（实现可选择真删除或 tombstone）
    fn delete(&self, id: &str) -> Result<()>;
    /// 当前活跃向量数（不含已删除）
    fn len(&self) -> usize;
    /// 是否为空
    fn is_empty(&self) -> bool { self.len() == 0 }
    /// 后端名（用于 `auto_select` 验证与日志）
    fn backend_name(&self) -> &'static str;
}

/// 向量过滤条件（R-04：sqlite-vec 原生不支持，`HnswIndex` 在检索时按 `ref_ids` 白名单过滤）
///
/// `layer` 字段对应 `MemoryKind::layer()` 的 `u8` 值（0=L0 原始, 1=L1 工作记忆,
/// 2=L2 核心, 3=L3 动态, 4=L4 人格, 5=L5 元认知）。使用 `u8` 而非 `MemoryLayer` trait
/// 以避免 `sparkfox-store` → `sparkfox-memory` 的循环依赖。
#[derive(Debug, Clone)]
pub struct VectorFilter {
    pub layer: u8,
    pub ref_ids: Option<Vec<String>>,
}

/// 向量检索匹配结果
///
/// `score` 语义：cosine 相似度，范围 `[-1, 1]`，越大越相似。
/// （`HnswIndex` 内部把 hnswlib-rs 的 cosine distance 转换为相似度：`score = 1 - distance`）
#[derive(Debug, Clone)]
pub struct VectorMatch {
    pub id: String,
    pub score: f32,
}

/// 按向量规模自动选择后端
///
/// - `size < 1000` → [`SqliteVecIndex`](sqlite_vec::SqliteVecIndex)（轻量场景）
/// - `size >= 1000` → [`HnswIndex`](hnsw::HnswIndex)（主力场景，HNSW 加速）
pub fn auto_select(size: usize, dim: usize) -> Result<Box<dyn VectorIndex>> {
    if size < 1_000 {
        Ok(Box::new(sqlite_vec::SqliteVecIndex::new(dim)?))
    } else {
        Ok(Box::new(hnsw::HnswIndex::new(dim)?))
    }
}

pub mod sqlite_vec;
pub mod hnsw;

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证规模阈值：999 选 sqlite-vec，1000 选 hnswlib-rs (placeholder)
    #[test]
    fn test_auto_select_threshold() {
        let small = auto_select(999, 8).expect("auto_select(999)");
        assert_eq!(small.backend_name(), "sqlite-vec");

        let large = auto_select(1000, 8).expect("auto_select(1000)");
        assert_eq!(large.backend_name(), "hnswlib-rs (placeholder)");
    }
}
