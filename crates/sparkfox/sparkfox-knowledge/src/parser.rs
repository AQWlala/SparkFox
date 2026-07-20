//! Sub-Step 10.2.3 — ResultParser（JSON 解析 + jieba 降级）
//!
//! ## 职责
//! 解析 LLM 输出为 `Vec<EventCandidate>`，提供 4 级降级链路（R-06 决策）：
//!
//! | 级别 | 路径 | 复用来源 | 触发条件 |
//! |------|------|----------|----------|
//! | 1 | JSON 直解（serde_json::from_str） | — | 输入是合法 JSON |
//! | 2 | JSON repair（repair_json） | Sub-Step 10.1.5 | JSON 含 trailing comma / markdown fence |
//! | 3 | 正则提取（subject + 谓词 + object） | — | 纯文本含"去/到/在/见/会"等谓词 |
//! | 4 | jieba NER 降级 | Sub-Step 10.6.1 | 上述全部失败，保证至少 1 个 EventCandidate |
//!
//! ## 空输出处理
//! 空字符串 / 仅空白字符直接返回空 `Vec`，不进入降级链（避免无意义 jieba 调用）。
//!
//! ## 与 10.2.2 的关系
//! 10.2.2 的 `LlmEventProcessor` 内部已有类似解析逻辑（`convert_response` /
//! `fallback_to_jieba`），但 10.2.3 不重构 `processor.rs`（避免回归），仅在
//! `parser.rs` 提供独立的 `ResultParser` 类型。后续 10.2.4 / W4 验收阶段可考虑
//! 让 `LlmEventProcessor` 内部调用 `ResultParser`（本次不做）。
//!
//! ## 设计参考
//! - `docs/SparkFox-v1.1.0-规划.md` Sub-Step 10.2.3
//! - R-06 决策：LLM 失败降级路径

#![forbid(unsafe_code)]

use crate::chunk::Chunk;
use crate::extractor::{EntityMention, EventCandidate};
use crate::jieba_ner::JiebaNer;
use regex::Regex;
use serde::Deserialize;
use sparkfox_core::Result;
use sparkfox_llm::repair_json;

// ---------------------------------------------------------------------------
// LLM 响应反序列化结构（与 10.2.2 processor.rs 独立定义，避免跨模块依赖）
// ---------------------------------------------------------------------------

/// LLM 响应根结构（`{"events": [...]}`）
#[derive(Debug, Deserialize)]
struct LlmResponse {
    #[serde(default)]
    events: Vec<LlmEvent>,
}

/// LLM 响应中的单个事件
#[derive(Debug, Deserialize)]
struct LlmEvent {
    #[serde(default)]
    title: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    entities: Vec<LlmEntity>,
}

/// LLM 响应中的单个实体（注意：JSON 字段名为 `type`，Rust 字段名为 `entity_type`）
#[derive(Debug, Deserialize)]
struct LlmEntity {
    #[serde(rename = "type")]
    entity_type: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    start: usize,
    #[serde(default)]
    end: usize,
}

// ---------------------------------------------------------------------------
// ResultParser — 结果解析器（4 级降级链路，R-06 决策）
// ---------------------------------------------------------------------------

/// 结果解析器（4 级降级链路，R-06 决策）
///
/// ## 职责
/// 解析 LLM 输出为 `Vec<EventCandidate>`，按以下顺序尝试 4 级降级：
///
/// 1. **JSON 直解**（`serde_json::from_str`）：最快路径，合法 JSON 直接解析
/// 2. **JSON repair**（`sparkfox_llm::repair_json`，复用 10.1.5）：修复 trailing
///    comma / markdown fence / 未引号键等国产模型常见格式错误
/// 3. **正则提取**：从纯文本中提取 subject + 谓词（去/到/在/见/会）+ object，
///    构造至少 1 个 `EventCandidate`
/// 4. **jieba NER 降级**（`JiebaNer`，复用 10.6.1）：保证至少 1 个
///    `EventCandidate`（entities 由 jieba 识别）
///
/// ## 空输出处理
/// 空字符串 / 仅空白字符直接返回空 `Vec`，不进入降级链（避免无意义 jieba 调用）。
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::parser::ResultParser;
/// let parser = ResultParser::new();
/// let events = parser.parse(&llm_output, &chunk)?;
/// ```
pub struct ResultParser {
    /// jieba+规则降级 NER（R-06 第 4 级降级）
    jieba: JiebaNer,
}

impl ResultParser {
    /// 创建 ResultParser，内部初始化 `JiebaNer` 实例（用于第 4 级降级）
    pub fn new() -> Self {
        Self {
            jieba: JiebaNer::new(),
        }
    }

    /// 解析 LLM 输出为 `Vec<EventCandidate>`
    ///
    /// ## 降级链路（R-06）
    /// 1. JSON 直解（`serde_json::from_str`）
    /// 2. JSON repair（`sparkfox_llm::repair_json`，复用 10.1.5）
    /// 3. 正则提取（从纯文本中提取 subject + 谓词 + object）
    /// 4. jieba NER 降级（复用 10.6.1 `JiebaNer`，保证至少 1 个 `EventCandidate`）
    ///
    /// ## 空输出处理
    /// 空字符串 / 仅空白字符直接返回空 `Vec`，不进入降级链
    ///
    /// ## 返回
    /// - `Ok(Vec<EventCandidate>)`：解析成功（可能为空 Vec，如空输入）
    /// - `Err`：当前实现不会返回 Err（4 级降级保证总能返回 Ok）
    pub fn parse(&self, llm_output: &str, chunk: &Chunk) -> Result<Vec<EventCandidate>> {
        // 空输出处理：不进入降级链，直接返回空 Vec
        if llm_output.trim().is_empty() {
            return Ok(Vec::new());
        }

        // 第 1 级：JSON 直解（最快路径）
        if let Some(events) = parse_strict(llm_output) {
            return Ok(events);
        }

        // 第 2 级：JSON repair（复用 10.1.5 repair_json，修复 trailing comma / markdown fence）
        if let Some(events) = parse_with_repair(llm_output) {
            return Ok(events);
        }

        // 第 3 级：正则提取（从纯文本中提取 subject + 谓词 + object）
        let regex_events = parse_with_regex(llm_output, chunk);
        if !regex_events.is_empty() {
            return Ok(regex_events);
        }

        // 第 4 级：jieba NER 降级（复用 10.6.1 JiebaNer，保证至少 1 个 EventCandidate）
        Ok(parse_with_jieba(chunk, &self.jieba))
    }
}

impl Default for ResultParser {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 第 1 级：JSON 直解（serde_json::from_str）
// ---------------------------------------------------------------------------

/// 第 1 级降级：直接用 `serde_json::from_str` 解析
///
/// 适用于：LLM 输出是严格的合法 JSON（无 trailing comma / 无 markdown fence）。
///
/// ## 返回
/// - `Some(Vec<EventCandidate>)`：解析成功
/// - `None`：解析失败，调用方应尝试第 2 级降级
fn parse_strict(llm_output: &str) -> Option<Vec<EventCandidate>> {
    serde_json::from_str::<LlmResponse>(llm_output)
        .ok()
        .map(convert_response)
}

// ---------------------------------------------------------------------------
// 第 2 级：JSON repair（复用 10.1.5 repair_json）
// ---------------------------------------------------------------------------

/// 第 2 级降级：调用 `sparkfox_llm::repair_json` 修复后再解析
///
/// 适用于：JSON 含 trailing comma / markdown fence（```json ... ```）/
/// 未引号键 / 单引号等国产模型常见格式错误。
///
/// ## 复用来源
/// Sub-Step 10.1.5 的 `sparkfox_llm::repair_json`（内部使用 `jsonrepair` crate，
/// 默认开启 `fenced_code_blocks` 可自动剥离 markdown 代码块包装）。
///
/// ## 返回
/// - `Some(Vec<EventCandidate>)`：修复后解析成功
/// - `None`：修复后仍无法解析为 `LlmResponse`，调用方应尝试第 3 级降级
fn parse_with_repair(llm_output: &str) -> Option<Vec<EventCandidate>> {
    repair_json(llm_output)
        .ok()
        .and_then(|value| serde_json::from_value::<LlmResponse>(value).ok())
        .map(convert_response)
}

// ---------------------------------------------------------------------------
// 第 3 级：正则提取（从纯文本中提取 subject + 谓词 + object）
// ---------------------------------------------------------------------------

/// 第 3 级降级：正则提取 subject + 谓词 + object
///
/// 适用于：LLM 输出是纯文本（非 JSON），但含中文动作谓词（去/到/在/见/会）。
///
/// ## 正则规则
/// `(.{1,20})(?:去|到|在|见|会)(.{1,20})`
/// - Group 1（subject）：谓词前 1-20 字符
/// - 谓词：去 / 到 / 在 / 见 / 会
/// - Group 2（object）：谓词后 1-20 字符
///
/// ## 构造 EventCandidate
/// - `title`：subject + object（如 "张三昨天北京出差"）
/// - `summary` / `content`：chunk 内容前 50 字符 / chunk 完整内容
/// - `category`："其他"（兜底分类）
/// - `keywords`：[subject, object]
/// - `entities`：空（正则不识别实体，留给第 4 级 jieba）
///
/// ## 返回
/// - `Vec<EventCandidate>`：匹配成功时含 1 条；匹配失败时为空 Vec
fn parse_with_regex(llm_output: &str, chunk: &Chunk) -> Vec<EventCandidate> {
    let re = match Regex::new(r"(.{1,20})(?:去|到|在|见|会)(.{1,20})") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    if let Some(caps) = re.captures(llm_output) {
        let subject = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let object = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let title = format!("{}{}", subject, object);
        let summary: String = chunk.content.chars().take(50).collect();

        vec![EventCandidate {
            title,
            summary,
            content: chunk.content.clone(),
            category: Some("其他".to_string()),
            keywords: vec![subject, object],
            entities: Vec::new(),
        }]
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// 第 4 级：jieba NER 降级（复用 10.6.1 JiebaNer）
// ---------------------------------------------------------------------------

/// 第 4 级降级：jieba NER 降级（R-06 决策，保证至少 1 个 EventCandidate）
///
/// 适用于：上述 3 级全部失败（LLM 输出无法解析 + 正则也提取不到）。
///
/// ## 复用来源
/// Sub-Step 10.6.1 的 `JiebaNer::extract`，识别 PERSON / ORGANIZATION /
/// LOCATION / TIME / NUMBER 5 类实体。
///
/// ## 构造 EventCandidate
/// - `title` / `summary`：chunk 内容前 50 字符
/// - `content`：chunk 完整内容
/// - `category`："其他"（兜底分类）
/// - `keywords`：jieba 识别的实体文本（最多 5 个）
/// - `entities`：jieba 识别的实体列表（转换为 `extractor::EntityMention`）
///
/// ## 注意
/// 即使 jieba 未识别到任何实体，也返回 1 个 EventCandidate（entities 为空 Vec），
/// 保证流程不中断（R-06 决策）。
fn parse_with_jieba(chunk: &Chunk, jieba: &JiebaNer) -> Vec<EventCandidate> {
    let jieba_entities = jieba.extract(&chunk.content);
    let title: String = chunk.content.chars().take(50).collect();
    let summary: String = chunk.content.chars().take(50).collect();
    let keywords: Vec<String> = jieba_entities
        .iter()
        .map(|e| e.text.clone())
        .take(5)
        .collect();
    let entities: Vec<EntityMention> = jieba_entities
        .iter()
        .map(|e| EntityMention {
            entity_type: e.entity_type.clone(),
            text: e.text.clone(),
            start: e.start,
            end: e.end,
        })
        .collect();

    vec![EventCandidate {
        title,
        summary,
        content: chunk.content.clone(),
        category: Some("其他".to_string()),
        keywords,
        entities,
    }]
}

// ---------------------------------------------------------------------------
// 内部辅助：LlmResponse → Vec<EventCandidate> 转换
// ---------------------------------------------------------------------------

/// 将 `LlmResponse` 转换为 `Vec<EventCandidate>`
///
/// 字段映射：
/// - `LlmEvent.title` → `EventCandidate.title`
/// - `LlmEvent.summary` → `EventCandidate.summary`
/// - `LlmEvent.content` → `EventCandidate.content`
/// - `LlmEvent.category` → `EventCandidate.category`（Option<String>）
/// - `LlmEvent.keywords` → `EventCandidate.keywords`
/// - `LlmEntity`（type/text/start/end）→ `EntityMention`（entity_type/text/start/end）
fn convert_response(resp: LlmResponse) -> Vec<EventCandidate> {
    resp.events
        .into_iter()
        .map(|e| EventCandidate {
            title: e.title,
            summary: e.summary,
            content: e.content,
            category: e.category,
            keywords: e.keywords,
            entities: e
                .entities
                .into_iter()
                .map(|en| EntityMention {
                    entity_type: en.entity_type,
                    text: en.text,
                    start: en.start,
                    end: en.end,
                })
                .collect(),
        })
        .collect()
}
