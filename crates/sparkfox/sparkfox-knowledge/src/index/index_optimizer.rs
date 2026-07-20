//! Sub-Step 11.7.1 — 索引优化器（spec §三 11.7.1）
//!
//! ## 设计动机
//! Task 11.7 ATOMIC 性能优化的最后一项：在 11.6.1 HnswIndex + 11.6.2 BidirectionalIndex
//! 之上提供「外部顾问」式的索引优化能力。**不修改**现有 HnswIndex / BidirectionalIndex
//! 实现，避免回归 11.6.x 已验证的 8.7x 加速和 683ms 端到端延迟。
//!
//! ## 核心功能
//! 1. **参数调优建议**：根据数据集规模推荐 HnswIndex 的 `M` / `ef_construction` / `ef_search`
//! 2. **索引预热**：启动期批量插入 + kNN 探测，提前建立 HNSW 图节点连接
//! 3. **性能基准**：测量 kNN 查询延迟 + Recall@10，给出优化建议
//!
//! ## 4 级参数推荐策略（基于数据集规模）
//! | 数据集规模     | M  | ef_construction | ef_search | 适用场景                 |
//! |----------------|----|------------------|-----------|--------------------------|
//! | < 1k           | 8  | 100              | 50        | 小数据集，低连接度足够   |
//! | 1k-10k         | 16 | 200              | 100       | 中等数据集，平衡精度性能 |
//! | 10k-100k       | 32 | 400              | 200       | 大数据集，高连接度保精度 |
//! | > 100k         | 48 | 500              | 300       | 超大数据集，最高连接度   |
//!
//! ## 与 11.6.1 HnswIndex 的关系
//! - HnswIndex（11.6.1）：使用固定参数 `M=16 / ef_construction=200` 建图
//! - IndexOptimizer（本模块）：**作为外部顾问**，根据数据集规模推荐参数
//! - 实际生产中可先调用 `recommend_params()` 获取建议，再决定是否重建索引
//!
//! ## 不修改现有索引的设计约束
//! - 不修改 `hnsw_index.rs`（保留 11.6.1 实现）
//! - 不修改 `bidirectional_index.rs`（保留 11.6.2 实现）
//! - 不修改 `search/` 目录
//! - IndexOptimizer 通过 `&mut HnswIndex` 调用现有 API（insert_batch / search）
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use std::time::Instant;

use sparkfox_core::{Error, Result};

use crate::index::hnsw_index::HnswIndex;

/// 数据集规模分档阈值
///
/// 与 spec §三 11.7.1 参数推荐策略表对齐：
/// - `< SMALL_THRESHOLD`（1k）：小数据集
/// - `[SMALL_THRESHOLD, MEDIUM_THRESHOLD)`（1k-10k）：中等数据集
/// - `[MEDIUM_THRESHOLD, LARGE_THRESHOLD)`（10k-100k）：大数据集
/// - `>= LARGE_THRESHOLD`（100k）：超大数据集
const SMALL_THRESHOLD: usize = 1_000;
const MEDIUM_THRESHOLD: usize = 10_000;
const LARGE_THRESHOLD: usize = 100_000;

/// Recall 建议阈值（spec §三 11.7.1 性能基准优化建议）
///
/// - `recall >= RECALL_GOOD`：参数适配良好，无需调整
/// - `RECALL_ACCEPTABLE <= recall < RECALL_GOOD`：可提升 ef_search
/// - `recall < RECALL_ACCEPTABLE`：应增大 M / ef_construction 重建索引
const RECALL_GOOD: f32 = 0.95;
const RECALL_ACCEPTABLE: f32 = 0.80;

/// 延迟建议阈值（微秒，spec §三 11.7.1 性能基准优化建议）
///
/// 单次查询平均延迟超过此阈值时建议降低 ef_search。
/// 10ms = 10000μs（适用于交互式 RAG 场景的延迟上限）
const LATENCY_HIGH_US: u64 = 10_000;

/// 索引优化器（spec §三 11.7.1）
///
/// 提供 HnswIndex 参数调优建议 + 启动期预热策略。
/// 不修改现有索引实现，作为「外部顾问」提供优化建议。
///
/// ## 功能
/// 1. 根据数据集规模推荐 HnswIndex 参数（M / ef_construction / ef_search）
/// 2. 索引预热：启动期批量插入 + kNN 探测，建立 HNSW 图
/// 3. 性能基准：测量索引查询延迟，给出优化建议
///
/// ## 示例
/// ```ignore
/// use sparkfox_knowledge::index::hnsw_index::HnswIndex;
/// use sparkfox_knowledge::index::index_optimizer::IndexOptimizer;
///
/// let mut index = HnswIndex::new(5000, 384);
/// let optimizer = IndexOptimizer::new(5000, 384);
///
/// // 1. 获取参数推荐
/// let params = optimizer.recommend_params();
/// println!("推荐参数: M={} ef_c={} ef_s={}", params.m, params.ef_construction, params.ef_search);
///
/// // 2. 预热索引
/// let vectors: Vec<(&[f32], &str)> = vec![(&[0.1; 384], "ent-1"), (&[0.2; 384], "ent-2")];
/// let inserted = optimizer.warmup(&mut index, &vectors)?;
///
/// // 3. 性能基准
/// // let result = optimizer.benchmark(&index, &queries, &ground_truth, 10)?;
/// ```
pub struct IndexOptimizer {
    /// 数据集规模（向量数）
    dataset_size: usize,
    /// 向量维度
    dim: usize,
}

/// HnswIndex 参数推荐（spec §三 11.7.1）
///
/// 由 [`IndexOptimizer::recommend_params`] 返回，包含 HnswIndex 建议参数及推荐理由。
#[derive(Debug, Clone)]
pub struct HnswParams {
    /// M 参数（图连接度，hnsw_rs 中对应 `max_nb_connection`）
    ///
    /// 值越大图越稠密，精度越高但内存和建图时间增加。
    pub m: usize,
    /// ef_construction 参数（建图探索深度）
    ///
    /// 值越大建图质量越好，但建图时间增加。
    pub ef_construction: usize,
    /// ef_search 参数（检索探索深度）
    ///
    /// 值越大检索精度越高，但查询延迟增加。
    pub ef_search: usize,
    /// 推荐理由（人类可读的中文说明）
    pub reason: String,
}

/// 性能基准结果（spec §三 11.7.1）
///
/// 由 [`IndexOptimizer::benchmark`] 返回，包含 kNN 查询延迟、Recall@10 及优化建议。
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// 单次查询平均延迟（微秒）
    pub avg_latency_us: u64,
    /// kNN 检索 Recall@10
    ///
    /// 计算方式：对每个查询，检查 top-10 结果中是否包含 ground_truth 中的任意一项，
    /// 命中率即为 Recall@10。
    pub recall_at_10: f32,
    /// 优化建议（基于 Recall 和延迟给出，至少 1 条）
    pub suggestions: Vec<String>,
}

impl IndexOptimizer {
    /// 新建优化器
    ///
    /// ## 参数
    /// - `dataset_size`：预期数据集规模（向量数）
    /// - `dim`：向量维度（如 384 / 768 / 1024）
    ///
    /// ## 返回
    /// 配置好数据集规模和维度的优化器实例
    pub fn new(dataset_size: usize, dim: usize) -> Self {
        Self { dataset_size, dim }
    }

    /// 推荐 HnswIndex 参数（基于数据集规模 + 向量维度）
    ///
    /// 根据 [`Self::dataset_size`] 落入的分档返回对应的推荐参数：
    ///
    /// | 数据集规模 | M  | ef_construction | ef_search |
    /// |------------|----|------------------|-----------|
    /// | < 1k       | 8  | 100              | 50        |
    /// | 1k-10k     | 16 | 200              | 100       |
    /// | 10k-100k   | 32 | 400              | 200       |
    /// | > 100k     | 48 | 500              | 300       |
    ///
    /// 同时基于 [`Self::dim`] 在 `reason` 中给出维度相关的补充建议
    /// （高维 >= 768 时建议配合 PQ/SQ 量化以压缩内存）。
    ///
    /// ## 返回
    /// [`HnswParams`] 结构体，包含 M / ef_construction / ef_search / reason
    pub fn recommend_params(&self) -> HnswParams {
        // 高维向量判断（>= 768 维，如 bge-large 1024 维）
        let is_high_dim = self.dim >= 768;

        if self.dataset_size < SMALL_THRESHOLD {
            // 小数据集（< 1k）：低连接度足够，优先性能
            HnswParams {
                m: 8,
                ef_construction: 100,
                ef_search: 50,
                reason: format!(
                    "小数据集（{} < {}）：低连接度 M=8 足够覆盖图连通性，\
                     ef_construction=100 / ef_search=50 兼顾建图速度与检索精度{}",
                    self.dataset_size,
                    SMALL_THRESHOLD,
                    if is_high_dim {
                        format!("（注意：dim={} >= 768 为高维向量，建议配合 PQ/SQ 量化压缩内存）", self.dim)
                    } else {
                        String::new()
                    }
                ),
            }
        } else if self.dataset_size < MEDIUM_THRESHOLD {
            // 中等数据集（1k-10k）：平衡精度与性能
            HnswParams {
                m: 16,
                ef_construction: 200,
                ef_search: 100,
                reason: format!(
                    "中等数据集（{} <= {} < {}）：M=16 平衡图稠密度与内存，\
                     ef_construction=200 / ef_search=100 为精度/性能折衷点{}",
                    SMALL_THRESHOLD,
                    self.dataset_size,
                    MEDIUM_THRESHOLD,
                    if is_high_dim {
                        format!("（注意：dim={} >= 768 为高维向量，建议配合 PQ/SQ 量化压缩内存）", self.dim)
                    } else {
                        String::new()
                    }
                ),
            }
        } else if self.dataset_size < LARGE_THRESHOLD {
            // 大数据集（10k-100k）：高连接度保精度
            HnswParams {
                m: 32,
                ef_construction: 400,
                ef_search: 200,
                reason: format!(
                    "大数据集（{} <= {} < {}）：M=32 提高图稠密度防精度退化，\
                     ef_construction=400 / ef_search=200 保证 Recall@10 >= 0.95{}",
                    MEDIUM_THRESHOLD,
                    self.dataset_size,
                    LARGE_THRESHOLD,
                    if is_high_dim {
                        format!("（注意：dim={} >= 768 为高维向量，建议配合 PQ/SQ 量化压缩内存）", self.dim)
                    } else {
                        String::new()
                    }
                ),
            }
        } else {
            // 超大数据集（>= 100k）：最高连接度
            HnswParams {
                m: 48,
                ef_construction: 500,
                ef_search: 300,
                reason: format!(
                    "超大数据集（{} >= {}）：M=48 维持大规模图连通性，\
                     ef_construction=500 / ef_search=300 抵消高维诅咒，\
                     建议配合量化（PQ/SQ）压缩内存（dim={}）",
                    self.dataset_size,
                    LARGE_THRESHOLD,
                    self.dim
                ),
            }
        }
    }

    /// 预热索引（批量插入 + kNN 探测）
    ///
    /// 启动期优化策略（spec §三 11.7.1）：
    /// 1. **批量插入**：调用 `HnswIndex::insert_batch` 一次性插入所有向量
    /// 2. **kNN 探测**：取首批向量做 kNN 查询，提前触发 HNSW 图节点连接建立
    ///
    /// ## 参数
    /// - `index`：可变引用的 HnswIndex（预热后向量已插入）
    /// - `vectors`：`&[(向量切片, entity_id)]` 列表
    ///
    /// ## 返回
    /// 成功插入的向量数（等于 `vectors.len()`）
    ///
    /// ## 错误
    /// - 任一向量维度不匹配：返回 `InvalidArgument` 错误（HnswIndex::insert_batch 透传）
    pub fn warmup(&self, index: &mut HnswIndex, vectors: &[(&[f32], &str)]) -> Result<usize> {
        if vectors.is_empty() {
            // 空列表：直接返回 0，无需操作
            return Ok(0);
        }

        // 步骤 1：批量插入（HnswIndex::insert_batch 内部循环调用 insert）
        index.insert_batch(vectors)?;

        let inserted = vectors.len();

        // 步骤 2：kNN 探测（取首条向量做一次 search，触发 HNSW 图节点连接建立）
        //
        // HNSW 算法在 insert 时已建立图结构，但 search 路径的"热点"缓存
        // （CPU cache 局部性）需要首次访问触发。预热可让后续用户查询跳过
        // cold-start 延迟。k 取 min(10, inserted) 避免空索引或单元素索引的边界问题。
        let probe_k = std::cmp::min(10, inserted);
        let (probe_vec, _) = vectors[0];
        // 探测查询的 Result 可忽略：失败不影响主流程（仅是 cache 预热）
        let _ = index.search(probe_vec, probe_k);

        Ok(inserted)
    }

    /// 性能基准（测量 kNN 查询延迟 + Recall）
    ///
    /// 对每个查询向量执行 kNN 检索，测量平均延迟和 Recall@k，并基于结果给出优化建议。
    ///
    /// ## 参数
    /// - `index`：已预热的 HnswIndex（不可变引用）
    /// - `queries`：查询向量列表 `&[&[f32]]`
    /// - `ground_truth`：每个查询的真值 ID 列表（与 queries 等长）
    /// - `k`：top-k 检索深度（用于 Recall@k 计算，常取 10）
    ///
    /// ## 返回
    /// [`BenchmarkResult`]，包含 `avg_latency_us` / `recall_at_10` / `suggestions`
    ///
    /// ## 错误
    /// - `queries` 与 `ground_truth` 长度不一致：返回 `InvalidArgument` 错误
    /// - `queries` 为空：返回 `InvalidArgument` 错误（无法计算平均延迟）
    /// - 任一查询维度不匹配：透传 HnswIndex::search 的 `InvalidArgument` 错误
    pub fn benchmark(
        &self,
        index: &HnswIndex,
        queries: &[&[f32]],
        ground_truth: &[Vec<String>],
        k: usize,
    ) -> Result<BenchmarkResult> {
        // 参数校验
        if queries.is_empty() {
            return Err(Error::invalid_argument(
                "queries 不能为空（无法计算平均延迟）".to_string(),
                "IndexOptimizer::benchmark",
            ));
        }
        if queries.len() != ground_truth.len() {
            return Err(Error::invalid_argument(
                format!(
                    "queries 长度 ({}) 与 ground_truth 长度 ({}) 不一致",
                    queries.len(),
                    ground_truth.len()
                ),
                "IndexOptimizer::benchmark",
            ));
        }

        // 性能基准测试
        let mut total_latency_us: u64 = 0;
        let mut hit_count: usize = 0;

        for (i, query) in queries.iter().enumerate() {
            let start = Instant::now();
            let results = index.search(query, k)?;
            let elapsed_us = start.elapsed().as_micros() as u64;
            total_latency_us += elapsed_us;

            // Recall 计算：top-k 结果中是否包含 ground_truth[i] 的任意一项
            let truth_set = &ground_truth[i];
            let hit = results.iter().any(|(id, _)| truth_set.contains(id));
            if hit {
                hit_count += 1;
            }
        }

        let n = queries.len() as u64;
        let avg_latency_us = total_latency_us / n;
        let recall_at_10 = hit_count as f32 / queries.len() as f32;

        // 生成优化建议
        let suggestions = self.generate_suggestions(avg_latency_us, recall_at_10);

        Ok(BenchmarkResult {
            avg_latency_us,
            recall_at_10,
            suggestions,
        })
    }

    /// 根据延迟和 Recall 生成优化建议（内部辅助方法）
    ///
    /// 建议规则（spec §三 11.7.1）：
    /// 1. Recall >= 0.95：参数适配良好
    /// 2. 0.80 <= Recall < 0.95：建议提升 ef_search
    /// 3. Recall < 0.80：建议增大 M / ef_construction 重建索引
    /// 4. 延迟 > 10ms：建议降低 ef_search 或量化向量
    /// 5. 延迟 <= 1ms 且 Recall >= 0.95：可尝试降低 M 减少内存
    fn generate_suggestions(&self, avg_latency_us: u64, recall_at_10: f32) -> Vec<String> {
        let mut suggestions: Vec<String> = Vec::new();

        // Recall 维度建议
        if recall_at_10 >= RECALL_GOOD {
            suggestions.push(format!(
                "Recall@10 = {:.3} >= {:.2}，参数适配良好，无需调整 ef_search/M",
                recall_at_10, RECALL_GOOD
            ));
        } else if recall_at_10 >= RECALL_ACCEPTABLE {
            suggestions.push(format!(
                "Recall@10 = {:.3} 处于 [{:.2}, {:.2}) 区间，建议提升 ef_search 至 {} 以提高精度",
                recall_at_10,
                RECALL_ACCEPTABLE,
                RECALL_GOOD,
                self.recommend_params().ef_search
            ));
        } else {
            suggestions.push(format!(
                "Recall@10 = {:.3} < {:.2}，建议增大 M 至 {} 和 ef_construction 至 {} 并重建索引",
                recall_at_10,
                RECALL_ACCEPTABLE,
                self.recommend_params().m,
                self.recommend_params().ef_construction
            ));
        }

        // 延迟维度建议
        if avg_latency_us > LATENCY_HIGH_US {
            suggestions.push(format!(
                "平均延迟 {}μs > {}μs（10ms），建议降低 ef_search 或采用 PQ/SQ 量化压缩向量维度",
                avg_latency_us, LATENCY_HIGH_US
            ));
        } else if avg_latency_us <= 1_000 && recall_at_10 >= RECALL_GOOD {
            // 延迟低 + Recall 高：可尝试降低 M 减少内存
            suggestions.push(format!(
                "平均延迟 {}μs <= 1ms 且 Recall 良好，可尝试降低 M 减少内存占用",
                avg_latency_us
            ));
        }

        suggestions
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommend_params_small_dataset() {
        let optimizer = IndexOptimizer::new(500, 128);
        let params = optimizer.recommend_params();
        assert_eq!(params.m, 8);
        assert_eq!(params.ef_construction, 100);
        assert_eq!(params.ef_search, 50);
    }

    #[test]
    fn test_recommend_params_large_dataset_threshold() {
        // 100k 边界：仍属于大数据集（10k-100k）
        let optimizer = IndexOptimizer::new(99_999, 128);
        let params = optimizer.recommend_params();
        assert_eq!(params.m, 32);
    }

    #[test]
    fn test_recommend_params_xlarge_dataset() {
        let optimizer = IndexOptimizer::new(200_000, 128);
        let params = optimizer.recommend_params();
        assert_eq!(params.m, 48);
        assert_eq!(params.ef_construction, 500);
        assert_eq!(params.ef_search, 300);
    }

    #[test]
    fn test_generate_suggestions_high_recall() {
        let optimizer = IndexOptimizer::new(5000, 128);
        let suggestions = optimizer.generate_suggestions(500, 0.98);
        assert!(!suggestions.is_empty());
        // Recall 高时应包含「无需调整」相关建议
        assert!(suggestions[0].contains("适配良好"));
    }

    #[test]
    fn test_generate_suggestions_low_recall() {
        let optimizer = IndexOptimizer::new(5000, 128);
        let suggestions = optimizer.generate_suggestions(500, 0.50);
        assert!(!suggestions.is_empty());
        // Recall 低时应包含「重建索引」相关建议
        assert!(suggestions[0].contains("重建索引"));
    }

    #[test]
    fn test_generate_suggestions_high_latency() {
        let optimizer = IndexOptimizer::new(5000, 128);
        let suggestions = optimizer.generate_suggestions(20_000, 0.98);
        // 高延迟时应包含降低 ef_search 或量化建议
        assert!(suggestions.iter().any(|s| s.contains("量化") || s.contains("降低 ef_search")));
    }
}
