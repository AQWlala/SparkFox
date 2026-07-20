//! Sub-Step 11.7.1 — 索引优化器集成测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.7.1，6 测试）
//! 验证 [`IndexOptimizer`](sparkfox_knowledge::index::index_optimizer::IndexOptimizer)
//! 作为「外部顾问」对 HnswIndex 提供参数调优建议 + 启动期预热策略：
//! 1. `test_index_optimizer_recommend_params_small_dataset`：< 1k 数据集，推荐 M=8 / ef_construction=100 / ef_search=50
//! 2. `test_index_optimizer_recommend_params_medium_dataset`：1k-10k 数据集，推荐 M=16 / ef_construction=200 / ef_search=100
//! 3. `test_index_optimizer_recommend_params_large_dataset`：10k-100k 数据集，推荐 M=32 / ef_construction=400 / ef_search=200
//! 4. `test_index_optimizer_warmup_returns_inserted_count`：预热返回插入的向量数
//! 5. `test_index_optimizer_benchmark_returns_latency_and_recall`：基准测试返回延迟和 Recall
//! 6. `test_index_optimizer_benchmark_suggestions_non_empty`：基准测试优化建议非空
//!
//! ## 设计约束
//! - **不修改** 现有 HnswIndex / BidirectionalIndex（保留 11.6.1 / 11.6.2 实现）
//! - **作为外部顾问**：IndexOptimizer 仅提供建议和预热，不侵入索引实现
//! - **参数推荐策略**：基于数据集规模的 4 级分档（< 1k / 1k-10k / 10k-100k / > 100k）
//!
//! ## 测试 fixture
//! - 128 维向量（PoC，实际生产用 768/1024 维）
//! - 10 条正交向量（mock entity_id: `ent-0` 到 `ent-9`）
//! - ground_truth：query[i] 的真值即 `ent-{i}` 自身（正交向量自检索）
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use sparkfox_knowledge::index::hnsw_index::HnswIndex;
use sparkfox_knowledge::index::index_optimizer::{BenchmarkResult, HnswParams, IndexOptimizer};

// ---------------------------------------------------------------------------
// 测试 fixture
// ---------------------------------------------------------------------------

/// 向量维度（PoC 用 128 维，实际生产用 768/1024 维）
const DIM: usize = 128;

/// 测试向量数量
const N_VECTORS: usize = 10;

/// 构造 10 条 128 维正交向量（mock entity_id: `ent-0` 到 `ent-9`）
///
/// `ent-i` → vec[i]=1.0，其余维度 0。两两正交，cosine 距离 = 1（非自身）。
fn make_orthogonal_vectors() -> Vec<(Vec<f32>, String)> {
    let mut out = Vec::with_capacity(N_VECTORS);
    for i in 0..N_VECTORS {
        let mut v = vec![0.0f32; DIM];
        v[i] = 1.0;
        out.push((v, format!("ent-{}", i)));
    }
    out
}

/// 构造查询向量列表（每个 ent-i 的向量，用于基准测试）
///
/// ground_truth[i] = `ent-{i}`（正交向量自检索，最近邻必为自身）
fn make_queries_with_ground_truth(
    vectors: &[(Vec<f32>, String)],
) -> (Vec<Vec<f32>>, Vec<Vec<String>>) {
    let mut queries = Vec::with_capacity(vectors.len());
    let mut ground_truth = Vec::with_capacity(vectors.len());
    for (v, id) in vectors {
        queries.push(v.clone());
        ground_truth.push(vec![id.clone()]);
    }
    (queries, ground_truth)
}

// ---------------------------------------------------------------------------
// 6 个测试（spec §三 11.7.1 验收指标）
// ---------------------------------------------------------------------------

/// 验收指标 1：小数据集（< 1k）参数推荐
///
/// `IndexOptimizer::new(500, 128)` 推荐：
/// - `M = 8`（低连接度，小数据集足够）
/// - `ef_construction = 100`（建图探索深度低）
/// - `ef_search = 50`（检索探索深度低）
#[test]
fn test_index_optimizer_recommend_params_small_dataset() {
    let optimizer = IndexOptimizer::new(500, DIM);
    let params: HnswParams = optimizer.recommend_params();
    assert_eq!(
        params.m, 8,
        "小数据集 (<1k) M 应为 8，实际: {}",
        params.m
    );
    assert_eq!(
        params.ef_construction, 100,
        "小数据集 (<1k) ef_construction 应为 100，实际: {}",
        params.ef_construction
    );
    assert_eq!(
        params.ef_search, 50,
        "小数据集 (<1k) ef_search 应为 50，实际: {}",
        params.ef_search
    );
    assert!(
        !params.reason.is_empty(),
        "推荐理由不应为空"
    );
}

/// 验收指标 2：中等数据集（1k-10k）参数推荐
///
/// `IndexOptimizer::new(5000, 128)` 推荐：
/// - `M = 16`（中等连接度，平衡精度和性能）
/// - `ef_construction = 200`（建图探索深度中等）
/// - `ef_search = 100`（检索探索深度中等）
#[test]
fn test_index_optimizer_recommend_params_medium_dataset() {
    let optimizer = IndexOptimizer::new(5000, DIM);
    let params: HnswParams = optimizer.recommend_params();
    assert_eq!(
        params.m, 16,
        "中等数据集 (1k-10k) M 应为 16，实际: {}",
        params.m
    );
    assert_eq!(
        params.ef_construction, 200,
        "中等数据集 (1k-10k) ef_construction 应为 200，实际: {}",
        params.ef_construction
    );
    assert_eq!(
        params.ef_search, 100,
        "中等数据集 (1k-10k) ef_search 应为 100，实际: {}",
        params.ef_search
    );
    assert!(
        !params.reason.is_empty(),
        "推荐理由不应为空"
    );
}

/// 验收指标 3：大数据集（10k-100k）参数推荐
///
/// `IndexOptimizer::new(50000, 128)` 推荐：
/// - `M = 32`（高连接度，保精度）
/// - `ef_construction = 400`（建图探索深度高）
/// - `ef_search = 200`（检索探索深度高）
#[test]
fn test_index_optimizer_recommend_params_large_dataset() {
    let optimizer = IndexOptimizer::new(50000, DIM);
    let params: HnswParams = optimizer.recommend_params();
    assert_eq!(
        params.m, 32,
        "大数据集 (10k-100k) M 应为 32，实际: {}",
        params.m
    );
    assert_eq!(
        params.ef_construction, 400,
        "大数据集 (10k-100k) ef_construction 应为 400，实际: {}",
        params.ef_construction
    );
    assert_eq!(
        params.ef_search, 200,
        "大数据集 (10k-100k) ef_search 应为 200，实际: {}",
        params.ef_search
    );
    assert!(
        !params.reason.is_empty(),
        "推荐理由不应为空"
    );
}

/// 验收指标 4：预热返回插入的向量数
///
/// 构造 10 条向量 → `warmup()` → 返回值应为 10（成功插入的向量数）。
/// 同时验证 HnswIndex 的 `len()` 也为 10（预热后索引非空）。
#[test]
fn test_index_optimizer_warmup_returns_inserted_count() {
    let vectors = make_orthogonal_vectors();
    let batch: Vec<(&[f32], &str)> = vectors
        .iter()
        .map(|(v, id)| (v.as_slice(), id.as_str()))
        .collect();

    let mut index = HnswIndex::new(100, DIM);
    let optimizer = IndexOptimizer::new(N_VECTORS, DIM);

    let inserted = optimizer
        .warmup(&mut index, &batch)
        .expect("warmup 失败");

    assert_eq!(
        inserted, N_VECTORS,
        "warmup 应返回插入的向量数 {}，实际: {}",
        N_VECTORS, inserted
    );
    assert_eq!(
        index.len(),
        N_VECTORS,
        "warmup 后 index.len() 应为 {}，实际: {}",
        N_VECTORS,
        index.len()
    );
}

/// 验收指标 5：基准测试返回延迟和 Recall
///
/// 构造 10 条正交向量 → warmup → benchmark（用 10 个查询，每个的真值是自身）：
/// - `avg_latency_us` 应 > 0（非零延迟）
/// - `recall_at_10` 应 ≈ 1.0（正交向量自检索，Recall@10 应为 1.0）
#[test]
fn test_index_optimizer_benchmark_returns_latency_and_recall() {
    let vectors = make_orthogonal_vectors();
    let batch: Vec<(&[f32], &str)> = vectors
        .iter()
        .map(|(v, id)| (v.as_slice(), id.as_str()))
        .collect();

    let mut index = HnswIndex::new(100, DIM);
    let optimizer = IndexOptimizer::new(N_VECTORS, DIM);
    optimizer.warmup(&mut index, &batch).expect("warmup 失败");

    // 构造查询与 ground_truth（每个查询的真值即自身）
    let (queries_owned, ground_truth) = make_queries_with_ground_truth(&vectors);
    let queries: Vec<&[f32]> = queries_owned.iter().map(|v| v.as_slice()).collect();

    let result: BenchmarkResult = optimizer
        .benchmark(&index, &queries, &ground_truth, 10)
        .expect("benchmark 失败");

    // 延迟应 > 0
    assert!(
        result.avg_latency_us > 0,
        "avg_latency_us 应 > 0，实际: {}",
        result.avg_latency_us
    );
    // Recall@10 应 >= 0.9（正交向量自检索，理论上应为 1.0，留 0.1 容差）
    assert!(
        result.recall_at_10 >= 0.9,
        "recall_at_10 应 >= 0.9（正交自检索预期 1.0），实际: {}",
        result.recall_at_10
    );
}

/// 验收指标 6：基准测试优化建议非空
///
/// benchmark 返回的 `suggestions` 应非空（至少 1 条建议），
/// 内容应包含可读的优化建议文本（非空字符串）。
#[test]
fn test_index_optimizer_benchmark_suggestions_non_empty() {
    let vectors = make_orthogonal_vectors();
    let batch: Vec<(&[f32], &str)> = vectors
        .iter()
        .map(|(v, id)| (v.as_slice(), id.as_str()))
        .collect();

    let mut index = HnswIndex::new(100, DIM);
    let optimizer = IndexOptimizer::new(N_VECTORS, DIM);
    optimizer.warmup(&mut index, &batch).expect("warmup 失败");

    let (queries_owned, ground_truth) = make_queries_with_ground_truth(&vectors);
    let queries: Vec<&[f32]> = queries_owned.iter().map(|v| v.as_slice()).collect();

    let result: BenchmarkResult = optimizer
        .benchmark(&index, &queries, &ground_truth, 10)
        .expect("benchmark 失败");

    assert!(
        !result.suggestions.is_empty(),
        "优化建议不应为空"
    );
    // 每条建议应非空字符串
    for (i, s) in result.suggestions.iter().enumerate() {
        assert!(
            !s.is_empty(),
            "第 {} 条优化建议应为非空字符串",
            i
        );
    }
}
