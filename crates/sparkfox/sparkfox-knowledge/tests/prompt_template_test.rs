//! Sub-Step 10.3.1 — 7 段式 prompt 模板测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标
//! 1. NER prompt 含 7 段（角色 / 任务 / 输入格式 / 输出格式 / 中文适配 / few-shot / 约束）
//! 2. NER prompt 含 10 个中文 few-shot 示例
//! 3. few-shot 覆盖 6 类实体（人名 / 地名 / 机构 / 时间 / 数字 / 事件）
//! 4. `render(&context)` 替换 `{chunk}` 占位符
//! 5. Extract prompt 同样含 7 段
//!
//! ## TDD-RED 说明
//! 本测试在 GREEN 实现前应全部失败（编译失败或断言失败）。
//! 第四波合并后已通过 `lib.rs` 正式导出，无需 `#[path]` 绕过。
//!
//! ## D2.15 决策
//! 7 段式 prompt 结构为本项目中文 NER/事件提取的统一模板规范。

#![forbid(unsafe_code)]

use sparkfox_knowledge::prompt::{ExtractPrompt, NerPrompt, PromptContext, PromptTemplate};

/// 7 段标题列表（顺序与 D2.15 决策一致）
const SEVEN_SECTIONS: [&str; 7] = [
    "角色",
    "任务",
    "输入格式",
    "输出格式",
    "中文适配",
    "few-shot",
    "约束",
];

/// 测试 1：NER prompt 含 7 段（按标题分割）
///
/// 步骤：
///   - 构造 PromptContext（chunk 为示例文本）
///   - 调用 `NerPrompt::render(&context)` 得到完整 prompt
///   - 按 7 个标题逐一断言存在
#[test]
fn test_ner_prompt_has_7_sections() {
    let context = PromptContext {
        chunk: "张三在北京大学发表演讲".to_string(),
        entity_types: vec![
            "PERSON".to_string(),
            "LOCATION".to_string(),
            "ORGANIZATION".to_string(),
            "TIME".to_string(),
            "NUMBER".to_string(),
            "EVENT".to_string(),
        ],
    };
    let ner = NerPrompt::new();
    let rendered = ner.render(&context);

    for section in SEVEN_SECTIONS.iter() {
        assert!(
            rendered.contains(section),
            "NER prompt 缺少段：「{}」\n完整 prompt:\n{}",
            section,
            rendered
        );
    }
}

/// 测试 2：NER prompt 含 10 个中文 few-shot 示例
///
/// 步骤：
///   - 渲染 NER prompt
///   - 统计 few-shot 段中示例数量（通过分隔标记或编号 1.~10.）
/// 期望：>= 10 个
#[test]
fn test_ner_prompt_includes_10_few_shot_cases() {
    let context = PromptContext {
        chunk: "测试文本".to_string(),
        entity_types: vec!["PERSON".to_string()],
    };
    let ner = NerPrompt::new();
    let rendered = ner.render(&context);

    // few-shot 段以「示例 1.」「示例 2.」… 形式编号
    let mut count = 0;
    for i in 1..=20 {
        let marker = format!("示例 {}.", i);
        if rendered.contains(&marker) {
            count += 1;
        }
    }
    assert!(
        count >= 10,
        "NER prompt 应含 >= 10 个 few-shot 示例，实际 = {}\n完整 prompt:\n{}",
        count,
        rendered
    );
}

/// 测试 3：few-shot 覆盖 6 类实体（人名 / 地名 / 机构 / 时间 / 数字 / 事件）
///
/// 步骤：
///   - 渲染 NER prompt
///   - 在 few-shot 段中检查每类实体类型英文枚举至少出现 1 次
#[test]
fn test_ner_prompt_few_shot_covers_6_entity_types() {
    let context = PromptContext {
        chunk: "测试文本".to_string(),
        entity_types: vec![
            "PERSON".to_string(),
            "LOCATION".to_string(),
            "ORGANIZATION".to_string(),
            "TIME".to_string(),
            "NUMBER".to_string(),
            "EVENT".to_string(),
        ],
    };
    let ner = NerPrompt::new();
    let rendered = ner.render(&context);

    // 6 类实体英文枚举（与 schema.rs::ENTITY_TYPES 前 6 项一致）
    let expected_types = ["PERSON", "LOCATION", "ORGANIZATION", "TIME", "NUMBER", "EVENT"];

    for ty in expected_types.iter() {
        assert!(
            rendered.contains(ty),
            "NER prompt few-shot 未覆盖实体类型：{}\n完整 prompt:\n{}",
            ty,
            rendered
        );
    }
}

/// 测试 4：render(&context) 替换 `{chunk}` 占位符
///
/// 步骤：
///   - 构造 chunk = "<<UNIQUE_CHUNK_MARKER_7Q9X>>"
///   - 调用 render 后应包含该 chunk 值，且不再包含 `{chunk}` 占位符
#[test]
fn test_ner_prompt_render_with_chunk_substitutes_placeholder() {
    let unique_chunk = "<<UNIQUE_CHUNK_MARKER_7Q9X>>";
    let context = PromptContext {
        chunk: unique_chunk.to_string(),
        entity_types: vec!["PERSON".to_string()],
    };
    let ner = NerPrompt::new();
    let rendered = ner.render(&context);

    assert!(
        rendered.contains(unique_chunk),
        "render 后的 prompt 应包含 chunk 值「{}」\n完整 prompt:\n{}",
        unique_chunk,
        rendered
    );
    assert!(
        !rendered.contains("{chunk}"),
        "render 后的 prompt 不应再包含 `{{chunk}}` 占位符\n完整 prompt:\n{}",
        rendered
    );
}

/// 测试 5：Extract prompt 含 7 段
///
/// 步骤：
///   - 构造 PromptContext
///   - 调用 `ExtractPrompt::render(&context)` 得到完整 prompt
///   - 按 7 个标题逐一断言存在
#[test]
fn test_extract_prompt_has_7_sections() {
    let context = PromptContext {
        chunk: "2025 年 5 月 18 日，OpenAI 发布 GPT-5".to_string(),
        entity_types: vec![
            "PERSON".to_string(),
            "ORGANIZATION".to_string(),
            "TIME".to_string(),
            "EVENT".to_string(),
        ],
    };
    let extract = ExtractPrompt::new();
    let rendered = extract.render(&context);

    for section in SEVEN_SECTIONS.iter() {
        assert!(
            rendered.contains(section),
            "Extract prompt 缺少段：「{}」\n完整 prompt:\n{}",
            section,
            rendered
        );
    }
}
