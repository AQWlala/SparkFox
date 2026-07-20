//! 思考过程可视化 — BaiLongma 思考块清洁室重写
//!
//! 设计思路（仅借鉴功能，未拷贝代码）：
//! - 模型可在消息中插入 `<think stage="...">...</think>` 标记，承载思考过程。
//! - `extract_thinking_blocks` 解析消息文本，提取所有思考块及其在原文中的偏移。
//! - `render_thinking` 将单个思考块渲染为可显示文本。
//!
//! 标记格式（清洁室自定，非 BaiLongma 私有协议）：
//!   <think>...</think>                       // 默认 Reasoning 阶段
//!   <think stage="analyzing">...</think>     // 指定阶段
//!   <think stage="ANALYZING">...</think>     // 大小写不敏感

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 聊天消息中的思考过程块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    /// 块唯一 ID
    pub id: String,
    /// 思考内容（已剥离外层标记）
    pub content: String,
    /// 思考阶段
    pub stage: ThinkingStage,
    /// 在消息中的起始字节偏移（指向 `<`）
    pub start_offset: usize,
    /// 在消息中的结束字节偏移（指向 `</think>` 后下一个字符）
    pub end_offset: usize,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 思考阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingStage {
    /// 分析问题
    Analyzing,
    /// 检索知识
    Retrieving,
    /// 推理中
    Reasoning,
    /// 组织答案
    Composing,
    /// 验证答案
    Verifying,
}

impl ThinkingStage {
    /// 中文标签
    pub fn label(self) -> &'static str {
        match self {
            ThinkingStage::Analyzing => "分析问题",
            ThinkingStage::Retrieving => "检索知识",
            ThinkingStage::Reasoning => "推理中",
            ThinkingStage::Composing => "组织答案",
            ThinkingStage::Verifying => "验证答案",
        }
    }

    /// 从字符串解析阶段（大小写不敏感），未识别时返回 None
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "analyzing" | "analyze" | "analysis" => Some(ThinkingStage::Analyzing),
            "retrieving" | "retrieve" | "retrieval" => Some(ThinkingStage::Retrieving),
            "reasoning" | "reason" => Some(ThinkingStage::Reasoning),
            "composing" | "compose" | "composition" => Some(ThinkingStage::Composing),
            "verifying" | "verify" | "verification" => Some(ThinkingStage::Verifying),
            _ => None,
        }
    }
}

/// 开标记前缀
const OPEN_PREFIX: &str = "<think";
/// 开标记后缀
const OPEN_SUFFIX: &str = ">";
/// 闭标记
const CLOSE_TAG: &str = "</think>";

/// 从消息中提取所有思考块
///
/// 解析规则：
/// - 寻找 `<think ...>...</think>` 配对
/// - `stage="xxx"` 属性可选，缺省为 `Reasoning`
/// - 未能闭合的开标记会被跳过
/// - 偏移以字节为单位（与 Rust 字符串索引一致）
pub fn extract_thinking_blocks(content: &str) -> Vec<ThinkingBlock> {
    let mut blocks = Vec::new();
    let bytes = content.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        // 定位下一个开标记
        let Some(open_rel) = content[cursor..].find(OPEN_PREFIX) else {
            break;
        };
        let open_start = cursor + open_rel;
        let after_prefix = open_start + OPEN_PREFIX.len();

        // 寻找回开标记的 `>`
        let Some(gt_rel) = content[after_prefix..].find(OPEN_SUFFIX) else {
            break;
        };
        let open_end = after_prefix + gt_rel; // 指向 `>`
        let attr_slice = &content[after_prefix..open_end];

        // 寻找闭标记
        let body_start = open_end + OPEN_SUFFIX.len();
        let Some(close_rel) = content[body_start..].find(CLOSE_TAG) else {
            // 未闭合，跳过这个开标记继续扫描
            cursor = body_start;
            continue;
        };
        let close_start = body_start + close_rel;
        let close_end = close_start + CLOSE_TAG.len();

        // 解析 stage 属性（如 stage="reasoning"）
        let stage = parse_stage_attr(attr_slice).unwrap_or(ThinkingStage::Reasoning);

        let block = ThinkingBlock {
            id: format!("think_{}", uuid::Uuid::new_v4().simple()),
            content: content[body_start..close_start].to_string(),
            stage,
            start_offset: open_start,
            end_offset: close_end,
            timestamp: chrono::Utc::now(),
        };
        blocks.push(block);

        cursor = close_end;
    }

    blocks
}

/// 从 `<think ...>` 的属性区解析 stage
fn parse_stage_attr(attrs: &str) -> Option<ThinkingStage> {
    let lower = attrs.to_ascii_lowercase();
    let key = "stage=\"";
    let Some(start) = lower.find(key) else {
        return None;
    };
    let value_start = start + key.len();
    let Some(end) = lower[value_start..].find('"') else {
        return None;
    };
    let value = &attrs[value_start..value_start + end];
    ThinkingStage::from_str_loose(value)
}

/// 渲染思考块为可显示文本
///
/// 格式：`[阶段标签] 内容`
pub fn render_thinking(block: &ThinkingBlock) -> String {
    format!("[{}] {}", block.stage.label(), block.content.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_single_default_stage() {
        let content = "前文 <think>正在推理</think> 后文";
        let blocks = extract_thinking_blocks(content);
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.content, "正在推理");
        assert_eq!(b.stage, ThinkingStage::Reasoning);
        assert_eq!(&content[b.start_offset..b.end_offset], "<think>正在推理</think>");
    }

    #[test]
    fn extract_with_stage_attr() {
        let content = r#"<think stage="analyzing">分析问题</think>"#;
        let blocks = extract_thinking_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].stage, ThinkingStage::Analyzing);
        assert_eq!(blocks[0].content, "分析问题");
    }

    #[test]
    fn extract_multiple_blocks() {
        let content = "<think>第一段</think> 中间 <think stage=\"verifying\">第二段</think>";
        let blocks = extract_thinking_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].stage, ThinkingStage::Reasoning);
        assert_eq!(blocks[1].stage, ThinkingStage::Verifying);
    }

    #[test]
    fn extract_unclosed_skipped() {
        // 没有任何闭标记 → 完全跳过
        let content = "<think>未闭合 没有结束标记";
        assert!(extract_thinking_blocks(content).is_empty());
    }

    #[test]
    fn extract_greedy_match_inner_tag_as_content() {
        // 当前实现采用就近匹配：外层 <think> 与最近的 </think> 配对，
        // 内层 <think> 文本作为外层内容的一部分。
        let content = "<think>外层 <think>内层</think>";
        let blocks = extract_thinking_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "外层 <think>内层");
    }

    #[test]
    fn extract_none_when_no_tag() {
        let content = "普通消息，无思考块";
        assert!(extract_thinking_blocks(content).is_empty());
    }

    #[test]
    fn render_format() {
        let block = ThinkingBlock {
            id: "think_x".into(),
            content: "  hello  ".into(),
            stage: ThinkingStage::Composing,
            start_offset: 0,
            end_offset: 10,
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(render_thinking(&block), "[组织答案] hello");
    }

    #[test]
    fn stage_from_str_loose_case_insensitive() {
        assert_eq!(ThinkingStage::from_str_loose("REASONING"), Some(ThinkingStage::Reasoning));
        assert_eq!(ThinkingStage::from_str_loose("  Verify "), Some(ThinkingStage::Verifying));
        assert_eq!(ThinkingStage::from_str_loose("unknown"), None);
    }

    #[test]
    fn block_serialization_roundtrip() {
        let block = ThinkingBlock {
            id: "think_abc".into(),
            content: "内容".into(),
            stage: ThinkingStage::Retrieving,
            start_offset: 5,
            end_offset: 20,
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&block).unwrap();
        let back: ThinkingBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, block.id);
        assert_eq!(back.stage, block.stage);
        assert_eq!(back.start_offset, block.start_offset);
    }
}
