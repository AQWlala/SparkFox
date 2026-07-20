//! Sub-Step 10.6.2 — F1 评估指标共享模块（REFACTOR 阶段提取）
//!
//! ## 用途
//! 提供 precision / recall / F1 计算函数，供多个 NER 评估测试文件复用：
//! - `tests/jieba_fallback_f1_test.rs`（10.6.2 jieba 降级 F1 验证）
//! - 后续 10.3.3 LLM F1 验证（若需）
//!
//! ## 使用方式
//! 在测试文件顶部声明：
//! ```ignore
//! mod common;
//! use common::metrics::{F1Metrics, compute_f1, compute_f1_per_type};
//! ```
//!
//! ## F1 计算公式
//! - precision = tp / (tp + fp)
//! - recall = tp / (tp + fn)
//! - F1 = 2 * P * R / (P + R)
//!
//! 当 tp + fp = 0 或 tp + fn = 0 时，对应指标返回 0.0（避免除零）。
//!
//! ## 跨测试文件 dead_code 说明
//! 每个 `tests/*.rs` 是独立 crate，引用 `common` 时会重新编译本模块。
//! 某个测试 crate 可能只使用本模块部分函数，未使用的函数会触发 dead_code warning。
//! 因此本模块整体 `#[allow(dead_code)]` 抑制跨 crate 的假阳性 warning。

#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::HashMap;

/// F1 评估指标（precision / recall / F1 + 计数）
///
/// 字段说明：
/// - `precision`：精确率 = tp / (tp + fp)
/// - `recall`：召回率 = tp / (tp + fn)
/// - `f1`：F1 分数 = 2 * P * R / (P + R)
/// - `tp`：True Positive（正确识别）
/// - `fp`：False Positive（误识别）
/// - `fn_`：False Negative（漏识别）
#[derive(Debug, Clone)]
pub struct F1Metrics {
    /// 精确率
    pub precision: f64,
    /// 召回率
    pub recall: f64,
    /// F1 分数
    pub f1: f64,
    /// True Positive
    pub tp: usize,
    /// False Positive
    pub fp: usize,
    /// False Negative（字段名加下划线避免 `fn` 关键字冲突）
    pub fn_: usize,
}

impl F1Metrics {
    /// 从 tp / fp / fn 计算指标
    ///
    /// 当 tp + fp = 0 或 tp + fn = 0 时，对应指标返回 0.0（避免除零）。
    pub fn from_tp_fp_fn(tp: usize, fp: usize, fn_: usize) -> Self {
        let precision = if tp + fp == 0 { 0.0 } else { tp as f64 / (tp + fp) as f64 };
        let recall = if tp + fn_ == 0 { 0.0 } else { tp as f64 / (tp + fn_) as f64 };
        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        };
        Self { precision, recall, f1, tp, fp, fn_ }
    }
}

/// 计算 F1 指标（`compute_f1` 别名，便于外部调用）
///
/// 等价于 [`F1Metrics::from_tp_fp_fn`]。
pub fn compute_f1(tp: usize, fp: usize, fn_: usize) -> F1Metrics {
    F1Metrics::from_tp_fp_fn(tp, fp, fn_)
}

/// 按实体类型计算每类 F1 指标
///
/// ## 参数
/// - `predictions`：每个 case 的预测实体列表 `Vec<(entity_type, text)>`
/// - `expectations`：每个 case 的期望实体列表 `Vec<(entity_type, text)>`
///
/// ## 返回
/// `HashMap<String, F1Metrics>`，key 为实体类型，value 为该类型的 F1 指标。
///
/// ## 匹配规则
/// `(entity_type, text)` 完全相等算 TP（简化匹配，不考虑 span 重叠）。
pub fn compute_f1_per_type(
    predictions: &[Vec<(String, String)>],
    expectations: &[Vec<(String, String)>],
) -> HashMap<String, F1Metrics> {
    let mut per_type: HashMap<String, (usize, usize, usize)> = HashMap::new();

    for (predicted, expected) in predictions.iter().zip(expectations.iter()) {
        // TP / FP
        for p in predicted {
            let entry = per_type.entry(p.0.clone()).or_insert((0, 0, 0));
            if expected.contains(p) {
                entry.0 += 1; // tp
            } else {
                entry.1 += 1; // fp
            }
        }
        // FN
        for e in expected {
            if !predicted.contains(e) {
                let entry = per_type.entry(e.0.clone()).or_insert((0, 0, 0));
                entry.2 += 1; // fn
            }
        }
    }

    per_type
        .into_iter()
        .map(|(k, (tp, fp, fn_))| (k, F1Metrics::from_tp_fp_fn(tp, fp, fn_)))
        .collect()
}

/// 计算总体 F1 指标（合并所有类型）
///
/// ## 参数
/// - `predictions`：每个 case 的预测实体列表
/// - `expectations`：每个 case 的期望实体列表
pub fn compute_f1_overall(
    predictions: &[Vec<(String, String)>],
    expectations: &[Vec<(String, String)>],
) -> F1Metrics {
    let mut tp = 0;
    let mut fp = 0;
    let mut fn_ = 0;

    for (predicted, expected) in predictions.iter().zip(expectations.iter()) {
        for p in predicted {
            if expected.contains(p) {
                tp += 1;
            } else {
                fp += 1;
            }
        }
        for e in expected {
            if !predicted.contains(e) {
                fn_ += 1;
            }
        }
    }

    F1Metrics::from_tp_fp_fn(tp, fp, fn_)
}
