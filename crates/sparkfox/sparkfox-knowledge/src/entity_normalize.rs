//! Sub-Step 10.4.1 — EntityNormalizer NFKC + 编辑距离（spec §三 10.5.1）
//!
//! ## 职责
//! 实现 [`saver::EntityNormalizer`] trait 的完整版本 [`NfkcNormalizer`]：
//! 1. **NFKC 归一化**：全角「ＡＢＣ」→ 半角「ABC」（依赖 `unicode-normalization` crate）
//! 2. **trim + 去标点**：去除首尾空白与中间标点（保留中文 / 字母 / 数字 / 空白）
//! 3. **空白压缩**：内部连续空白压缩为单个空格
//! 4. **编辑距离**：[`levenshtein_normalized`] 返回 [0, 1] 归一化距离，供调用方按阈值合并
//!
//! 注：本实现**不**应用 `to_lowercase()`，保留原始大小写（spec §三 10.5.1 测试 1
//! 要求全角「ＡＢＣ」→ 半角「ABC」而非「abc」）。如需大小写不敏感匹配，调用方可
//! 在比较前自行 lowercase。
//!
//! ## RISK-SAG-08 阈值 0.2
//! 编辑距离 > 0.2 时**不**合并两个实体，避免「北京大学」vs「北京」误合并：
//! - `levenshtein_normalized("北京大学", "北京") = 2/4 = 0.5 > 0.2` → 不合并
//! - `levenshtein_normalized("北京大学", "北京大学 ") = 0`（trim 后等同）→ 合并
//! - `levenshtein_normalized("北京", "北京市") = 1/3 ≈ 0.33 > 0.2` → 不合并
//!
//! ## 别名解析链路（10.4.2 补全）
//! 调用方通常按以下顺序解析：
//! 1. `AliasTable::resolve(raw)` 命中 → 直接返回 canonical
//! 2. 未命中 → `NfkcNormalizer::normalize()` 归一化
//! 3. 与已有 entity 的 normalized_name 比较 `levenshtein_normalized` < 0.2 → 合并
//!
//! ## 注入
//! `NfkcNormalizer` impl [`saver::EntityNormalizer`]，可通过
//! `EventSaver::with_normalizer(Arc::new(NfkcNormalizer::new()))` 替换默认的
//! [`saver::DefaultEntityNormalizer`]。

#![forbid(unsafe_code)]

pub use crate::saver::EntityNormalizer;
use unicode_normalization::UnicodeNormalization;

/// NFKC 归一化 + trim + 去标点的 EntityNormalizer 实现
///
/// 见模块级文档 [`crate::entity_normalize`]。
pub struct NfkcNormalizer;

impl NfkcNormalizer {
    /// 创建 NfkcNormalizer（无内部状态）
    pub fn new() -> Self {
        Self
    }
}

impl Default for NfkcNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityNormalizer for NfkcNormalizer {
    fn normalize(&self, _entity_type: &str, text: &str) -> String {
        // 1. NFKC 归一化（全角→半角、兼容性等价分解）
        let nfkc: String = text.nfkc().collect();
        // 2. trim 首尾空白
        let trimmed = nfkc.trim();
        // 3. 去标点（保留字母 / 数字 / 空白，过滤中英文标点）
        let cleaned: String = trimmed
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        // 4. 内部连续空白压缩为单个空格
        // 注：不应用 to_lowercase()，保留原始大小写（spec 测试 1 要求
        // 全角「ＡＢＣ」→ 半角「ABC」而非「abc」）。
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// 归一化 Levenshtein 编辑距离：`levenshtein(a, b) / max(len_a, len_b)`
///
/// 返回 `[0, 1]` 范围的归一化距离：
/// - `0.0` 表示完全相同
/// - `1.0` 表示完全不同（一个为空，另一个非空）
///
/// ## 空白预处理
/// 比较前对输入做 trim + 连续空白压缩为单个空格，确保「北京大学」vs
/// 「北京大学 」（末尾空格）距离为 0（视为同一实体）。
///
/// ## RISK-SAG-08 阈值 0.2
/// 调用方按阈值 `0.2` 合并实体：
/// - `< 0.2`：视为同一实体（如「北京大学」vs「北京大学 」末尾空格）
/// - `> 0.2`：视为不同实体（如「北京大学」vs「北京」）
pub fn levenshtein_normalized(a: &str, b: &str) -> f64 {
    // 空白预处理：trim + 压缩连续空白为单个空格
    let normalize_ws = |s: &str| -> String {
        s.trim().split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let a_norm = normalize_ws(a);
    let b_norm = normalize_ws(b);
    let a_chars: Vec<char> = a_norm.chars().collect();
    let b_chars: Vec<char> = b_norm.chars().collect();
    let max_len = a_chars.len().max(b_chars.len());
    if max_len == 0 {
        return 0.0;
    }
    let dist = levenshtein(&a_chars, &b_chars);
    dist as f64 / max_len as f64
}

/// Levenshtein 距离（字符级动态规划）
///
/// 返回将 `a` 转换为 `b` 所需的最少单字符编辑（插入 / 删除 / 替换）次数。
fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() {
        dp[i][0] = i;
    }
    for j in 0..=b.len() {
        dp[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_basic() {
        assert_eq!(levenshtein(&['a', 'b', 'c'], &['a', 'b', 'c']), 0);
        assert_eq!(levenshtein(&['a', 'b', 'c'], &['a', 'b', 'd']), 1);
        assert_eq!(levenshtein(&['北', '京'], &['北', '京', '市']), 1);
    }

    #[test]
    fn test_levenshtein_normalized_bounds() {
        assert_eq!(levenshtein_normalized("", ""), 0.0);
        assert_eq!(levenshtein_normalized("abc", "abc"), 0.0);
        // 一个为空 → 距离 = 1.0
        assert_eq!(levenshtein_normalized("abc", ""), 1.0);
    }
}
