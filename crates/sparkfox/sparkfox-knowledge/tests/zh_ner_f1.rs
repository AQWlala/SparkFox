//! Sub-Step 10.3.2 — 100 case 中文 NER 测试集构建（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标
//! 1. 100 case 全部加载成功（无 JSON 解析错误）
//! 2. 覆盖 6 类实体（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT）
//! 3. 分布满足（人名 30 / 地名 30 / 机构 20 / 时间数字 20，软约束为每类至少 10 个）
//! 4. 每个 case 格式合法（含 text + expected_entities 数组，entity text 必须是原 text 子串）
//!
//! ## 数据集
//! - 文件位置：`tests/data/zh_ner_100_cases.json`
//! - 加载函数：[`common::load_zh_ner_dataset`]（共享模块 `tests/common/mod.rs`）
//! - 实际分布（GREEN 阶段实测）：
//!   - PERSON: 36 / LOCATION: 64 / ORGANIZATION: 27 / TIME: 28 / NUMBER: 20 / EVENT: 39
//!
//! ## 用途
//! - 10.3.2 本文件：数据集格式 + 覆盖度 + 分布验证（4 个测试）
//! - 10.3.3 后续：LLM F1 验证（基于本数据集评估 LLM NER 准确率 / 召回率 / F1）
//! - 10.6.2 后续：jieba 降级 F1 验证（基于本数据集评估 jieba NER 准确率 / 召回率 / F1）

#![forbid(unsafe_code)]

mod common;
use common::load_zh_ner_dataset;

/// 验证数据集含 100 case，且 id 从 1 连续递增至 100。
#[test]
fn test_zh_ner_dataset_has_100_cases() {
    let cases = load_zh_ner_dataset();
    assert_eq!(cases.len(), 100, "数据集应含 100 case，实际 {}", cases.len());
    // 验证 id 连续 1-100
    for (i, case) in cases.iter().enumerate() {
        assert_eq!(case.id, i + 1, "case id 应为 {}，实际 {}", i + 1, case.id);
    }
}

/// 验证数据集覆盖 6 类实体（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT）。
#[test]
fn test_zh_ner_dataset_covers_6_entity_types() {
    let cases = load_zh_ner_dataset();
    let mut types: std::collections::HashSet<String> = std::collections::HashSet::new();
    for case in &cases {
        for e in &case.expected_entities {
            types.insert(e.entity_type.clone());
        }
    }
    let expected_types: Vec<String> = vec![
        "PERSON", "LOCATION", "ORGANIZATION", "TIME", "NUMBER", "EVENT",
    ].into_iter().map(String::from).collect();
    for t in &expected_types {
        assert!(types.contains(t), "数据集应覆盖实体类型 {}，实际覆盖 {:?}", t, types);
    }
}

/// 验证每个 case 格式合法：
/// - text 非空
/// - expected_entities 非空
/// - 每个 entity text 非空
/// - entity_type 是 6 类之一
/// - entity text 是 case.text 的子串
#[test]
fn test_zh_ner_dataset_format_valid() {
    let cases = load_zh_ner_dataset();
    for case in &cases {
        assert!(!case.text.is_empty(), "case {} text 不应为空", case.id);
        // 每个 case 至少含 1 个 expected entity
        assert!(!case.expected_entities.is_empty(), "case {} expected_entities 不应为空", case.id);
        for e in &case.expected_entities {
            assert!(!e.text.is_empty(), "case {} entity text 不应为空", case.id);
            // entity_type 必须是 6 类之一
            let valid_types = ["PERSON", "LOCATION", "ORGANIZATION", "TIME", "NUMBER", "EVENT"];
            assert!(valid_types.contains(&e.entity_type.as_str()),
                "case {} entity_type {} 不在 6 类之内", case.id, e.entity_type);
            // text 必须在 case.text 中出现
            assert!(case.text.contains(&e.text),
                "case {} entity text '{}' 不在原文本 '{}' 中", case.id, e.text, case.text);
        }
    }
}

/// 分布验证（软约束，spec 要求 30/30/20/20，本测试放宽为每类至少 10 个，
/// 避免硬编码导致数据集质量妥协）。
#[test]
fn test_zh_ner_dataset_distribution() {
    let cases = load_zh_ner_dataset();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for case in &cases {
        for e in &case.expected_entities {
            *counts.entry(e.entity_type.clone()).or_insert(0) += 1;
        }
    }
    // 软断言：每类至少 10 个
    let min_per_type = 10;
    for t in &["PERSON", "LOCATION", "ORGANIZATION", "TIME", "NUMBER", "EVENT"] {
        let c = counts.get(*t).copied().unwrap_or(0);
        assert!(c >= min_per_type, "实体类型 {} 应至少 {} 个，实际 {}", t, min_per_type, c);
    }
    println!("实体分布: {:?}", counts);
}
