//! Sub-Step 12.3.2 REFACTOR — Benchmark 指标共享模块
//!
//! ## 用途
//! 提供 Recall@k / Precision@k / percentile 计算，供 `tests/bench_compare_4_strategies.rs`
//! 及后续 benchmark 测试文件复用，避免在每个测试文件中重写指标计算逻辑。
//!
//! ## 使用方式
//! 在测试文件顶部声明：
//! ```ignore
//! mod common;
//! use common::bench_metrics::{compute_recall_at_k, compute_precision_at_k, percentile};
//! ```
//!
//! ## 指标定义
//! - **Recall@k（命中式）**：top-k 含 ground_truth 任一 event_id → 该 case 记为命中。
//!   Recall = 命中 case 数 / 总 case 数。返回 `bool` 便于逐 case 累加。
//! - **Precision@k**：top-k 中命中 ground_truth 的 event_id 数量（绝对值）。
//!   返回 `usize`，调用方按需除以 (k × case 数) 得平均 Precision。
//! - **percentile**：升序排序后取 p 百分位（p∈[0,1]），index = floor(p × n)。
//!   边界保护：n=0 时返回 0；index 越界时取最后一个元素。
//!
//! ## 跨测试文件 dead_code 说明
//! 每个 `tests/*.rs` 是独立 crate，引用 `common` 时会重新编译本模块。
//! 某个测试 crate 可能只使用本模块部分函数，未使用的函数会触发 dead_code warning。
//! 因此本模块整体 `#[allow(dead_code)]` 抑制跨 crate 的假阳性 warning。

#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::HashSet;

/// 计算 Recall@k（命中式）
///
/// ## 定义
/// top-k 检索结果中，若命中 `ground_truth` 中任一 event_id，则该 case 记为命中。
/// Recall = 命中 case 数 / 总 case 数（在调用方累加）。
///
/// ## 参数
/// - `top_k_hits`: 检索结果 top-k 的 event_id 列表（调用方应已 take(k)）
/// - `ground_truth`: 期望 event_id 列表
///
/// ## 返回
/// `true`：top-k 含 ground_truth 中任一 event_id（该 case 命中）
/// `false`：top-k 与 ground_truth 无交集（该 case 未命中）
pub fn compute_recall_at_k(top_k_hits: &[String], ground_truth: &[String]) -> bool {
    if ground_truth.is_empty() {
        return false;
    }
    let top_set: HashSet<&str> = top_k_hits.iter().map(|s| s.as_str()).collect();
    let gt_set: HashSet<&str> = ground_truth.iter().map(|s| s.as_str()).collect();
    !top_set.is_disjoint(&gt_set)
}

/// 计算 Precision@k（命中数，非比例）
///
/// ## 定义
/// top-k 检索结果中，命中 `ground_truth` 的 event_id 数量（绝对值）。
/// 调用方按需除以 k 得到 Precision 比例，或除以 (k × case 数) 得平均 Precision。
///
/// ## 参数
/// - `top_k_hits`: 检索结果 top-k 的 event_id 列表
/// - `ground_truth`: 期望 event_id 列表
///
/// ## 返回
/// `usize`: top-k 中命中 ground_truth 的 event_id 数量
pub fn compute_precision_at_k(top_k_hits: &[String], ground_truth: &[String]) -> usize {
    let top_set: HashSet<&str> = top_k_hits.iter().map(|s| s.as_str()).collect();
    let gt_set: HashSet<&str> = ground_truth.iter().map(|s| s.as_str()).collect();
    top_set.intersection(&gt_set).count()
}

/// 计算百分位数
///
/// ## 定义
/// 升序排序后取 p 百分位（p ∈ [0, 1]），index = floor(p × n)。
///
/// ## 边界保护
/// - `samples` 为空：返回 0
/// - `index >= n`：取最后一个元素（避免越界）
///
/// ## 参数
/// - `samples`: 样本切片（无需预先排序，函数内部 clone + sort）
/// - `p`: 百分位（如 0.99 表示 p99）
///
/// ## 返回
/// 样本中第 p 百分位的值（u64 类型，与 latency_ms 对齐）
///
/// ## 示例
/// ```ignore
/// // 50 case 的 p99 latency
/// let latencies: Vec<u64> = vec![10, 20, 30, 40, 50];
/// let p99 = common::bench_metrics::percentile(&latencies, 0.99);
/// // p99 = latencies[floor(0.99 * 5)] = latencies[4] = 50
/// ```
pub fn percentile(samples: &[u64], p: f64) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut sorted: Vec<u64> = samples.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let idx = ((p * n as f64) as usize).min(n - 1);
    sorted[idx]
}
