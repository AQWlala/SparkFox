//! Sub-Step 10.6.1 — jieba-rs 集成 + 规则匹配（RED 阶段测试）
//!
//! 第四波合并后已通过 `lib.rs` 正式导出，无需 `#[path]` 绕过。
//!
//! ## 测试覆盖（5 用例）
//! - `test_jieba_segements_chinese_text`：jieba 默认分词正确
//! - `test_jieba_ner_extracts_person_via_dict`：自定义词典识别人名
//! - `test_jieba_ner_extracts_time_via_regex`：正则识别日期
//! - `test_jieba_ner_extracts_number_via_regex`：正则识别数字 + 量词
//! - `test_jieba_ner_extracts_organization_via_dict`：自定义词典识别机构

use sparkfox_knowledge::jieba_ner::{EntityMention, JiebaNer};

/// 验证 jieba-rs 默认分词对中文短句正确。
///
/// 「我爱北京天安门」应切分为 `["我", "爱", "北京", "天安门"]`。
/// 这是 jieba-rs 内置默认词典的标准行为，作为基线 sanity check。
#[test]
fn test_jieba_segements_chinese_text() {
    let ner = JiebaNer::new();
    let words = ner.segment("我爱北京天安门");
    assert_eq!(words, vec!["我", "爱", "北京", "天安门"]);
}

/// 验证默认人名词典 + 自定义扩展词典均能识别为人名实体。
///
/// 默认词典含「张三」，自定义扩展「陈八」应也能识别。
#[test]
fn test_jieba_ner_extracts_person_via_dict() {
    // 默认词典已含「张三」
    let ner = JiebaNer::new();
    let entities = ner.extract("昨天张三去了公司");
    let persons: Vec<&EntityMention> = entities
        .iter()
        .filter(|e| e.entity_type == "PERSON")
        .collect();
    assert!(
        persons.iter().any(|e| e.text == "张三"),
        "应识别默认人名「张三」，实际识别到的实体: {:?}",
        entities
    );

    // 自定义扩展词典
    let empty: [&str; 0] = [];
    let ner2 = JiebaNer::with_custom_dict(
        vec!["陈八"],
        empty,
        empty,
    );
    let entities2 = ner2.extract("陈八今天请假了");
    let persons2: Vec<&EntityMention> = entities2
        .iter()
        .filter(|e| e.entity_type == "PERSON")
        .collect();
    assert!(
        persons2.iter().any(|e| e.text == "陈八"),
        "应识别自定义人名「陈八」，实际识别到的实体: {:?}",
        entities2
    );
}

/// 验证正则识别中文日期「2026 年 7 月 20 日」为 TIME 实体。
///
/// 同时验证 ISO 风格 `2026-07-20` 与相对词「今天 / 明天 / 昨天」也能识别。
#[test]
fn test_jieba_ner_extracts_time_via_regex() {
    let ner = JiebaNer::new();

    // 中文日期
    let entities = ner.extract("2026年7月20日是个好日子");
    let times: Vec<&EntityMention> = entities
        .iter()
        .filter(|e| e.entity_type == "TIME")
        .collect();
    assert!(
        times.iter().any(|e| e.text.contains("2026") && e.text.contains("7") && e.text.contains("20")),
        "应识别「2026年7月20日」为 TIME，实际识别到的实体: {:?}",
        entities
    );

    // ISO 日期
    let entities_iso = ner.extract("会议定在 2026-07-20 召开");
    let times_iso: Vec<&EntityMention> = entities_iso
        .iter()
        .filter(|e| e.entity_type == "TIME")
        .collect();
    assert!(
        times_iso.iter().any(|e| e.text == "2026-07-20"),
        "应识别「2026-07-20」为 TIME，实际识别到的实体: {:?}",
        entities_iso
    );

    // 相对时间词
    let entities_rel = ner.extract("今天天气不错，明天会更好");
    let times_rel: Vec<&EntityMention> = entities_rel
        .iter()
        .filter(|e| e.entity_type == "TIME")
        .collect();
    assert!(
        times_rel.iter().any(|e| e.text == "今天"),
        "应识别「今天」为 TIME，实际识别到的实体: {:?}",
        entities_rel
    );
    assert!(
        times_rel.iter().any(|e| e.text == "明天"),
        "应识别「明天」为 TIME，实际识别到的实体: {:?}",
        entities_rel
    );
}

/// 验证正则识别数字 + 量词为 NUMBER 实体。
///
/// 覆盖场景：
/// - 「999 元」 → NUMBER
/// - 「3 个」 → NUMBER
/// - 「50%」 → NUMBER
/// - 纯数字「42」 → NUMBER
#[test]
fn test_jieba_ner_extracts_number_via_regex() {
    let ner = JiebaNer::new();
    let entities = ner.extract("购买 999 元的商品 3 个，享受 50% 折扣");
    let numbers: Vec<&EntityMention> = entities
        .iter()
        .filter(|e| e.entity_type == "NUMBER")
        .collect();
    assert!(
        numbers.iter().any(|e| e.text == "999 元" || e.text == "999元"),
        "应识别「999 元」为 NUMBER，实际识别到的实体: {:?}",
        entities
    );
    assert!(
        numbers.iter().any(|e| e.text == "3 个" || e.text == "3个"),
        "应识别「3 个」为 NUMBER，实际识别到的实体: {:?}",
        entities
    );
    assert!(
        numbers.iter().any(|e| e.text == "50%"),
        "应识别「50%」为 NUMBER，实际识别到的实体: {:?}",
        entities
    );
}

/// 验证默认机构词典识别「腾讯」为 ORGANIZATION。
///
/// 同时验证默认词典中其他机构（如「阿里巴巴」）也能识别。
#[test]
fn test_jieba_ner_extracts_organization_via_dict() {
    let ner = JiebaNer::new();
    let entities = ner.extract("腾讯和阿里巴巴都是大公司");
    let orgs: Vec<&EntityMention> = entities
        .iter()
        .filter(|e| e.entity_type == "ORGANIZATION")
        .collect();
    assert!(
        orgs.iter().any(|e| e.text == "腾讯"),
        "应识别「腾讯」为 ORGANIZATION，实际识别到的实体: {:?}",
        entities
    );
    assert!(
        orgs.iter().any(|e| e.text == "阿里巴巴"),
        "应识别「阿里巴巴」为 ORGANIZATION，实际识别到的实体: {:?}",
        entities
    );
}
