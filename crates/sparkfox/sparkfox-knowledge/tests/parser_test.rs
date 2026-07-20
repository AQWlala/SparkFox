//! Sub-Step 10.2.3 — ResultParser 测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标
//! 1. 合法 JSON（含 events + entities 数组）解析为 Vec<EventCandidate>
//! 2. JSON 含 trailing comma → repair_json 修复后解析成功
//! 3. LLM 输出含 ```json markdown 代码块 → 剥离 fence 后解析
//! 4. 纯文本（非 JSON）→ 正则提取 subject/predicate/object
//! 5. 纯文本 + 正则无法提取 → jieba NER 降级
//! 6. 空字符串 / 仅空白 → 返回空 Vec（不报错）
//!
//! ## TDD-RED 说明
//! 本测试在 GREEN 实现前应全部失败（parser 模块尚未创建，编译错误）。
//!
//! ## 降级链路 R-06
//! JSON 直解 → JSON repair（10.1.5）→ 正则提取 → jieba NER（10.6.1）

#![forbid(unsafe_code)]

use sparkfox_knowledge::chunk::{Chunk, ChunkMetadata};
use sparkfox_knowledge::parser::ResultParser;

// ---------------------------------------------------------------------------
// 测试辅助：构造 Chunk
// ---------------------------------------------------------------------------

/// 构造测试用 Chunk（id 固定为 "test-chunk-id"，start_offset=0）
fn make_chunk(content: &str) -> Chunk {
    let char_count = content.chars().count();
    Chunk {
        id: "test-chunk-id".to_string(),
        content: content.to_string(),
        start_offset: 0,
        end_offset: char_count,
        metadata: ChunkMetadata {
            doc_id: "test-doc".to_string(),
            index: 0,
            char_count,
        },
    }
}

// ---------------------------------------------------------------------------
// 测试 1：合法 JSON 解析为 Vec<EventCandidate>，验证字段映射
// ---------------------------------------------------------------------------

/// 验证 ResultParser 解析合法 JSON（含 events 数组 + entities 数组），
/// 返回的 Vec<EventCandidate> 字段映射正确（title/summary/content/category/
/// keywords/entities 全部对齐）。
#[test]
fn test_parser_parses_valid_llm_json_output() {
    let json = r#"{"events":[{"title":"张三出差","summary":"张三昨天去北京出差","content":"张三昨天去北京出差，会见了李四","category":"出差","keywords":["出差","北京"],"entities":[{"type":"PERSON","text":"张三","start":0,"end":2},{"type":"LOCATION","text":"北京","start":5,"end":7}]}]}"#;
    let parser = ResultParser::new();
    let chunk = make_chunk("张三昨天去北京出差，会见了李四");
    let events = parser
        .parse(json, &chunk)
        .expect("合法 JSON 应解析成功（第 1 级：JSON 直解）");

    assert_eq!(events.len(), 1, "应返回 1 条 EventCandidate");
    let e = &events[0];
    assert_eq!(e.title, "张三出差");
    assert_eq!(e.summary, "张三昨天去北京出差");
    assert_eq!(e.content, "张三昨天去北京出差，会见了李四");
    assert_eq!(e.category.as_deref(), Some("出差"));
    assert_eq!(e.keywords, vec!["出差".to_string(), "北京".to_string()]);
    assert_eq!(e.entities.len(), 2, "应有 2 个实体");
    assert_eq!(e.entities[0].entity_type, "PERSON");
    assert_eq!(e.entities[0].text, "张三");
    assert_eq!(e.entities[0].start, 0);
    assert_eq!(e.entities[0].end, 2);
    assert_eq!(e.entities[1].entity_type, "LOCATION");
    assert_eq!(e.entities[1].text, "北京");
    assert_eq!(e.entities[1].start, 5);
    assert_eq!(e.entities[1].end, 7);
}

// ---------------------------------------------------------------------------
// 测试 2：trailing comma JSON 经 repair_json 修复后解析
// ---------------------------------------------------------------------------

/// 验证 JSON 含 trailing comma（数组尾逗号 + 对象尾逗号）时，
/// 第 1 级 JSON 直解失败 → 第 2 级调用 `sparkfox_llm::repair_json` 修复后解析成功。
///
/// 国产模型常见错误：`["关键词",]` / `{"end":2,}`
#[test]
fn test_parser_repairs_trailing_comma_json() {
    let json = r#"{"events":[{"title":"事件","summary":"摘要","content":"内容","category":"其他","keywords":["关键词",],"entities":[{"type":"PERSON","text":"张三","start":0,"end":2,}]}]}"#;
    let parser = ResultParser::new();
    let chunk = make_chunk("张三做了某事");
    let events = parser
        .parse(json, &chunk)
        .expect("第 2 级 repair_json 应修复 trailing comma");

    assert_eq!(events.len(), 1, "应返回 1 条 EventCandidate");
    assert_eq!(events[0].title, "事件");
    assert_eq!(events[0].entities.len(), 1, "应有 1 个实体");
    assert_eq!(events[0].entities[0].text, "张三");
    assert_eq!(events[0].entities[0].entity_type, "PERSON");
}

// ---------------------------------------------------------------------------
// 测试 3：markdown fence 剥离后解析
// ---------------------------------------------------------------------------

/// 验证 LLM 输出含 ```json\n{...}\n``` markdown 代码块包装时，
/// 第 1 级直解失败 → 第 2 级 repair_json（内部开启 fenced_code_blocks）剥离 fence 后解析成功。
#[test]
fn test_parser_extracts_json_from_markdown_fence() {
    let json = r#"```json
{"events":[{"title":"事件","summary":"摘要","content":"内容","category":"其他","keywords":[],"entities":[]}]}
```"#;
    let parser = ResultParser::new();
    let chunk = make_chunk("测试内容");
    let events = parser
        .parse(json, &chunk)
        .expect("应剥离 markdown fence 后解析成功");

    assert_eq!(events.len(), 1, "应返回 1 条 EventCandidate");
    assert_eq!(events[0].title, "事件");
    assert_eq!(events[0].summary, "摘要");
}

// ---------------------------------------------------------------------------
// 测试 4：纯文本 → 正则降级提取 subject/predicate/object
// ---------------------------------------------------------------------------

/// 验证 JSON 完全无法解析（如纯文本"张三昨天去北京出差"）时，
/// 第 1/2 级 JSON 解析失败 → 第 3 级正则提取（匹配 subject + 谓词 + object）
/// 构造至少 1 个 EventCandidate。
#[test]
fn test_parser_fallback_regex_extraction_on_invalid_json() {
    // 纯文本，非 JSON，含 "去" 谓词 → 第 3 级正则提取
    let llm_output = "张三昨天去北京出差";
    let parser = ResultParser::new();
    let chunk = make_chunk("张三昨天去北京出差");
    let events = parser
        .parse(llm_output, &chunk)
        .expect("第 3 级正则降级不应报错");

    assert!(
        !events.is_empty(),
        "正则提取应返回至少 1 个 EventCandidate"
    );
    // 验证 title 含 subject（张三）
    assert!(
        events[0].title.contains("张三"),
        "title 应含 subject '张三'，实际: {}",
        events[0].title
    );
    // 验证 title 含 object（北京）
    assert!(
        events[0].title.contains("北京"),
        "title 应含 object '北京'，实际: {}",
        events[0].title
    );
}

// ---------------------------------------------------------------------------
// 测试 5：正则也无法提取 → jieba NER 降级
// ---------------------------------------------------------------------------

/// 验证 JSON 完全无法解析 + 正则也提取不到（无谓词 去/到/在/见/会）时，
/// 第 4 级降级到 jieba NER，返回至少 1 个 EventCandidate（entities 由 jieba 识别）。
///
/// 注意：正则应用于 llm_output（无谓词），jieba 应用于 chunk.content（含可识别实体）。
#[test]
fn test_parser_fallback_jieba_on_complete_parse_failure() {
    // llm_output 无谓词（去/到/在/见/会），正则无法提取
    let llm_output = "无意义文本数据";
    let parser = ResultParser::new();
    // chunk 含 jieba 默认词典可识别的实体（张三=PERSON / 北京=LOCATION / 腾讯=ORGANIZATION）
    let chunk = make_chunk("张三在北京加入腾讯");
    let events = parser
        .parse(llm_output, &chunk)
        .expect("第 4 级 jieba 降级不应报错");

    assert!(
        !events.is_empty(),
        "jieba 降级应返回至少 1 个 EventCandidate"
    );
    // 验证 entities 由 jieba 识别（至少 1 个）
    let total_entities: usize = events.iter().map(|e| e.entities.len()).sum();
    assert!(
        total_entities >= 1,
        "jieba 应识别至少 1 个实体，实际 {} 个",
        total_entities
    );
    // 验证实体类型在 jieba 可识别范围内
    let entity_types: Vec<&str> = events
        .iter()
        .flat_map(|e| e.entities.iter())
        .map(|en| en.entity_type.as_str())
        .collect();
    assert!(
        entity_types
            .iter()
            .any(|t| matches!(*t, "PERSON" | "LOCATION" | "ORGANIZATION" | "TIME" | "NUMBER")),
        "jieba 应识别 PERSON/LOCATION/ORGANIZATION/TIME/NUMBER 之一，实际: {:?}",
        entity_types
    );
}

// ---------------------------------------------------------------------------
// 测试 6：空输出处理（空字符串 / 仅空白 → 空 Vec）
// ---------------------------------------------------------------------------

/// 验证空字符串 / 仅空白字符时，parse 直接返回空 Vec，不报错，不进入降级链
///（避免无意义 jieba 调用）。
#[test]
fn test_parser_handles_empty_llm_output() {
    let parser = ResultParser::new();
    let chunk = make_chunk("测试内容");

    // 空字符串
    let events = parser.parse("", &chunk).expect("空字符串不应报错");
    assert!(events.is_empty(), "空字符串应返回空 Vec");

    // 仅空白字符
    let events = parser
        .parse("   \n\t  ", &chunk)
        .expect("仅空白字符不应报错");
    assert!(events.is_empty(), "仅空白字符应返回空 Vec");
}
