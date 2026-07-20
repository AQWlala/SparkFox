//! 引用协议（spec 1.0 Task 3.6）
//!
//! ## 协议格式
//! LLM 响应中的引用标记：`[citation:{kdoc_id}:{chunk_id}:{span_start}:{span_end}]`
//!
//! ChatView 解析后渲染为 `CitationChip`（React 组件），点击跳转到原文对应位置。
//!
//! ## 数据流
//! 1. [`RagEngine`](crate::rag::RagEngine) 检索返回 [`SearchHit`] 列表
//! 2. LLM 生成回答时，在引用处插入 [`Citation::to_marker`] 标记
//! 3. ChatView 用 [`parse_markers`](parse_markers) 解析标记 → [`CitationSpan`] 列表
//! 4. [`inject_citations`] 将标记替换为可视化引用编号（如 `[1]`、`[2]`）
//!
//! ## 设计参考
//! - AnythingLLM citation 设计思路（MIT，见 NOTICE）
//! - FastGPT 引用块设计思路（仅算法公式，不抄代码）

use serde::{Deserialize, Serialize};

/// 引用标记前缀
pub const CITATION_MARKER_PREFIX: &str = "[citation:";
/// 引用标记后缀
pub const CITATION_MARKER_SUFFIX: &str = "]";

/// 引用来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CitationSource {
    /// 向量召回
    Vector,
    /// 关键词召回
    Keyword,
    /// RRF 融合
    Rrf,
}

impl CitationSource {
    /// 序列化为单字符标识（用于 marker）
    pub fn as_char(&self) -> char {
        match self {
            Self::Vector => 'V',
            Self::Keyword => 'K',
            Self::Rrf => 'R',
        }
    }

    /// 反序列化
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'V' => Some(Self::Vector),
            'K' => Some(Self::Keyword),
            'R' => Some(Self::Rrf),
            _ => None,
        }
    }
}

/// 引用 — 指向知识库中某个文档的某个分块
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    /// 文档句柄（格式：`kdoc_{base64}`，见 SAG schema）
    pub kdoc_id: String,
    /// 分块 ID（格式：`{doc_id}#{idx}`，见 [`crate::chunk::Chunk`])
    pub chunk_id: String,
    /// 块内字符起始偏移（含）
    pub span_start: usize,
    /// 块内字符结束偏移（不含）
    pub span_end: usize,
    /// 检索分数（越大越相关）
    pub score: f32,
    /// 命中来源
    pub source: CitationSource,
    /// 可选页码（PDF 等分页文档）
    pub page: Option<usize>,
}

impl Citation {
    /// 生成 LLM 响应中的引用标记
    ///
    /// 格式：`[citation:{kdoc_id}:{chunk_id}:{span_start}:{span_end}]`
    pub fn to_marker(&self) -> String {
        format!(
            "{CITATION_MARKER_PREFIX}{}:{}:{}:{}{CITATION_MARKER_SUFFIX}",
            self.kdoc_id, self.chunk_id, self.span_start, self.span_end,
        )
    }

    /// 从标记解析（ChatView 用）
    ///
    /// 返回 `None` 表示格式不合法。
    /// 注意：marker 不含 `score` / `source` / `page`，解析后这些字段为默认值。
    pub fn parse_marker(marker: &str) -> Option<Self> {
        let inner = marker
            .strip_prefix(CITATION_MARKER_PREFIX)?
            .strip_suffix(CITATION_MARKER_SUFFIX)?;
        let parts: Vec<&str> = inner.split(':').collect();
        if parts.len() != 4 {
            return None;
        }
        Some(Self {
            kdoc_id: parts[0].to_string(),
            chunk_id: parts[1].to_string(),
            span_start: parts[2].parse().ok()?,
            span_end: parts[3].parse().ok()?,
            score: 0.0,
            source: CitationSource::Vector,
            page: None,
        })
    }

    /// 带来源信息的扩展标记格式（v1.0.0 增强）
    ///
    /// 格式：`[citation:{kdoc_id}:{chunk_id}:{span_start}:{span_end}:{source_char}:{score}]`
    pub fn to_marker_ext(&self) -> String {
        format!(
            "{CITATION_MARKER_PREFIX}{}:{}:{}:{}:{}:{:.4}{CITATION_MARKER_SUFFIX}",
            self.kdoc_id,
            self.chunk_id,
            self.span_start,
            self.span_end,
            self.source.as_char(),
            self.score,
        )
    }

    /// 解析扩展标记
    pub fn parse_marker_ext(marker: &str) -> Option<Self> {
        let inner = marker
            .strip_prefix(CITATION_MARKER_PREFIX)?
            .strip_suffix(CITATION_MARKER_SUFFIX)?;
        let parts: Vec<&str> = inner.split(':').collect();
        if parts.len() != 6 {
            return None;
        }
        let source_char = parts[4].chars().next()?;
        Some(Self {
            kdoc_id: parts[0].to_string(),
            chunk_id: parts[1].to_string(),
            span_start: parts[2].parse().ok()?,
            span_end: parts[3].parse().ok()?,
            source: CitationSource::from_char(source_char)?,
            score: parts[5].parse().ok()?,
            page: None,
        })
    }
}

/// 引用跨度 — 标记在 LLM 响应文本中的位置
#[derive(Debug, Clone, PartialEq)]
pub struct CitationSpan {
    /// 在响应文本中的字符起始偏移（含）
    pub start: usize,
    /// 在响应文本中的字符结束偏移（不含）
    pub end: usize,
    /// 引用内容
    pub citation: Citation,
}

/// 引用编号注入结果
#[derive(Debug, Clone, PartialEq)]
pub struct InjectedCitation {
    /// 注入的引用编号（从 1 开始）
    pub index: usize,
    /// 引用内容
    pub citation: Citation,
}

/// 引用注入结果
#[derive(Debug, Clone)]
pub struct InjectionResult {
    /// 注入编号后的文本（`[citation:...]` → `[1]`、`[2]`...）
    pub text: String,
    /// 注入的引用列表（按编号排序）
    pub citations: Vec<InjectedCitation>,
}

/// 将文本中的 `[citation:...]` 标记替换为 `[1]`、`[2]`... 编号
///
/// 同一 `kdoc_id:chunk_id` 的多次引用使用相同编号。
/// 返回 [`InjectionResult`]（含替换后文本与引用列表）。
///
/// ## 示例
/// ```ignore
/// use sparkfox_knowledge::{inject_citations, Citation};
/// let text = "Rust 是系统级语言[citation:kdoc_1:doc1#0:0:10]。";
/// let result = inject_citations(text);
/// assert_eq!(result.text, "Rust 是系统级语言[1]。");
/// assert_eq!(result.citations.len(), 1);
/// ```
pub fn inject_citations(text: &str) -> InjectionResult {
    let spans = find_citation_markers(text);
    if spans.is_empty() {
        return InjectionResult {
            text: text.to_string(),
            citations: Vec::new(),
        };
    }

    // 按 (kdoc_id, chunk_id) 去重分配编号
    let mut id_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut next_index = 1usize;
    let mut injected = Vec::with_capacity(spans.len());

    // 从后往前替换（避免偏移失效）
    let chars: Vec<char> = text.chars().collect();
    let mut result_chars = chars.clone();
    let mut offset_adj: i64 = 0; // 替换造成的偏移调整

    for span in &spans {
        // 计算去重编号
        let key = format!("{}:{}", span.citation.kdoc_id, span.citation.chunk_id);
        let index = *id_map.entry(key).or_insert_with(|| {
            let i = next_index;
            next_index += 1;
            i
        });

        injected.push(InjectedCitation {
            index,
            citation: span.citation.clone(),
        });

        // 替换 [citation:...] 为 [index]
        let replacement = format!("[{index}]");
        let rep_chars: Vec<char> = replacement.chars().collect();

        // 计算实际偏移（考虑之前的替换调整）
        let actual_start = (span.start as i64 + offset_adj) as usize;
        let actual_end = (span.end as i64 + offset_adj) as usize;

        result_chars.splice(actual_start..actual_end, rep_chars.iter().cloned());
        let old_len = span.end - span.start;
        let new_len = rep_chars.len();
        offset_adj += new_len as i64 - old_len as i64;
    }

    let text: String = result_chars.into_iter().collect();
    InjectionResult {
        text,
        citations: injected,
    }
}

/// 查找文本中所有 `[citation:...]` 标记
///
/// 返回按 `start` 升序排列的 [`CitationSpan`] 列表。
/// 支持标准 4 字段格式与扩展 6 字段格式。
pub fn find_citation_markers(text: &str) -> Vec<CitationSpan> {
    let chars: Vec<char> = text.chars().collect();
    let prefix: Vec<char> = CITATION_MARKER_PREFIX.chars().collect();
    let suffix: Vec<char> = CITATION_MARKER_SUFFIX.chars().collect();
    let mut spans = Vec::new();

    let mut i = 0usize;
    while i < chars.len() {
        // 查找前缀
        if i + prefix.len() <= chars.len() && chars[i..i + prefix.len()] == prefix[..] {
            // 查找对应后缀
            let mut j = i + prefix.len();
            while j < chars.len() {
                if j + suffix.len() <= chars.len() && chars[j..j + suffix.len()] == suffix[..] {
                    break;
                }
                j += 1;
            }
            if j < chars.len() {
                // 提取 marker（含前缀与后缀）
                let marker: String = chars[i..j + suffix.len()].iter().collect();

                // 优先尝试扩展格式（6 字段），退化为标准格式（4 字段）
                let citation = Citation::parse_marker_ext(&marker)
                    .or_else(|| Citation::parse_marker(&marker));
                if let Some(citation) = citation {
                    spans.push(CitationSpan {
                        start: i,
                        end: j + suffix.len(),
                        citation,
                    });
                }
                i = j + suffix.len();
                continue;
            }
        }
        i += 1;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_citation() -> Citation {
        Citation {
            kdoc_id: "kdoc_abc123".to_string(),
            chunk_id: "doc1#0".to_string(),
            span_start: 10,
            span_end: 50,
            score: 0.95,
            source: CitationSource::Vector,
            page: Some(3),
        }
    }

    #[test]
    fn test_marker_roundtrip_standard() {
        let c = make_citation();
        let marker = c.to_marker();
        assert!(marker.starts_with("[citation:"));
        assert!(marker.ends_with(']'));
        let parsed = Citation::parse_marker(&marker).expect("解析失败");
        assert_eq!(parsed.kdoc_id, c.kdoc_id);
        assert_eq!(parsed.chunk_id, c.chunk_id);
        assert_eq!(parsed.span_start, c.span_start);
        assert_eq!(parsed.span_end, c.span_end);
        // 标准格式不含 score/source/page
        assert_eq!(parsed.score, 0.0);
        assert_eq!(parsed.source, CitationSource::Vector);
    }

    #[test]
    fn test_marker_roundtrip_extended() {
        let c = make_citation();
        let marker = c.to_marker_ext();
        let parsed = Citation::parse_marker_ext(&marker).expect("解析失败");
        assert_eq!(parsed.kdoc_id, c.kdoc_id);
        assert_eq!(parsed.chunk_id, c.chunk_id);
        assert_eq!(parsed.span_start, c.span_start);
        assert_eq!(parsed.span_end, c.span_end);
        assert!((parsed.score - c.score).abs() < 1e-3);
        assert_eq!(parsed.source, c.source);
    }

    #[test]
    fn test_parse_invalid_marker() {
        assert!(Citation::parse_marker("not a marker").is_none());
        assert!(Citation::parse_marker("[citation:only:two:fields]").is_none());
        assert!(Citation::parse_marker("[citation:a:b:not_a_number:d]").is_none());
        assert!(Citation::parse_marker("[nope:a:b:0:1]").is_none());
    }

    #[test]
    fn test_citation_source_char_roundtrip() {
        for s in [CitationSource::Vector, CitationSource::Keyword, CitationSource::Rrf] {
            let c = s.as_char();
            assert_eq!(CitationSource::from_char(c), Some(s));
        }
        assert_eq!(CitationSource::from_char('X'), None);
    }

    #[test]
    fn test_find_citation_markers() {
        let text = "前文[citation:kdoc_1:doc1#0:0:10]中文字[citation:kdoc_2:doc2#1:5:20]后文";
        let spans = find_citation_markers(text);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].citation.kdoc_id, "kdoc_1");
        assert_eq!(spans[1].citation.kdoc_id, "kdoc_2");
        assert!(spans[0].start < spans[1].start);
    }

    #[test]
    fn test_inject_citations_basic() {
        let text = "Rust 是系统级语言[citation:kdoc_1:doc1#0:0:10]。性能优秀。";
        let result = inject_citations(text);
        assert_eq!(result.text, "Rust 是系统级语言[1]。性能优秀。");
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].index, 1);
        assert_eq!(result.citations[0].citation.kdoc_id, "kdoc_1");
    }

    #[test]
    fn test_inject_citations_dedup() {
        let text = "[citation:kdoc_1:doc1#0:0:10] 重复引用 [citation:kdoc_1:doc1#0:0:10]";
        let result = inject_citations(text);
        assert_eq!(result.text, "[1] 重复引用 [1]");
        assert_eq!(result.citations.len(), 2);
        // 两处引用编号都为 1（去重）
        assert_eq!(result.citations[0].index, 1);
        assert_eq!(result.citations[1].index, 1);
    }

    #[test]
    fn test_inject_citations_multiple() {
        let text = "A[citation:kdoc_1:doc1#0:0:10]B[citation:kdoc_2:doc2#0:0:20]C";
        let result = inject_citations(text);
        assert_eq!(result.text, "A[1]B[2]C");
        assert_eq!(result.citations.len(), 2);
        assert_eq!(result.citations[0].index, 1);
        assert_eq!(result.citations[1].index, 2);
    }

    #[test]
    fn test_inject_citations_no_markers() {
        let text = "纯文本无引用标记。";
        let result = inject_citations(text);
        assert_eq!(result.text, text);
        assert!(result.citations.is_empty());
    }

    #[test]
    fn test_inject_citations_extended_format() {
        // 扩展格式应正确解析并替换
        let text = "X[citation:kdoc_1:doc1#0:0:10:V:0.9500]Y";
        let result = inject_citations(text);
        assert_eq!(result.text, "X[1]Y");
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].citation.source, CitationSource::Vector);
        assert!((result.citations[0].citation.score - 0.95).abs() < 1e-3);
    }

    #[test]
    fn test_inject_citations_chinese() {
        // 中文场景：标记内不含中文，但前后文为中文
        let text = "根据文档[citation:kdoc_1:doc1#0:0:10]所述，Rust 性能优秀。";
        let result = inject_citations(text);
        assert!(result.text.contains("[1]"));
        assert!(!result.text.contains("[citation:"));
    }
}
