//! tests/common/metrics — 排序质量评估指标
//!
//! 提供 nDCG@k（Normalized Discounted Cumulative Gain）等排序指标计算，
//! 供 reranker 测试文件复用。
//!
//! ## nDCG 公式
//!
//! - rel_i = 1 / (ground_truth 中位置 + 1)（若不在 ground truth 中则 rel = 0）
//! - DCG@k = sum(rel_i / log2(i + 1)) for i in 1..=k（i 为 1-indexed 排名）
//! - IDCG@k = sum(1 / (j + 1) / log2(j + 2)) for j in 0..=(k-1)（理想排序）
//! - nDCG@k = DCG@k / IDCG@k
//!
//! 返回 [0, 1] 区间的 f32，1.0 表示完美排序，0.0 表示完全无关排序。

/// 计算 nDCG@10（Normalized Discounted Cumulative Gain at rank 10）
///
/// 评估 reranker 输出排序与 ground truth 的吻合度。
///
/// # 参数
///
/// - `reranked`: reranker 输出的文档索引顺序（最相关在前）
/// - `ground_truth`: 真实相关性排序（索引按相关性从高到低）
///
/// # 返回
///
/// [0, 1] 区间的 f32：
/// - 1.0：reranker 输出与 ground_truth 完全一致
/// - 0.0：reranker 输出与 ground_truth 完全相反（或 ground_truth 为空）
///
/// # 示例
///
/// ```ignore
/// use common::metrics::compute_ndcg_at_10;
///
/// // 完美排序：reranked 与 ground_truth 一致
/// let reranked = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// let ground_truth = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// let ndcg = compute_ndcg_at_10(&reranked, &ground_truth);
/// assert!((ndcg - 1.0).abs() < 1e-6, "完美排序 nDCG 应为 1.0");
/// ```
pub fn compute_ndcg_at_10(reranked: &[usize], ground_truth: &[usize]) -> f32 {
    compute_ndcg_at_k(reranked, ground_truth, 10)
}

/// 计算 nDCG@k（通用 k 值版本）
///
/// # 参数
///
/// - `reranked`: reranker 输出的文档索引顺序
/// - `ground_truth`: 真实相关性排序
/// - `k`: 截断位置（通常为 5 / 10 / 20）
///
/// # 返回
///
/// [0, 1] 区间的 f32
pub fn compute_ndcg_at_k(reranked: &[usize], ground_truth: &[usize], k: usize) -> f32 {
    let effective_k = k.min(reranked.len()).min(ground_truth.len());
    if effective_k == 0 {
        return 0.0;
    }

    // 构建 ground_truth 位置查找表：doc_index -> position in ground_truth
    let mut gt_pos: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (pos, &doc_idx) in ground_truth.iter().enumerate() {
        gt_pos.insert(doc_idx, pos);
    }

    // DCG@k = sum(rel_i / log2(rank + 1)) for rank in 1..=k
    let mut dcg = 0.0f32;
    for (i, &doc_idx) in reranked.iter().take(effective_k).enumerate() {
        let rank = i + 1; // 1-indexed
        let rel = match gt_pos.get(&doc_idx) {
            Some(&pos) => 1.0 / (pos + 1) as f32,
            None => 0.0,
        };
        dcg += rel / (rank as f32 + 1.0).log2();
    }

    // IDCG@k：理想排序下（reranked = ground_truth）的 DCG
    // 此时 position i 的 doc 在 ground_truth 中也是位置 i，rel = 1/(i+1)
    let mut idcg = 0.0f32;
    for i in 0..effective_k {
        let rank = i + 1;
        let rel = 1.0 / (i + 1) as f32;
        idcg += rel / (rank as f32 + 1.0).log2();
    }

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// 计算 nDCG@5（top-5 评估，常用于强截断场景）
#[allow(dead_code)]
pub fn compute_ndcg_at_5(reranked: &[usize], ground_truth: &[usize]) -> f32 {
    compute_ndcg_at_k(reranked, ground_truth, 5)
}

/// 计算 nDCG@20（top-20 评估，常用于召回场景）
#[allow(dead_code)]
pub fn compute_ndcg_at_20(reranked: &[usize], ground_truth: &[usize]) -> f32 {
    compute_ndcg_at_k(reranked, ground_truth, 20)
}

// ============================================================================
// 单元测试（验证 nDCG 计算正确性）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndcg_perfect_rank_is_one() {
        // 完美排序：reranked = ground_truth，nDCG 应为 1.0
        let reranked = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ground_truth = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ndcg = compute_ndcg_at_10(&reranked, &ground_truth);
        assert!(
            (ndcg - 1.0).abs() < 1e-6,
            "完美排序 nDCG 应为 1.0，实际: {ndcg}"
        );
    }

    #[test]
    fn ndcg_worst_rank_is_low() {
        // 完全反序：reranked 与 ground_truth 完全相反
        // 注意：由于 rel_i = 1/(pos+1) 不会为 0（只要 doc 在 ground_truth 中），
        // 10 元素反序的 nDCG 数学上约为 0.564（DCG 反序 / IDCG），
        // 不是 0。但应明显低于完美排序 1.0，也低于部分匹配排序。
        let reranked = vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let ground_truth = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ndcg = compute_ndcg_at_10(&reranked, &ground_truth);
        assert!(
            ndcg < 0.7,
            "完全反序 nDCG 应 < 0.7（10 元素反序数学上约 0.564），实际: {ndcg}"
        );
        assert!(
            ndcg > 0.0,
            "完全反序 nDCG 应 > 0.0（因所有 doc 仍在 ground_truth 中），实际: {ndcg}"
        );
        // 反序应明显差于完美排序
        let perfect = compute_ndcg_at_10(&ground_truth, &ground_truth);
        assert!(
            ndcg < perfect,
            "反序 nDCG ({ndcg}) 应 < 完美 nDCG ({perfect})"
        );
    }

    #[test]
    fn ndcg_partial_match() {
        // 部分匹配：reranked 前 3 个与 ground_truth 一致，后 7 个乱序
        let reranked = vec![0, 1, 2, 9, 8, 7, 6, 5, 4, 3];
        let ground_truth = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ndcg = compute_ndcg_at_10(&reranked, &ground_truth);
        assert!(
            ndcg > 0.5,
            "前 3 个匹配 nDCG 应 > 0.5，实际: {ndcg}"
        );
        assert!(
            ndcg < 1.0,
            "部分匹配 nDCG 应 < 1.0，实际: {ndcg}"
        );
    }

    #[test]
    fn ndcg_empty_inputs_returns_zero() {
        let empty: Vec<usize> = vec![];
        assert_eq!(compute_ndcg_at_10(&empty, &empty), 0.0);
        assert_eq!(compute_ndcg_at_10(&[0, 1], &empty), 0.0);
        assert_eq!(compute_ndcg_at_10(&empty, &[0, 1]), 0.0);
    }

    #[test]
    fn ndcg_at_k_consistency() {
        // 不同 k 值应给出合理结果
        let reranked = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ground_truth = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert!((compute_ndcg_at_k(&reranked, &ground_truth, 5) - 1.0).abs() < 1e-6);
        assert!((compute_ndcg_at_k(&reranked, &ground_truth, 10) - 1.0).abs() < 1e-6);
        assert!((compute_ndcg_at_k(&reranked, &ground_truth, 20) - 1.0).abs() < 1e-6);
    }
}
