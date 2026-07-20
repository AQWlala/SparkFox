//! 文档分块器（spec 1.0 Task 3.2）
//!
//! ## 策略
//! v1.0.0 实现 **固定大小 + 滑动窗口 + 分隔符感知** 三层分块：
//! 1. 按分隔符（默认 `\n\n`）将文档切成段落
//! 2. 段落长度 ≤ `chunk_size`：整段作为一个 chunk
//! 3. 段落长度 > `chunk_size`：按 `chunk_size` 字符滑动窗口切分，窗口间 `overlap` 字符重叠
//! 4. 相邻短段落合并至接近 `chunk_size`（减少碎片）
//!
//! ## 默认参数
//! - `chunk_size = 500`（字符，非 token；v1.1.0+ 接入 tokenizer 分词）
//! - `overlap = 50`（字符）
//! - `separator = "\n\n"`（段落分隔符）
//!
//! ## 中文处理
//! 按字符（`char`）而非字节切分，避免 UTF-8 多字节字符被截断。
//! v1.1.0+ 将接入 `tokenizers::Tokenizer`（bge-small-zh-v1.5）按 token 分块。
//!
//! ## 设计参考
//! - RAGFlow DeepDoc chunking 思路（Apache-2.0，见 NOTICE）
//! - LangChain RecursiveCharacterTextSplitter 思路

/// 分块结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// 分块 ID（格式：`{doc_id}#{idx}`）
    pub id: String,
    /// 分块文本
    pub content: String,
    /// 在原文中的字符起始偏移（含）
    pub start_offset: usize,
    /// 在原文中的字符结束偏移（不含）
    pub end_offset: usize,
    /// 分块元数据
    pub metadata: ChunkMetadata,
}

/// 分块元数据
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChunkMetadata {
    /// 所属文档 ID
    pub doc_id: String,
    /// 分块序号（从 0 开始）
    pub index: usize,
    /// 分块字符数
    pub char_count: usize,
}

/// 文档分块器
///
/// 线程安全（不可变结构），可在多线程间共享。
#[derive(Debug, Clone)]
pub struct Chunker {
    /// 单块最大字符数
    chunk_size: usize,
    /// 滑动窗口重叠字符数
    overlap: usize,
    /// 段落分隔符（默认 `\n\n`）
    separator: String,
}

impl Default for Chunker {
    fn default() -> Self {
        Self {
            chunk_size: 500,
            overlap: 50,
            separator: "\n\n".to_string(),
        }
    }
}

impl Chunker {
    /// 创建默认配置的分块器（500 字符 + 50 重叠 + `\n\n` 分隔符）
    pub fn new() -> Self {
        Self::default()
    }

    /// 自定义配置
    ///
    /// - `chunk_size` 必须 > 0
    /// - `overlap` 必须 < `chunk_size`（否则会无限循环）
    pub fn with_config(chunk_size: usize, overlap: usize, separator: impl Into<String>) -> Self {
        assert!(chunk_size > 0, "chunk_size 必须 > 0");
        assert!(
            overlap < chunk_size,
            "overlap 必须 < chunk_size（否则会无限循环）"
        );
        Self {
            chunk_size,
            overlap,
            separator: separator.into(),
        }
    }

    /// 分块大小（字符数）
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// 重叠大小（字符数）
    pub fn overlap(&self) -> usize {
        self.overlap
    }

    /// 段落分隔符
    pub fn separator(&self) -> &str {
        &self.separator
    }

    /// 对文档文本执行分块
    ///
    /// 返回的 [`Chunk`] 列表按 `start_offset` 升序排列。
    /// 每个分块的 `id` 格式为 `{doc_id}#{idx}`，其中 `idx` 从 0 开始。
    ///
    /// 空文档返回空列表。
    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        self.chunk_with_doc_id(text, "doc")
    }

    /// 带文档 ID 的分块（用于关联 [`crate::Document`])
    pub fn chunk_with_doc_id(&self, text: &str, doc_id: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();

        // 步骤 1：按分隔符切段落（记录每段在原文的字符偏移）
        let paragraphs = self.split_paragraphs(&chars);

        // 步骤 2：段落合并 + 长段落滑窗切分
        let mut chunks = Vec::new();
        let mut idx = 0usize;

        // 合并缓冲：累积短段落直到接近 chunk_size
        let mut buf_chars: Vec<char> = Vec::new();
        let mut buf_start: Option<usize> = None;

        for para in &paragraphs {
            let para_chars = &para.chars;
            let para_len = para_chars.len();

            if para_len > self.chunk_size {
                // 先 flush 缓冲区
                if !buf_chars.is_empty() {
                    self.flush_buffer(
                        &mut chunks,
                        &mut buf_chars,
                        &mut buf_start,
                        &mut idx,
                        doc_id,
                    );
                }

                // 长段落：滑动窗口切分
                let mut start = 0usize;
                while start < para_len {
                    let end = (start + self.chunk_size).min(para_len);
                    let content: String = para_chars[start..end].iter().collect();
                    let abs_start = para.start + start;
                    let abs_end = para.start + end;
                    chunks.push(Chunk {
                        id: format!("{doc_id}#{idx}"),
                        content,
                        start_offset: abs_start,
                        end_offset: abs_end,
                        metadata: ChunkMetadata {
                            doc_id: doc_id.to_string(),
                            index: idx,
                            char_count: abs_end - abs_start,
                        },
                    });
                    idx += 1;
                    if end >= para_len {
                        break;
                    }
                    start = end.saturating_sub(self.overlap);
                    // 防御：若 chunk_size <= overlap（不应发生，构造时已断言），避免死循环
                    if start == 0 && end < self.chunk_size {
                        break;
                    }
                }
            } else {
                // 短段落：尝试合并
                let would_be = buf_chars.len() + para_len;
                if buf_chars.is_empty() {
                    buf_chars.extend_from_slice(para_chars);
                    buf_start = Some(para.start);
                } else if would_be <= self.chunk_size {
                    // 合并：加分隔符
                    buf_chars.extend(self.separator.chars());
                    buf_chars.extend_from_slice(para_chars);
                } else {
                    // 缓冲区已满，先 flush 再开始新缓冲
                    self.flush_buffer(
                        &mut chunks,
                        &mut buf_chars,
                        &mut buf_start,
                        &mut idx,
                        doc_id,
                    );
                    buf_chars.extend_from_slice(para_chars);
                    buf_start = Some(para.start);
                }
            }
        }
        // flush 残留缓冲
        self.flush_buffer(&mut chunks, &mut buf_chars, &mut buf_start, &mut idx, doc_id);

        // 防御：确保不超过原文长度（边界情况）
        debug_assert!(
            chunks.iter().all(|c| c.end_offset <= total),
            "分块 end_offset 超过原文长度"
        );
        chunks
    }

    /// flush 合并缓冲区为一个 chunk
    fn flush_buffer(
        &self,
        chunks: &mut Vec<Chunk>,
        buf: &mut Vec<char>,
        buf_start: &mut Option<usize>,
        idx: &mut usize,
        doc_id: &str,
    ) {
        if buf.is_empty() {
            return;
        }
        let start = buf_start.unwrap_or(0);
        let end = start + buf.len();
        let content: String = buf.iter().collect();
        chunks.push(Chunk {
            id: format!("{doc_id}#{idx}"),
            content,
            start_offset: start,
            end_offset: end,
            metadata: ChunkMetadata {
                doc_id: doc_id.to_string(),
                index: *idx,
                char_count: buf.len(),
            },
        });
        *idx += 1;
        buf.clear();
        *buf_start = None;
    }

    /// 按分隔符切段落，记录每段的字符偏移
    fn split_paragraphs(&self, chars: &[char]) -> Vec<Paragraph> {
        let sep_chars: Vec<char> = self.separator.chars().collect();
        let sep_len = sep_chars.len();
        let mut paragraphs = Vec::new();
        let mut start = 0usize;
        let mut i = 0usize;

        while i + sep_len <= chars.len() {
            if &chars[i..i + sep_len] == sep_chars.as_slice() {
                // 找到分隔符，记录 [start, i) 段
                if i > start {
                    let para_chars: Vec<char> = chars[start..i].to_vec();
                    paragraphs.push(Paragraph {
                        chars: para_chars,
                        start,
                    });
                }
                i += sep_len;
                start = i;
            } else {
                i += 1;
            }
        }
        // 尾段
        if start < chars.len() {
            let para_chars: Vec<char> = chars[start..].to_vec();
            paragraphs.push(Paragraph {
                chars: para_chars,
                start,
            });
        }
        // 全文无分隔符 → 整段
        if paragraphs.is_empty() && !chars.is_empty() {
            paragraphs.push(Paragraph {
                chars: chars.to_vec(),
                start: 0,
            });
        }
        paragraphs
    }
}

/// 段落（内部结构）
struct Paragraph {
    chars: Vec<char>,
    start: usize,
}

impl Chunker {
    /// v1.0.0 占位：语义分块（基于嵌入的边界检测）
    ///
    /// v1.1.0+ 将实现：对相邻段落计算嵌入余弦相似度，在相似度骤降处切分。
    /// 当前返回与 [`chunk`](Self::chunk) 相同的结果。
    pub fn semantic_chunk(&self, text: &str) -> Vec<Chunk> {
        // v1.0.0：退化为固定大小分块
        self.chunk(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let chunker = Chunker::new();
        let chunks = chunker.chunk("");
        assert!(chunks.is_empty(), "空文本应返回空分块列表");
    }

    #[test]
    fn test_short_text_single_chunk() {
        let chunker = Chunker::new();
        let text = "这是一段短文本，不足以触发分块。";
        let chunks = chunker.chunk(text);
        assert_eq!(chunks.len(), 1, "短文本应为单个分块");
        assert_eq!(chunks[0].content, text);
        assert_eq!(chunks[0].start_offset, 0);
        assert_eq!(chunks[0].end_offset, text.chars().count());
        assert_eq!(chunks[0].metadata.index, 0);
    }

    #[test]
    fn test_long_text_multiple_chunks() {
        let chunker = Chunker::with_config(10, 2, "\n\n");
        // 30 字符无分隔符 → 长段落滑窗切分
        let text = "abcdefghijklmnopqrstuvwxyz0123"; // 30 chars
        let chunks = chunker.chunk(text);
        assert!(chunks.len() > 1, "长文本应分多块，实际 {} 块", chunks.len());

        // 第一块应为 [0, 10)
        assert_eq!(chunks[0].start_offset, 0);
        assert_eq!(chunks[0].end_offset, 10);
        assert_eq!(chunks[0].content.chars().count(), 10);

        // 第二块应为 [8, 18)（overlap=2 → start = 10-2=8）
        assert_eq!(chunks[1].start_offset, 8);
        assert_eq!(chunks[1].end_offset, 18);

        // 所有块应覆盖全文
        let last = chunks.last().unwrap();
        assert_eq!(last.end_offset, 30, "最后一块应覆盖到文末");
    }

    #[test]
    fn test_paragraph_separator() {
        let chunker = Chunker::with_config(100, 10, "\n\n");
        let text = "第一段内容。\n\n第二段内容。\n\n第三段内容。";
        let chunks = chunker.chunk(text);
        // 三个短段落应合并为一个 chunk（总长 < 100）
        assert_eq!(
            chunks.len(),
            1,
            "三个短段落应合并为一个 chunk，实际 {} 块",
            chunks.len()
        );
        assert!(chunks[0].content.contains("第一段"));
        assert!(chunks[0].content.contains("第二段"));
        assert!(chunks[0].content.contains("第三段"));
    }

    #[test]
    fn test_chunk_id_format() {
        let chunker = Chunker::with_config(5, 1, "\n\n");
        let text = "abcdefghij"; // 10 chars → 2+ chunks
        let chunks = chunker.chunk_with_doc_id(text, "doc_001");
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].id, "doc_001#0");
        assert_eq!(chunks[1].id, "doc_001#1");
        assert_eq!(chunks[0].metadata.doc_id, "doc_001");
    }

    #[test]
    fn test_offset_monotonic() {
        let chunker = Chunker::with_config(20, 5, "\n\n");
        let text: String = std::iter::repeat('x').take(100).collect();
        let chunks = chunker.chunk(&text);
        for w in chunks.windows(2) {
            assert!(
                w[0].start_offset < w[1].start_offset,
                "分块 start_offset 应单调递增"
            );
        }
    }

    #[test]
    fn test_no_utf8_split() {
        let chunker = Chunker::with_config(3, 1, "\n\n");
        let text = "你好世界测试"; // 6 个中文字符
        let chunks = chunker.chunk(text);
        // 每个块的内容都应是有效的 UTF-8（String 保证）
        for c in &chunks {
            assert!(c.content.chars().count() <= 3 || c.content.chars().count() == 0);
        }
    }

    #[test]
    fn test_chunk_size_accessors() {
        let chunker = Chunker::with_config(256, 32, "\n");
        assert_eq!(chunker.chunk_size(), 256);
        assert_eq!(chunker.overlap(), 32);
        assert_eq!(chunker.separator(), "\n");
    }

    #[test]
    #[should_panic(expected = "chunk_size 必须 > 0")]
    fn test_zero_chunk_size_panics() {
        let _ = Chunker::with_config(0, 0, "\n\n");
    }

    #[test]
    #[should_panic(expected = "overlap 必须 < chunk_size")]
    fn test_overlap_ge_chunk_size_panics() {
        let _ = Chunker::with_config(10, 10, "\n\n");
    }
}
